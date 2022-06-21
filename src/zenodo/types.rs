use crate::metadata;

use anyhow::{ensure, Result};
use crypto::digest::Digest;
use crypto::md5::Md5;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time;
use url::Url;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Deposition {
    pub upload_type: String,
    pub title: String,
    pub creators: Vec<Creator>,
    pub description: String,
    pub access_right: String,
    pub license: String,
    pub keywords: Vec<String>,
    pub communities: Vec<Community>,
    pub version: String,
}

impl Deposition {
    pub fn new(
        meta: &metadata::types::Metadata,
        repo: impl AsRef<str>,
        zenodo_community: &Option<impl AsRef<str>>,
    ) -> Self {
        let communities = match zenodo_community {
            Some(zenodo_community) => vec![Community {
                identifier: zenodo_community.as_ref().to_string(),
            }],
            None => vec![],
        };
        Self {
            upload_type: "dataset".to_string(),
            title: meta.id.to_string(),
            creators: meta.authors.iter().map(Creator::new).collect(),
            description: format!(
                r#"These data sets are one of the workflows in <a href="https://github.com/{}">{}</a>."#,
                repo.as_ref(),
                repo.as_ref()
            ),
            access_right: "open".to_string(),
            license: meta.license.clone(),
            keywords: vec!["yevis-workflow".to_string()],
            communities,
            version: meta.version.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub name: String,
    pub affiliation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
}

impl Creator {
    fn new(author: &metadata::types::Author) -> Self {
        Self {
            name: author.name.clone(),
            affiliation: author.affiliation.clone(),
            orcid: author.orcid.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Community {
    pub identifier: String,
}

pub enum DepositionStatus {
    Draft,
    Published,
}

impl fmt::Display for DepositionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DepositionStatus::Draft => write!(f, "draft"),
            DepositionStatus::Published => write!(f, "published"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct DepositionFile {
    pub id: String,
    pub filename: String,
    pub filesize: u64,
    pub checksum: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MetaFile {
    pub filename: String,
    pub file_path: PathBuf,
    pub checksum: String,
}

impl MetaFile {
    pub fn new_from_url(file_url: &Url, target: impl AsRef<Path>) -> Result<Self> {
        // timeout is set to 60 * 60 seconds
        let client = reqwest::blocking::Client::builder()
            .timeout(time::Duration::from_secs(3600))
            .build()?;
        let res = client.get(file_url.as_str()).send()?;
        let status = res.status();
        let res_bytes = res.bytes()?;
        ensure!(
            status.is_success(),
            "Failed to download file from {} with status: {}",
            file_url.as_str(),
            status
        );

        let (mut file, file_path) = tempfile::NamedTempFile::new()?.keep()?;
        file.write_all(&res_bytes)?;

        let mut md5 = Md5::new();
        md5.input(&res_bytes);
        let checksum = md5.result_str();

        Ok(Self {
            filename: target
                .as_ref()
                .iter()
                .map(|x| x.to_string_lossy())
                .collect::<Vec<_>>()
                .join("_"),
            file_path,
            checksum,
        })
    }

    pub fn new_from_str(content: impl AsRef<str>, target: impl AsRef<Path>) -> Result<Self> {
        let content_bytes = content.as_ref().as_bytes();

        let (mut file, file_path) = tempfile::NamedTempFile::new()?.keep()?;
        file.write_all(content_bytes)?;

        let mut md5 = Md5::new();
        md5.input(content_bytes);
        let checksum = md5.result_str();

        Ok(Self {
            filename: target
                .as_ref()
                .iter()
                .map(|x| x.to_string_lossy())
                .collect::<Vec<_>>()
                .join("_"),
            file_path,
            checksum,
        })
    }
}
