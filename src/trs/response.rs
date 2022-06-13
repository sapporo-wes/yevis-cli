use crate::metadata;
use crate::remote;
use crate::trs;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
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

    pub fn generate_contents(&self) -> Result<HashMap<PathBuf, String>> {
        let mut map: HashMap<PathBuf, String> = HashMap::new();
        map.insert(
            PathBuf::from("service-info/index.json"),
            serde_json::to_string(&self.service_info)?,
        );
        map.insert(
            PathBuf::from("toolClasses/index.json"),
            serde_json::to_string(&self.tool_classes)?,
        );
        map.insert(
            PathBuf::from("tools/index.json"),
            serde_json::to_string(&self.tools)?,
        );
        for ((id, version), config) in self.gh_trs_config.iter() {
            let tools_id = self.tools.iter().find(|t| &t.id == id).unwrap();
            let tools_id_versions = tools_id.versions.clone();
            let tools_id_versions_version = tools_id_versions
                .iter()
                .find(|v| &v.version() == version)
                .unwrap();
            let tools_descriptor = self.tools_descriptor.get(&(*id, version.clone())).unwrap();
            let tools_files = self.tools_files.get(&(*id, version.clone())).unwrap();
            let tools_tests = self.tools_tests.get(&(*id, version.clone())).unwrap();

            let desc_type = config.workflow.language.r#type.clone().unwrap().to_string();

            map.insert(
                PathBuf::from(format!(
                    "tools/{}/versions/{}/gh-trs-config.json",
                    id, version
                )),
                serde_json::to_string(&config)?,
            );
            map.insert(
                PathBuf::from(format!("tools/{}/index.json", id)),
                serde_json::to_string(&tools_id)?,
            );
            map.insert(
                PathBuf::from(format!("tools/{}/versions/index.json", id)),
                serde_json::to_string(&tools_id_versions)?,
            );
            map.insert(
                PathBuf::from(format!("tools/{}/versions/{}/index.json", id, version)),
                serde_json::to_string(&tools_id_versions_version)?,
            );
            map.insert(
                PathBuf::from(format!(
                    "tools/{}/versions/{}/{}/descriptor/index.json",
                    id, version, desc_type
                )),
                serde_json::to_string(&tools_descriptor)?,
            );
            map.insert(
                PathBuf::from(format!(
                    "tools/{}/versions/{}/{}/files/index.json",
                    id, version, desc_type
                )),
                serde_json::to_string(&tools_files)?,
            );
            map.insert(
                PathBuf::from(format!(
                    "tools/{}/versions/{}/{}/tests/index.json",
                    id, version, desc_type
                )),
                serde_json::to_string(&tools_tests)?,
            );
            map.insert(
                PathBuf::from(format!(
                    "tools/{}/versions/{}/containerfile/index.json",
                    id, version
                )),
                serde_json::to_string(&Vec::<trs::types::FileWrapper>::new())?,
            );
        }
        Ok(map)
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
    "path": "https://raw.githubusercontent.com/suecharo/gh-trs/458d0524e667f2442a5effb730b523c1f15748d4/tests/CWL/wf/fastqc.cwl",
    "file_type": "SECONDARY_DESCRIPTOR",
    "checksum": {
      "checksum": "1bd771a51336a782b695db8334872e00f305cd7c49c4978e7e58786ea4714437",
      "type": "sha256"
    }
  },
  {
    "path": "https://raw.githubusercontent.com/suecharo/gh-trs/458d0524e667f2442a5effb730b523c1f15748d4/tests/CWL/wf/trimming_and_qc.cwl",
    "file_type": "PRIMARY_DESCRIPTOR",
    "checksum": {
      "checksum": "33ef70b2d5ee38cb394c5ca6354243f44a85118271026eb9fc61365a703e730b",
      "type": "sha256"
    }
  },
  {
    "path": "https://raw.githubusercontent.com/suecharo/gh-trs/458d0524e667f2442a5effb730b523c1f15748d4/tests/CWL/wf/trimmomatic_pe.cwl",
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
    "content": "{\"id\":\"test_1\",\"files\":[{\"url\":\"https://raw.githubusercontent.com/suecharo/gh-trs/4e7e2e3ddb42bdaaf5e294f4bf67319f23c4eaa4/tests/CWL/test/wf_params.json\",\"target\":\"wf_params.json\",\"type\":\"wf_params\"},{\"url\":\"https://raw.githubusercontent.com/suecharo/gh-trs/4e7e2e3ddb42bdaaf5e294f4bf67319f23c4eaa4/tests/CWL/test/ERR034597_1.small.fq.gz\",\"target\":\"ERR034597_1.small.fq.gz\",\"type\":\"other\"},{\"url\":\"https://raw.githubusercontent.com/suecharo/gh-trs/4e7e2e3ddb42bdaaf5e294f4bf67319f23c4eaa4/tests/CWL/test/ERR034597_2.small.fq.gz\",\"target\":\"ERR034597_2.small.fq.gz\",\"type\":\"other\"}]}",
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
