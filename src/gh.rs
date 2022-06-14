pub mod api;
pub mod gist;
pub mod pr;

use anyhow::{ensure, Result};
use regex::Regex;
use serde_json::Value;
use url::Url;

pub fn parse_repo(repo: impl AsRef<str>) -> Result<(String, String)> {
    let re = Regex::new(r"^[\w-]+/[\w-]+$")?;
    ensure!(
        re.is_match(repo.as_ref()),
        "Invalid repository name: {}. It should be in the format of `owner/name`.",
        repo.as_ref()
    );
    let parts = repo.as_ref().split('/').collect::<Vec<_>>();
    Ok((parts[0].to_string(), parts[1].to_string()))
}

pub fn get_request(gh_token: impl AsRef<str>, url: &Url, query: &[(&str, &str)]) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", gh_token.as_ref()),
        )
        .query(query)
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        status.is_success(),
        "Failed to get request to {}. Response: {}",
        url,
        match res_body.get("message") {
            Some(message) => message.as_str().unwrap_or_else(|| status.as_str()),
            None => status.as_str(),
        }
    );
    Ok(res_body)
}

pub fn post_request(gh_token: impl AsRef<str>, url: &Url, body: &Value) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", gh_token.as_ref()),
        )
        .json(body)
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        status.is_success(),
        "Failed to post request to {}. Response: {}",
        url,
        match res_body.get("message") {
            Some(message) => message.as_str().unwrap_or_else(|| status.as_str()),
            None => status.as_str(),
        }
    );
    Ok(res_body)
}

pub fn patch_request(gh_token: impl AsRef<str>, url: &Url, body: &Value) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .patch(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", gh_token.as_ref()),
        )
        .json(body)
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        status.is_success(),
        "Failed to patch request to {}. Response: {}",
        url,
        match res_body.get("message") {
            Some(message) => message.as_str().unwrap_or_else(|| status.as_str()),
            None => status.as_str(),
        }
    );
    Ok(res_body)
}
