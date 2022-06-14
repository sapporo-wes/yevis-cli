use crate::gh;

use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use url::Url;

/// https://docs.github.com/ja/rest/gists/gists#get-a-gist
pub fn get_gist(gh_token: impl AsRef<str>, gist_id: impl AsRef<str>) -> Result<Value> {
    let res = gh::get_request(
        gh_token,
        &Url::parse(&format!(
            "https://api.github.com/gists/{}",
            gist_id.as_ref()
        ))?,
        &[],
    )?;
    Ok(res)
}

/// If Gist contains more than one file, an error is returned.
pub fn get_gist_raw_url(gh_token: impl AsRef<str>, gist_id: impl AsRef<str>) -> Result<String> {
    let res = get_gist(gh_token, gist_id.as_ref())?;
    let err_msg = "Failed to parse raw_url when getting Gist";
    let mut files = res
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("files")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .values();
    if files.len() != 1 {
        bail!("Gist ID {} contains more than one file; please specify the Gist raw URL containing the file path", gist_id.as_ref())
    } else {
        Ok(files
            .next()
            .ok_or_else(|| anyhow!(err_msg))?
            .get("raw_url")
            .ok_or_else(|| anyhow!(err_msg))?
            .as_str()
            .ok_or_else(|| anyhow!(err_msg))?
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_get_gist() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let gist_id = "9c6aa4ba5d7464066d55175f59e428ac";
        get_gist(gh_token, gist_id)?;
        Ok(())
    }

    #[test]
    fn test_get_gist_raw_url_single() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let gist_id = "cdd4bcbb6f13ae797947cd7981e35b5f";
        let raw_url = get_gist_raw_url(gh_token, gist_id)?;
        assert_eq!(
            raw_url,
            "https://gist.githubusercontent.com/suecharo/cdd4bcbb6f13ae797947cd7981e35b5f/raw/330cd87f6b5dc90614cecfd36bca0c60f5c50622/trimming_and_qc.cwl"
        );
        Ok(())
    }

    #[test]
    fn test_get_gist_raw_url_multiple() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let gist_id = "9c6aa4ba5d7464066d55175f59e428ac";
        let result = get_gist_raw_url(gh_token, gist_id);
        assert!(result.is_err());
        Ok(())
    }
}
