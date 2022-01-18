use crate::make_template::parse_wf_loc;
use anyhow::{anyhow, bail, ensure, Result};
use base64::encode;
use colored::Colorize;
use dotenv::dotenv;
use log::{debug, info};
use reqwest;
use serde_json::{json, Value};
use std::env;
use std::path::{Path, PathBuf};
use std::thread;
use std::time;
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub struct WfRepoInfo {
    pub owner: String,
    pub name: String,
    pub commit_hash: String,
    pub file_path: PathBuf,
}

impl WfRepoInfo {
    /// Obtain and organize information about the GitHub repository, where the main workflow is located.
    pub fn new(github_token: impl AsRef<str>, wf_loc: &Url) -> Result<Self> {
        let parse_result = parse_wf_loc(&wf_loc)?;
        let get_repos_res = get_repos(&github_token, &parse_result.owner, &parse_result.name)?;
        ensure!(
            get_repos_res.private == false,
            format!(
                "Repository {}/{} is private",
                parse_result.owner, parse_result.name
            )
        );
        let branch = match &parse_result.branch {
            Some(branch) => branch.to_string(),
            None => get_repos_res.default_branch,
        };
        let commit_hash = match &parse_result.commit_hash {
            Some(commit_hash) => commit_hash.to_string(),
            None => get_latest_commit_hash(
                &github_token,
                &parse_result.owner,
                &parse_result.name,
                &branch,
            )?,
        };
        Ok(WfRepoInfo {
            owner: parse_result.owner,
            name: parse_result.name,
            commit_hash,
            file_path: parse_result.file_path,
        })
    }
}

pub fn raw_url_from_path(wf_repo_info: &WfRepoInfo, file_path: impl AsRef<Path>) -> Result<Url> {
    Ok(Url::parse(&format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        &wf_repo_info.owner,
        &wf_repo_info.name,
        &wf_repo_info.commit_hash,
        file_path
            .as_ref()
            .display()
            .to_string()
            .trim_start_matches("/")
    ))?)
}

pub fn to_raw_url_from_url(github_token: impl AsRef<str>, url: &Url) -> Result<Url> {
    match url.host_str() {
        Some("github.com") | Some("raw.githubusercontent.com") => {
            let wf_repo_info = WfRepoInfo::new(&github_token, &url)?;
            let wf_loc = raw_url_from_path(&wf_repo_info, &wf_repo_info.file_path)?;
            Ok(wf_loc)
        }
        Some(_) => Ok(url.clone()),
        None => bail!("URL is not a valid URL"),
    }
}

