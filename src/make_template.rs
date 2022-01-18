use crate::{
    args::FileFormat,
    github_api::{
        get_file_list_recursive, get_latest_commit_hash, get_repos, get_user, raw_url_from_path,
        read_github_token, WfRepoInfo,
    },
    path_utils::{dir_path, file_stem},
    pull_request::parse_repo,
    remote::fetch_config,
    type_config::{
        Author, Config, File, FileType, Repo, TestFile, TestFileType, Testing, Workflow,
    },
    workflow_type_version::inspect_wf_type_version,
};
use anyhow::{anyhow, bail, ensure, Result};
use log::debug;
use log::info;
use regex::Regex;
use serde_json;
use serde_yaml;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::string::ToString;
use url::Url;
use uuid::Uuid;

pub fn make_template(
    workflow_location: &Url,
    arg_github_token: &Option<impl AsRef<str>>,
    repository: impl AsRef<str>,
    output: impl AsRef<Path>,
    format: &FileFormat,
    update: &Option<Uuid>,
) -> Result<()> {
    let github_token = read_github_token(&arg_github_token)?;
    ensure!(
        !github_token.is_empty(),
        "GitHub token is empty. Please set it with --github-token option or set GITHUB_TOKEN environment variable."
    );

    info!(
        "Making template config from workflow location: {}",
        workflow_location.as_str()
    );
    let wf_repo_info = WfRepoInfo::new(&github_token, &workflow_location)?;
    let github_user = get_user(&github_token)?;

    let (wf_id, wf_version, wf_name, authors, testing) = match *update {
        Some(wf_id) => {
            let (repo_owner, repo_name) = parse_repo(&repository)?;
            let latest_version =
                find_latest_version(&github_token, &repo_owner, &repo_name, &wf_id)?;
            let wf_version = latest_version.get_new_version().to_string();
            let config = fetch_config(
                &github_token,
                &repo_owner,
                &repo_name,
                &wf_id.to_string(),
                &latest_version.to_string(),
            )?;
            (
                wf_id,
                wf_version,
                config.workflow.name,
                config.authors,
                config.workflow.testing,
            )
        }
        None => {
            let wf_id = Uuid::new_v4();
            let wf_version = "1.0.0".to_string();
            let wf_name = file_stem(&wf_repo_info.file_path)?;
            let authors = vec![
                Author::new_from_github_user(&github_user),
                Author::new_ddbj(),
            ];
            let testing = vec![Testing {
                id: "test_1".to_string(),
                files: vec![
                    TestFile::new_file_template(TestFileType::WfParams)?,
                    TestFile::new_file_template(TestFileType::WfEngineParams)?,
                    TestFile::new_file_template(TestFileType::Other)?,
                ],
            }];
            (wf_id, wf_version, wf_name, authors, testing)
        }
    };
    let wf_loc = raw_url_from_path(&wf_repo_info, &wf_repo_info.file_path)?;
    let wf_type_version = inspect_wf_type_version(&wf_loc)?;
    let readme_url = raw_url_from_path(&wf_repo_info, "README.md")?;
    let files = obtain_wf_files(&github_token, &wf_repo_info)?;

    let template_config = Config {
        id: wf_id,
        version: wf_version,
        license: "CC0-1.0".to_string(),
        authors,
        workflow: Workflow {
            name: wf_name,
            repo: Repo::new(&wf_repo_info),
            readme: readme_url,
            language: wf_type_version,
            files,
            testing,
        },
    };
    debug!("template_config:\n{:#?}", template_config);

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
        format!("yevis-cli is only supported on `github.com` and `raw.githubusercontent.com` as the workflow location. Your inputted workflow location is: {}", wf_loc),
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

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Version {
    major: usize,
    minor: usize,
    patch: usize,
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut version_parts = s.split('.');
        let major = version_parts
            .next()
            .ok_or(anyhow!("Failed to parse major version"))?
            .parse()?;
        let minor = version_parts
            .next()
            .ok_or(anyhow!("Failed to parse minor version"))?
            .parse()?;
        let patch = version_parts
            .next()
            .ok_or(anyhow!("Failed to parse patch version"))?
            .parse()?;
        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => match self.patch.cmp(&other.patch) {
                    Ordering::Equal => Ordering::Equal,
                    ordering => ordering,
                },
                ordering => ordering,
            },
            ordering => ordering,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ToString for Version {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    fn get_new_version(&self) -> Version {
        Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
        }
    }
}

