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
    pub gh_trs_config: HashMap<(Uuid, String), metadata::types::Config>,
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
            gh_trs_config: HashMap::new(),
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
        config: &metadata::types::Config,
        verified: bool,
    ) -> Result<()> {
        match self.tools.iter_mut().find(|t| t.id == config.id) {
            Some(tool) => {
                // update tool
                tool.add_new_tool_version(config, &owner, &name, verified)?;
            }
            None => {
                // create tool and add
                let mut tool = trs::types::Tool::new(config, &owner, &name)?;
                tool.add_new_tool_version(config, &owner, &name, verified)?;
                self.tools.push(tool);
            }
        };

        self.tools_descriptor.insert(
            (config.id, config.version.clone()),
            generate_descriptor(config)?,
        );
        self.tools_files
            .insert((config.id, config.version.clone()), generate_files(config)?);
        self.tools_tests
            .insert((config.id, config.version.clone()), generate_tests(config)?);

        self.gh_trs_config
            .insert((config.id, config.version.clone()), config.clone());

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

pub fn generate_descriptor(config: &metadata::types::Config) -> Result<trs::types::FileWrapper> {
    let primary_wf = config.workflow.primary_wf()?;
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

pub fn generate_files(config: &metadata::types::Config) -> Result<Vec<trs::types::ToolFile>> {
    Ok(config
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

pub fn generate_tests(config: &metadata::types::Config) -> Result<Vec<trs::types::FileWrapper>> {
    config
        .workflow
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
        let config = metadata::io::read_config("./tests/test_config_CWL_validated.yml")?;
        generate_descriptor(&config)?;
        Ok(())
    }

    #[test]
    fn test_generate_files() -> Result<()> {
        let config = metadata::io::read_config("./tests/test_config_CWL_validated.yml")?;
        let files = generate_files(&config)?;
        let expect = serde_json::from_str::<Vec<trs::types::ToolFile>>(
            r#"
[
  {
    "path": "https://raw.githubusercontent.com/ddbj/yevis-cli/458d0524e667f2442a5effb730b523c1f15748d4/tests/CWL/wf/fastqc.cwl",
    "file_type": "SECONDARY_DESCRIPTOR",
    "checksum": {
      "checksum": "1bd771a51336a782b695db8334872e00f305cd7c49c4978e7e58786ea4714437",
      "type": "sha256"
    }
  },
  {
    "path": "https://raw.githubusercontent.com/ddbj/yevis-cli/458d0524e667f2442a5effb730b523c1f15748d4/tests/CWL/wf/trimming_and_qc.cwl",
    "file_type": "PRIMARY_DESCRIPTOR",
    "checksum": {
      "checksum": "33ef70b2d5ee38cb394c5ca6354243f44a85118271026eb9fc61365a703e730b",
      "type": "sha256"
    }
  },
  {
    "path": "https://raw.githubusercontent.com/ddbj/yevis-cli/458d0524e667f2442a5effb730b523c1f15748d4/tests/CWL/wf/trimmomatic_pe.cwl",
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
    fn test_generate_tests() -> Result<()> {
        let config = metadata::io::read_config("./tests/test_config_CWL_validated.yml")?;
        let tests = generate_tests(&config)?;
        let expect = serde_json::from_str::<Vec<trs::types::FileWrapper>>(
            r#"
[
  {
    "content": "{\"id\":\"test_1\",\"files\":[{\"url\":\"https://raw.githubusercontent.com/ddbj/yevis-cli/4e7e2e3ddb42bdaaf5e294f4bf67319f23c4eaa4/tests/CWL/test/wf_params.json\",\"target\":\"wf_params.json\",\"type\":\"wf_params\"},{\"url\":\"https://raw.githubusercontent.com/ddbj/yevis-cli/4e7e2e3ddb42bdaaf5e294f4bf67319f23c4eaa4/tests/CWL/test/ERR034597_1.small.fq.gz\",\"target\":\"ERR034597_1.small.fq.gz\",\"type\":\"other\"},{\"url\":\"https://raw.githubusercontent.com/ddbj/yevis-cli/4e7e2e3ddb42bdaaf5e294f4bf67319f23c4eaa4/tests/CWL/test/ERR034597_2.small.fq.gz\",\"target\":\"ERR034597_2.small.fq.gz\",\"type\":\"other\"}]}",
    "checksum": [
      {
        "checksum": "e6de556f3d71919d6e678d319231f9cf8d240bec594b09d1eff137c8de4dd9e9",
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