pub fn to_file_path(raw_url: &Url) -> Result<PathBuf> {
    let path_segments = raw_url
        .path_segments()
        .ok_or(anyhow!(
            "Failed to get path segments from url: {}",
            &raw_url
        ))?
        .collect::<Vec<_>>();
    Ok(path_segments[3..].into_iter().collect())
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
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}",
        owner.as_ref(),
        name.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to get repos from GitHub with status: {:?}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => {
            let parent = match &body["parent"].as_object() {
                Some(parent) => {
                    let parts = parent["full_name"]
                        .as_str()
                        .ok_or(anyhow!("Failed to parse response when getting repos"))?
                        .split("/")
                        .collect::<Vec<_>>();
                    ensure!(
                        parts.len() == 2,
                        "Failed to parse response when getting repos"
                    );
                    Some(ForkParent {
                        owner: parts[0].to_string(),
                        name: parts[1].to_string(),
                    })
                }
                None => None::<ForkParent>,
            };
            Ok(GetReposResponse {
                private: body["private"]
                    .as_bool()
                    .ok_or(anyhow!("Failed to parse response when getting repos"))?,
                default_branch: body["default_branch"]
                    .as_str()
                    .ok_or(anyhow!("Failed to parse response when getting repos"))?
                    .to_string(),
                license: match &body["license"].as_object() {
                    Some(license) => Some(
                        license["spdx_id"]
                            .as_str()
                            .ok_or(anyhow!("Failed to parse response when getting repos"))?
                            .to_string(),
                    ),
                    None => None,
                },
                fork: body["fork"]
                    .as_bool()
                    .ok_or(anyhow!("Failed to parse response when getting repos"))?,
                fork_parent: parent,
            })
        }
        false => bail!("Failed to parse response when getting repos"),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GetReposResponse {
    pub private: bool,
    pub default_branch: String,
    pub license: Option<String>,
    pub fork: bool,
    pub fork_parent: Option<ForkParent>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForkParent {
    pub owner: String,
    pub name: String,
}

pub fn get_latest_commit_hash(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/branches/{}",
        owner.as_ref(),
        name.as_ref(),
        branch.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to get latest commit hash from GitHub with status: {:?}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => {
            let commit = body["commit"].as_object().ok_or(anyhow!(
                "Failed to parse response when getting latest commit hash"
            ))?;
            let sha = commit["sha"].as_str().ok_or(anyhow!(
                "Failed to parse response when getting latest commit hash"
            ))?;
            Ok(sha.to_string())
        }
        false => bail!("Failed to parse response when getting latest commit hash"),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GithubUser {
    pub login: String,
    pub name: String,
    pub company: String,
}

pub fn get_user(github_token: impl AsRef<str>) -> Result<GithubUser> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to get user from GitHub with status: {:?}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => Ok(GithubUser {
            login: body["login"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response when getting user"))?
                .to_string(),
            name: body["name"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response when getting user"))?
                .to_string(),
            company: body["company"]
                .as_str()
                .ok_or(anyhow!("Failed to parse response when getting user"))?
                .to_string(),
        }),
        false => bail!("Failed to parse response when getting user"),
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
    dir_path: impl AsRef<Path>,
) -> Result<Vec<PathBuf>> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner.as_ref(),
        name.as_ref(),
        dir_path
            .as_ref()
            .display()
            .to_string()
            .trim_start_matches("/")
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .query(&[("ref", commit_hash.as_ref())])
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to get file list from GitHub with status: {:?}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_array() {
        true => {
            let mut file_list: Vec<PathBuf> = Vec::new();
            for obj in body.as_array().ok_or(anyhow!("Failed to parse response"))? {
                let obj_type = obj["type"]
                    .as_str()
                    .ok_or(anyhow!("Failed to parse response when getting file list"))?;
                match obj_type {
                    "file" => {
                        let path = obj["path"]
                            .as_str()
                            .ok_or(anyhow!("Failed to parse response when getting file list"))?;
                        file_list.push(PathBuf::from(path));
                    }
                    "dir" => {
                        let path = obj["path"]
                            .as_str()
                            .ok_or(anyhow!("Failed to parse response when getting file list"))?;
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
        false => bail!("Failed to parse response when getting file list"),
    }
}

pub fn head_request(url: &Url, retry: usize) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client.head(url.as_str()).send()?;
    if !response.status().is_success() {
        if retry < 3 {
            debug!("Retrying head request to {}", url.as_str());
            thread::sleep(time::Duration::from_millis(500));
            head_request(url, retry + 1)?;
        } else {
            info!(
                "{}: Failed to HEAD request to {} with status: {:?}. So retry using GET request.",
                "Warning".yellow(),
                url.as_str(),
                response.status()
            );
            let get_response = client.get(url.as_str()).send()?;
            ensure!(
                get_response.status().is_success(),
                format!(
                    "Failed to HEAD and GET request to {} with status: {:?}",
                    url.as_str(),
                    get_response.status()
                )
            );
        }
    }
    Ok(())
}

pub fn post_fork(
    github_token: impl AsRef<str>,
    from_repo_owner: impl AsRef<str>,
    from_repo_name: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/forks",
        from_repo_owner.as_ref(),
        from_repo_name.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to post fork to GitHub with status: {:?}",
            response.status()
        )
    );

    Ok(())
}

pub fn has_forked_repo(
    github_token: impl AsRef<str>,
    user_name: impl AsRef<str>,
    repo_owner: impl AsRef<str>,
    repo_name: impl AsRef<str>,
) -> Result<bool> {
    let response = match get_repos(&github_token, &user_name, &repo_name) {
        Ok(response) => response,
        Err(err) => {
            if err.to_string().contains("404") {
                return Ok(false);
            }
            bail!(err)
        }
    };
    match response.fork {
        true => match &response.fork_parent {
            Some(fork_parent) => {
                if fork_parent.owner == repo_owner.as_ref()
                    && fork_parent.name == repo_name.as_ref()
                {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        },
        false => Ok(false),
    }
}

/// https://docs.github.com/en/rest/reference/git#get-a-reference
pub fn get_ref_sha(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/ref/heads/{}",
        owner.as_ref(),
        name.as_ref(),
        branch.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to get ref from GitHub with status: {:?}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match body.is_object() {
        true => match body["object"].is_object() {
            true => {
                let sha = body["object"]["sha"]
                    .as_str()
                    .ok_or(anyhow!("Failed to parse response when getting ref"))?;
                Ok(sha.to_string())
            }
            false => bail!("Failed to parse response when getting ref"),
        },
        false => bail!("Failed to parse response when getting ref"),
    }
}

/// https://docs.github.com/en/rest/reference/git#create-a-reference
pub fn create_ref(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
    sha: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/refs",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .json(&json!({
            "sha": sha.as_ref(),
            "ref": format!("refs/heads/{}", branch.as_ref()),
        }))
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to update ref to GitHub with status: {:?}",
            response.status()
        )
    );

    Ok(())
}

