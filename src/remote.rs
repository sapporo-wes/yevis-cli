pub mod gh_url;
pub mod gist_url;

pub use gh_url::GitHubUrl;
pub use gh_url::UrlType;
pub use gist_url::GistUrl;

use crate::metadata;

use anyhow::{anyhow, ensure, Result};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};
use url::Url;

pub enum Remote {
    Gist(GistUrl),
    GitHub(GitHubUrl),
    Zenodo(Url),
    Other(Url),
}

impl Remote {
    /// Possible URL:
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
        url: &Url,
        gh_token: impl AsRef<str>,
        branch_memo: Option<&mut HashMap<String, String>>,
        commit_memo: Option<&mut HashMap<String, String>>,
    ) -> Result<Self> {
        let host = url.host_str().ok_or_else(|| anyhow!("No host in URL"))?;
        match host {
            "github.com" | "raw.githubusercontent.com" => Ok(Self::GitHub(GitHubUrl::new(
                url,
                gh_token,
                branch_memo,
                commit_memo,
            )?)),
            "gist.github.com" | "gist.githubusercontent.com" => {
                Ok(Self::Gist(GistUrl::new(url, gh_token)?))
            }
            "zenodo.org" | "sandbox.zenodo.org" => Ok(Self::Zenodo(url.clone())),
            _ => Ok(Self::Other(url.clone())),
        }
    }

    pub fn to_url(&self) -> Result<Url> {
        match self {
            Self::GitHub(gh) => gh.to_url(),
            Self::Gist(gist) => gist.to_url(),
            Self::Zenodo(zenodo) => Ok(zenodo.clone()),
            Self::Other(other) => Ok(other.clone()),
        }
    }

    pub fn to_typed_url(&self, url_type: &UrlType) -> Result<Url> {
        match self {
            Self::GitHub(gh) => gh.to_typed_url(url_type),
            Self::Gist(gist) => gist.to_url(),
            Self::Zenodo(zenodo) => Ok(zenodo.clone()),
            Self::Other(other) => Ok(other.clone()),
        }
    }

    /// https://example.com/dir1/dir2/file.txt
    /// -> file.txt
    pub fn file_name(&self) -> Result<String> {
        let url = self.to_url()?;
        let path = Path::new(url.path());
        let name = path.file_name().ok_or_else(|| anyhow!("No file name"))?;
        Ok(name
            .to_str()
            .ok_or_else(|| anyhow!("No file name"))?
            .to_string())
    }

    pub fn file_prefix(&self) -> Result<String> {
        let name = self.file_name()?;
        let prefix = name
            .split('.')
            .next()
            .ok_or_else(|| anyhow!("No file name"))?
            .to_string();
        Ok(prefix)
    }

    pub fn readme(&self, gh_token: impl AsRef<str>, url_type: &UrlType) -> Result<Url> {
        let default_url = Url::parse("https://example.com/PATH/TO/README.md")?;
        let readme = match self {
            Self::GitHub(gh_url) => gh_url.readme(gh_token, url_type)?,
            Self::Gist(_) => default_url,
            Self::Zenodo(_) => default_url,
            Self::Other(_) => default_url,
        };
        Ok(readme)
    }

    pub fn wf_files(
        &self,
        gh_token: impl AsRef<str>,
        url_type: &UrlType,
    ) -> Result<Vec<metadata::types::File>> {
        match self {
            Self::GitHub(gh_url) => gh_url.wf_files(gh_token, url_type),
            Self::Gist(gist_url) => gist_url.wf_files(gh_token),
            Self::Zenodo(url) => Ok(vec![metadata::types::File::new(
                url,
                &None::<PathBuf>,
                metadata::types::FileType::Primary,
            )?]),
            Self::Other(url) => Ok(vec![metadata::types::File::new(
                url,
                &None::<PathBuf>,
                metadata::types::FileType::Primary,
            )?]),
        }
    }
}

pub fn fetch_json_content(remote_loc: &Url) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(remote_loc.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to fetch json content from {} with status code {}",
        remote_loc.as_str(),
        response.status()
    );

    Ok(response.text()?)
}

pub fn fetch_raw_content(remote_loc: &Url) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(remote_loc.as_str())
        .header(reqwest::header::ACCEPT, "plain/text")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to fetch raw content from {} with status code {}",
        remote_loc.as_str(),
        response.status()
    );

    Ok(response.text()?)
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_fetch_raw_content() -> Result<()> {
        let remote_loc =
            Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md")?;
        let content = fetch_raw_content(&remote_loc)?;
        assert!(content.contains("yevis-cli"));
        Ok(())
    }
}
