use anyhow::Result;
use gh_trs;
use log::info;

pub fn publish(
    configs: &Vec<gh_trs::config::types::Config>,
    gh_token: &Option<impl AsRef<str>>,
    repo: impl AsRef<str>,
    branch: impl AsRef<str>,
    verified: bool,
) -> Result<()> {
    let gh_token = gh_trs::env::github_token(gh_token)?;

    let (owner, name) = gh_trs::github_api::parse_repo(repo)?;

    info!(
        "Publishing to repo: {}/{}, branch: {}",
        &owner,
        &name,
        branch.as_ref(),
    );

    match gh_trs::github_api::exists_branch(&gh_token, &owner, &name, branch.as_ref()) {
        Ok(_) => {}
        Err(_) => {
            info!("Branch {} does not exist, creating it", branch.as_ref());
            gh_trs::github_api::create_empty_branch(&gh_token, &owner, &name, branch.as_ref())?;
            info!("Branch {} created", branch.as_ref());
        }
    }

    let branch_sha = gh_trs::github_api::get_branch_sha(&gh_token, &owner, &name, branch.as_ref())?;
    let latest_commit_sha =
        gh_trs::github_api::get_latest_commit_sha(&gh_token, &owner, &name, branch.as_ref(), None)?;
    let mut trs_response = gh_trs::trs::response::TrsResponse::new(&owner, &name)?;
    for config in configs {
        trs_response.add(&owner, &name, &config, verified)?;
    }
    let trs_contents = trs_response.generate_contents()?;
    let new_tree_sha =
        gh_trs::github_api::create_tree(&gh_token, &owner, &name, Some(&branch_sha), trs_contents)?;
    let in_ci = gh_trs::env::in_ci();
    let mut commit_message = if configs.len() == 1 {
        format!(
            "Publish a workflow, id: {} version: {} by gh-trs",
            configs[0].id, configs[0].version,
        )
    } else {
        "Publish multiple workflows by gh-trs".to_string()
    };
    if in_ci {
        commit_message.push_str(" in CI");
    }
    let new_commit_sha = gh_trs::github_api::create_commit(
        &gh_token,
        &owner,
        &name,
        Some(&latest_commit_sha),
        &new_tree_sha,
        &commit_message,
    )?;
    gh_trs::github_api::update_ref(&gh_token, &owner, &name, branch.as_ref(), &new_commit_sha)?;

    info!(
        "Published to repo: {}/{}, branch: {}",
        &owner,
        &name,
        branch.as_ref()
    );
    Ok(())
}
