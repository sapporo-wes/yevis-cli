use crate::github_api;

use anyhow::{anyhow, ensure, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub struct RawUrl {
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub commit: String,
    pub file_path: PathBuf,
}

pub enum UrlType {
    Branch,
    Commit,
}

impl RawUrl {
    /// Parse the workflow location.
    /// The workflow location should be in the format of:
    ///
    /// - https://github.com/<owner>/<name>/blob/<branch>/<path_to_file>
    /// - https://github.com/<owner>/<name>/blob/<commit_hash>/<path_to_file>
    /// - https://github.com/<owner>/<name>/tree/<branch>/<path_to_file>
    /// - https://github.com/<owner>/<name>/tree/<commit_hash>/<path_to_file>
    /// - https://github.com/<owner>/<name>/raw/<branch>/<path_to_file>
    /// - https://github.com/<owner>/<name>/raw/<commit_hash>/<path_to_file>
    /// - https://raw.githubusercontent.com/<owner>/<name>/<branch>/<path_to_file>
    /// - https://raw.githubusercontent.com/<owner>/<name>/<commit_hash>/<path_to_file>
    pub fn new(
        gh_token: impl AsRef<str>,
        url: &Url,
        branch_memo: Option<&mut HashMap<String, String>>,
        commit_memo: Option<&mut HashMap<String, String>>,
    ) -> Result<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("No host found in URL: {}", url))?;
        ensure!(
            host == "github.com" || host == "raw.githubusercontent.com",
            "Only GitHub URLs are supported, your input URL: {}",
            url
        );
        let path_segments = url
            .path_segments()
            .ok_or_else(|| anyhow!("Failed to parse URL path: {}", url))?
            .collect::<Vec<_>>();
        let owner = path_segments
            .get(0)
            .ok_or_else(|| anyhow!("No repo owner found in URL: {}", url))?
            .to_string();
        let name = path_segments
            .get(1)
            .ok_or_else(|| anyhow!("No repo name found in URL: {}", url))?
            .to_string();
        let branch_or_commit = match host {
            "github.com" => path_segments
                .get(3)
                .ok_or_else(|| anyhow!("No branch or commit found in URL: {}", url))?,
            "raw.githubusercontent.com" => path_segments
                .get(2)
                .ok_or_else(|| anyhow!("No branch or commit found in URL: {}", url))?,
            _ => unreachable!(),
        };
        let (branch, commit) = match is_commit_hash(&branch_or_commit) {
            Ok(_) => {
                let commit = branch_or_commit.to_string();
                let branch = github_api::get_default_branch(gh_token, &owner, &name, branch_memo)?;
                (branch, commit)
            }
            Err(_) => {
                let branch = branch_or_commit.to_string();
                let commit = github_api::get_latest_commit_sha(
                    gh_token,
                    &owner,
                    &name,
                    &branch,
                    commit_memo,
                )?;
                (branch, commit)
            }
        };
        let file_path = match host {
            "github.com" => PathBuf::from(path_segments[4..].join("/")),
            "raw.githubusercontent.com" => PathBuf::from(path_segments[3..].join("/")),
            _ => unreachable!(),
        };
        Ok(Self {
            owner,
            name,
            branch,
            commit,
            file_path,
        })
    }

    pub fn file_stem(&self) -> Result<String> {
        Ok(self
            .file_path
            .file_stem()
            .ok_or_else(|| {
                anyhow!(
                    "Failed to get file stem from {} ",
                    self.file_path.to_string_lossy()
                )
            })?
            .to_string_lossy()
            .to_string())
    }

    pub fn base_dir(&self) -> Result<PathBuf> {
        Ok(self
            .file_path
            .parent()
            .ok_or_else(|| {
                anyhow!(
                    "Failed to get parent dir from {} ",
                    self.file_path.to_string_lossy()
                )
            })?
            .to_path_buf())
    }

    // UrlType::Branch
    // -> https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md
    // UrlType::Commit
    // -> https://raw.githubusercontent.com/ddbj/yevis-cli/<commit_hash>/README.md
    pub fn to_url(&self, url_type: &UrlType) -> Result<Url> {
        Ok(Url::parse(&format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.owner,
            self.name,
            match url_type {
                UrlType::Branch => &self.branch,
                UrlType::Commit => &self.commit,
            },
            self.file_path.to_string_lossy()
        ))?)
    }

    pub fn to_base_url(&self, url_type: &UrlType) -> Result<Url> {
        let path = format!(
            "{}/{}/{}/{}",
            self.owner,
            self.name,
            match url_type {
                UrlType::Branch => &self.branch,
                UrlType::Commit => &self.commit,
            },
            self.file_path
                .parent()
                .ok_or_else(|| anyhow!(
                    "Failed to get parent dir from {}",
                    self.file_path.to_string_lossy()
                ))?
                .to_string_lossy()
        );
        // remove trailing slash
        let path = path.trim_end_matches('/');
        // need to add a trailing slash to make it a valid URL
        let url = Url::parse(&format!(
            "
            https://raw.githubusercontent.com/{}/",
            path
        ))?;
        Ok(url)
    }
}

