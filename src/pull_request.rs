use crate::env;
use crate::github_api;
use crate::metadata;

use anyhow::{anyhow, bail, ensure, Result};
use log::info;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::thread;
use std::time;
use url::Url;

pub fn pull_request(
    configs: &Vec<metadata::types::Config>,
    gh_token: &Option<impl AsRef<str>>,
    repo: impl AsRef<str>,
) -> Result<()> {
    let gh_token = env::github_token(gh_token)?;

    let (user, _, _) = github_api::get_author_info(&gh_token)?;
    let (repo_owner, repo_name) = github_api::parse_repo(&repo)?;
    let default_branch = github_api::get_default_branch(&gh_token, &repo_owner, &repo_name, None)?;
    let default_branch_sha =
        github_api::get_branch_sha(&gh_token, &repo_owner, &repo_name, &default_branch)?;
    if user != repo_owner {
        fork_repository(&gh_token, &user, &repo_owner, &repo_name, &default_branch)?;
    }

    for config in configs {
        info!(
            "Creating a pull request based on workflow_id: {}, version: {}",
            config.id, config.version
        );
        create_branch(
            &gh_token,
            &user,
            &repo_name,
            &config.id.to_string(),
            &default_branch_sha,
        )?;
        commit_config(&gh_token, &user, &repo_name, config)?;
        create_pull_request(
            &gh_token,
            &user,
            &repo_owner,
            &repo_name,
            &default_branch,
            config,
        )?;
    }
    Ok(())
}

fn fork_repository(
    gh_token: impl AsRef<str>,
    user: impl AsRef<str>,
    ori_repo_owner: impl AsRef<str>,
    ori_repo_name: impl AsRef<str>,
    ori_default_branch: impl AsRef<str>,
) -> Result<()> {
    match has_forked_repo(&gh_token, &user, &ori_repo_owner, &ori_repo_name) {
        true => {
            info!(
                "Repository {}/{} has already been forked to {}",
                ori_repo_owner.as_ref(),
                ori_repo_name.as_ref(),
                user.as_ref()
            );
            info!("Sync the forked repository with the original repository");
            synk_fork_from_upstream(&gh_token, &user, &ori_repo_name, &ori_default_branch)?;
        }
        false => {
            info!(
                "Forking {}/{} to {}",
                ori_repo_owner.as_ref(),
                ori_repo_name.as_ref(),
                user.as_ref()
            );
            create_fork(&gh_token, &ori_repo_owner, &ori_repo_name)?;
            // waiting
            let mut retry = 0;
            while retry < 10 {
                match has_forked_repo(&gh_token, &user, &ori_repo_owner, &ori_repo_name) {
                    true => {
                        info!(
                            "Repository {}/{} has been forked to {}",
                            ori_repo_owner.as_ref(),
                            ori_repo_name.as_ref(),
                            user.as_ref()
                        );
                        break;
                    }
                    false => {
                        info!("Waiting for forking...");
                        thread::sleep(time::Duration::from_secs(6));
                    }
                }
                retry += 1;
            }
            ensure!(
                retry < 10,
                "Failed to fork repository {}/{} to {}",
                ori_repo_owner.as_ref(),
                ori_repo_name.as_ref(),
                user.as_ref()
            );
        }
    };
    Ok(())
}

fn has_forked_repo(
    gh_token: impl AsRef<str>,
    user: impl AsRef<str>,
    ori_repo_owner: impl AsRef<str>,
    ori_repo_name: impl AsRef<str>,
) -> bool {
    let res = match github_api::get_repos(&gh_token, &user, &ori_repo_name) {
        Ok(res) => res,
        Err(_) => return false,
    };
    match parse_fork_response(res) {
        Ok(fork) => match fork.fork {
            true => {
                fork.fork_parent.owner.as_str() == ori_repo_owner.as_ref()
                    && fork.fork_parent.name.as_str() == ori_repo_name.as_ref()
            }
            false => false,
        },
        Err(_) => false,
    }
}

struct Fork {
    pub fork: bool,
    pub fork_parent: ForkParent,
}

struct ForkParent {
    pub owner: String,
    pub name: String,
}

fn parse_fork_response(res: Value) -> Result<Fork> {
    let err_msg = "Failed to parse the response when getting repo info";
    let fork = res
        .get("fork")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_bool()
        .ok_or_else(|| anyhow!(err_msg))?;
    let fork_parent = res
        .get("parent")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?;
    let fork_parent_owner = fork_parent
        .get("owner")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("login")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let fork_parent_name = fork_parent
        .get("name")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    Ok(Fork {
        fork,
        fork_parent: ForkParent {
            owner: fork_parent_owner.to_string(),
            name: fork_parent_name.to_string(),
        },
    })
}

