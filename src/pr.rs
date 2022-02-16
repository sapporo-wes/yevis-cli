use anyhow::{anyhow, Result};
use gh_trs;
use url::Url;

/// https://docs.github.com/en/rest/reference/pulls#list-pull-requests-files
pub fn list_modified_files(
    gh_token: &Option<impl AsRef<str>>,
    pr_url: impl AsRef<str>,
) -> Result<Vec<String>> {
    let pr_url = Url::parse(pr_url.as_ref())?;
    let path_segments = pr_url
        .path_segments()
        .ok_or(anyhow!("Failed to get PR number"))?
        .collect::<Vec<_>>();
    let repo_owner = path_segments
        .get(0)
        .ok_or(anyhow!("Failed to get repo owner from PR URL"))?;
    let repo_name = path_segments
        .get(1)
        .ok_or(anyhow!("Failed to get repo name from PR URL"))?;
    let pr_number = path_segments
        .get(3)
        .ok_or(anyhow!("Failed to get PR number from PR URL"))?
        .parse::<u64>()
        .map_err(|_| anyhow!("Failed to parse PR number from PR URL"))?;

    let gh_token = gh_trs::env::github_token(gh_token)?;
    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/files",
        repo_owner, repo_name, pr_number
    ))?;
    let res = gh_trs::github_api::get_request(gh_token, &url, &[])?;
    let err_msg = "Failed to parse the response when listing modified files";
    let raw_urls: Vec<String> = res
        .as_array()
        .ok_or(anyhow!(err_msg))?
        .into_iter()
        .map(|x| {
            x.as_object()
                .ok_or(anyhow!(err_msg))
                .and_then(|x| x.get("raw_url").ok_or(anyhow!(err_msg)))
                .and_then(|x| x.as_str().ok_or(anyhow!(err_msg)))
                .map(|x| x.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(raw_urls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_list_modified_files() -> Result<()> {
        let pr_url = Url::parse("https://github.com/ddbj/yevis-workflows-dev/pull/15")?;
        list_modified_files(&None::<String>, &pr_url)?;
        Ok(())
    }
}