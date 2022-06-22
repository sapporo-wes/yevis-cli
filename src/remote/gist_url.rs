use crate::gh;
use crate::metadata;

use anyhow::{anyhow, bail, ensure, Result};
use regex::Regex;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub struct GistUrl {
    pub id: String,
    pub owner: String,
    pub version: String,
    pub file_path: PathBuf,
}

impl GistUrl {
    /// Parse the workflow location.
    /// The workflow location should be in the format of:
    ///
    /// - https://gist.github.com/<id>
    /// - https://gist.github.com/<id>/raw/
    /// - https://gist.github.com/<owner>/<id>
    /// - https://gist.github.com/<owner>/<id>/raw/
    /// - https://gist.github.com/<owner>/<id>/raw/<filename>
    /// - https://gist.github.com/<owner>/<id>/raw/<version>/<filename>
    /// - https://gist.githubusercontent.com/<id>
    /// - https://gist.githubusercontent.com/<owner>/<id>
    /// - https://gist.githubusercontent.com/<owner>/<id>/raw/
    /// - https://gist.githubusercontent.com/<owner>/<id>/raw/<version>/<filename>
    ///
    /// Single file Gist ID: cdd4bcbb6f13ae797947cd7981e35b5f
    /// Multiple files Gist ID: 9c6aa4ba5d7464066d55175f59e428ac
    /// Version example: a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1
    pub fn new(url: &Url, gh_token: impl AsRef<str>) -> Result<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("Invalid URL: {}", url))?;
        ensure!(
            host == "gist.github.com" || host == "gist.githubusercontent.com",
            "Host {} is not supported",
            url
        );
        let (owner, id) = extract_gist_id(url)?;

        let mut version = None;
        let mut file_path = None;
        let version_re = Regex::new(r"^[a-f0-9]{40}$")?;
        let path_segments = url
            .path_segments()
            .ok_or_else(|| anyhow!("No path found in your input URL: {}", url))?;
        for segment in path_segments.into_iter().skip(2) {
            if segment == "raw" {
                continue;
            } else if version_re.is_match(segment) {
                version = Some(segment.to_string());
            } else {
                file_path = Some(PathBuf::from(segment));
            }
        }

        let (api_owner, api_version) = gh::gist::get_owner_and_version(&gh_token, &id)?;
        let (owner, version) = match (owner, version) {
            (Some(owner), Some(version)) => (owner, version),
            (Some(owner), None) => (owner, api_version),
            (None, Some(version)) => (api_owner, version),
            (None, None) => (api_owner, api_version),
        };
        let file_path = match file_path {
            Some(file_path) => file_path,
            None => {
                let files = gh::gist::get_gist_files(&gh_token, &id, &Some(&version))?;
                ensure!(
                    files.len() == 1,
                    "Gist {} has multiple files, please specify a file path",
                    id
                );
                PathBuf::from(files[0].clone())
            }
        };

        Ok(Self {
            id,
            owner,
            version,
            file_path,
        })
    }

    /// Call complement before calling this function.
    pub fn to_url(&self) -> Result<Url> {
        Ok(Url::parse(&format!(
            "https://gist.githubusercontent.com/{}/{}/raw/{}/{}",
            self.owner,
            self.id,
            self.version,
            self.file_path.to_string_lossy()
        ))?)
    }

    pub fn wf_files(&self, gh_token: impl AsRef<str>) -> Result<Vec<metadata::types::File>> {
        let files = gh::gist::get_gist_files(&gh_token, &self.id, &Some(self.version.clone()))?;
        files
            .iter()
            .map(|file| -> Result<metadata::types::File> {
                let mut gist_url = self.clone();
                gist_url.file_path = PathBuf::from(file);
                let url = gist_url.to_url()?;
                let r#type = if self.file_path == gist_url.file_path {
                    metadata::types::FileType::Primary
                } else {
                    metadata::types::FileType::Secondary
                };
                metadata::types::File::new(&url, &Some(gist_url.file_path), r#type)
            })
            .collect::<Result<Vec<_>>>()
    }
}

