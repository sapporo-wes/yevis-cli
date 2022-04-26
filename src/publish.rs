use anyhow::{anyhow, bail, Result};
use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

pub fn publish(
    configs: &Vec<gh_trs::config::types::Config>,
    gh_token: &Option<impl AsRef<str>>,
    repo: impl AsRef<str>,
    verified: bool,
) -> Result<()> {
    let gh_token = gh_trs::env::github_token(gh_token)?;

    let (owner, name) = gh_trs::github_api::parse_repo(repo)?;
    let branch = get_gh_pages_branch(&gh_token, &owner, &name)?;

    info!(
        "Publishing to repo: {}/{}, branch: {}",
        &owner, &name, branch,
    );

    if gh_trs::github_api::exists_branch(&gh_token, &owner, &name, &branch).is_err() {
        info!("Branch {} does not exist, creating it...", &branch);
        gh_trs::github_api::create_empty_branch(&gh_token, &owner, &name, &branch)?;
        info!("Branch {} created", &branch);
    }

    let branch_sha = gh_trs::github_api::get_branch_sha(&gh_token, &owner, &name, &branch)?;
    let latest_commit_sha =
        gh_trs::github_api::get_latest_commit_sha(&gh_token, &owner, &name, &branch, None)?;
    let mut trs_response = gh_trs::trs::response::TrsResponse::new(&owner, &name)?;
    for config in configs {
        trs_response.add(&owner, &name, config, verified)?;
    }
    let trs_contents = generate_trs_contents(trs_response)?;
    let new_tree_sha =
        gh_trs::github_api::create_tree(&gh_token, &owner, &name, Some(&branch_sha), trs_contents)?;
    let mut commit_message = if configs.len() == 1 {
        format!(
            "Publish workflow, id: {} version: {} by yevis",
            configs[0].id, configs[0].version,
        )
    } else {
        "Publish multiple workflows by yevis".to_string()
    };
    if gh_trs::env::in_ci() {
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
    gh_trs::github_api::update_ref(&gh_token, &owner, &name, &branch, &new_commit_sha)?;

    info!(
        "Published to repo: {}/{}, branch: {}",
        &owner, &name, &branch
    );
    Ok(())
}

/// https://docs.github.com/en/rest/reference/pages#get-a-github-pages-site
fn get_gh_pages_branch(
    gh_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/pages",
        owner.as_ref(),
        name.as_ref(),
    ))?;
    let res = match gh_trs::github_api::get_request(gh_token, &url, &[]) {
        Ok(res) => res,
        Err(err) => {
            if err.to_string().contains("Not Found") {
                return Ok("gh-pages".to_string());
            }
            bail!(err);
        }
    };
    let err_msg = "Failed to parse the response when getting the gh-pages branch";
    let branch = res
        .get("source")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("branch")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    Ok(branch.to_string())
}

/// modified from gh-trs::response::TrsResponse::generate_contents
fn generate_trs_contents(
    trs_res: gh_trs::trs::response::TrsResponse,
) -> Result<HashMap<PathBuf, String>> {
    let mut map: HashMap<PathBuf, String> = HashMap::new();
    map.insert(
        PathBuf::from("service-info/index.json"),
        serde_json::to_string(&trs_res.service_info)?,
    );
    map.insert(
        PathBuf::from("toolClasses/index.json"),
        serde_json::to_string(&trs_res.tool_classes)?,
    );
    map.insert(
        PathBuf::from("tools/index.json"),
        serde_json::to_string(&trs_res.tools)?,
    );
    for ((id, version), config) in trs_res.gh_trs_config.iter() {
        let tools_id = trs_res.tools.iter().find(|t| &t.id == id).unwrap();
        let tools_id_versions = tools_id.versions.clone();
        let tools_id_versions_version = tools_id_versions
            .iter()
            .find(|v| &v.version() == version)
            .unwrap();
        let tools_descriptor = trs_res
            .tools_descriptor
            .get(&(*id, version.clone()))
            .unwrap();
        let tools_files = trs_res.tools_files.get(&(*id, version.clone())).unwrap();
        let tools_tests = trs_res.tools_tests.get(&(*id, version.clone())).unwrap();

        let desc_type = config.workflow.language.r#type.clone().unwrap().to_string();

        map.insert(
            PathBuf::from(format!(
                "tools/{}/versions/{}/yevis-metadata.json",
                id, version
            )),
            serde_json::to_string(&config)?,
        );
        map.insert(
            PathBuf::from(format!("tools/{}/index.json", id)),
            serde_json::to_string(&tools_id)?,
        );
        map.insert(
            PathBuf::from(format!("tools/{}/versions/index.json", id)),
            serde_json::to_string(&tools_id_versions)?,
        );
        map.insert(
            PathBuf::from(format!("tools/{}/versions/{}/index.json", id, version)),
            serde_json::to_string(&tools_id_versions_version)?,
        );
        map.insert(
            PathBuf::from(format!(
                "tools/{}/versions/{}/{}/descriptor/index.json",
                id, version, desc_type
            )),
            serde_json::to_string(&tools_descriptor)?,
        );
        map.insert(
            PathBuf::from(format!(
                "tools/{}/versions/{}/{}/files/index.json",
                id, version, desc_type
            )),
            serde_json::to_string(&tools_files)?,
        );
        map.insert(
            PathBuf::from(format!(
                "tools/{}/versions/{}/{}/tests/index.json",
                id, version, desc_type
            )),
            serde_json::to_string(&tools_tests)?,
        );
        map.insert(
            PathBuf::from(format!(
                "tools/{}/versions/{}/containerfile/index.json",
                id, version
            )),
            serde_json::to_string(&Vec::<gh_trs::trs::types::FileWrapper>::new())?,
        );
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_gh_pages_branch() -> Result<()> {
        let gh_token = gh_trs::env::github_token(&None::<String>)?;
        let branch = get_gh_pages_branch(&gh_token, "ddbj", "workflow-registry-dev")?;
        assert_eq!(branch, "gh-pages");
        Ok(())
    }

    #[test]
    fn test_get_gh_pages_branch_no_branch() -> Result<()> {
        let gh_token = gh_trs::env::github_token(&None::<String>)?;
        let branch = get_gh_pages_branch(&gh_token, "ddbj", "yevis-cli")?;
        assert_eq!(branch, "gh-pages");
        Ok(())
    }
}
