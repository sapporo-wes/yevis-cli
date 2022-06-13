use anyhow::{anyhow, bail, Result};
use dotenv::dotenv;
use std::env;
use url::Url;

#[cfg(not(tarpaulin_include))]
pub fn github_token(arg_token: &Option<impl AsRef<str>>) -> Result<String> {
    dotenv().ok();
    match arg_token {
        Some(token) => Ok(token.as_ref().to_string()),
        None => match env::var("GITHUB_TOKEN") {
            Ok(token) => Ok(token),
            Err(_) => bail!("No GitHub token provided. Please set the environment variable `GITHUB_TOKEN` or pass the `--gh-token` flag."),
        },
    }
}

#[cfg(not(tarpaulin_include))]
pub fn sapporo_run_dir() -> Result<String> {
    dotenv().ok();
    match env::var("SAPPORO_RUN_DIR") {
        Ok(run_dir) => Ok(run_dir),
        Err(_) => {
            let cwd = env::current_dir()?;
            Ok(cwd
                .join("sapporo_run")
                .to_str()
                .ok_or_else(|| anyhow!("Invalid path"))?
                .to_string())
        }
    }
}

#[cfg(not(tarpaulin_include))]
pub fn in_ci() -> bool {
    dotenv().ok();
    env::var("CI").is_ok()
}

#[cfg(not(tarpaulin_include))]
pub fn gh_actions_url() -> Result<Url> {
    dotenv().ok();
    let gh_server_url = env::var("GITHUB_SERVER_URL")?;
    let gh_repo = env::var("GITHUB_REPOSITORY")?;
    let gh_run_id = env::var("GITHUB_RUN_ID")?;
    Ok(Url::parse(&format!(
        "{}/{}/actions/runs/{}",
        gh_server_url, gh_repo, gh_run_id
    ))?)
}
