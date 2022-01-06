#[derive(Debug, PartialEq)]
pub struct Config {
    pub id: String,
    pub authors: Vec<Author>,
    pub license: String,
    pub workflow_name: String,
    pub workflow_language: WorkflowLanguage,
    pub files: Vec<File>,
    pub testing: Vec<Testing>,
}

#[derive(Debug, PartialEq)]
pub struct Author {
    pub github_account: String,
    pub name: String,
    pub affiliation: String,
    pub orcid: String,
}

#[derive(Debug, PartialEq)]
pub struct WorkflowLanguage {
    pub r#type: String,
    pub version: String,
}

#[derive(Debug, PartialEq)]
pub struct File {
    pub url: String,
    pub target: String,
    pub r#type: String,
}

#[derive(Debug, PartialEq)]
pub struct Testing {
    pub id: String,
    pub files: Vec<File>,
}
