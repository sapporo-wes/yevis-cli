use crate::{github_api::get_repos, type_config::Config};
use anyhow::{ensure, Result};
use reqwest;
use serde_yaml;
use url::Url;

pub fn fetch_raw_content(remote_location: impl AsRef<str>) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(remote_location.as_ref())
        .header(reqwest::header::USER_AGENT, "yevis")
        .send()?;
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to fetch contents from {} with status {}",
            remote_location.as_ref(),
            response.status()
        )
    );
    Ok(response.text()?)
}

pub fn fetch_config(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    wf_id: impl AsRef<str>,
    version: impl AsRef<str>,
) -> Result<Config> {
    let branch = get_repos(&github_token, &owner, &name)?.default_branch;
    let remote_location = Url::parse(&format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}/yevis_config_{}.yml",
        owner.as_ref(),
        name.as_ref(),
        &branch,
        wf_id.as_ref(),
        version.as_ref()
    ))?;
    let contents = fetch_raw_content(remote_location.as_str())?;
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_wf_content() -> Result<()> {
        let wf_content =
            fetch_raw_content("https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md")?;
        assert!(wf_content.contains("yevis-cli"));
        Ok(())
    }
}
