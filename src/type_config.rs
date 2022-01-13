use crate::github_api;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub id: Uuid,
    pub version: String,
    pub license: String,
    pub authors: Vec<Author>,
    pub workflow: Workflow,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Author {
    pub github_account: String,
    pub name: String,
    pub affiliation: String,
    pub orcid: String,
}

impl Author {
    pub fn new_from_github_user_info(github_user_info: &github_api::GetUserResponse) -> Self {
        Self {
            github_account: github_user_info.login.clone(),
            name: github_user_info.name.clone(),
            affiliation: github_user_info.company.clone(),
            orcid: "".to_string(),
        }
    }

    pub fn new_ddbj() -> Self {
        Self {
            github_account: "ddbj".to_string(),
            name: "ddbj-workflow".to_string(),
            affiliation: "DNA Data Bank of Japan".to_string(),
            orcid: "DO NOT ENTER".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub repo: Repo,
    pub readme: Url,
    pub language: Language,
    pub files: Vec<File>,
    pub testing: Vec<Testing>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub owner: String,
    pub name: String,
    pub commit: String,
}

impl Repo {
    pub fn new(repo_info: &github_api::WfRepoInfo) -> Self {
        Self {
            owner: repo_info.owner.clone(),
            name: repo_info.name.clone(),
            commit: repo_info.commit_hash.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct License {
    pub label: String,
    pub file: Url,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LanguageType {
    Cwl,
    Wdl,
    Nfl,
    Smk,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Language {
    pub r#type: LanguageType,
    pub version: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FileType {
    Primary,
    Secondary,
    Test,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct File {
    pub url: Url,
    pub target: PathBuf,
    pub r#type: FileType,
}

impl File {
    pub fn new_test_file_template() -> Self {
        Self {
            url: Url::parse("https://github.com/ddbj/yevis-cli/path/to/test_file").unwrap(),
            target: PathBuf::from("path/to/test_file"),
            r#type: FileType::Test,
        }
    }

    pub fn new_from_raw_url(
        raw_url: &Url,
        base_dir: impl AsRef<Path>,
        r#type: FileType,
    ) -> Result<Self> {
        Ok(Self {
            url: raw_url.clone(),
            target: github_api::to_file_path(&raw_url)?
                .strip_prefix(base_dir.as_ref())?
                .to_path_buf(),
            r#type,
        })
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Testing {
    pub id: String,
    pub files: Vec<File>,
}
