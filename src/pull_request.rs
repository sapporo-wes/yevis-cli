use crate::{
    github_api::{
        create_or_update_file, create_ref, get_ref_sha, get_repos, get_user, has_forked_repo,
        post_fork, post_pulls, read_github_token, synk_fork_from_upstream,
    },
    type_config::Config,
};
use anyhow::{ensure, Result};
use log::info;
use regex::Regex;
use serde_yaml;
use std::thread;
use std::time;

pub fn pull_request(
    config: &Config,
    arg_github_token: &Option<impl AsRef<str>>,
    repo: impl AsRef<str>,
) -> Result<()> {
    let github_token = read_github_token(&arg_github_token)?;
    ensure!(
        !github_token.is_empty(),
        "GitHub token is empty. Please set it with --github-token option or set GITHUB_TOKEN environment variable."
    );

    let user_name = get_user(&github_token)?.login;
    let (repo_owner, repo_name) = parse_repo(&repo)?;
    let default_branch = get_repos(&github_token, &repo_owner, &repo_name)?.default_branch;
    let original_repo_ref_sha =
        get_ref_sha(&github_token, &repo_owner, &repo_name, &default_branch)?;
    let new_branch = config.id.to_string();

    fork_repository(
        &github_token,
        &user_name,
        &repo_owner,
        &repo_name,
        &default_branch,
        &original_repo_ref_sha,
    )?;
    create_branch(
        &github_token,
        &user_name,
        &repo_name,
        &new_branch,
        &original_repo_ref_sha,
    )?;
    commit_config(&github_token, &user_name, &repo_name, &new_branch, &config)?;
    create_pull_request(
        &github_token,
        &repo_owner,
        &repo_name,
        &default_branch,
        &user_name,
        &config,
    )?;

    Ok(())
}

pub fn parse_repo(repo: impl AsRef<str>) -> Result<(String, String)> {
    let re = Regex::new(r"^[\w-]+/[\w-]+$")?;
    ensure!(
        re.is_match(repo.as_ref()),
        "Invalid repository name: {}. It should be in the format of `owner/repo` like `ddbj/yevis-workflows`.",
        repo.as_ref()
    );
    let parts = repo.as_ref().split("/").collect::<Vec<_>>();
    ensure!(
        parts.len() == 2,
        "Invalid repository name: {}. It should be in the format of `owner/repo` like `ddbj/yevis-workflows`.",
        repo.as_ref()
    );
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn fork_repository(
    github_token: impl AsRef<str>,
    user_name: impl AsRef<str>,
    repo_owner: impl AsRef<str>,
    repo_name: impl AsRef<str>,
    default_branch: impl AsRef<str>,
    original_repo_ref_sha: impl AsRef<str>,
) -> Result<()> {
    match has_forked_repo(&github_token, &user_name, &repo_owner, &repo_name)? {
        true => {
            info!(
                "Repository {}/{} has already been forked to {}",
                repo_owner.as_ref(),
                repo_name.as_ref(),
                user_name.as_ref()
            );
            let fork_repo_ref_sha =
                get_ref_sha(&github_token, &user_name, &repo_name, &default_branch)?;
            if original_repo_ref_sha.as_ref() != fork_repo_ref_sha {
                info!(
                    "Repository {}/{} branch {} has been updated. Pulling changes.",
                    repo_owner.as_ref(),
                    repo_name.as_ref(),
                    default_branch.as_ref()
                );
                synk_fork_from_upstream(&github_token, &user_name, &repo_name, &default_branch)?;
            }
        }
        false => {
            info!(
                "Forking {}/{} to {}",
                repo_owner.as_ref(),
                repo_owner.as_ref(),
                user_name.as_ref()
            );
            post_fork(&github_token, &repo_owner, &repo_name)?;
            // waiting
            let mut retry = 0;
            while retry < 10 {
                match has_forked_repo(&github_token, &user_name, &repo_owner, &repo_name)? {
                    true => {
                        info!(
                            "Repository {}/{} has been forked to {}",
                            repo_owner.as_ref(),
                            repo_name.as_ref(),
                            user_name.as_ref()
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
        }
    };
    Ok(())
}

fn create_branch(
    github_token: impl AsRef<str>,
    repo_owner: impl AsRef<str>,
    repo_name: impl AsRef<str>,
    branch: impl AsRef<str>,
    original_repo_ref_sha: impl AsRef<str>,
) -> Result<()> {
    info!("Creating branch {}", branch.as_ref());
    match create_ref(
        &github_token,
        &repo_owner,
        &repo_name,
        &branch,
        &original_repo_ref_sha,
    ) {
        Ok(_) => info!("Created branch {}", branch.as_ref()),
        Err(_) => {
            info!("Branch {} already exists", &branch.as_ref());
        }
    };
    Ok(())
}

fn commit_config(
    github_token: impl AsRef<str>,
    repo_owner: impl AsRef<str>,
    repo_name: impl AsRef<str>,
    branch: impl AsRef<str>,
    config: &Config,
) -> Result<()> {
    let config_file_path = format!(
        "{}/yevis_config_{}.yml",
        &config.id.to_string(),
        &config.version
    );
    let config_content = serde_yaml::to_string(&config)?;
    let commit_message = format!(
        "Add workflow, id: {} version: {}",
        &config.id, &config.version
    );
    info!("Committing config file {}", &config_file_path);
    create_or_update_file(
        &github_token,
        &repo_owner,
        &repo_name,
        &config_file_path,
        &commit_message,
        &config_content,
        &branch,
    )?;
    Ok(())
}

fn create_pull_request(
    github_token: impl AsRef<str>,
    to_repo_owner: impl AsRef<str>,
    to_repo_name: impl AsRef<str>,
    branch: impl AsRef<str>,
    user_name: impl AsRef<str>,
    config: &Config,
) -> Result<()> {
    let title = format!(
        "Add workflow, id: {} version: {}",
        &config.id, &config.version
    );
    let head = format!("{}:{}", user_name.as_ref(), &config.id);
    info!(
        "Creating pull request to {}/{}",
        to_repo_owner.as_ref(),
        to_repo_name.as_ref()
    );
    let pull_request_url = post_pulls(
        &github_token,
        &to_repo_owner,
        &to_repo_name,
        &title,
        &head,
        &branch,
    )?;
    info!("Pull request URL: {}", &pull_request_url);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo() -> Result<()> {
        assert_eq!(
            parse_repo("ddbj/yevis-workflows")?,
            ("ddbj".to_string(), "yevis-workflows".to_string())
        );
        Ok(())
    }
}