pub fn find_latest_version(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    wf_id: &Uuid,
) -> Result<Version> {
    let branch = get_repos(&github_token, &owner, &name)?.default_branch;
    let commit_hash = get_latest_commit_hash(&github_token, &owner, &name, &branch)?;
    let file_list = match get_file_list_recursive(
        &github_token,
        &owner,
        &name,
        &commit_hash,
        &wf_id.to_string(),
    ) {
        Ok(file_list) => file_list,
        Err(err) => {
            bail!(
                "{}. Does Workflow ID: {} exist in repository {}/{}",
                err,
                &wf_id,
                owner.as_ref(),
                name.as_ref()
            )
        }
    };
    // file like: yevis_config_1.0.0.yml
    let versions = file_list
        .iter()
        .map(|f| f.file_name().ok_or(anyhow!("Failed to get file name")))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .map(|f| f.to_str().ok_or(anyhow!("Failed to get file name")))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .map(|f| {
            f.split('_')
                .nth(2)
                .ok_or(anyhow!("Failed to get file name"))
        })
        .collect::<Result<Vec<_>>>()?
        .iter()
        .map(|f| Version::from_str(f))
        .collect::<Result<Vec<_>>>()?;
    let latest_version = versions
        .into_iter()
        .max()
        .ok_or(anyhow!("Failed to get latest version"))?;
    Ok(latest_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::default_ddbj_workflows;
    use std::env::temp_dir;

    #[test]
    fn test_make_template_cwl() -> Result<()> {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_cwl.yml");
        make_template(
            &Url::parse(
                "https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl",
            )?,
            &None::<String>,
            default_ddbj_workflows(),
            &temp_file,
            &FileFormat::Yaml,
            &None::<Uuid>,
        )?;
        Ok(())
    }

    #[test]
    fn test_make_template_wdl() -> Result<()> {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_wdl.yml");
        make_template(
            &Url::parse(
                "https://github.com/ddbj/yevis-cli/blob/main/tests/WDL/wf/dockstore-tool-bamstats.wdl",
            )
            ?,
            &None::<String>,
            default_ddbj_workflows(),
            &temp_file,
            &FileFormat::Yaml,
            &None::<Uuid>
        )?;
        Ok(())
    }

    #[test]
    fn test_make_template_nfl() -> Result<()> {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_nfl.yml");
        make_template(
            &Url::parse("https://github.com/ddbj/yevis-cli/blob/main/tests/NFL/wf/file_input.nf")?,
            &None::<String>,
            default_ddbj_workflows(),
            &temp_file,
            &FileFormat::Yaml,
            &None::<Uuid>,
        )?;
        Ok(())
    }

    #[test]
    fn test_make_template_smk() -> Result<()> {
        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("yevis_test_template_smk.yml");
        make_template(
            &Url::parse("https://github.com/ddbj/yevis-cli/blob/main/tests/SMK/wf/Snakefile")?,
            &None::<String>,
            default_ddbj_workflows(),
            &temp_file,
            &FileFormat::Yaml,
            &None::<Uuid>,
        )?;
        Ok(())
    }

    #[test]
    fn test_make_template_with_not_github() -> Result<()> {
        let wf_loc = Url::parse("https://example.com")?;
        let result = make_template(
            &wf_loc,
            &None::<String>,
            default_ddbj_workflows(),
            &PathBuf::from("yevis_config.yml"),
            &FileFormat::Yaml,
            &None::<Uuid>,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("yevis-cli is only supported on `github.com` and `raw.githubusercontent.com` as the workflow location."));
        Ok(())
    }

    #[test]
    fn test_make_template_with_invalid_github_token() -> Result<()> {
        let wf_loc = Url::parse(
            "https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl",
        )?;
        let arg_github_token: Option<&str> = Some("invalid_token");
        let result = make_template(
            &wf_loc,
            &arg_github_token,
            default_ddbj_workflows(),
            &PathBuf::from("yevis_config.yml"),
            &FileFormat::Yaml,
            &None::<Uuid>,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to authenticate with GitHub."));
        Ok(())
    }

    #[test]
    fn test_make_template_with_invalid_wf_loc() -> Result<()> {
        let wf_loc = Url::parse("https://github.com/ddbj/yevis-cli/blob/main/invalid_wf_loc")?;
        let result = make_template(
            &wf_loc,
            &None::<String>,
            default_ddbj_workflows(),
            &PathBuf::from("yevis_config.yml"),
            &FileFormat::Yaml,
            &None::<Uuid>,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to fetch contents from your inputted workflow location"));
        Ok(())
    }

    #[test]
    fn test_parse_wf_loc() -> Result<()> {
        let parse_result_1 = parse_wf_loc(&Url::parse(
            "https://github.com/ddbj/yevis-cli/blob/main/path/to/workflow",
        )?)?;
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
        let parse_result_2 = parse_wf_loc(&Url::parse("https://github.com/ddbj/yevis-cli/blob/752eab2a3b34f0c2fe4489a591303ded6906169d/path/to/workflow")?)?;
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
        let parse_result_3 = parse_wf_loc(&Url::parse(
            "https://raw.githubusercontent.com/ddbj/yevis-cli/main/path/to/workflow",
        )?)?;
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
        let parse_result_4 = parse_wf_loc(&Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/752eab2a3b34f0c2fe4489a591303ded6906169d/path/to/workflow")?)?;
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
        Ok(())
    }

    #[test]
    fn test_is_commit_hash() -> Result<()> {
        is_commit_hash("752eab2a3b34f0c2fe4489a591303ded6906169d")?;
        Ok(())
    }

    #[test]
    fn test_obtain_wf_files() -> Result<()> {
        let github_token = read_github_token(&None::<String>)?;
        let wf_loc = Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md")?;
        let wf_repo_info = WfRepoInfo::new(&github_token, &wf_loc)?;
        let result = obtain_wf_files(&github_token, &wf_repo_info)?;
        let readme = result
            .iter()
            .find(|f| f.target == PathBuf::from("README.md"))
            .ok_or(anyhow!("Failed to find README.md"))?;
        assert_eq!(readme.r#type, FileType::Primary);
        let license = result
            .iter()
            .find(|f| f.target == PathBuf::from("LICENSE"))
            .ok_or(anyhow!("Failed to find LICENSE"))?;
        assert_eq!(license.r#type, FileType::Secondary);
        Ok(())
    }

    #[test]
    fn test_version_cmp() -> Result<()> {
        let v1 = Version::from_str("1.0.0")?;
        let v2 = Version::from_str("1.0.1")?;
        let v3 = Version::from_str("0.1.0")?;
        assert!(v1 < v2);
        assert!(v1 > v3);
        assert!(v2 > v3);
        let latest_version = vec![v1, v2, v3].into_iter().max().unwrap();
        assert_eq!(latest_version, Version::from_str("1.0.1")?);
        Ok(())
    }
}
