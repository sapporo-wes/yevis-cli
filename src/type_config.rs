use crate::github_api;
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Config {
    pub id: String,
    pub version: String,
    pub authors: Vec<Author>,
    pub readme_url: String,
    pub license: String,
    pub license_url: String,
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
pub struct WorkflowLanguage {
    pub r#type: String,
    pub version: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct File {
    pub url: String,
    pub target: String,
    pub r#type: String,
}

impl File {
    pub fn new_template() -> Self {
        Self {
            url: "".to_string(),
            target: "".to_string(),
            r#type: "".to_string(),
        }
    }

    pub fn new_from_raw_url(raw_url: impl AsRef<str>, r#type: impl AsRef<str>) -> Self {
        Self {
            url: raw_url.as_ref().to_string(),
            target: github_api::to_file_path(&raw_url).unwrap().to_string(),
            r#type: r#type.as_ref().to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Testing {
    pub id: String,
    pub files: Vec<File>,
}
