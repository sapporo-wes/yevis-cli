use crate::gh;

use anyhow::{anyhow, Result};
use url::Url;

/// https://docs.github.com/en/rest/reference/pulls#list-pull-requests-files
pub fn list_modified_files(
    gh_token: impl AsRef<str>,
    pr_url: impl AsRef<str>,
) -> Result<Vec<String>> {
    let pr_url = Url::parse(pr_url.as_ref())?;
    let err_msg = "Failed to parse Pull Request URL";
    let path_segments = pr_url
        .path_segments()
        .ok_or_else(|| anyhow!(err_msg))?
        .collect::<Vec<_>>();
    let repo_owner = path_segments.get(0).ok_or_else(|| anyhow!(err_msg))?;
    let repo_name = path_segments.get(1).ok_or_else(|| anyhow!(err_msg))?;
    let pr_number = path_segments
        .get(3)
        .ok_or_else(|| anyhow!(err_msg))?
        .parse::<u64>()
        .map_err(|_| anyhow!(err_msg))?;

    let url = Url::parse(&format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/files",
        repo_owner, repo_name, pr_number
    ))?;
    let res = gh::get_request(gh_token, &url, &[])?;
    let err_msg = "Failed to parse the response when listing modified files";
    let raw_urls: Vec<String> = res
        .as_array()
        .ok_or_else(|| anyhow!(err_msg))?
        .iter()
        .map(|x| {
            x.as_object()
                .ok_or_else(|| anyhow!(err_msg))
                .and_then(|x| x.get("raw_url").ok_or_else(|| anyhow!(err_msg)))
                .and_then(|x| x.as_str().ok_or_else(|| anyhow!(err_msg)))
                .map(|x| x.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(raw_urls)
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     #[ignore]
//     fn test_list_modified_files() -> Result<()> {
//         let pr_url = Url::parse("https://github.com/ddbj/workflow-registry-dev/pull/15")?;
//         list_modified_files(&None::<String>, &pr_url)?;
//         Ok(())
//     }
// }
