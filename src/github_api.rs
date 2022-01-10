use anyhow::{anyhow, ensure, Result};
use dotenv::dotenv;
use reqwest;
use serde_json::Value;
use std::env;
use url::Url;

pub fn to_raw_url(
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    commit_hash: impl AsRef<str>,
    file_path: impl AsRef<str>,
) -> Result<String> {
    Ok(Url::parse(&format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        owner.as_ref(),
        name.as_ref(),
        commit_hash.as_ref(),
        file_path.as_ref()
    ))?
    .to_string())
}

pub fn to_file_path(raw_url: impl AsRef<str>) -> Result<String> {
    let raw_url = Url::parse(raw_url.as_ref())?;
    let path_segments = raw_url
        .path_segments()
        .ok_or(anyhow!("Could not parse path segments"))?
        .collect::<Vec<_>>();
    Ok(path_segments[3..].join("/"))
}

pub fn read_github_token(arg_token: &Option<impl AsRef<str>>) -> Result<String> {
    match arg_token {
        Some(token) => Ok(token.as_ref().to_string()),
        None => {
            dotenv().ok();
            Ok(env::var("GITHUB_TOKEN")?)
        }
    }
}

pub fn get_repos(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<GetReposResponse> {
    let url = format!(
        "https://api.github.com/repos/{}/{}",
        owner.as_ref(),
        name.as_ref()
    );
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(response.status().is_success(), "Failed to get repos");
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => Ok(GetReposResponse {
            private: body["private"]
                .as_bool()
                .ok_or(anyhow!("Failed to parse response"))?,
            default_branch: body["default_branch"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response"))?
                .to_string(),
            license: match &body["license"] {
                Value::Object(license) => Some(
                    license["spdx_id"]
                        .as_str()
                        .ok_or(anyhow!("Failed to parse response"))?
                        .to_string(),
                ),
                _ => None,
            },
        }),
        false => Err(anyhow!("Failed to parse response")),
    }
}

#[derive(Debug, PartialEq)]
pub struct GetReposResponse {
    pub private: bool,
    pub default_branch: String,
    pub license: Option<String>,
}

pub fn get_latest_commit_hash(
    github_token: impl AsRef<str>,
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
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to get latest commit hash"
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => {
            let commit = body["commit"]
                .as_object()
                .ok_or(anyhow!("Failed to parse response"))?;
            let sha = commit["sha"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response"))?;
            Ok(sha.to_string())
        }
        false => Err(anyhow!("Failed to parse response")),
    }
}

#[derive(Debug, PartialEq)]
pub struct GetUserResponse {
    pub login: String,
    pub name: String,
    pub company: String,
}

pub fn get_user(github_token: impl AsRef<str>) -> Result<GetUserResponse> {
    let url = format!("https://api.github.com/user",);
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to get latest commit hash"
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => Ok(GetUserResponse {
            login: body["login"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response"))?
                .to_string(),
            name: body["name"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response"))?
                .to_string(),
            company: body["company"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response"))?
                .to_string(),
        }),
        false => Err(anyhow!("Failed to parse response")),
    }
}

/// dir_path examples:
///
/// - `.`
/// - `path/to/dir`
/// - `/path/to/dir`
pub fn get_file_list_recursive(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    commit_hash: impl AsRef<str>,
    dir_path: impl AsRef<str>,
) -> Result<Vec<String>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner.as_ref(),
        name.as_ref(),
        dir_path.as_ref(),
    );
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .query(&[("ref", commit_hash.as_ref())])
        .send()?;
    ensure!(response.status().is_success(), "Failed to get file list");
    let body = response.json::<Value>()?;

    match &body.is_array() {
        true => {
            let mut file_list = Vec::new();
            for obj in body.as_array().ok_or(anyhow!("Failed to parse response"))? {
                let obj_type = obj["type"]
                    .as_str()
                    .ok_or(anyhow!("Failed to parse response"))?;
                match obj_type {
                    "file" => {
                        let path = obj["path"]
                            .as_str()
                            .ok_or(anyhow!("Failed to parse response"))?;
                        file_list.push(path.to_string());
                    }
                    "dir" => {
                        let path = obj["path"]
                            .as_str()
                            .ok_or(anyhow!("Failed to parse response"))?;
                        let mut sub_file_list = get_file_list_recursive(
                            github_token.as_ref(),
                            owner.as_ref(),
                            name.as_ref(),
                            commit_hash.as_ref(),
                            path,
                        )?;
                        file_list.append(&mut sub_file_list);
                    }
                    _ => {}
                }
            }
            Ok(file_list)
        }
        false => Err(anyhow!("Failed to parse response")),
    }
}

pub fn get_license_path(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/license",
        owner.as_ref(),
        name.as_ref(),
    );
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(response.status().is_success(), "Failed to get license");
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => {
            let path = body["path"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response"))?;
            Ok(path.to_string())
        }
        false => Err(anyhow!("Failed to parse response")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::make_template::is_commit_hash;

    #[test]
    fn test_read_github_token_args() {
        let token = read_github_token(&Some("token")).unwrap();
        assert_eq!(token, "token");
    }

    #[test]
    fn test_read_github_token_env() {
        let arg_token: Option<&str> = None;
        let token = read_github_token(&arg_token).unwrap();
        assert!(token.chars().count() > 0);
    }

    #[test]
    fn test_get_repos() {
        let arg_token: Option<&str> = None;
        let token = read_github_token(&arg_token).unwrap();
        let response = get_repos(&token, "ddbj", "yevis-cli").unwrap();
        assert_eq!(
            response,
            GetReposResponse {
                private: false,
                default_branch: "main".to_string(),
                license: Some("Apache-2.0".to_string())
            }
        );
    }

    #[test]
    fn test_get_latest_commit_hash() {
        let arg_token: Option<&str> = None;
        let token = read_github_token(&arg_token).unwrap();
        let response = get_latest_commit_hash(&token, "ddbj", "yevis-cli", "main").unwrap();
        is_commit_hash(&response).unwrap();
    }

    #[test]
    fn test_get_user() {
        let arg_token: Option<&str> = None;
        let token = read_github_token(&arg_token).unwrap();
        get_user(&token).unwrap();
    }

    #[test]
    fn test_get_file_list_recursive() {
        let arg_token: Option<&str> = None;
        let token = read_github_token(&arg_token).unwrap();
        let commit_hash = get_latest_commit_hash(&token, "ddbj", "yevis-cli", "main").unwrap();
        let response =
            get_file_list_recursive(&token, "ddbj", "yevis-cli", &commit_hash, ".").unwrap();
        assert!(response.contains(&"README.md".to_string()));
        assert!(response.contains(&"LICENSE".to_string()));
        assert!(response.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_get_license_path() {
        let arg_token: Option<&str> = None;
        let token = read_github_token(&arg_token).unwrap();
        let response = get_license_path(&token, "ddbj", "yevis-cli").unwrap();
        assert_eq!(response, "LICENSE");
    }
}
