use crate::gh_trs;
use crate::gh_trs::raw_url;
use crate::gh_trs::raw_url::RawUrl as GitHubUrl;

use anyhow::{anyhow, bail, ensure, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub struct GistUrl {
    pub id: String,
    pub owner: String,
    pub raw_url: Url,
}

impl GistUrl {
    /// Accept Url:
    /// - https://gist.github.com/9c6aa4ba5d7464066d55175f59e428ac
    /// - https://gist.github.com/9c6aa4ba5d7464066d55175f59e428ac/raw/
    /// - https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac
    /// - https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/
    /// - https://gist.githubusercontent.com/9c6aa4ba5d7464066d55175f59e428ac
    /// - https://gist.githubusercontent.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac
    /// - https://gist.githubusercontent.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/
    /// - https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl
    /// - https://gist.githubusercontent.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl
    ///
    /// Single file Gist: cdd4bcbb6f13ae797947cd7981e35b5f
    /// Multiple files Gist: 9c6aa4ba5d7464066d55175f59e428ac
    pub fn new(gh_token: impl AsRef<str>, url: &Url) -> Result<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("No host found in your input URL: {}", url))?;
        ensure!(
            host == "gist.github.com" || host == "gist.githubusercontent.com",
            "Only Gist URL is supported, but found: {}",
            url,
        );
        let gist_id = extract_gist_id(url)?;
        let path_length = url
            .path_segments()
            .ok_or_else(|| anyhow!("No path found in your input URL: {}", url))?
            .count();
        let raw_url = if path_length > 3 {
            // Input URL is a raw URL
            url.to_string().replace(
                "https://gist.github.com",
                "https://gist.githubusercontent.com",
            )
        } else {
            // Obtain raw URL from GitHub API
            get_gist_raw_url(gh_token, &gist_id)?
        };
        let raw_url = Url::parse(&raw_url)?;
        let owner = raw_url
            .path_segments()
            .ok_or_else(|| anyhow!("No path found in your input URL: {}", url))?
            .next()
            .ok_or_else(|| anyhow!("No owner found in your input URL: {}", url))?;
        Ok(Self {
            id: gist_id,
            owner: owner.to_string(),
            raw_url,
        })
    }

    pub fn file_stem(&self) -> Result<String> {
        let file_name = self
            .raw_url
            .path()
            .split('/')
            .last()
            .ok_or_else(|| anyhow!("No file name found in your raw URL: {}", self.raw_url))?;
        Ok(file_name.to_string())
    }

    pub fn wf_files(&self, gh_token: impl AsRef<str>) -> Result<Vec<gh_trs::config::types::File>> {
        let primary_target = self.file_stem()?;
        let res = get_gist(gh_token, &self.id)?;
        let err_msg = "Failed to parse raw_url when getting Gist";
        res.as_object()
            .ok_or_else(|| anyhow!(err_msg))?
            .get("files")
            .ok_or_else(|| anyhow!(err_msg))?
            .as_object()
            .ok_or_else(|| anyhow!(err_msg))?
            .values()
            .into_iter()
            .map(|file| -> Result<gh_trs::config::types::File> {
                let target = file
                    .get("filename")
                    .ok_or_else(|| anyhow!(err_msg))?
                    .as_str()
                    .ok_or_else(|| anyhow!(err_msg))?;
                let url = file
                    .get("raw_url")
                    .ok_or_else(|| anyhow!(err_msg))?
                    .as_str()
                    .ok_or_else(|| anyhow!(err_msg))?;
                let r#type = if target == primary_target {
                    gh_trs::config::types::FileType::Primary
                } else {
                    gh_trs::config::types::FileType::Secondary
                };
                gh_trs::config::types::File::new(&Url::parse(url)?, &Some(target), r#type)
            })
            .collect::<Result<Vec<_>>>()
    }
}

/// gist_id example: 9c6aa4ba5d7464066d55175f59e428ac
fn extract_gist_id(url: &Url) -> Result<String> {
    let gist_id_re = Regex::new(r"^[a-f0-9]{32}$")?;
    let path_segments = url
        .path_segments()
        .ok_or_else(|| anyhow!("No path found in your input URL: {}", url))?
        .collect::<Vec<_>>();
    let err_msg = format!("No gist_id found in your input URL: {}", url);
    match path_segments.get(0) {
        Some(segment) => {
            if gist_id_re.is_match(segment) {
                Ok(segment.to_string())
            } else {
                match path_segments.get(1) {
                    Some(segment) => {
                        if gist_id_re.is_match(segment) {
                            Ok(segment.to_string())
                        } else {
                            bail!(err_msg)
                        }
                    }
                    None => bail!(err_msg),
                }
            }
        }
        None => {
            bail!(err_msg)
        }
    }
}