/// gist_id example: 9c6aa4ba5d7464066d55175f59e428ac
/// Return: (owner, gist_id)
fn extract_gist_id(url: &Url) -> Result<(Option<String>, String)> {
    let gist_id_re = Regex::new(r"^[a-f0-9]{32}$")?;
    let path_segments = url
        .path_segments()
        .ok_or_else(|| anyhow!("No path found in your input URL: {}", url))?
        .collect::<Vec<_>>();
    let err_msg = format!("No gist_id found in your input URL: {}", url);
    match path_segments.get(0) {
        Some(segment) => {
            if gist_id_re.is_match(segment) {
                Ok((None, segment.to_string()))
            } else {
                match path_segments.get(1) {
                    Some(segment) => {
                        if gist_id_re.is_match(segment) {
                            Ok((
                                Some(path_segments.get(0).unwrap().to_string()),
                                segment.to_string(),
                            ))
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

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_extract_gist_id() -> Result<()> {
        let url = Url::parse("https://gist.github.com/9c6aa4ba5d7464066d55175f59e428ac")?;
        assert_eq!(
            extract_gist_id(&url)?,
            (None, "9c6aa4ba5d7464066d55175f59e428ac".to_string())
        );
        let url = Url::parse("https://gist.github.com/9c6aa4ba5d7464066d55175f59e428ac/raw/")?;
        assert_eq!(
            extract_gist_id(&url)?,
            (None, "9c6aa4ba5d7464066d55175f59e428ac".to_string())
        );
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac")?;
        assert_eq!(
            extract_gist_id(&url)?,
            (
                Some("suecharo".to_string()),
                "9c6aa4ba5d7464066d55175f59e428ac".to_string()
            )
        );
        let url =
            Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/")?;
        assert_eq!(
            extract_gist_id(&url)?,
            (
                Some("suecharo".to_string()),
                "9c6aa4ba5d7464066d55175f59e428ac".to_string()
            )
        );
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl")?;
        assert_eq!(
            extract_gist_id(&url)?,
            (
                Some("suecharo".to_string()),
                "9c6aa4ba5d7464066d55175f59e428ac".to_string()
            )
        );
        Ok(())
    }

    #[test]
    fn test_gist_url_new_single() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://gist.github.com/cdd4bcbb6f13ae797947cd7981e35b5f")?;
        let gist_url = GistUrl::new(&url, gh_token)?;
        assert_eq!(
            gist_url,
            GistUrl {
                id: "cdd4bcbb6f13ae797947cd7981e35b5f".to_string(),
                owner: "suecharo".to_string(),
                version: "8aa64e99bb2e8fc0bc56e486f798197363854074".to_string(),
                file_path: PathBuf::from("trimming_and_qc.cwl"),
            }
        );
        Ok(())
    }

    #[test]
    fn test_gist_url_new_multiple() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl")?;
        let gist_url = GistUrl::new(&url, gh_token)?;
        assert_eq!(
            gist_url,
            GistUrl {
                id: "9c6aa4ba5d7464066d55175f59e428ac".to_string(),
                owner: "suecharo".to_string(),
                version: "a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1".to_string(),
                file_path: PathBuf::from("trimming_and_qc.cwl"),
            }
        );
        Ok(())
    }

    #[test]
    fn test_gist_url_wf_files() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://gist.github.com/suecharo/9c6aa4ba5d7464066d55175f59e428ac/raw/a8848dfc4c4b8d5dc07bf286d6076e0846b2c7d1/trimming_and_qc.cwl")?;
        let gist_url = GistUrl::new(&url, &gh_token)?;
        let files = gist_url.wf_files(&gh_token)?;
        assert_eq!(files.len(), 3);
        Ok(())
    }
}
