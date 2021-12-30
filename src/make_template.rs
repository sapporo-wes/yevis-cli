use anyhow::{anyhow, ensure, Result};
use std::fmt;
use std::path::Path;
use url::Url;

pub fn make_template(
    workflow_location: impl AsRef<str>,
    output: impl AsRef<Path>,
    format: impl AsRef<str>,
) -> Result<()> {
    parse_wf_loc(&workflow_location);
    println!("make-template");
    Ok(())
}

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
/// - https://github.com/owner/repo/tree/branch/path/to/file
/// - https://github.com/owner/repo/blob/branch/path/to/file
/// - https://github.com/owner/repo/raw/branch/path/to/file
/// - https://github.com/owner/repo/blob/commit_hash/path/to/file
/// - https://raw.githubusercontent.com/owner/repo/branch/path/to/file
fn parse_wf_loc(wf_loc: impl AsRef<str>) -> Result<ParseResult> {
    let wf_loc_url = Url::parse(wf_loc.as_ref())?;
    let host = wf_loc_url
        .host_str()
        .ok_or(anyhow!("Could not parse host from the workflow location"))?;
    ensure!(
        host == "github.com" || host == "raw.githubusercontent.com",
        "yevis is only supported on github.com and raw.githubusercontent.com"
    );
    Ok(())
}

// fn check_host_url(wf_loc: impl AsRef<str>) {}

struct Config {
    id: String,
    workflow_name: String,
    authors: Vec<Author>,
    license: String,
    workflow_language: WorkflowLanguage,
    files: Vec<File>,
    testing: Vec<Testing>,
}

struct Author {
    github_account: String,
    name: String,
    affiliation: String,
    ORCID: String,
}

enum WorkflowLanguageType {
    CWL,
    WDL,
    NFL,
    SMK,
}

impl fmt::Display for WorkflowLanguageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WorkflowLanguageType::CWL => write!(f, "CWL"),
            WorkflowLanguageType::WDL => write!(f, "WDL"),
            WorkflowLanguageType::NFL => write!(f, "NFL"),
            WorkflowLanguageType::SMK => write!(f, "SMK"),
        }
    }
}

struct WorkflowLanguage {
    r#type: String,
    version: String,
}

struct File {
    url: String,
    target: String,
    r#type: String,
}

struct Testing {
    id: String,
    files: Vec<File>,
}
