use crate::remote;
use crate::type_config;
use crate::type_config::WorkflowLanguageType as WfType;
use anyhow::{anyhow, Result};
use regex::Regex;
use serde_yaml;
use std::collections::BTreeMap;

pub fn inspect_wf_type_version(wf_loc: impl AsRef<str>) -> Result<type_config::WorkflowLanguage> {
    let wf_content = remote::fetch_raw_content(&wf_loc)?;
    let r#type = match inspect_wf_type(&wf_content) {
        Ok(wf_type) => wf_type,
        Err(_) => WfType::Cwl,
    };
    let version = match &inspect_wf_version(&wf_content, &r#type) {
        Ok(wf_version) => wf_version.to_string(),
        Err(_) => "1.0".to_string(),
    };
    Ok(type_config::WorkflowLanguage { r#type, version })
}

pub fn inspect_wf_type(wf_content: impl AsRef<str>) -> Result<WfType> {
    match check_by_shebang(&wf_content) {
        Ok(wf_type) => return Ok(wf_type),
        Err(_) => {}
    };
    match check_by_regexp(&wf_content) {
        Ok(wf_type) => return Ok(wf_type),
        Err(_) => {}
    };
    Err(anyhow!("Failed to parse workflow type"))
}

fn check_by_shebang(wf_content: impl AsRef<str>) -> Result<WfType> {
    let first_line = wf_content.as_ref().lines().next().unwrap_or("");
    if first_line.starts_with("#!") {
        if first_line.contains("cwl") {
            return Ok(WfType::Cwl);
        } else if first_line.contains("cromwell") {
            return Ok(WfType::Wdl);
        } else if first_line.contains("nextflow") {
            return Ok(WfType::Nfl);
        } else if first_line.contains("snakemake") {
            return Ok(WfType::Smk);
        }
    }
    Err(anyhow!("Unknown workflow type"))
}

fn check_by_regexp(wf_content: impl AsRef<str>) -> Result<WfType> {
    let pattern_wdl = Regex::new(r"^(workflow|task) \w* \{$")?;
    let pattern_nfl = Regex::new(r"^process \w* \{$")?;
    let pattern_smk = Regex::new(r"^rule \w*:$")?;
    for line in wf_content.as_ref().lines() {
        if pattern_wdl.is_match(line) {
            return Ok(WfType::Wdl);
        } else if pattern_nfl.is_match(line) {
            return Ok(WfType::Nfl);
        } else if pattern_smk.is_match(line) {
            return Ok(WfType::Smk);
        }
    }
    Err(anyhow!("Unknown workflow type"))
}

pub fn inspect_wf_version(wf_content: impl AsRef<str>, wf_type: &WfType) -> Result<String> {
    match &wf_type {
        WfType::Cwl => inspect_cwl_version(&wf_content),
        WfType::Wdl => inspect_wdl_version(&wf_content),
        WfType::Nfl => inspect_nfl_version(&wf_content),
        WfType::Smk => inspect_smk_version(&wf_content),
    }
}

/// https://www.commonwl.org/v1.2/CommandLineTool.html#CWLVersion
fn inspect_cwl_version(wf_content: impl AsRef<str>) -> Result<String> {
    let cwl_docs: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(wf_content.as_ref())?;
    match cwl_docs.contains_key("cwlVersion") {
        true => match cwl_docs.get("cwlVersion").unwrap() {
            serde_yaml::Value::String(version) => Ok(version.to_string()),
            _ => Ok("v1.0".to_string()),
        },
        false => Ok("v1.0".to_string()),
    }
}

fn inspect_wdl_version(wf_content: impl AsRef<str>) -> Result<String> {
    let pattern_wdl_version = Regex::new(r"^version \d\.\d$")?;
    for line in wf_content.as_ref().lines() {
        if pattern_wdl_version.is_match(line) {
            let version = line.split_whitespace().nth(1).unwrap_or("1.0");
            return Ok(version.to_string());
        }
    }
    Ok("1.0".to_string())
}

fn inspect_nfl_version(wf_content: impl AsRef<str>) -> Result<String> {
    for line in wf_content.as_ref().lines() {
        if line == "nextflow.enable.dsl=2" {
            return Ok("DSL2".to_string());
        }
    }
    Ok("1.0".to_string())
}

fn inspect_smk_version(_wf_content: impl AsRef<str>) -> Result<String> {
    Ok("1.0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_wf_type_version() {
        let wf_loc = "https://raw.githubusercontent.com/sapporo-wes/sapporo-service/main/tests/resources/cwltool/trimming_and_qc.cwl";
        let wf_type_version = inspect_wf_type_version(wf_loc).unwrap();
        assert_eq!(
            wf_type_version,
            type_config::WorkflowLanguage {
                r#type: WfType::Cwl,
                version: "v1.0".to_string()
            }
        );
    }

    #[test]
    fn test_inspect_wf_type() {
        let wf_content = "\
#!/usr/bin/env cwl-runner
cwlVersion: v1.2
class: CommandLineTool
baseCommand: echo
inputs:
  message:
    type: string
    inputBinding:
      position: 1
outputs: []";
        assert_eq!(inspect_wf_type(wf_content).unwrap(), WfType::Cwl);
    }

    #[test]
    fn test_inspect_wf_version() {
        let wf_content = "\
#!/usr/bin/env cwl-runner
cwlVersion: v1.2
class: CommandLineTool
baseCommand: echo
inputs:
  message:
    type: string
    inputBinding:
      position: 1
outputs: []";
        assert_eq!(
            inspect_wf_version(wf_content, &WfType::Cwl).unwrap(),
            "v1.2".to_string()
        );
    }
}
