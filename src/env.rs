use anyhow::{anyhow, bail, Result};
use dotenv::dotenv;
use std::env;
use url::Url;

pub fn yevis_dev() -> bool {
    dotenv().ok();
    env::var("YEVIS_DEV").is_ok()
}

pub fn zenodo_token() -> Result<String> {
    dotenv().ok();
    match env::var("ZENODO_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) => {
            bail!("No Zenodo token provided. Please set the environment variable `ZENODO_TOKEN`.")
        }
    }
}

pub fn zenodo_host() -> String {
    match yevis_dev() {
        true => "sandbox.zenodo.org".to_string(),
        false => "zenodo.org".to_string(),
    }
}

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

pub fn sapporo_run_dir() -> Result<String> {
    dotenv().ok();
    match env::var("SAPPORO_RUN_DIR") {
        Ok(run_dir) => Ok(run_dir),
        Err(_) => {
            let cwd = env::current_dir()?;
            Ok(cwd
                .join("sapporo-run")
                .to_str()
                .ok_or_else(|| anyhow!("Could not convert current directory to string."))?
                .to_string())
        }
    }
}

pub fn in_ci() -> bool {
    dotenv().ok();
    env::var("CI").is_ok()
}

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
