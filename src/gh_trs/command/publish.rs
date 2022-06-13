use crate::gh_trs::config;
use crate::gh_trs::env;
use crate::gh_trs::github_api;
use crate::gh_trs::trs;

use anyhow::{anyhow, Result};
use log::info;

#[cfg(not(tarpaulin_include))]
pub fn publish(
    configs: &Vec<config::types::Config>,
    gh_token: &Option<impl AsRef<str>>,
    repo: impl AsRef<str>,
    branch: impl AsRef<str>,
    verified: bool,
) -> Result<()> {
    let gh_token = env::github_token(gh_token)?;

    let (owner, name) = github_api::parse_repo(repo)?;
    github_api::get_repos(&gh_token, &owner, &name)
        .map_err(|e| anyhow!("Failed to get repo: {}/{} caused by: {}", owner, name, e))?;

    info!(
        "Publishing to repo: {}/{}, branch: {}",
        &owner,
        &name,
        branch.as_ref(),
    );

    match github_api::exists_branch(&gh_token, &owner, &name, branch.as_ref()) {
        Ok(_) => {}
        Err(_) => {
            info!("Branch: {} does not exist, creating it", branch.as_ref());
            github_api::create_empty_branch(&gh_token, &owner, &name, branch.as_ref())?;
            info!("Branch: {} created", branch.as_ref());
        }
    }

    let branch_sha = github_api::get_branch_sha(&gh_token, &owner, &name, branch.as_ref())?;
    let latest_commit_sha =
        github_api::get_latest_commit_sha(&gh_token, &owner, &name, branch.as_ref(), None)?;
    let mut trs_response = trs::response::TrsResponse::new(&owner, &name)?;
    for config in configs {
        trs_response.add(&owner, &name, config, verified)?;
    }
    let trs_contents = trs_response.generate_contents()?;
    let new_tree_sha =
        github_api::create_tree(&gh_token, &owner, &name, Some(&branch_sha), trs_contents)?;
    let in_ci = env::in_ci();
    let commit_message = if configs.len() == 1 {
        format!(
            "Publish a workflow {} version {} by gh-trs{}",
            configs[0].id,
            configs[0].version,
            if in_ci { " in CI" } else { "" }
        )
    } else {
        format!(
            "Publish multiple workflows from TRS by gh-trs{}",
            if in_ci { " in CI" } else { "" }
        )
    };
    let new_commit_sha = github_api::create_commit(
        &gh_token,
        &owner,
        &name,
        Some(&latest_commit_sha),
        &new_tree_sha,
        &commit_message,
    )?;
    github_api::update_ref(&gh_token, &owner, &name, branch.as_ref(), &new_commit_sha)?;

    info!(
        "Published to repo: {}/{} branch: {}",
        &owner,
        &name,
        branch.as_ref()
    );
    info!("Please wait for GitHub Pages to be built and published (https://github.com/{}/{}/actions/workflows/pages/pages-build-deployment).", &owner, &name);
    info!(
        "You can get TRS response as:\n    curl -L https://{}.github.io/{}/tools",
        &owner, &name
    );

    Ok(())
}