/// https://docs.github.com/en/rest/reference/git#update-a-reference
pub fn update_ref(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
    sha: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/git/refs/heads/{}",
        owner.as_ref(),
        name.as_ref(),
        branch.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .patch(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .json(&json!({
            "sha": sha.as_ref(),
            "force": true
        }))
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to update ref to GitHub with status: {:?}",
            response.status()
        )
    );

    Ok(())
}

#[derive(Debug, PartialEq, Clone)]
struct BlobResponse {
    content: String,
    sha: String,
}

/// https://docs.github.com/en/rest/reference/repos#get-repository-content
fn get_contents_blob_sha(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<BlobResponse> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner.as_ref(),
        name.as_ref(),
        path.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .query(&[("ref", branch.as_ref())])
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to get contents sha from GitHub with status: {:?}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match body.is_object() {
        true => Ok(BlobResponse {
            content: body["content"]
                .as_str()
                .ok_or(anyhow!(
                    "Failed to parse response when getting contents sha"
                ))?
                .to_string(),
            sha: body["sha"]
                .as_str()
                .ok_or(anyhow!(
                    "Failed to parse response when getting contents sha"
                ))?
                .to_string(),
        }),
        false => bail!("Failed to parse response when getting contents sha"),
    }
}

/// https://docs.github.com/en/rest/reference/repos#create-or-update-file-contents
pub fn create_or_update_file(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<str>,
    message: impl AsRef<str>,
    content: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<()> {
    let encoded_content = encode(content.as_ref());
    let request_body = match get_contents_blob_sha(&github_token, &owner, &name, &path, &branch) {
        Ok(res) => {
            if res.content == encoded_content {
                return Ok(());
            }
            json!({
                "message": message.as_ref(),
                "content": encode(content.as_ref()),
                "sha": res.sha,
                "branch": branch.as_ref()
            })
        }
        Err(err) => {
            if err.to_string().contains("404") {
                json!({
                    "message": message.as_ref(),
                    "content": encoded_content,
                    "branch": branch.as_ref()
                })
            } else {
                bail!(err)
            }
        }
    };

    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner.as_ref(),
        name.as_ref(),
        path.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .put(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .json(&request_body)
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to create or update file to GitHub with status: {:?}",
            response.status()
        )
    );

    Ok(())
}

