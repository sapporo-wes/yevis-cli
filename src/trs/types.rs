use crate::env;
use crate::metadata;

use anyhow::{ensure, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

/// https://raw.githubusercontent.com/ga4gh-discovery/ga4gh-service-info/v1.0.0/service-info.yaml#/paths/~1service-info
#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub r#type: ServiceType,
    pub description: Option<String>,
    pub organization: Organization,
    pub contact_url: Option<Url>,
    pub documentation_url: Option<Url>,
    #[serde(serialize_with = "serialize_date_time")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(serialize_with = "serialize_date_time")]
    pub updated_at: Option<DateTime<Utc>>,
    pub environment: Option<String>,
    pub version: String,
}

pub fn serialize_date_time<S>(dt: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match dt {
        Some(dt) => serializer.serialize_str(&format!("{}", dt.format("%Y-%m-%dT%H:%M:%SZ"))),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ServiceType {
    pub group: String,
    pub artifact: String,
    pub version: String,
}

impl Default for ServiceType {
    fn default() -> Self {
        Self {
            group: "yevis".to_string(),
            artifact: "yevis".to_string(),
            version: "2.0.1".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub name: String,
    pub url: Url,
}

impl ServiceInfo {
    pub fn new(owner: impl AsRef<str>, name: impl AsRef<str>) -> Result<Self> {
        let created_at = Utc::now();
        Ok(Self {
            id: format!("io.github.{}.{}", owner.as_ref(), name.as_ref()),
            name: format!(
                "Yevis workflow registry {}/{}",
                owner.as_ref(),
                name.as_ref()
            ),
            r#type: ServiceType::default(),
            description: Some(
                "The GA4GH TRS API generated by Yevis (https://github.com/sapporo-wes/yevis-cli)"
                    .to_string(),
            ),
            organization: Organization {
                name: owner.as_ref().to_string(),
                url: Url::parse(&format!("https://github.com/{}", owner.as_ref(),))?,
            },
            contact_url: None,
            documentation_url: None,
            created_at: Some(created_at),
            updated_at: Some(created_at),
            environment: None,
            version: created_at.format("%Y%m%d%H%M%S").to_string(),
        })
    }

    /// Basically, prev has priority in all fields.
    /// This is only for service-info, because there may be cases where to modify service-info by hand.
    pub fn new_or_update(
        prev: Option<Self>,
        owner: impl AsRef<str>,
        name: impl AsRef<str>,
    ) -> Result<Self> {
        let mut new = Self::new(owner, name)?;
        if let Some(prev) = prev {
            if prev.name == "Yevis workflow registry sapporo-wes/yevis-workflow-registry-template" {
                // do nothing
            } else {
                new.id = prev.id;
                new.name = prev.name;
                new.r#type = prev.r#type;
                new.description = prev.description;
                new.organization = prev.organization;
                new.contact_url = prev.contact_url;
                new.documentation_url = prev.documentation_url;
                new.created_at = prev.created_at;
                new.environment = prev.environment;
            }
        };
        Ok(new)
    }
}

// --- GA4GH TRS API v2.0.1 type definition ---
// https://editor.swagger.io/?url=https://raw.githubusercontent.com/ga4gh/tool-registry-schemas/develop/openapi/openapi.yaml

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Checksum {
    pub checksum: String,
    pub r#type: String,
}

impl Checksum {
    pub fn new_from_string(s: impl AsRef<str>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(s.as_ref().as_bytes());
        let checksum = format!("{:x}", hasher.finalize());
        Self {
            checksum,
            r#type: "sha256".to_string(),
        }
    }

    pub fn new_from_url(url: &Url) -> Result<Self> {
        let res = reqwest::blocking::get(url.as_str())?;
        ensure!(
            res.status().is_success(),
            "Failed to get {} with status {}",
            url,
            res.status()
        );
        let mut hasher = Sha256::new();
        hasher.update(res.bytes()?);
        let checksum = format!("{:x}", hasher.finalize());
        Ok(Self {
            checksum,
            r#type: "sha256".to_string(),
        })
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum FileType {
    #[serde(rename = "TEST_FILE")]
    TestFile,
    #[serde(rename = "PRIMARY_DESCRIPTOR")]
    PrimaryDescriptor,
    #[serde(rename = "SECONDARY_DESCRIPTOR")]
    SecondaryDescriptor,
    #[serde(rename = "CONTAINERFILE")]
    Containerfile,
    #[serde(rename = "OTHER")]
    Other,
}

impl FileType {
    pub fn new_from_file_type(file_type: &metadata::types::FileType) -> Self {
        match file_type {
            metadata::types::FileType::Primary => FileType::PrimaryDescriptor,
            metadata::types::FileType::Secondary => FileType::SecondaryDescriptor,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ToolFile {
    pub path: Option<Url>,
    pub file_type: Option<FileType>,
    pub checksum: Option<Checksum>,
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ToolClass {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Default for ToolClass {
    fn default() -> Self {
        ToolClass {
            id: Some("workflow".to_string()),
            name: Some("Workflow".to_string()),
            description: Some("A computational workflow".to_string()),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub url: Url,
    pub id: Uuid,
    pub aliases: Option<Vec<String>>,
    pub organization: String,
    pub name: Option<String>,
    pub toolclass: ToolClass,
    pub description: Option<Url>,
    pub meta_version: Option<String>,
    pub has_checker: Option<bool>,
    pub checker_url: Option<Url>,
    pub versions: Vec<ToolVersion>,
}

impl Tool {
    pub fn new(
        meta: &metadata::types::Metadata,
        owner: impl AsRef<str>,
        name: impl AsRef<str>,
    ) -> Result<Self> {
        let organization = meta
            .authors
            .iter()
            .map(|a| format!("@{}", a.github_account))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(Self {
            url: Url::parse(&format!(
                "https://{}.github.io/{}/tools/{}",
                owner.as_ref(),
                name.as_ref(),
                meta.id,
            ))?,
            id: meta.id,
            aliases: None,
            organization,
            name: Some(meta.workflow.name.clone()),
            toolclass: ToolClass::default(),
            description: Some(meta.workflow.readme.clone()),
            meta_version: None,
            has_checker: Some(true),
            checker_url: Some(Url::parse("https://github.com/sapporo-wes/yevis-cli")?),
            versions: vec![],
        })
    }

    /// Scans for versions field and updates them based on the version of the meta.
    /// If the same version already exists, it will be overwritten.
    pub fn add_new_tool_version(
        &mut self,
        meta: &metadata::types::Metadata,
        owner: impl AsRef<str>,
        name: impl AsRef<str>,
        verified: bool,
    ) -> Result<()> {
        let mut versions = self
            .versions
            .clone()
            .into_iter()
            .filter(|v| v.version() != meta.version)
            .collect::<Vec<ToolVersion>>();
        let has_same_version = self.versions.iter().any(|v| v.version() == meta.version);
        if has_same_version {
            // update
            let mut same_version = self
                .versions
                .iter()
                .find(|v| v.version() == meta.version)
                .unwrap()
                .clone();
            same_version.update(meta, &owner, &name, verified)?;
            versions.push(same_version);
        } else {
            // new
            versions.push(ToolVersion::new(meta, &owner, &name, verified)?);
        }
        self.versions = versions;
        Ok(())
    }
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ToolVersion {
    pub author: Option<Vec<String>>,
    pub name: Option<String>,
    pub url: Url,
    pub id: String, // Version
    pub is_production: Option<bool>,
    pub images: Option<Vec<ImageData>>,
    pub descriptor_type: Option<Vec<DescriptorType>>,
    pub containerfile: Option<bool>,
    pub meta_version: Option<String>,
    pub verified: Option<bool>,
    pub verified_source: Option<Vec<String>>,
    pub signed: Option<bool>,
    pub included_apps: Option<Vec<String>>,
}

impl ToolVersion {
    pub fn new(
        meta: &metadata::types::Metadata,
        owner: impl AsRef<str>,
        name: impl AsRef<str>,
        verified: bool,
    ) -> Result<Self> {
        let verified_source = if verified {
            if env::in_ci() {
                match env::gh_actions_url() {
                    Ok(url) => Some(vec![url.to_string()]),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            author: Some(
                meta.authors
                    .iter()
                    .map(|a| a.github_account.clone())
                    .collect::<Vec<String>>(),
            ),
            name: Some(meta.workflow.name.clone()),
            url: Url::parse(&format!(
                "https://{}.github.io/{}/tools/{}/versions/{}",
                owner.as_ref(),
                name.as_ref(),
                meta.id,
                &meta.version
            ))?,
            id: meta.version.clone(),
            is_production: None,
            images: None,
            descriptor_type: Some(vec![DescriptorType::new(&meta.workflow.language.r#type)]),
            containerfile: None,
            meta_version: None,
            verified: Some(verified),
            verified_source,
            signed: None,
            included_apps: None,
        })
    }

    pub fn update(
        &mut self,
        meta: &metadata::types::Metadata,
        owner: impl AsRef<str>,
        name: impl AsRef<str>,
        verified: bool,
    ) -> Result<()> {
        let new_verified_source = if verified {
            if env::in_ci() {
                match env::gh_actions_url() {
                    Ok(url) => Some(vec![url.to_string()]),
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };
        let merged_verified_source = match (self.verified_source.clone(), new_verified_source) {
            (Some(prev), Some(new)) => Some(prev.into_iter().chain(new).collect()),
            (Some(prev), None) => Some(prev),
            (None, Some(new)) => Some(new),
            (None, None) => None,
        };

        self.author = Some(
            meta.authors
                .iter()
                .map(|a| a.github_account.clone())
                .collect::<Vec<String>>(),
        );
        self.name = Some(meta.workflow.name.clone());
        self.url = Url::parse(&format!(
            "https://{}.github.io/{}/tools/{}/versions/{}",
            owner.as_ref(),
            name.as_ref(),
            meta.id,
            &meta.version
        ))?;
        self.id = meta.version.clone();
        self.descriptor_type = Some(vec![DescriptorType::new(&meta.workflow.language.r#type)]);
        self.verified = match merged_verified_source {
            Some(_) => Some(true),
            None => Some(false),
        };
        self.verified_source = merged_verified_source;
        Ok(())
    }

    pub fn version(&self) -> String {
        let path_segments = self.url.path_segments().unwrap();
        path_segments.last().unwrap().to_string()
    }
}

#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub registry_host: Option<String>,
    pub image_name: Option<String>,
    pub size: Option<String>,
    pub updated: Option<String>,
    pub checksum: Option<Checksum>,
    pub image_type: Option<ImageType>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ImageType {
    Docker,
    Singularity,
    Conda,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DescriptorType {
    Cwl,
    Wdl,
    Nfl,
    Smk,
    Galaxy,
    Unknown,
}

impl DescriptorType {
    pub fn new(wf_type: &metadata::types::LanguageType) -> Self {
        match wf_type {
            metadata::types::LanguageType::Cwl => DescriptorType::Cwl,
            metadata::types::LanguageType::Wdl => DescriptorType::Wdl,
            metadata::types::LanguageType::Nfl => DescriptorType::Nfl,
            metadata::types::LanguageType::Smk => DescriptorType::Smk,
            metadata::types::LanguageType::Unknown => DescriptorType::Unknown,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DescriptorTypeWithPlain {
    Cwl,
    Wdl,
    Nfl,
    Smk,
    Galaxy,
    PlainCwl,
    PlainWdl,
    PlainNfl,
    PlainSmk,
    PlainGalaxy,
}

/// One of url or content is required.
#[skip_serializing_none]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FileWrapper {
    pub content: Option<String>, // The content of the file itself. One of url or content is required.
    pub checksum: Option<Vec<Checksum>>,
    pub url: Option<Url>,
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_new_or_update_service_info() -> Result<()> {
        let service_info = ServiceInfo::new_or_update(None, "test_owner", "test_name")?;

        let expect = serde_json::from_str::<ServiceInfo>(
            r#"
{
  "id": "io.github.test_owner.test_name",
  "name": "Yevis workflow registry test_owner/test_name",
  "type": {
    "group": "yevis",
    "artifact": "yevis",
    "version": "2.0.1"
  },
  "description": "The GA4GH TRS API generated by Yevis (https://github.com/sapporo-wes/yevis-cli)",
  "organization": {
    "name": "test_owner",
    "url": "https://github.com/test_owner"
  },
  "createdAt": "2022-02-07T14:05:57Z",
  "updatedAt": "2022-02-07T14:05:57Z",
  "version": "20220207140557"
}"#,
        )?;
        assert_eq!(service_info.id, expect.id);
        assert_eq!(service_info.name, expect.name);
        assert_eq!(service_info.r#type, expect.r#type);
        assert_eq!(service_info.description, expect.description);
        assert_eq!(service_info.organization, expect.organization);
        Ok(())
    }

    #[test]
    fn test_file_type_new_from_file_type() -> Result<()> {
        let file_type = FileType::new_from_file_type(&metadata::types::FileType::Primary);
        assert_eq!(file_type, FileType::PrimaryDescriptor);
        let file_type = FileType::new_from_file_type(&metadata::types::FileType::Secondary);
        assert_eq!(file_type, FileType::SecondaryDescriptor);
        Ok(())
    }

    #[test]
    fn test_default_tool_class() -> Result<()> {
        let tool_class = ToolClass::default();
        assert_eq!(tool_class.id, Some("workflow".to_string()));
        assert_eq!(tool_class.name, Some("Workflow".to_string()));
        assert_eq!(
            tool_class.description,
            Some("A computational workflow".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_tool_new() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        let tool = Tool::new(&meta, "test_owner", "test_name")?;

        let expect = serde_json::from_str::<Tool>(
            r#"
{
  "url": "https://test_owner.github.io/test_name/tools/c13b6e27-a4ee-426f-8bdb-8cf5c4310bad",
  "id": "c13b6e27-a4ee-426f-8bdb-8cf5c4310bad",
  "organization": "@suecharo",
  "name": "CWL_trimming_and_qc",
  "toolclass": {
    "id": "workflow",
    "name": "Workflow",
    "description": "A computational workflow"
  },
  "description": "https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/README.md",
  "has_checker": true,
  "checker_url": "https://github.com/sapporo-wes/yevis-cli",
  "versions": []
}
"#,
        )?;
        assert_eq!(tool, expect);
        Ok(())
    }

    #[test]
    fn test_tool_add_new_tool_version() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        let mut tool = Tool::new(&meta, "test_owner", "test_name")?;
        tool.add_new_tool_version(&meta, "test_owner", "test_name", true)?;
        assert_eq!(tool.versions.len(), 1);
        tool.add_new_tool_version(&meta, "test_owner", "test_name", true)?;
        assert_eq!(tool.versions.len(), 1);

        let expect = serde_json::from_str::<Tool>(
            r#"
{
  "url": "https://test_owner.github.io/test_name/tools/c13b6e27-a4ee-426f-8bdb-8cf5c4310bad",
  "id": "c13b6e27-a4ee-426f-8bdb-8cf5c4310bad",
  "organization": "@suecharo",
  "name": "CWL_trimming_and_qc",
  "toolclass": {
    "id": "workflow",
    "name": "Workflow",
    "description": "A computational workflow"
  },
  "description": "https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/README.md",
  "has_checker": true,
  "checker_url": "https://github.com/sapporo-wes/yevis-cli",
  "versions": [
    {
      "author": [
        "suecharo"
      ],
      "name": "CWL_trimming_and_qc",
      "url": "https://test_owner.github.io/test_name/tools/c13b6e27-a4ee-426f-8bdb-8cf5c4310bad/versions/1.0.0",
      "id": "1.0.0",
      "descriptor_type": [
        "CWL"
      ],
      "verified": false
    }
  ]
}"#,
        )?;

        assert_eq!(tool, expect);
        Ok(())
    }

    #[test]
    fn test_tool_version_new() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        let tool_version = ToolVersion::new(&meta, "test_owner", "test_name", true)?;
        let expect = serde_json::from_str::<ToolVersion>(
            r#"
{
  "author": [
    "suecharo"
  ],
  "name": "CWL_trimming_and_qc",
  "url": "https://test_owner.github.io/test_name/tools/c13b6e27-a4ee-426f-8bdb-8cf5c4310bad/versions/1.0.0",
  "id": "1.0.0",
  "descriptor_type": [
    "CWL"
  ],
  "verified": true
}"#,
        )?;
        assert_eq!(tool_version, expect);
        Ok(())
    }

    #[test]
    fn test_tool_version_version() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        let tool_version = ToolVersion::new(&meta, "test_owner", "test_name", true)?;
        let version = tool_version.version();
        assert_eq!(version, "1.0.0");
        Ok(())
    }
}
