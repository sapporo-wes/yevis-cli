use crate::gh;

use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use url::Url;

/// https://docs.github.com/ja/rest/gists/gists#get-a-gist
pub fn get_gist(gh_token: impl AsRef<str>, id: impl AsRef<str>) -> Result<Value> {
    let res = gh::get_request(
        gh_token,
        &Url::parse(&format!("https://api.github.com/gists/{}", id.as_ref()))?,
        &[],
    )?;
    Ok(res)
}

pub fn get_gist_with_version(
    gh_token: impl AsRef<str>,
    id: impl AsRef<str>,
    version: impl AsRef<str>,
) -> Result<Value> {
    let res = gh::get_request(
        gh_token,
        &Url::parse(&format!(
            "https://api.github.com/gists/{}/{}",
            id.as_ref(),
            version.as_ref()
        ))?,
        &[],
    )?;
    Ok(res)
}

pub fn get_owner_and_version(
    gh_token: impl AsRef<str>,
    id: impl AsRef<str>,
) -> Result<(String, String)> {
    let res = get_gist(gh_token, id.as_ref())?;
    let err_msg = "Failed to parse version when getting Gist";
    let history = res
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("history")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_array()
        .ok_or_else(|| anyhow!(err_msg))?
        .get(0)
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?;
    let user = history
        .get("user")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("login")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?
        .to_string();
    let version = history
        .get("version")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?
        .to_string();
    Ok((user, version))
}

/// If Gist contains more than one file, an error is returned.
pub fn get_gist_files(
    gh_token: impl AsRef<str>,
    id: impl AsRef<str>,
    version: &Option<impl AsRef<str>>,
) -> Result<Vec<String>> {
    let res = match version {
        Some(version) => get_gist_with_version(gh_token, id.as_ref(), version)?,
        None => get_gist(gh_token, id.as_ref())?,
    };
    let err_msg = "Failed to parse files when getting Gist";
    let file_names = res
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("files")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .keys()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    Ok(file_names)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::env;

//     #[test]
//     fn test_get_gist() -> Result<()> {
//         let gh_token = env::github_token(&None::<String>)?;
//         let id = "9c6aa4ba5d7464066d55175f59e428ac";
//         get_gist(gh_token, id)?;
//         Ok(())
//     }

//     #[test]
//     fn test_get_gist_raw_url_single() -> Result<()> {
//         let gh_token = env::github_token(&None::<String>)?;
//         let id = "cdd4bcbb6f13ae797947cd7981e35b5f";
//         let raw_url = get_gist_raw_url(gh_token, id)?;
//         assert_eq!(
//             raw_url,
//             "https://gist.githubusercontent.com/suecharo/cdd4bcbb6f13ae797947cd7981e35b5f/raw/330cd87f6b5dc90614cecfd36bca0c60f5c50622/trimming_and_qc.cwl"
//         );
//         Ok(())
//     }

//     #[test]
//     fn test_get_gist_raw_url_multiple() -> Result<()> {
//         let gh_token = env::github_token(&None::<String>)?;
//         let id = "9c6aa4ba5d7464066d55175f59e428ac";
//         let result = get_gist_raw_url(gh_token, id);
//         assert!(result.is_err());
//         Ok(())
//     }
// }
