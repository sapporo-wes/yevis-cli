use anyhow::{ensure, Result};
use reqwest;
use serde::Deserialize;

pub fn get_repos(owner: impl AsRef<str>, name: impl AsRef<str>) -> Result<GetReposResponse> {
    let url = format!(
        "https://api.github.com/repos/{}/{}",
        owner.as_ref(),
        name.as_ref()
    );
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "yevis")
        .send()?;
    ensure!(response.status().is_success(), "Failed to get repos");
    Ok(response.json::<GetReposResponse>()?)
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct GetReposResponse {
    pub private: bool,
    pub default_branch: String,
    pub license: Option<GetReposResponseLicense>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct GetReposResponseLicense {
    pub spdx_id: String,
}

pub fn get_latest_commit_hash(
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/branches/{}",
        owner.as_ref(),
        name.as_ref(),
        branch.as_ref()
    );
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "yevis")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to get latest commit hash"
    );
    let response_json = response.json::<GetBranchResponse>()?;
    Ok(response_json.commit.sha)
}

#[derive(Debug, Deserialize)]
struct GetBranchResponse {
    commit: GetBranchResponseCommit,
}

#[derive(Debug, Deserialize)]
struct GetBranchResponseCommit {
    sha: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::make_template::is_commit_hash;

    #[test]
    fn test_get_repos() {
        let response = get_repos("ddbj", "yevis-cli").unwrap();
        assert_eq!(
            response,
            GetReposResponse {
                private: false,
                default_branch: "main".to_string(),
                license: Some(GetReposResponseLicense {
                    spdx_id: "Apache-2.0".to_string()
                }),
            }
        );
    }

    #[test]
    fn test_get_latest_commit_hash() {
        let response = get_latest_commit_hash("ddbj", "yevis-cli", "main").unwrap();
        is_commit_hash(&response).unwrap();
    }
}
