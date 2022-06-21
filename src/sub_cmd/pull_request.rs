use crate::gh;
use crate::metadata;

use anyhow::{ensure, Result};
use log::info;
use std::path::PathBuf;
use std::thread;
use std::time;

pub fn pull_request(
    meta_vec: &Vec<metadata::types::Metadata>,
    gh_token: impl AsRef<str>,
    repo: impl AsRef<str>,
) -> Result<()> {
    let (user, _, _) = gh::api::get_author_info(&gh_token)?;
    let (repo_owner, repo_name) = gh::parse_repo(&repo)?;
    let default_branch = gh::api::get_default_branch(&gh_token, &repo_owner, &repo_name, None)?;
    let default_branch_sha =
        gh::api::get_branch_sha(&gh_token, &repo_owner, &repo_name, &default_branch)?;
    if user != repo_owner {
        fork_repository(&gh_token, &user, &repo_owner, &repo_name, &default_branch)?;
    }

    for meta in meta_vec {
        info!(
            "Creating a pull request based on workflow_id: {}, version: {}",
            meta.id, meta.version
        );
        info!("Creating branch {}", meta.id);
        match gh::api::create_branch(
            &gh_token,
            &user,
            &repo_name,
            &meta.id.to_string(),
            &default_branch_sha,
        ) {
            Ok(_) => info!("Branch {} has been created", meta.id),
            Err(_) => info!("Branch {} already exists", meta.id),
        };
        commit_meta(&gh_token, &user, &repo_name, meta)?;
        create_pull_request(
            &gh_token,
            &user,
            &repo_owner,
            &repo_name,
            &default_branch,
            meta,
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
    match gh::api::has_forked_repo(&gh_token, &user, &ori_repo_owner, &ori_repo_name) {
        true => {
            info!(
                "Repository {}/{} has already been forked to {}",
                ori_repo_owner.as_ref(),
                ori_repo_name.as_ref(),
                user.as_ref()
            );
            info!("Sync the forked repository with the original repository");
            gh::api::merge_upstream(&gh_token, &user, &ori_repo_name, &ori_default_branch)?;
        }
        false => {
            info!(
                "Forking {}/{} to {}",
                ori_repo_owner.as_ref(),
                ori_repo_name.as_ref(),
                user.as_ref()
            );
            gh::api::create_fork(&gh_token, &ori_repo_owner, &ori_repo_name)?;
            // waiting
            let mut retry = 0;
            while retry < 10 {
                match gh::api::has_forked_repo(&gh_token, &user, &ori_repo_owner, &ori_repo_name) {
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

fn commit_meta(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    meta: &metadata::types::Metadata,
) -> Result<()> {
    let meta_path = PathBuf::from(format!("{}/yevis-metadata-{}.yml", &meta.id, &meta.version));
    let meta_content = serde_yaml::to_string(&meta)?;
    let commit_message = format!("Add workflow, id: {} version: {}", &meta.id, &meta.version);
    gh::api::create_or_update_file(
        &gh_token,
        &owner,
        &name,
        &meta_path,
        &commit_message,
        &meta_content,
        &meta.id.to_string(),
    )?;
    Ok(())
}

fn create_pull_request(
    gh_token: impl AsRef<str>,
    user: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    branch: impl AsRef<str>,
    meta: &metadata::types::Metadata,
) -> Result<()> {
    let title = format!("Add workflow: {}", meta.workflow.name);
    let head = format!("{}:{}", user.as_ref(), &meta.id);
    info!(
        "Creating pull request to {}/{}",
        owner.as_ref(),
        name.as_ref()
    );
    // https://api.github.com/repos/ddbj/yevis-cli/pulls/1
    let pull_request_apt_url =
        gh::api::post_pulls(&gh_token, &owner, &name, &title, &head, &branch)?;
    // https://github.com/suecharo/yevis-getting-started/pull/1
    let pull_request_url = pull_request_apt_url
        .as_str()
        .replace("https://api.github.com/repos/", "https://github.com/");
    info!("Pull Request URL: {}", &pull_request_url);
    Ok(())
}