/// https://docs.github.com/en/rest/reference/pulls#create-a-pull-request
/// base: the branch to merge into
/// head: the branch to merge from
///
/// return -> pull_request_URL
pub fn post_pulls(
    github_token: impl AsRef<str>,
    to_owner: impl AsRef<str>,
    to_name: impl AsRef<str>,
    title: impl AsRef<str>,
    head: impl AsRef<str>,
    base: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/pulls",
        to_owner.as_ref(),
        to_name.as_ref(),
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::USER_AGENT, "yevis")
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("token {}", github_token.as_ref()),
        )
        .json(&json!({
            "title": title.as_ref(),
            "head": head.as_ref(),
            "base": base.as_ref(),
            "maintainer_can_modify": true
        }))
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with GitHub. Please check your GitHub token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to create pull request to GitHub with status: {:?}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match body.is_object() {
        true => Ok(body["url"]
            .as_str()
            .ok_or(anyhow!(
                "Failed to parse response when posting pull request"
            ))?
            .to_string()),
        false => bail!("Failed to parse response when posting pull request"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::make_template::is_commit_hash;

    #[test]
    fn test_wf_repo_info_new() -> Result<()> {
        let github_token = read_github_token(&None::<String>)?;
        let wf_loc = Url::parse(
            "https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl",
        )?;
        let wf_repo_info = WfRepoInfo::new(&github_token, &wf_loc)?;
        assert_eq!(wf_repo_info.owner, "ddbj");
        assert_eq!(wf_repo_info.name, "yevis-cli");
        is_commit_hash(&wf_repo_info.commit_hash)?;
        assert_eq!(
            wf_repo_info.file_path,
            PathBuf::from("tests/CWL/wf/trimming_and_qc.cwl")
        );
        Ok(())
    }

    #[test]
    fn test_raw_url_from_path() -> Result<()> {
        let github_token = read_github_token(&None::<String>)?;
        let wf_loc = Url::parse(
            "https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl",
        )?;
        let wf_repo_info = WfRepoInfo::new(&github_token, &wf_loc)?;
        let raw_url = raw_url_from_path(&wf_repo_info, &Path::new("path/to/file"))?;
        assert_eq!(raw_url.host_str(), Some("raw.githubusercontent.com"));
        Ok(())
    }

    #[test]
    fn test_to_raw_url_from_url() -> Result<()> {
        let github_token = read_github_token(&None::<String>)?;
        let wf_loc = Url::parse(
            "https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl",
        )?;
        let raw_url = to_raw_url_from_url(&github_token, &wf_loc)?;
        assert_eq!(raw_url.host_str(), Some("raw.githubusercontent.com"));
        Ok(())
    }

    #[test]
    fn test_to_file_path() -> Result<()> {
        let raw_url = Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/36d23db735623e0e87a69a02d23ff08c754e6f13/tests/CWL/wf/trimming_and_qc.cwl")?;
        let file_path = to_file_path(&raw_url)?;
        assert_eq!(file_path, PathBuf::from("tests/CWL/wf/trimming_and_qc.cwl"));
        Ok(())
    }

    #[test]
    fn test_read_github_token_args() -> Result<()> {
        let token = read_github_token(&Some("token"))?;
        assert_eq!(token, "token");
        Ok(())
    }

    #[test]
    fn test_read_github_token_env() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        assert!(token.chars().count() > 0);
        Ok(())
    }

    #[test]
    fn test_get_repos() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let response = get_repos(&token, "ddbj", "yevis-cli")?;
        assert_eq!(
            response,
            GetReposResponse {
                private: false,
                default_branch: "main".to_string(),
                license: Some("Apache-2.0".to_string()),
                fork: false,
                fork_parent: None,
            }
        );
        Ok(())
    }

    #[test]
    fn test_get_latest_commit_hash() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let response = get_latest_commit_hash(&token, "ddbj", "yevis-cli", "main")?;
        is_commit_hash(&response)?;
        Ok(())
    }

    #[test]
    fn test_get_user() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        get_user(&token)?;
        Ok(())
    }

    #[test]
    fn test_get_file_list_recursive() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let commit_hash = get_latest_commit_hash(&token, "ddbj", "yevis-cli", "main")?;
        let response = get_file_list_recursive(&token, "ddbj", "yevis-cli", &commit_hash, ".")?;
        assert!(response.contains(&PathBuf::from("README.md")));
        assert!(response.contains(&PathBuf::from("LICENSE")));
        assert!(response.contains(&PathBuf::from("src/main.rs")));
        Ok(())
    }

    #[test]
    fn test_head_request_ok() -> Result<()> {
        let url = Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/36d23db735623e0e87a69a02d23ff08c754e6f13/tests/CWL/wf/trimming_and_qc.cwl")?;
        assert!(head_request(&url, 0).is_ok());
        Ok(())
    }

    #[test]
    fn test_head_request_error() -> Result<()> {
        let url = Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/36d23db735623e0e87a69a02d23ff08c754e6f13/nothing")?;
        assert!(head_request(&url, 0).is_err());
        Ok(())
    }

    #[test]
    fn test_post_fork() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        post_fork(&token, "ddbj", "yevis-workflows-dev")?;
        Ok(())
    }

    #[test]
    fn test_has_forked_repo() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let response = has_forked_repo(&token, "suecharo", "ddbj", "yevis-workflows-dev")?;
        assert!(response);
        Ok(())
    }

    #[test]
    fn test_has_forked_repo_invalid_name() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let response = has_forked_repo(&token, "suecharo", "ddbj", "invalid_name")?;
        assert_eq!(response, false);
        Ok(())
    }

    #[test]
    fn test_get_ref() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let response = get_ref_sha(&token, "ddbj", "yevis-cli", "main")?;
        assert!(response.len() > 0);
        Ok(())
    }

    #[test]
    fn test_update_ref() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let original_ref = get_ref_sha(&token, "ddbj", "yevis-workflows-dev", "main")?;
        update_ref(
            &token,
            "suecharo",
            "yevis-workflows-dev",
            "main",
            &original_ref,
        )?;
        Ok(())
    }

    #[test]
    fn test_get_contents_blob_sha() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let response =
            get_contents_blob_sha(&token, "ddbj", "yevis-workflows-dev", "README.md", "main")?;
        assert!(response.sha.len() > 0);
        Ok(())
    }

    #[test]
    fn test_get_contents_blob_sha_invalid_file() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let result = get_contents_blob_sha(
            &token,
            "ddbj",
            "yevis-workflows-dev",
            "invalid_file",
            "main",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
        Ok(())
    }

    #[test]
    fn test_create_or_update_file() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        create_or_update_file(
            &token,
            "ddbj",
            "yevis-workflows-dev",
            "test.txt",
            "test commit",
            "test",
            "main",
        )?;
        Ok(())
    }

    #[test]
    fn test_create_or_update_file_include_dir() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        create_or_update_file(
            &token,
            "ddbj",
            "yevis-workflows-dev",
            "test_dir/test.txt",
            "test commit",
            "test",
            "main",
        )?;
        Ok(())
    }

    #[test]
    fn test_create_ref() -> Result<()> {
        let token = read_github_token(&None::<String>)?;
        let original_ref = get_ref_sha(&token, "ddbj", "yevis-workflows-dev", "main")?;
        let result = create_ref(&token, "ddbj", "yevis-workflows-dev", "test", &original_ref);
        if result.is_err() {
            assert!(result.unwrap_err().to_string().contains("422"));
        }
        Ok(())
    }
}