/// Check if input is a valid commit SHA.
pub fn is_commit_hash(hash: impl AsRef<str>) -> Result<()> {
    let re = Regex::new(r"^[0-9a-f]{40}$")?;
    ensure!(
        re.is_match(hash.as_ref()),
        "Not a valid commit hash: {}",
        hash.as_ref()
    );
    Ok(())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_raw_url() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let owner = "ddbj".to_string();
        let name = "yevis-cli".to_string();
        let branch = "main".to_string();
        let commit = "f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9".to_string();
        let file_path = PathBuf::from("path/to/workflow.yml");

        let url_1 = Url::parse(&format!(
            "https://github.com/{}/{}/blob/{}/{}",
            &owner,
            &name,
            &branch,
            &file_path.to_string_lossy()
        ))?;
        let url_2 = Url::parse(&format!(
            "https://github.com/{}/{}/blob/{}/{}",
            &owner,
            &name,
            &commit,
            &file_path.to_string_lossy()
        ))?;
        let url_3 = Url::parse(&format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            &owner,
            &name,
            &branch,
            &file_path.to_string_lossy()
        ))?;
        let url_4 = Url::parse(&format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            &owner,
            &name,
            &commit,
            &file_path.to_string_lossy()
        ))?;

        let raw_url_1 = RawUrl::new(&gh_token, &url_1, None, None)?;
        let raw_url_2 = RawUrl::new(&gh_token, &url_2, None, None)?;
        let raw_url_3 = RawUrl::new(&gh_token, &url_3, None, None)?;
        let raw_url_4 = RawUrl::new(&gh_token, &url_4, None, None)?;

        let expect = RawUrl {
            owner,
            name,
            branch,
            commit,
            file_path,
        };

        assert_eq!(raw_url_1.owner, expect.owner);
        assert_eq!(raw_url_1.name, expect.name);
        assert_eq!(raw_url_1.branch, expect.branch);
        assert_eq!(raw_url_1.file_path, expect.file_path);

        assert_eq!(raw_url_2, expect);

        assert_eq!(raw_url_3.owner, expect.owner);
        assert_eq!(raw_url_3.name, expect.name);
        assert_eq!(raw_url_3.branch, expect.branch);
        assert_eq!(raw_url_3.file_path, expect.file_path);

        assert_eq!(raw_url_4, expect);

        Ok(())
    }

    #[test]
    fn test_raw_url_invalid_url() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://example.com/path/to/file")?;
        let err = RawUrl::new(&gh_token, &url, None, None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Only GitHub URLs are supported, your input URL: https://example.com/path/to/file"
        );
        Ok(())
    }

    #[test]
    fn test_raw_url_invalid_host() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://example.com/path/to/file")?;
        let err = RawUrl::new(&gh_token, &url, None, None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Only GitHub URLs are supported, your input URL: https://example.com/path/to/file"
        );
        Ok(())
    }

    #[test]
    fn test_raw_url_invalid_path() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url =
            Url::parse("https://github.com/ddbj/yevis-cli/blob/invalid_branch/path/to/workflow")?;
        assert!(RawUrl::new(&gh_token, &url, None, None).is_err());
        Ok(())
    }

    #[test]
    fn test_is_commit_hash() -> Result<()> {
        let commit = "f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9";
        is_commit_hash(commit)?;
        Ok(())
    }

    #[test]
    fn test_base_dir() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let owner = "ddbj".to_string();
        let name = "yevis-cli".to_string();
        let commit = "f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9".to_string();
        let file_path = PathBuf::from("path/to/workflow.yml");
        let url = Url::parse(&format!(
            "https://github.com/{}/{}/blob/{}/{}",
            &owner,
            &name,
            &commit,
            &file_path.to_string_lossy()
        ))?;
        let raw_url = RawUrl::new(&gh_token, &url, None, None)?;
        let base_dir = raw_url.base_dir()?;
        assert_eq!(base_dir, PathBuf::from("path/to"));
        Ok(())
    }

    #[test]
    fn test_to_url() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let owner = "ddbj".to_string();
        let name = "yevis-cli".to_string();
        let commit = "f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9".to_string();
        let file_path = PathBuf::from("path/to/workflow.yml");
        let url = Url::parse(&format!(
            "https://github.com/{}/{}/blob/{}/{}",
            &owner,
            &name,
            &commit,
            &file_path.to_string_lossy()
        ))?;
        let raw_url = RawUrl::new(&gh_token, &url, None, None)?;
        let to_url = raw_url.to_url(&UrlType::Commit)?;
        assert_eq!(
            to_url,
            Url::parse(&format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}",
                &owner,
                &name,
                &commit,
                &file_path.to_string_lossy()
            ))?
        );
        Ok(())
    }

    #[test]
    fn test_to_base_url() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let owner = "ddbj".to_string();
        let name = "yevis-cli".to_string();
        let commit = "f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9".to_string();
        let file_path = PathBuf::from("path/to/workflow.yml");
        let url = Url::parse(&format!(
            "https://github.com/{}/{}/blob/{}/{}",
            &owner,
            &name,
            &commit,
            &file_path.to_string_lossy()
        ))?;
        let raw_url = RawUrl::new(&gh_token, &url, None, None)?;
        let to_url = raw_url.to_base_url(&UrlType::Commit)?;
        assert_eq!(
            to_url,
            Url::parse(&format!(
                "https://raw.githubusercontent.com/{}/{}/{}/path/to/",
                &owner, &name, &commit,
            ))?
        );
        Ok(())
    }
}
