use crate::{
    github_api::{to_file_path, GithubUser, WfRepoInfo},
    path_utils::file_name,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
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
    pub fn new_from_github_user(github_user: &GithubUser) -> Self {
        Self {
            github_account: github_user.login.clone(),
            name: github_user.name.clone(),
            affiliation: github_user.company.clone(),
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
    pub fn new(repo_info: &WfRepoInfo) -> Self {
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
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Primary,
    Secondary,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct File {
    pub url: Url,
    pub target: PathBuf,
    pub r#type: FileType,
}

impl File {
    pub fn new_from_raw_url(
        raw_url: &Url,
        base_dir: impl AsRef<Path>,
        r#type: FileType,
    ) -> Result<Self> {
        Ok(Self {
            url: raw_url.clone(),
            target: to_file_path(&raw_url)?
                .strip_prefix(base_dir.as_ref())?
                .to_path_buf(),
            r#type,
        })
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestFileType {
    WfParams,
    WfEngineParams,
    Other,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TestFile {
    pub url: Url,
    pub target: PathBuf,
    pub r#type: TestFileType,
}

impl TestFile {
    pub fn new_file_template(r#type: TestFileType) -> Result<Self> {
        let url = match &r#type {
            TestFileType::WfParams => Url::parse("https://example.com/path/to/wf_params.json")?,
            TestFileType::WfEngineParams => {
                Url::parse("https://example.com/path/to/wf_engine_params.json")?
            }
            TestFileType::Other => Url::parse("https://example.com/path/to/data.fq")?,
        };
        let target = PathBuf::from(file_name(url.path().trim_start_matches("/"))?);
        Ok(Self {
            url,
            target,
            r#type,
        })
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Testing {
    pub id: String,
    pub files: Vec<TestFile>,
}
