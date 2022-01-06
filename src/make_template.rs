// use crate::github_api;
use crate::github_api;
use crate::remote;
use crate::workflow_type_version;
use anyhow::{anyhow, bail, ensure, Result};
use regex::Regex;
use std::path::Path;
use url::Url;

pub fn make_template(
    workflow_location: impl AsRef<str>,
    output: impl AsRef<Path>,
    format: impl AsRef<str>,
) -> Result<()> {
    let parse_result = parse_wf_loc(&workflow_location)?;
    let get_repos_response = github_api::get_repos(&parse_result.owner, &parse_result.name)?;
    ensure!(
        get_repos_response.private == false,
        format!(
            "Repo {}/{} is private",
            parse_result.owner, parse_result.name
        )
    );
    let license = match &get_repos_response.license {
        Some(license) => license.spdx_id.clone(),
        None => {
            bail!(
                "No license found for repo {}/{}",
                parse_result.owner,
                parse_result.name
            );
        }
    };
    let branch = match &parse_result.branch {
        Some(branch) => branch.to_string(),
        None => get_repos_response.default_branch.clone(),
    };
    let commit_hash = match &parse_result.commit_hash {
        Some(commit_hash) => commit_hash.to_string(),
        None => {
            github_api::get_latest_commit_hash(&parse_result.owner, &parse_result.name, &branch)?
        }
    };
    let main_wf_loc = Url::parse(&format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        &parse_result.owner, &parse_result.name, &commit_hash, &parse_result.file_path
    ))?;
    let main_wf_content = remote::fetch_raw_content(&main_wf_loc)?;
    let main_wf_type = match workflow_type_version::inspect_wf_type(&main_wf_content) {
        Ok(wf_type) => wf_type,
        Err(_) => "CWL".to_string(),
    };
    let main_wf_version =
        match workflow_type_version::inspect_wf_version(&main_wf_content, &main_wf_type) {
            Ok(wf_version) => wf_version,
            Err(_) => "1.0".to_string(),
        };

    let template_config = Config {
        id: "".to_string(),
        workflow_name: "".to_string(),
        authors: vec![Author {
            github_account: "".to_string(),
            name: "".to_string(),
            affiliation: "".to_string(),
            orcid: "".to_string(),
        }],
        license: license,
        workflow_language: WorkflowLanguage {
            r#type: main_wf_type,
            version: main_wf_version,
        },
        files: vec![File {
            url: "".to_string(),
            target: "".to_string(),
            r#type: "".to_string(),
        }],
        testing: vec![Testing {
            id: "".to_string(),
            files: vec![File {
                url: "".to_string(),
                target: "".to_string(),
                r#type: "".to_string(),
            }],
        }],
    };

    Ok(())
}

#[derive(Debug, PartialEq)]
struct ParseResult {
    owner: String,
    name: String,
    branch: Option<String>,
    commit_hash: Option<String>,
    file_path: String,
}

