use crate::github_api;
use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, PartialEq, Serialize)]
pub struct Config {
    pub id: String,
    pub version: String,
    pub authors: Vec<Author>,
    pub readme_url: Url,
    pub license: String,
    pub license_url: Url,
    pub workflow_name: String,
    pub workflow_language: WorkflowLanguage,
    pub files: Vec<File>,
    pub testing: Vec<Testing>,
}

#[derive(Debug, PartialEq, Serialize)]
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
            name: "DBCLS".to_string(),
            affiliation: "DBCLS (Database Center for Life Science)".to_string(),
            orcid: "".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum WorkflowLanguageType {
    Cwl,
    Wdl,
    Nfl,
    Smk,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct WorkflowLanguage {
    pub r#type: WorkflowLanguageType,
    pub version: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum FileType {
    Primary,
    Secondary,
    Test,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct File {
    pub url: Url,
    pub target: PathBuf,
    pub r#type: FileType,
}

impl File {
    pub fn new_test_file_template() -> Self {
        Self {
            url: Url::parse("https://example.com/path/to/test_file").unwrap(),
            target: PathBuf::from("path/to/test_file"),
            r#type: FileType::Test,
        }
    }

    pub fn new_from_raw_url(raw_url: &Url, r#type: FileType) -> Result<Self> {
        Ok(Self {
            url: raw_url.clone(),
            target: github_api::to_file_path(&raw_url)?,
            r#type,
        })
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Testing {
    pub id: String,
    pub files: Vec<File>,
}