/// https://docs.github.com/ja/rest/gists/gists#get-a-gist
fn get_gist(gh_token: impl AsRef<str>, gist_id: impl AsRef<str>) -> Result<Value> {
    let res = gh_trs::github_api::get_request(
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
fn get_gist_raw_url(gh_token: impl AsRef<str>, gist_id: impl AsRef<str>) -> Result<String> {
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

pub enum FileUrl {
    Gist(GistUrl),
    GitHub(GitHubUrl),
    Zenodo(Url),
    Other(Url),
}

impl FileUrl {
    /// Accept Url:
    ///   - GitHub:
    ///     - https://github.com/...
    ///     - https://raw.githubusercontent.com/...
    ///   - Gist:
    ///     - https://gist.github.com/...
    ///     - https://gist.githubusercontent.com/...
    ///   - Zenodo:
    ///     - https://zenodo.org/...
    ///     - https://sandbox.zenodo.org/...
    ///   - Other:
    ///     - https://...
    pub fn new(
        gh_token: impl AsRef<str>,
        url: &Url,
        branch_memo: Option<&mut HashMap<String, String>>,
        commit_memo: Option<&mut HashMap<String, String>>,
    ) -> Result<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("No host found in your input URL: {}", url))?;
        let file_url = if host == "github.com" || host == "raw.githubusercontent.com" {
            Self::GitHub(GitHubUrl::new(gh_token, url, branch_memo, commit_memo)?)
        } else if host == "gist.github.com" || host == "gist.githubusercontent.com" {
            Self::Gist(GistUrl::new(gh_token, url)?)
        } else if host == "zenodo.org" {
            Self::Zenodo(url.clone())
        } else {
            Self::Other(url.clone())
        };
        Ok(file_url)
    }

    pub fn file_stem(&self) -> Result<String> {
        match self {
            Self::GitHub(url) => url.file_stem(),
            Self::Gist(url) => url.file_stem(),
            Self::Zenodo(url) => {
                let file_name = url
                    .path()
                    .split('/')
                    .last()
                    .ok_or_else(|| anyhow!("No file name found in your raw URL: {}", url))?;
                Ok(file_name.to_string())
            }
            Self::Other(url) => {
                let file_name = url
                    .path()
                    .split('/')
                    .last()
                    .ok_or_else(|| anyhow!("No file name found in your raw URL: {}", url))?;
                Ok(file_name.to_string())
            }
        }
    }

    pub fn file_name(&self) -> Result<String> {
        Ok(self
            .file_stem()?
            .split('.')
            .next()
            .ok_or_else(|| anyhow!("No file name found in your raw URL"))?
            .to_string())
    }

    pub fn readme(
        &self,
        gh_token: impl AsRef<str>,
        url_type: &gh_trs::raw_url::UrlType,
    ) -> Result<Url> {
        let readme = match self {
            Self::GitHub(url) => gh_trs::raw_url::RawUrl::new(
                &gh_token,
                &gh_trs::github_api::get_readme_url(&gh_token, &url.owner, &url.name)?,
                None,
                None,
            )?
            .to_url(url_type)?,
            Self::Gist(_) => Url::parse("https://example.com/PATH/TO/README.md")?,
            Self::Zenodo(_) => Url::parse("https://example.com/PATH/TO/README.md")?,
            Self::Other(_) => Url::parse("https://example.com/PATH/TO/README.md")?,
        };
        Ok(readme)
    }

    pub fn to_url(&self, url_type: &gh_trs::raw_url::UrlType) -> Result<Url> {
        match self {
            Self::GitHub(url) => Ok(url.to_url(url_type)?),
            Self::Gist(url) => Ok(url.raw_url.clone()),
            Self::Zenodo(url) => Ok(url.clone()),
            Self::Other(url) => Ok(url.clone()),
        }
    }

    pub fn wf_files(
        &self,
        gh_token: impl AsRef<str>,
        url_type: &gh_trs::raw_url::UrlType,
    ) -> Result<Vec<gh_trs::config::types::File>> {
        match self {
            Self::GitHub(url) => obtain_wf_files(&gh_token, url, url_type),
            Self::Gist(url) => url.wf_files(&gh_token),
            Self::Zenodo(url) => Ok(vec![gh_trs::config::types::File::new(
                url,
                &Some(self.file_stem()?),
                gh_trs::config::types::FileType::Primary,
            )?]),
            Self::Other(url) => Ok(vec![gh_trs::config::types::File::new(
                url,
                &Some(self.file_stem()?),
                gh_trs::config::types::FileType::Primary,
            )?]),
        }
    }
}

pub fn obtain_wf_files(
    gh_token: impl AsRef<str>,
    primary_wf: &raw_url::RawUrl,
    url_type: &raw_url::UrlType,
) -> Result<Vec<gh_trs::config::types::File>> {
    let primary_wf_url = primary_wf.to_url(url_type)?;
    let base_dir = primary_wf.base_dir()?;
    let base_url = primary_wf.to_base_url(url_type)?;
    let files = gh_trs::github_api::get_file_list_recursive(
        gh_token,
        &primary_wf.owner,
        &primary_wf.name,
        &base_dir,
        &primary_wf.commit,
    )?;
    files
        .into_iter()
        .map(|file| -> Result<gh_trs::config::types::File> {
            let target = file.strip_prefix(&base_dir)?;
            let url = base_url.join(target.to_str().ok_or_else(|| anyhow!("Invalid URL"))?)?;
            let r#type = if url == primary_wf_url {
                gh_trs::config::types::FileType::Primary
            } else {
                gh_trs::config::types::FileType::Secondary
            };
            gh_trs::config::types::File::new(&url, &Some(target), r#type)
        })
        .collect::<Result<Vec<_>>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_extract_gist_id() -> Result<()> {
        let url = Url::parse("https://gist.github.com/9c6aa4ba5d7464066d55175f59e428ac")?;
        assert_eq!(extract_gist_id(&url)?, "9c6aa4ba5d7464066d55175f59e428ac");
        let url = Url::parse("https://gist.github.com/9c6aa4ba5d7464066d55175f59e428ac/raw/")?;
        assert_eq!(extract_gist_id(&url)?, "9c6aa4ba5d7464066d55175f59e428ac");
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac")?;
        assert_eq!(extract_gist_id(&url)?, "9c6aa4ba5d7464066d55175f59e428ac");
        let url =
            Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/")?;
        assert_eq!(extract_gist_id(&url)?, "9c6aa4ba5d7464066d55175f59e428ac");
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl")?;
        assert_eq!(extract_gist_id(&url)?, "9c6aa4ba5d7464066d55175f59e428ac");
        Ok(())
    }

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

    #[test]
    fn test_gist_url_new_single() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://gist.github.com/cdd4bcbb6f13ae797947cd7981e35b5f")?;
        let gist_url = GistUrl::new(gh_token, &url)?;
        assert_eq!(
            gist_url,
            GistUrl {
                 id: "cdd4bcbb6f13ae797947cd7981e35b5f".to_string(),
                 owner: "suecharo".to_string(),
                 raw_url: Url::parse("https://gist.githubusercontent.com/suecharo/cdd4bcbb6f13ae797947cd7981e35b5f/raw/330cd87f6b5dc90614cecfd36bca0c60f5c50622/trimming_and_qc.cwl")?,
            }
        );
        Ok(())
    }

    #[test]
    fn test_gist_url_new_multiple() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl")?;
        let gist_url = GistUrl::new(gh_token, &url)?;
        assert_eq!(
            gist_url,
            GistUrl {
                 id: "9c6aa4ba5d7464066d55175f59e428ac".to_string(),
                 owner: "suecharo".to_string(),
                 raw_url: Url::parse("https://gist.githubusercontent.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl")?,
            }
        );
        Ok(())
    }

    #[test]
    fn test_gist_url_wf_files() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl")?;
        let gist_url = GistUrl::new(&gh_token, &url)?;
        let files = gist_url.wf_files(&gh_token)?;
        assert_eq!(files.len(), 3);
        Ok(())
    }

    #[test]
    fn test_obtain_wf_files() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let primary_wf = raw_url::RawUrl::new(
            &gh_token,
            &Url::parse(
                "https://github.com/suecharo/gh-trs/blob/main/tests/CWL/wf/trimming_and_qc.cwl",
            )?,
            None,
            None,
        )?;
        let files = obtain_wf_files(&gh_token, &primary_wf, &raw_url::UrlType::Commit)?;
        assert_eq!(files.len(), 3);
        Ok(())
    }
}
