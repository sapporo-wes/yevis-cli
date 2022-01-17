use crate::{
    github_api::{get_repos, get_user, has_forked_repo, post_fork, read_github_token},
    type_config::Config,
};
use anyhow::{ensure, Result};
use log::info;
use regex::Regex;
use std::thread;
use std::time;

pub fn pull_request(
    _config: &Config,
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

    match has_forked_repo(&github_token, &user_name, &repo_owner, &repo_name)? {
        true => {
            info!(
                "Repository {} has already been forked to {}.",
                repo.as_ref(),
                &user_name
            );
        }
        false => {
            info!("Forking {} to {}...", repo.as_ref(), &user_name);
            post_fork(&github_token, &repo_owner, &repo_name)?;
            // waiting
            let mut retry = 0;
            while retry < 10 {
                match has_forked_repo(&github_token, &user_name, &repo_owner, &repo_name)? {
                    true => {
                        info!(
                            "Repository {} has been forked to {}.",
                            repo.as_ref(),
                            &user_name
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

fn parse_repo(repo: impl AsRef<str>) -> Result<(String, String)> {
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
