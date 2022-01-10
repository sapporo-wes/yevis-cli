use crate::github_api;
use crate::type_config;
use crate::workflow_type_version;
use anyhow::{anyhow, bail, ensure, Result};
use regex::Regex;
use serde_json;
use serde_yaml;
use std::fs;
use std::path::Path;
use url::Url;

pub fn make_template(
    workflow_location: impl AsRef<str>,
    arg_github_token: &Option<impl AsRef<str>>,
    output: impl AsRef<Path>,
    format: impl AsRef<str>,
) -> Result<()> {
    let github_token = github_api::read_github_token(&arg_github_token)?;

    let wf_repo_info = obtain_wf_repo_info(&workflow_location, &github_token)?;
    let github_user_info = github_api::get_user(&github_token)?;

    let wf_loc = github_api::to_raw_url(
        &wf_repo_info.owner,
        &wf_repo_info.name,
        &wf_repo_info.commit_hash,
        &wf_repo_info.file_path,
    )?;
    let wf_type_version = workflow_type_version::inspect_wf_type_version(&wf_loc)?;
    let wf_name = Path::new(&wf_repo_info.file_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let wf_id = format!(
        "{}_{}_{}",
        &wf_repo_info.owner, &wf_repo_info.name, &wf_name
    );
    let wf_version = "1.0.0".to_string(); // TODO update
    let readme_url = github_api::to_raw_url(
        &wf_repo_info.owner,
        &wf_repo_info.name,
        &wf_repo_info.commit_hash,
        "README.md",
    )?;
    let license_url = github_api::to_raw_url(
        &wf_repo_info.owner,
        &wf_repo_info.name,
        &wf_repo_info.commit_hash,
        github_api::get_license_path(&github_token, &wf_repo_info.owner, &wf_repo_info.name)?,
    )?;
    let files = obtain_wf_files(&github_token, &wf_repo_info)?;

    let template_config = type_config::Config {
        id: wf_id,
        version: wf_version,
        authors: vec![
            type_config::Author::new_from_github_user_info(&github_user_info),
            type_config::Author::new_ddbj(),
        ],
        readme_url,
        license: wf_repo_info.license,
        license_url,
        workflow_name: wf_name,
        workflow_language: wf_type_version,
        files,
        testing: vec![type_config::Testing {
            id: "test_1".to_string(),
            files: vec![type_config::File::new_template()],
        }],
    };

    let template_config_str = match format.as_ref() {
        "json" => serde_json::to_string_pretty(&template_config)?,
        "yaml" => serde_yaml::to_string(&template_config)?,
        _ => bail!("unknown format: {}", format.as_ref()),
    };
    let mut output_path_buf = output.as_ref().to_path_buf();
    match format.as_ref() {
        "json" => {
            output_path_buf.set_extension("json");
        }
        "yaml" => {
            output_path_buf.set_extension("yml");
        }
        _ => bail!("unknown format: {}", format.as_ref()),
    };
    fs::write(output_path_buf, template_config_str)?;

    Ok(())
}

#[derive(Debug, PartialEq)]
struct WfRepoInfo {
    owner: String,
    name: String,
    license: String,
    commit_hash: String,
    file_path: String,
}

/// Obtain and organize information about the GitHub repository, where the main workflow is located.
fn obtain_wf_repo_info(
    workflow_location: impl AsRef<str>,
    github_token: impl AsRef<str>,
) -> Result<WfRepoInfo> {
    let parse_result = parse_wf_loc(&workflow_location)?;
    let get_repos_response =
        github_api::get_repos(&github_token, &parse_result.owner, &parse_result.name)?;
    ensure!(
        get_repos_response.private == false,
        format!(
            "Repo {}/{} is private",
            parse_result.owner, parse_result.name
        )
    );
    let license = match &get_repos_response.license {
        Some(license) => license.to_string(),
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
        None => get_repos_response.default_branch,
    };
    let commit_hash = match &parse_result.commit_hash {
        Some(commit_hash) => commit_hash.to_string(),
        None => github_api::get_latest_commit_hash(
            &github_token,
            &parse_result.owner,
            &parse_result.name,
            &branch,
        )?,
    };
    Ok(WfRepoInfo {
        owner: parse_result.owner,
        name: parse_result.name,
        license,
        commit_hash,
        file_path: parse_result.file_path,
    })
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

fn obtain_wf_files(
    github_token: impl AsRef<str>,
    wf_repo_info: &WfRepoInfo,
) -> Result<Vec<type_config::File>> {
    let dir_path = Path::new(&wf_repo_info.file_path)
        .parent()
        .ok_or(anyhow!(
            "Could not parse dir path from the workflow location"
        ))?
        .to_str()
        .ok_or(anyhow!(
            "Could not parse dir path from the workflow location"
        ))?;
    let files = github_api::get_file_list_recursive(
        &github_token,
        &wf_repo_info.owner,
        &wf_repo_info.name,
        &wf_repo_info.commit_hash,
        &dir_path,
    )?;
    Ok(files
        .into_iter()
        .map(|file| -> Result<type_config::File> {
            Ok(type_config::File::new_from_raw_url(
                &github_api::to_raw_url(
                    &wf_repo_info.owner,
                    &wf_repo_info.name,
                    &wf_repo_info.commit_hash,
                    &file,
                )?,
                if file == wf_repo_info.file_path {
                    "PRIMARY"
                } else {
                    "SECONDARY"
                },
            ))
        })
        .collect::<Result<Vec<type_config::File>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obtain_wf_repo_info() {
        let arg_github_token: Option<&str> = None;
        let github_token = github_api::read_github_token(&arg_github_token).unwrap();
        let wf_loc = "https://raw.githubusercontent.com/sapporo-wes/sapporo-service/main/tests/resources/cwltool/trimming_and_qc.cwl";
        let wf_repo_info = obtain_wf_repo_info(&wf_loc, &github_token).unwrap();
        assert_eq!(wf_repo_info.owner, "sapporo-wes");
        assert_eq!(wf_repo_info.name, "sapporo-service");
        assert_eq!(wf_repo_info.license, "Apache-2.0");
        is_commit_hash(&wf_repo_info.commit_hash).unwrap();
        assert_eq!(
            wf_repo_info.file_path,
            "tests/resources/cwltool/trimming_and_qc.cwl"
        );
    }

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

    #[test]
    fn test_is_commit_hash() {
        is_commit_hash("752eab2a3b34f0c2fe4489a591303ded6906169d").unwrap();
    }

    #[test]
    fn test_obtain_wf_files() {
        let arg_github_token: Option<&str> = None;
        let github_token = github_api::read_github_token(&arg_github_token).unwrap();
        let wf_loc = "https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md";
        let wf_repo_info = obtain_wf_repo_info(&wf_loc, &github_token).unwrap();
        let result = obtain_wf_files(&github_token, &wf_repo_info).unwrap();
        let readme = result.iter().find(|f| f.target == "README.md").unwrap();
        assert_eq!(readme.r#type, "PRIMARY");
        let license = result.iter().find(|f| f.target == "LICENSE").unwrap();
        assert_eq!(license.r#type, "SECONDARY");
    }
}
