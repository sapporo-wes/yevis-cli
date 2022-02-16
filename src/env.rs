use anyhow::{bail, Result};
use dotenv::dotenv;
use std::env;

pub fn yevis_dev() -> bool {
    dotenv().ok();
    match env::var("YEVIS_DEV") {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn default_pr_repo() -> &'static str {
    match yevis_dev() {
        true => "ddbj/yevis-workflows-dev",
        false => "ddbj/yevis-workflows",
    }
}

pub fn zenodo_token() -> Result<String> {
    dotenv().ok();
    match env::var("ZENODO_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) => {
            bail!("No Zenodo token provided. Please set the ZENODO_TOKEN environment variable.")
        }
    }
}

pub fn zenodo_host() -> String {
    match yevis_dev() {
        true => "sandbox.zenodo.org".to_string(),
        false => "zenodo.org".to_string(),
    }
}
