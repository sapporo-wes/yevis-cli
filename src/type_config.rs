use crate::github_api;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub id: String,
    pub version: String,
    pub license: String,
    pub authors: Vec<Author>,
    pub workflow: Workflow,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Author {
    github_account: String,
    name: String,
    affiliation: String,
    orcid: String,
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
            orcid: "".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub repo: Repo,
    pub readme: Url,
    pub language: Language,
    pub files: Vec<File>,
    pub testing: Vec<Testing>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Repo {
    pub owner: String,
    pub name: String,
    pub commit: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct License {
    pub label: String,
    pub file: Url,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LanguageType {
    Cwl,
    Wdl,
    Nfl,
    Smk,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Language {
    pub r#type: LanguageType,
    pub version: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FileType {
    Primary,
    Secondary,
    Test,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Testing {
    pub id: String,
    pub files: Vec<File>,
}
