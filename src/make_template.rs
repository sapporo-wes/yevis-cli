use crate::{
    args::FileFormat,
    github_api::{
        get_file_list_recursive, get_user, raw_url_from_path, read_github_token, WfRepoInfo,
    },
    path_utils::{dir_path, file_stem},
    type_config::{
        Author, Config, File, FileType, Repo, TestFile, TestFileType, Testing, Workflow,
    },
    workflow_type_version::inspect_wf_type_version,
};
use anyhow::{anyhow, ensure, Result};
use regex::Regex;
use serde_json;
use serde_yaml;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;

pub fn make_template(
    workflow_location: &Url,
    arg_github_token: &Option<impl AsRef<str>>,
    output: impl AsRef<Path>,
    format: &FileFormat,
) -> Result<()> {
    let github_token = read_github_token(&arg_github_token)?;

    let wf_repo_info = WfRepoInfo::new(&github_token, &workflow_location)?;
    let github_user = get_user(&github_token)?;

    let wf_loc = raw_url_from_path(&wf_repo_info, &wf_repo_info.file_path)?;
    let wf_type_version = inspect_wf_type_version(&wf_loc)?;
    let wf_name = file_stem(&wf_repo_info.file_path)?;
    let wf_version = "1.0.0".to_string(); // TODO update
    let readme_url = raw_url_from_path(&wf_repo_info, "README.md")?;
    let files = obtain_wf_files(&github_token, &wf_repo_info)?;

    let template_config = Config {
        id: Uuid::new_v4(),
        version: wf_version,
        license: "CC0-1.0".to_string(),
        authors: vec![
            Author::new_from_github_user(&github_user),
            Author::new_ddbj(),
        ],
        workflow: Workflow {
            name: wf_name,
            repo: Repo::new(&wf_repo_info),
            readme: readme_url,
            language: wf_type_version,
            files,
            testing: vec![Testing {
                id: "test_1".to_string(),
                files: vec![
                    TestFile::new_file_template(TestFileType::WfParams)?,
                    TestFile::new_file_template(TestFileType::WfEngineParams)?,
                    TestFile::new_file_template(TestFileType::Other)?,
                ],
            }],
        },
    };

    let mut output_path_buf = output.as_ref().to_path_buf();
    let template_config_str = match &format {
        FileFormat::Json => {
            output_path_buf.set_extension("yml");
            serde_json::to_string_pretty(&template_config)?
        }
        FileFormat::Yaml => {
            output_path_buf.set_extension("yml");
            serde_yaml::to_string(&template_config)?
        }
    };
    let mut buffer = BufWriter::new(fs::File::create(&output_path_buf)?);
    buffer.write(template_config_str.as_bytes())?;

    Ok(())
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParseResult {
    pub owner: String,
    pub name: String,
    pub branch: Option<String>,
    pub commit_hash: Option<String>,
    pub file_path: PathBuf,
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
pub fn parse_wf_loc(wf_loc: &Url) -> Result<ParseResult> {
    let host = wf_loc.host_str().ok_or(anyhow!(
        "Failed to parse the host from the workflow location"
    ))?;
    ensure!(
        host == "github.com" || host == "raw.githubusercontent.com",
        "yevis is only supported on github.com and raw.githubusercontent.com"
    );
    let path_segments = wf_loc
        .path_segments()
        .ok_or(anyhow!(
            "Failed to parse path segments from the workflow location"
        ))?
        .collect::<Vec<_>>();
    let branch_or_commit_hash = if host == "github.com" {
        path_segments
            .get(3)
            .ok_or(anyhow!(
                "Failed to parse branch or commit hash from the workflow location"
            ))?
            .to_string()
    } else {
        path_segments
            .get(2)
            .ok_or(anyhow!(
                "Failed to parse branch or commit hash from the workflow location"
            ))?
            .to_string()
    };
    let is_commit_hash = is_commit_hash(&branch_or_commit_hash);
    let file_path = if host == "github.com" {
        PathBuf::from(path_segments[4..].join("/"))
    } else {
        PathBuf::from(path_segments[3..].join("/"))
    };
    Ok(ParseResult {
        owner: path_segments
            .get(0)
            .ok_or(anyhow!(
                "Failed to parse repo's owner from the workflow location"
            ))?
            .to_string(),
        name: path_segments
            .get(1)
            .ok_or(anyhow!(
                "Failed to parse repo's name from the workflow location"
            ))?
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
    ensure!(
        re.is_match(hash.as_ref()),
        "Not a valid commit hash: {}",
        hash.as_ref()
    );
    Ok(())
}

fn obtain_wf_files(github_token: impl AsRef<str>, wf_repo_info: &WfRepoInfo) -> Result<Vec<File>> {
    let base_dir = dir_path(&wf_repo_info.file_path)?;
    let files = get_file_list_recursive(
        &github_token,
        &wf_repo_info.owner,
        &wf_repo_info.name,
        &wf_repo_info.commit_hash,
        &base_dir,
    )?;
    Ok(files
        .into_iter()
        .map(|file| -> Result<File> {
            Ok(File::new_from_raw_url(
                &raw_url_from_path(&wf_repo_info, &file)?,
                &base_dir,
                if file == wf_repo_info.file_path {
                    FileType::Primary
                } else {
                    FileType::Secondary
                },
            )?)
        })
        .collect::<Result<Vec<File>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_make_template_cwl() {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_cwl.yml");
        let arg_github_token: Option<&str> = None;
        let result = make_template(
            &Url::parse(
                "https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl",
            )
            .unwrap(),
            &arg_github_token,
            &temp_file,
            &FileFormat::Yaml,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_make_template_wdl() {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_wdl.yml");
        let arg_github_token: Option<&str> = None;
        let result = make_template(
            &Url::parse(
                "https://github.com/ddbj/yevis-cli/blob/main/tests/WDL/wf/dockstore-tool-bamstats.wdl",
            )
            .unwrap(),
            &arg_github_token,
            &temp_file,
            &FileFormat::Yaml,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_make_template_nfl() {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_nfl.yml");
        let arg_github_token: Option<&str> = None;
        let result = make_template(
            &Url::parse("https://github.com/ddbj/yevis-cli/blob/main/tests/NFL/wf/file_input.nf")
                .unwrap(),
            &arg_github_token,
            &temp_file,
            &FileFormat::Yaml,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_make_template_smk() {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_smk.yml");
        let arg_github_token: Option<&str> = None;
        let result = make_template(
            &Url::parse("https://github.com/ddbj/yevis-cli/blob/main/tests/SMK/wf/Snakefile")
                .unwrap(),
            &arg_github_token,
            &temp_file,
            &FileFormat::Yaml,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_wf_loc() {
        let parse_result_1 = parse_wf_loc(
            &Url::parse("https://github.com/ddbj/yevis-cli/blob/main/path/to/workflow").unwrap(),
        )
        .unwrap();
        assert_eq!(
            parse_result_1,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: Some("main".to_string()),
                commit_hash: None,
                file_path: PathBuf::from("path/to/workflow"),
            },
        );
        let parse_result_2 = parse_wf_loc(&Url::parse("https://github.com/ddbj/yevis-cli/blob/752eab2a3b34f0c2fe4489a591303ded6906169d/path/to/workflow").unwrap()).unwrap();
        assert_eq!(
            parse_result_2,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: None,
                commit_hash: Some("752eab2a3b34f0c2fe4489a591303ded6906169d".to_string()),
                file_path: PathBuf::from("path/to/workflow"),
            },
        );
        let parse_result_3 = parse_wf_loc(
            &Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/main/path/to/workflow")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            parse_result_3,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: Some("main".to_string()),
                commit_hash: None,
                file_path: PathBuf::from("path/to/workflow"),
            },
        );
        let parse_result_4 = parse_wf_loc(&Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/752eab2a3b34f0c2fe4489a591303ded6906169d/path/to/workflow").unwrap()).unwrap();
        assert_eq!(
            parse_result_4,
            ParseResult {
                owner: "ddbj".to_string(),
                name: "yevis-cli".to_string(),
                branch: None,
                commit_hash: Some("752eab2a3b34f0c2fe4489a591303ded6906169d".to_string()),
                file_path: PathBuf::from("path/to/workflow"),
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
        let github_token = read_github_token(&arg_github_token).unwrap();
        let wf_loc =
            Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md").unwrap();
        let wf_repo_info = WfRepoInfo::new(&github_token, &wf_loc).unwrap();
        let result = obtain_wf_files(&github_token, &wf_repo_info).unwrap();
        let readme = result
            .iter()
            .find(|f| f.target == PathBuf::from("README.md"))
            .unwrap();
        assert_eq!(readme.r#type, FileType::Primary);
        let license = result
            .iter()
            .find(|f| f.target == PathBuf::from("LICENSE"))
            .unwrap();
        assert_eq!(license.r#type, FileType::Secondary);
    }
}