/// Parse the workflow location.
/// The workflow location should be in the format of:
///
/// - https://github.com/<owner>/<name>/blob/<branch>/<path_to_file>
/// - https://github.com/<owner>/<name>/blob/<commit_hash>/<path_to_file>
/// - https://github.com/<owner>/<name>/tree/<branch>/<path_to_file>
/// - https://github.com/<owner>/<name>/tree/<commit_hash>/<path_to_file>
/// - https://github.com/<owner>/<name>/raw/<branch>/<path_to_file>
/// - https://github.com/<owner>/<name>/raw/<commit_hash>/<path_to_file>
/// - https://raw.githubusercontent.com/<owner>/<name>/<branch>/<path_to_file>
/// - https://raw.githubusercontent.com/<owner>/<name>/<commit_hash>/<path_to_file>
fn parse_wf_loc(wf_loc: impl AsRef<str>) -> Result<ParseResult> {
    let wf_loc_url = Url::parse(wf_loc.as_ref())?;
    let host = wf_loc_url
        .host_str()
        .ok_or(anyhow!("Could not parse host from the workflow location"))?;
    ensure!(
        host == "github.com" || host == "raw.githubusercontent.com",
        "yevis is only supported on github.com and raw.githubusercontent.com"
    );
    let path_segments = wf_loc_url
        .path_segments()
        .ok_or(anyhow!("Could not parse path segments"))?
        .collect::<Vec<_>>();
    let branch_or_commit_hash = if host == "github.com" {
        path_segments
            .get(3)
            .ok_or(anyhow!("Could not parse branch or commit hash"))?
            .to_string()
    } else {
        path_segments
            .get(2)
            .ok_or(anyhow!("Could not parse branch or commit hash"))?
            .to_string()
    };
    let is_commit_hash = is_commit_hash(&branch_or_commit_hash);
    let file_path = if host == "github.com" {
        path_segments[4..].join("/")
    } else {
        path_segments[3..].join("/")
    };
    Ok(ParseResult {
        owner: path_segments
            .get(0)
            .ok_or(anyhow!("Could not parse owner from the workflow location"))?
            .to_string(),
        name: path_segments
            .get(1)
            .ok_or(anyhow!("Could not parse name"))?
            .to_string(),
        branch: match &is_commit_hash {
            Ok(_) => None,
            Err(_) => Some(branch_or_commit_hash.clone()),
        },
        commit_hash: match &is_commit_hash {
            Ok(_) => Some(branch_or_commit_hash.clone()),
            Err(_) => None,
        },
        file_path: file_path,
    })
}

// Check if a str is in a 40 character git commit hash.
pub fn is_commit_hash(hash: impl AsRef<str>) -> Result<()> {
    let re = Regex::new(r"^[0-9a-f]{40}$")?;
    ensure!(re.is_match(hash.as_ref()), "Not a valid commit hash");
    Ok(())
}

#[derive(Debug, PartialEq)]
struct Config {
    id: String,
    workflow_name: String,
    authors: Vec<Author>,
    license: String,
    workflow_language: WorkflowLanguage,
    files: Vec<File>,
    testing: Vec<Testing>,
}

#[derive(Debug, PartialEq)]
struct Author {
    github_account: String,
    name: String,
    affiliation: String,
    orcid: String,
}

#[derive(Debug, PartialEq)]
struct WorkflowLanguage {
    r#type: String,
    version: String,
}

#[derive(Debug, PartialEq)]
struct File {
    url: String,
    target: String,
    r#type: String,
}

#[derive(Debug, PartialEq)]
struct Testing {
    id: String,
    files: Vec<File>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wf_loc() {
        let parse_result_1 =
            parse_wf_loc("https://github.com/ddbj/yevis-cli/blob/main/path/to/workflow").unwrap();
        assert_eq!(
            parse_result_1,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: Some("main".to_string()),
                commit_hash: None,
                file_path: "path/to/workflow".to_string(),
            },
        );
        let parse_result_2 = parse_wf_loc("https://github.com/ddbj/yevis-cli/blob/752eab2a3b34f0c2fe4489a591303ded6906169d/path/to/workflow").unwrap();
        assert_eq!(
            parse_result_2,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: None,
                commit_hash: Some("752eab2a3b34f0c2fe4489a591303ded6906169d".to_string()),
                file_path: "path/to/workflow".to_string(),
            },
        );
        let parse_result_3 =
            parse_wf_loc("https://raw.githubusercontent.com/ddbj/yevis-cli/main/path/to/workflow")
                .unwrap();
        assert_eq!(
            parse_result_3,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: Some("main".to_string()),
                commit_hash: None,
                file_path: "path/to/workflow".to_string(),
            },
        );
        let parse_result_4 = parse_wf_loc("https://raw.githubusercontent.com/ddbj/yevis-cli/752eab2a3b34f0c2fe4489a591303ded6906169d/path/to/workflow").unwrap();
        assert_eq!(
            parse_result_4,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: None,
                commit_hash: Some("752eab2a3b34f0c2fe4489a591303ded6906169d".to_string()),
                file_path: "path/to/workflow".to_string(),
            },
        );
    }
}
