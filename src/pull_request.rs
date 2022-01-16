use crate::{
    github_api::{get_user, read_github_token},
    type_config::Config,
};
use anyhow::{anyhow, bail, ensure, Result};
use regex::Regex;

pub fn pull_request(
    config: &Config,
    arg_github_token: &Option<impl AsRef<str>>,
    repository: impl AsRef<str>,
    wes_location: &Option<impl AsRef<str>>,
    docker_host: impl AsRef<str>,
) -> Result<()> {
    let github_token = read_github_token(&arg_github_token)?;
    ensure!(
        !github_token.is_empty(),
        "GitHub token is empty. Please set it with --github-token option or set GITHUB_TOKEN environment variable."
    );
    let github_user = get_user(&github_token)?;
    println!("pull-request");
    Ok(())
}

fn validate_parse_repo(repo: impl AsRef<str>) -> Result<(String, String)> {
    let re = Regex::new(r"^\w+\/\w+$")?;
    ensure!(
        re.is_match(repo.as_ref()),
        "Invalid repository name: {}. It should be in the format of `owner/repo` like `ddbj/yevis-workflows`.",
        repo.as_ref()
    );
    let parts = repo.as_ref().split("/").collect::<Vec<_>>();
    Ok((parts[0].to_string(), parts[1].to_string()))
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_validate_parse_repo() -> Result<()> {
//     }
// }