/// https://docs.github.com/en/rest/reference/branches#sync-a-fork-branch-with-the-upstream-repository
fn synk_fork_from_upstream(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/merge-upstream",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = json!({
        "branch": branch.as_ref(),
    });
    github_api::post_request(gh_token, &url, &body)?;
    Ok(())
}

/// https://docs.github.com/en/rest/reference/repos#create-a-fork
fn create_fork(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/forks",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = json!({});
    github_api::post_request(gh_token, &url, &body)?;
    Ok(())
}

fn create_branch(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
    default_branch_sha: impl AsRef<str>,
) -> Result<()> {
    info!("Creating branch {}", branch.as_ref());
    match github_api::create_ref(
        &gh_token,
        &owner,
        &name,
        format!("refs/heads/{}", branch.as_ref()),
        &default_branch_sha,
    ) {
        Ok(_) => info!("Branch {} has been created", branch.as_ref()),
        Err(_) => info!("Branch {} already exists", branch.as_ref()),
    };
    Ok(())
}

/// https://docs.github.com/en/rest/reference/repos#create-or-update-file-contents
pub fn create_or_update_file(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<Path>,
    message: impl AsRef<str>,
    content: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<()> {
    let encoded_content = base64::encode(content.as_ref());
    let body = match get_contents_blob_sha(&gh_token, &owner, &name, &path, &branch) {
        Ok(blob) => {
            // If the file already exists, update it
            if blob.content == encoded_content {
                // If the file already exists and the content is the same, do nothing
                return Ok(());
            }
            json!({
                "message": message.as_ref(),
                "content": encoded_content,
                "sha": blob.sha,
                "branch": branch.as_ref()
            })
        }
        Err(e) => {
            // If the file does not exist, create it
            if e.to_string().contains("Not Found") {
                json!({
                    "message": message.as_ref(),
                    "content": encoded_content,
                    "branch": branch.as_ref()
                })
            } else {
                bail!(e)
            }
        }
    };

    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/contents/{}",
        owner.as_ref(),
        name.as_ref(),
        path.as_ref().display(),
    ))?;
    put_request(&gh_token, &url, &body)?;
    Ok(())
}

fn put_request(gh_token: impl AsRef<str>, url: &Url, body: &Value) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .put(url.as_str())
        .header(reqwest::header::USER_AGENT, "gh-trs")
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

struct Blob {
    pub content: String,
    pub sha: String,
}

/// https://docs.github.com/en/rest/reference/repos#get-repository-content
fn get_contents_blob_sha(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    path: impl AsRef<Path>,
    branch: impl AsRef<str>,
) -> Result<Blob> {
    let res = github_api::get_contents(&gh_token, &owner, &name, &path, &branch)?;
    let err_msg = "Failed to parse the response when getting contents";
    let content = res
        .get("content")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let sha = res
        .get("sha")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    Ok(Blob {
        content: content.to_string(),
        sha: sha.to_string(),
    })
}

fn commit_config(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    config: &metadata::types::Config,
) -> Result<()> {
    let config_path = PathBuf::from(format!(
        "{}/yevis-metadata-{}.yml",
        &config.id, &config.version
    ));
    let config_content = serde_yaml::to_string(&config)?;
    let commit_message = format!(
        "Add workflow, id: {} version: {}",
        &config.id, &config.version
    );
    create_or_update_file(
        &gh_token,
        &owner,
        &name,
        &config_path,
        &commit_message,
        &config_content,
        &config.id.to_string(),
    )?;
    Ok(())
}

fn create_pull_request(
    gh_token: impl AsRef<str>,
    user: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
    config: &metadata::types::Config,
) -> Result<()> {
    let title = format!("Add workflow: {}", config.workflow.name);
    let head = format!("{}:{}", user.as_ref(), &config.id);
    info!(
        "Creating pull request to {}/{}",
        owner.as_ref(),
        name.as_ref()
    );
    // https://api.github.com/repos/suecharo/yevis-getting-started/pulls/1
    let pull_request_apt_url = post_pulls(&gh_token, &owner, &name, &title, &head, &branch)?;
    // https://github.com/suecharo/yevis-getting-started/pull/1
    let pull_request_url = pull_request_apt_url
        .as_str()
        .replace("https://api.github.com/repos/", "https://github.com/");
    info!("Pull Request URL: {}", &pull_request_url);
    Ok(())
}

/// https://docs.github.com/en/rest/reference/pulls#create-a-pull-request
/// head: the branch to merge from
/// base: the branch to merge into
///
/// return -> pull_request_url
pub fn post_pulls(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    title: impl AsRef<str>,
    head: impl AsRef<str>,
    base: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/pulls",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let body = json!({
        "title": title.as_ref(),
        "head": head.as_ref(),
        "base": base.as_ref(),
        "maintainer_can_modify": true
    });
    let res = github_api::post_request(gh_token, &url, &body)?;
    let err_msg = "Failed to parse the response when positing pull request";
    Ok(res
        .get("url")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?
        .to_string())
}
