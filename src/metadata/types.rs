use crate::gh;
use crate::inspect;
use crate::remote;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::fmt;
use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;

#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub id: Uuid,
    pub version: String,
    pub license: String,
    pub authors: Vec<Author>,
    pub zenodo: Option<Zenodo>,
    pub workflow: Workflow,
}

impl Metadata {
    pub fn new(
        wf_loc: &Url,
        gh_token: impl AsRef<str>,
        url_type: &remote::UrlType,
    ) -> Result<Self> {
        let primary_wf = remote::Remote::new(wf_loc, &gh_token, None, None)?;
        Ok(Self {
            id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            license: "CC0-1.0".to_string(),
            authors: vec![Author::new_via_api(&gh_token)?],
            zenodo: None,
            workflow: Workflow {
                name: primary_wf.file_prefix()?,
                readme: primary_wf.readme(&gh_token, url_type)?,
                language: inspect::inspect_wf_type_version(&primary_wf.to_url()?)?,
                files: primary_wf.wf_files(&gh_token, url_type)?,
                testing: vec![Testing::default()],
            },
        })
    }
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Author {
    pub github_account: String,
    pub name: String,
    pub affiliation: String,
    pub orcid: Option<String>,
}

impl Author {
    pub fn new_via_api(gh_token: impl AsRef<str>) -> Result<Self> {
        let (github_account, name, affiliation) = gh::api::get_author_info(gh_token)?;
        Ok(Self {
            github_account,
            name,
            affiliation,
            orcid: Some("PUT YOUR ORCID OR REMOVE THIS LINE".to_string()),
        })
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub readme: Url,
    pub language: Language,
    pub files: Vec<File>,
    pub testing: Vec<Testing>,
}

impl Workflow {
    pub fn primary_wf(&self) -> Result<File> {
        Ok(self
            .files
            .iter()
            .find(|f| f.is_primary())
            .ok_or_else(|| anyhow!("No primary workflow file"))?
            .clone())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Language {
    pub r#type: LanguageType,
    pub version: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LanguageType {
    Cwl,
    Wdl,
    Nfl,
    Smk,
    Unknown,
}

impl fmt::Display for LanguageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageType::Cwl => write!(f, "CWL"),
            LanguageType::Wdl => write!(f, "WDL"),
            LanguageType::Nfl => write!(f, "NFL"),
            LanguageType::Smk => write!(f, "SMK"),
            LanguageType::Unknown => write!(f, "UNKNOWN, PLEASE FILL"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct File {
    pub url: Url,
    pub target: Option<PathBuf>,
    pub r#type: FileType,
}

impl File {
    pub fn new(url: &Url, target: &Option<impl AsRef<Path>>, r#type: FileType) -> Result<Self> {
        let target = match target {
            Some(target) => target.as_ref().to_path_buf(),
            None => {
                let path = Path::new(url.path());
                PathBuf::from(path.file_name().ok_or_else(|| anyhow!("No file name"))?)
            }
        };
        Ok(Self {
            url: url.clone(),
            target: Some(target),
            r#type,
        })
    }

    pub fn is_primary(&self) -> bool {
        self.r#type == FileType::Primary
    }

    pub fn complement_target(&mut self) -> Result<()> {
        if self.target.is_none() {
            let path = Path::new(self.url.path());
            let target = PathBuf::from(path.file_name().ok_or_else(|| anyhow!("No file name"))?);
            self.target = Some(target);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Primary,
    Secondary,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Testing {
    pub id: String,
    pub files: Vec<TestFile>,
}

impl Default for Testing {
    fn default() -> Self {
        Self {
            id: "test_1".to_string(),
            files: vec![
                TestFile::new(
                    &Url::parse("https://example.com/path/to/wf_params.json").unwrap(),
                    &None::<PathBuf>,
                    TestFileType::WfParams,
                )
                .unwrap(),
                TestFile::new(
                    &Url::parse("https://example.com/path/to/wf_engine_params.json").unwrap(),
                    &None::<PathBuf>,
                    TestFileType::WfEngineParams,
                )
                .unwrap(),
                TestFile::new(
                    &Url::parse("https://example.com/path/to/data.fq").unwrap(),
                    &None::<PathBuf>,
                    TestFileType::Other,
                )
                .unwrap(),
            ],
        }
    }
}

impl Testing {
    pub fn wf_params(&self) -> Result<String> {
        match self
            .files
            .iter()
            .find(|f| f.r#type == TestFileType::WfParams)
        {
            Some(f) => remote::fetch_raw_content(&f.url),
            None => Ok("{}".to_string()),
        }
    }

    pub fn wf_engine_params(&self) -> Result<String> {
        match self
            .files
            .iter()
            .find(|f| f.r#type == TestFileType::WfEngineParams)
        {
            Some(f) => remote::fetch_raw_content(&f.url),
            None => Ok("{}".to_string()),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TestFile {
    pub url: Url,
    pub target: Option<PathBuf>,
    pub r#type: TestFileType,
}

impl TestFile {
    pub fn new(url: &Url, target: &Option<impl AsRef<Path>>, r#type: TestFileType) -> Result<Self> {
        let target = match target {
            Some(target) => target.as_ref().to_path_buf(),
            None => {
                let path = Path::new(url.path());
                PathBuf::from(path.file_name().ok_or_else(|| anyhow!("No file name"))?)
            }
        };
        Ok(Self {
            url: url.clone(),
            target: Some(target),
            r#type,
        })
    }

    pub fn complement_target(&mut self) -> Result<()> {
        if self.target.is_none() {
            let path = Path::new(self.url.path());
            let target = PathBuf::from(path.file_name().ok_or_else(|| anyhow!("No file name"))?);
            self.target = Some(target);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestFileType {
    WfParams,
    WfEngineParams,
    Other,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Zenodo {
    pub url: Url,
    pub id: u64,
    pub doi: String,
    pub concept_doi: String,
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_file_new() -> Result<()> {
        let url = Url::parse("https://example.com/path/to/file.txt")?;
        let target = Some(PathBuf::from("path/to/file.txt"));
        let file = File::new(&url, &target, FileType::Primary)?;
        assert_eq!(file.url, url);
        assert_eq!(file.target, target);
        assert_eq!(file.r#type, FileType::Primary);
        Ok(())
    }

    #[test]
    fn test_file_new_no_target() -> Result<()> {
        let url = Url::parse("https://example.com/path/to/file.txt")?;
        let file = File::new(&url, &None::<PathBuf>, FileType::Primary)?;
        assert_eq!(file.url, url);
        assert_eq!(file.target, Some(PathBuf::from("file.txt")));
        assert_eq!(file.r#type, FileType::Primary);
        Ok(())
    }

    #[test]
    fn test_testing_default() -> Result<()> {
        let testing = Testing::default();
        assert_eq!(testing.id, "test_1");
        assert_eq!(testing.files.len(), 3);
        Ok(())
    }

    #[test]
    fn test_test_file_new() -> Result<()> {
        let url = Url::parse("https://example.com/path/to/file.txt")?;
        let target = Some(PathBuf::from("path/to/file.txt"));
        let file = TestFile::new(&url, &target, TestFileType::WfParams)?;
        assert_eq!(file.url, url);
        assert_eq!(file.target, target);
        assert_eq!(file.r#type, TestFileType::WfParams);
        Ok(())
    }

    #[test]
    fn test_test_file_no_target() -> Result<()> {
        let url = Url::parse("https://example.com/path/to/file.txt")?;
        let file = TestFile::new(&url, &None::<PathBuf>, TestFileType::WfParams)?;
        assert_eq!(file.url, url);
        assert_eq!(file.target, Some(PathBuf::from("file.txt")));
        assert_eq!(file.r#type, TestFileType::WfParams);
        Ok(())
    }
}
