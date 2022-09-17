use crate::metadata;
use crate::remote;
use crate::trs;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TrsResponse {
    pub yevis_meta: HashMap<(Uuid, String), metadata::types::Metadata>,
    pub service_info: trs::types::ServiceInfo,
    pub tool_classes: Vec<trs::types::ToolClass>,
    pub tools: Vec<trs::types::Tool>,
    pub tools_descriptor: HashMap<(Uuid, String), trs::types::FileWrapper>,
    pub tools_files: HashMap<(Uuid, String), Vec<trs::types::ToolFile>>,
    pub tools_tests: HashMap<(Uuid, String), Vec<trs::types::FileWrapper>>,
}

impl TrsResponse {
    pub fn new(owner: impl AsRef<str>, name: impl AsRef<str>) -> Result<Self> {
        let trs_endpoint = trs::api::TrsEndpoint::new_gh_pages(&owner, &name)?;
        let service_info = trs::types::ServiceInfo::new_or_update(
            trs::api::get_service_info(&trs_endpoint).ok(),
            &owner,
            &name,
        )?;
        let tool_classes = generate_tool_classes(&trs_endpoint)?;
        let tools = match trs::api::get_tools(&trs_endpoint) {
            Ok(tools) => tools,
            Err(_) => vec![],
        };

        Ok(Self {
            yevis_meta: HashMap::new(),
            service_info,
            tool_classes,
            tools,
            tools_descriptor: HashMap::new(),
            tools_files: HashMap::new(),
            tools_tests: HashMap::new(),
        })
    }

    pub fn add(
        &mut self,
        owner: impl AsRef<str>,
        name: impl AsRef<str>,
        meta: &metadata::types::Metadata,
        verified: bool,
    ) -> Result<()> {
        match self.tools.iter_mut().find(|t| t.id == meta.id) {
            Some(tool) => {
                // update tool
                tool.add_new_tool_version(meta, &owner, &name, verified)?;
            }
            None => {
                // create tool and add
                let mut tool = trs::types::Tool::new(meta, &owner, &name)?;
                tool.add_new_tool_version(meta, &owner, &name, verified)?;
                self.tools.push(tool);
            }
        };

        self.tools_descriptor
            .insert((meta.id, meta.version.clone()), generate_descriptor(meta)?);
        self.tools_files
            .insert((meta.id, meta.version.clone()), generate_files(meta)?);
        self.tools_tests
            .insert((meta.id, meta.version.clone()), generate_tests(meta)?);

        self.yevis_meta
            .insert((meta.id, meta.version.clone()), meta.clone());

        Ok(())
    }
}

pub fn generate_tool_classes(
    trs_endpoint: &trs::api::TrsEndpoint,
) -> Result<Vec<trs::types::ToolClass>> {
    match trs::api::get_tool_classes(trs_endpoint) {
        Ok(mut tool_classes) => {
            let has_workflow = tool_classes
                .iter()
                .find(|tc| tc.id == Some("workflow".to_string()));
            if has_workflow.is_none() {
                tool_classes.push(trs::types::ToolClass::default());
            };
            Ok(tool_classes)
        }
        Err(_) => Ok(vec![trs::types::ToolClass::default()]),
    }
}

pub fn generate_descriptor(meta: &metadata::types::Metadata) -> Result<trs::types::FileWrapper> {
    let primary_wf = meta.workflow.primary_wf()?;
    let (content, checksum) = match remote::fetch_raw_content(&primary_wf.url) {
        Ok(content) => {
            let checksum = trs::types::Checksum::new_from_string(content.clone());
            (Some(content), Some(vec![checksum]))
        }
        Err(_) => (None, None),
    };
    Ok(trs::types::FileWrapper {
        content,
        checksum,
        url: Some(primary_wf.url),
    })
}

