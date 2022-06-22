use crate::gh;
use crate::metadata;

use anyhow::{anyhow, ensure, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub struct GitHubUrl {
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub commit: String,
    pub file_path: PathBuf,
    pub ori_url_type: UrlType,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UrlType {
    Branch,
    Commit,
}

impl GitHubUrl {
    /// Parse the workflow location.
    /// The workflow location should be in the format of:
    ///
    /// - https://github.com/<owner>/<name>/blob/<branch>/<path_to_file>
    /// - https://github.com/<owner>/<name>/blob/<commit>/<path_to_file>
    /// - https://github.com/<owner>/<name>/tree/<branch>/<path_to_file>
    /// - https://github.com/<owner>/<name>/tree/<commit>/<path_to_file>
    /// - https://github.com/<owner>/<name>/raw/<branch>/<path_to_file>
    /// - https://github.com/<owner>/<name>/raw/<commit>/<path_to_file>
    /// - https://raw.githubusercontent.com/<owner>/<name>/<branch>/<path_to_file>
    /// - https://raw.githubusercontent.com/<owner>/<name>/<commit>/<path_to_file>
    pub fn new(
        url: &Url,
        gh_token: impl AsRef<str>,
        branch_memo: Option<&mut HashMap<String, String>>,
        commit_memo: Option<&mut HashMap<String, String>>,
    ) -> Result<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("Invalid URL: {}", url))?;
        ensure!(
            host == "github.com" || host == "raw.githubusercontent.com",
            "Host {} is not supported",
            host
        );
        let path_segments = url
            .path_segments()
            .ok_or_else(|| anyhow!("No path segments in URL"))?
            .collect::<Vec<_>>();
        let owner = path_segments
            .get(0)
            .ok_or_else(|| anyhow!("No repo owner in URL"))?
            .to_string();
        let name = path_segments
            .get(1)
            .ok_or_else(|| anyhow!("No repo name in URL"))?
            .to_string();
        let branch_or_commit = match host {
            "github.com" => path_segments
                .get(3)
                .ok_or_else(|| anyhow!("No branch or commit in URL"))?
                .to_owned(),
            "raw.githubusercontent.com" => path_segments
                .get(2)
                .ok_or_else(|| anyhow!("No branch or commit in URL"))?
                .to_owned(),
            _ => unreachable!(),
        };
        let (branch, commit, ori_url_type) = match is_commit_hash(branch_or_commit)? {
            true => {
                let branch = gh::api::get_default_branch(&gh_token, &owner, &name, branch_memo)?;
                let commit = branch_or_commit.to_string();
                (branch, commit, UrlType::Commit)
            }
            false => {
                let branch = branch_or_commit.to_string();
                let commit =
                    gh::api::get_latest_commit_sha(&gh_token, &owner, &name, &branch, commit_memo)?;
                (branch, commit, UrlType::Branch)
            }
        };
        let file_path = match host {
            "github.com" => path_segments.into_iter().skip(4).collect(),
            "raw.githubusercontent.com" => path_segments.into_iter().skip(3).collect(),
            _ => unreachable!(),
        };

        Ok(Self {
            owner,
            name,
            branch,
            commit,
            file_path,
            ori_url_type,
        })
    }

    /// default: UrlType::Branch
    pub fn to_url(&self) -> Result<Url> {
        self.to_typed_url(&self.ori_url_type)
    }

    /// Call complement before calling this function.
    ///
    /// UrlType::Branch
    /// -> https://raw.githubusercontent.com/<owner>/<name>/<branch>/<path_to_file>
    /// UrlType::Commit
    /// -> https://raw.githubusercontent.com/<owner>/<name>/<commit>/<path_to_file>
    pub fn to_typed_url(&self, url_type: &UrlType) -> Result<Url> {
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

    pub fn readme(&self, gh_token: impl AsRef<str>, url_type: &UrlType) -> Result<Url> {
        let readme_url = gh::api::get_readme_url(&gh_token, &self.owner, &self.name)?;
        let readme_remote = Self::new(&readme_url, &gh_token, None, None)?;
        readme_remote.to_typed_url(url_type)
    }

    pub fn wf_files(
        &self,
        gh_token: impl AsRef<str>,
        url_type: &UrlType,
    ) -> Result<Vec<metadata::types::File>> {
        let primary_wf_url = self.to_typed_url(url_type)?;
        let path_parent = self.file_path.parent().ok_or_else(|| {
            anyhow!(
                "No parent path in file path: {}",
                self.file_path.to_string_lossy()
            )
        })?;
        let files = gh::api::get_file_list_recursive(
            &gh_token,
            &self.owner,
            &self.name,
            path_parent,
            &self.commit,
        )?;
        files
            .iter()
            .map(|file| -> Result<metadata::types::File> {
                let mut gh_url = self.clone();
                gh_url.file_path = file.to_path_buf();
                let url = gh_url.to_typed_url(url_type)?;
                let target = file.strip_prefix(&path_parent)?;
                let r#type = if primary_wf_url == url {
                    metadata::types::FileType::Primary
                } else {
                    metadata::types::FileType::Secondary
                };
                metadata::types::File::new(&url, &Some(target.to_path_buf()), r#type)
            })
            .collect::<Result<Vec<_>>>()
    }
}

/// Check if input is a valid commit SHA.
pub fn is_commit_hash(hash: impl AsRef<str>) -> Result<bool> {
    let re = Regex::new(r"^[0-9a-f]{40}$")?;
    Ok(re.is_match(hash.as_ref()))
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_gh_url() -> Result<()> {
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

        let raw_url_1 = GitHubUrl::new(&url_1, &gh_token, None, None)?;
        let raw_url_2 = GitHubUrl::new(&url_2, &gh_token, None, None)?;
        let raw_url_3 = GitHubUrl::new(&url_3, &gh_token, None, None)?;
        let raw_url_4 = GitHubUrl::new(&url_4, &gh_token, None, None)?;

        let expect_branch = GitHubUrl {
            owner,
            name,
            branch,
            commit,
            file_path,
            ori_url_type: UrlType::Branch,
        };
        let mut expect_commit = expect_branch.clone();
        expect_commit.ori_url_type = UrlType::Commit;

        assert_eq!(raw_url_1.owner, expect_branch.owner);
        assert_eq!(raw_url_1.name, expect_branch.name);
        assert_eq!(raw_url_1.branch, expect_branch.branch);
        assert_eq!(raw_url_1.file_path, expect_branch.file_path);
        assert_eq!(raw_url_1.ori_url_type, expect_branch.ori_url_type);
        assert_eq!(raw_url_2, expect_commit);
        assert_eq!(raw_url_3.owner, expect_branch.owner);
        assert_eq!(raw_url_3.name, expect_branch.name);
        assert_eq!(raw_url_3.branch, expect_branch.branch);
        assert_eq!(raw_url_3.file_path, expect_branch.file_path);
        assert_eq!(raw_url_3.ori_url_type, expect_branch.ori_url_type);
        assert_eq!(raw_url_4, expect_commit);

        Ok(())
    }

    #[test]
    fn test_gh_url_invalid_url() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://example.com/path/to/file")?;
        let err = GitHubUrl::new(&url, &gh_token, None, None).unwrap_err();
        assert_eq!(err.to_string(), "Host example.com is not supported");
        Ok(())
    }

    #[test]
    fn test_gh_url_invalid_host() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url = Url::parse("https://example.com/path/to/file")?;
        let err = GitHubUrl::new(&url, &gh_token, None, None).unwrap_err();
        assert_eq!(err.to_string(), "Host example.com is not supported");
        Ok(())
    }

    #[test]
    fn test_gh_url_invalid_path() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let url =
            Url::parse("https://github.com/ddbj/yevis-cli/blob/invalid_branch/path/to/workflow")?;
        assert!(GitHubUrl::new(&url, &gh_token, None, None).is_err());
        Ok(())
    }

    #[test]
    fn test_is_commit_hash() -> Result<()> {
        let commit = "f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9f9";
        is_commit_hash(commit)?;
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
        let raw_url = GitHubUrl::new(&url, &gh_token, None, None)?;
        let to_url = raw_url.to_url()?;
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
}