pub fn generate_files(meta: &metadata::types::Metadata) -> Result<Vec<trs::types::ToolFile>> {
    Ok(meta
        .workflow
        .files
        .iter()
        .map(|f| {
            let checksum = match trs::types::Checksum::new_from_url(&f.url) {
                Ok(checksum) => Some(checksum),
                Err(_) => None,
            };
            trs::types::ToolFile {
                path: Some(f.url.clone()),
                file_type: Some(trs::types::FileType::new_from_file_type(&f.r#type)),
                checksum,
            }
        })
        .collect())
}

pub fn generate_tests(meta: &metadata::types::Metadata) -> Result<Vec<trs::types::FileWrapper>> {
    meta.workflow
        .testing
        .iter()
        .map(|t| {
            let test_str = serde_json::to_string(&t)?;
            Ok(trs::types::FileWrapper {
                content: Some(test_str.clone()),
                checksum: Some(vec![trs::types::Checksum::new_from_string(test_str)]),
                url: None,
            })
        })
        .collect::<Result<Vec<_>>>()
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_trs_response_new() -> Result<()> {
        TrsResponse::new("test_owner", "test_name")?;
        Ok(())
    }

    #[test]
    fn test_generate_tool_classes() -> Result<()> {
        let trs_endpoint = trs::api::TrsEndpoint::new_gh_pages("test_owner", "test_name")?;
        let tool_classes = generate_tool_classes(&trs_endpoint)?;
        let expect = serde_json::from_str::<Vec<trs::types::ToolClass>>(
            r#"
[
  {
    "id": "workflow",
    "name": "Workflow",
    "description": "A computational workflow"
  }
]"#,
        )?;
        assert_eq!(tool_classes, expect);
        Ok(())
    }

    #[test]
    fn test_generate_descriptor() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        generate_descriptor(&meta)?;
        Ok(())
    }

    #[test]
    fn test_generate_files() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        let files = generate_files(&meta)?;
        let expect = serde_json::from_str::<Vec<trs::types::ToolFile>>(
            r#"
[
  {
    "path": "https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/tests/CWL/wf/fastqc.cwl",
    "file_type": "SECONDARY_DESCRIPTOR",
    "checksum": {
      "checksum": "1bd771a51336a782b695db8334872e00f305cd7c49c4978e7e58786ea4714437",
      "type": "sha256"
    }
  },
  {
    "path": "https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/tests/CWL/wf/trimming_and_qc.cwl",
    "file_type": "PRIMARY_DESCRIPTOR",
    "checksum": {
      "checksum": "33ef70b2d5ee38cb394c5ca6354243f44a85118271026eb9fc61365a703e730b",
      "type": "sha256"
    }
  },
  {
    "path": "https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/tests/CWL/wf/trimmomatic_pe.cwl",
    "file_type": "SECONDARY_DESCRIPTOR",
    "checksum": {
      "checksum": "531d0a38116347cade971c211056334f7cae48e1293e2bb0e334894e55636f8e",
      "type": "sha256"
    }
  }
]"#,
        )?;
        assert_eq!(files, expect);
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_generate_tests() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        let tests = generate_tests(&meta)?;
        let expect = serde_json::from_str::<Vec<trs::types::FileWrapper>>(
            r#"
[
  {
    "content": "{\"id\":\"test_1\",\"files\":[{\"url\":\"https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/tests/CWL/test/wf_params.json\",\"target\":\"wf_params.json\",\"type\":\"wf_params\"},{\"url\":\"https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/tests/CWL/test/ERR034597_1.small.fq.gz\",\"target\":\"ERR034597_1.small.fq.gz\",\"type\":\"other\"},{\"url\":\"https://raw.githubusercontent.com/sapporo-wes/yevis-cli/d81e0e38143c63ead17d475b85c9b639958b1b47/tests/CWL/test/ERR034597_2.small.fq.gz\",\"target\":\"ERR034597_2.small.fq.gz\",\"type\":\"other\"}]}",
    "checksum": [
      {
        "checksum": "243f87fe67d5abb9555069ca658e5f9709c3554001845d5dca3b73011237e4ec",
        "type": "sha256"
      }
    ]
  }
]"#,
        )?;
        assert_eq!(tests, expect);
        Ok(())
    }
}
