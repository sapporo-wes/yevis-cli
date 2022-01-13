use crate::{
    remote::fetch_raw_content,
    type_config::{Language, LanguageType},
};
use anyhow::{anyhow, Context, Result};
use log::info;
use regex::Regex;
use serde_yaml;
use std::collections::BTreeMap;

pub fn inspect_wf_type_version(wf_loc: impl AsRef<str>) -> Result<Language> {
    let wf_content = fetch_raw_content(&wf_loc).context(format!(
        "Failed to fetch contents from your inputted workflow location: {}. Please check your inputted workflow location.",
        wf_loc.as_ref()
    ))?;
    let r#type = match inspect_wf_type(&wf_content) {
        Ok(wf_type) => wf_type,
        Err(_) => {
            info!("`wf_type` not found in {}", wf_loc.as_ref());
            info!("Using default `wf_type` as `CWL`");
            LanguageType::Cwl
        }
    };
    let version = match &inspect_wf_version(&wf_content, &r#type) {
        Ok(wf_version) => wf_version.to_string(),
        Err(_) => {
            info!("`wf_version` not found in {}", wf_loc.as_ref());
            info!("Using default `wf_version` as `1.0`");
            "1.0".to_string()
        }
    };
    Ok(Language { r#type, version })
}

pub fn inspect_wf_type(wf_content: impl AsRef<str>) -> Result<LanguageType> {
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

fn check_by_shebang(wf_content: impl AsRef<str>) -> Result<LanguageType> {
    let first_line = wf_content.as_ref().lines().next().unwrap_or("");
    if first_line.starts_with("#!") {
        if first_line.contains("cwl") {
            return Ok(LanguageType::Cwl);
        } else if first_line.contains("cromwell") {
            return Ok(LanguageType::Wdl);
        } else if first_line.contains("nextflow") {
            return Ok(LanguageType::Nfl);
        } else if first_line.contains("snakemake") {
            return Ok(LanguageType::Smk);
        }
    }
    Err(anyhow!("Unknown workflow type"))
}

fn check_by_regexp(wf_content: impl AsRef<str>) -> Result<LanguageType> {
    let pattern_wdl = Regex::new(r"^(workflow|task) \w* \{$")?;
    let pattern_nfl = Regex::new(r"^process \w* \{$")?;
    let pattern_smk = Regex::new(r"^rule \w*:$")?;
    for line in wf_content.as_ref().lines() {
        if line.contains("cwlVersion") {
            return Ok(LanguageType::Cwl);
        } else if pattern_wdl.is_match(line) {
            return Ok(LanguageType::Wdl);
        } else if pattern_nfl.is_match(line) {
            return Ok(LanguageType::Nfl);
        } else if pattern_smk.is_match(line) {
            return Ok(LanguageType::Smk);
        }
    }
    Err(anyhow!("Unknown workflow type"))
}

pub fn inspect_wf_version(wf_content: impl AsRef<str>, wf_type: &LanguageType) -> Result<String> {
    match &wf_type {
        LanguageType::Cwl => inspect_cwl_version(&wf_content),
        LanguageType::Wdl => inspect_wdl_version(&wf_content),
        LanguageType::Nfl => inspect_nfl_version(&wf_content),
        LanguageType::Smk => inspect_smk_version(&wf_content),
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
    fn test_inspect_wf_type_version_cwl() {
        let wf_loc = "https://raw.githubusercontent.com/ddbj/yevis-cli/36d23db735623e0e87a69a02d23ff08c754e6f13/tests/CWL/wf/trimming_and_qc.cwl";
        let wf_type_version = inspect_wf_type_version(wf_loc).unwrap();
        assert_eq!(
            wf_type_version,
            Language {
                r#type: LanguageType::Cwl,
                version: "v1.0".to_string()
            }
        );
    }

    #[test]
    fn test_inspect_wf_type_version_wdl() {
        let wf_loc =
            "https://raw.githubusercontent.com/ddbj/yevis-cli/36d23db735623e0e87a69a02d23ff08c754e6f13/tests/WDL/wf/dockstore-tool-bamstats.wdl";
        let wf_type_version = inspect_wf_type_version(wf_loc).unwrap();
        assert_eq!(
            wf_type_version,
            Language {
                r#type: LanguageType::Wdl,
                version: "1.0".to_string()
            }
        );
    }

    #[test]
    fn test_inspect_wf_type_version_nfl() {
        let wf_loc = "https://raw.githubusercontent.com/ddbj/yevis-cli/36d23db735623e0e87a69a02d23ff08c754e6f13/tests/NFL/wf/file_input.nf";
        let wf_type_version = inspect_wf_type_version(wf_loc).unwrap();
        assert_eq!(
            wf_type_version,
            Language {
                r#type: LanguageType::Nfl,
                version: "1.0".to_string()
            }
        );
    }

    #[test]
    fn test_inspect_wf_type_version_smk() {
        let wf_loc = "https://raw.githubusercontent.com/ddbj/yevis-cli/36d23db735623e0e87a69a02d23ff08c754e6f13/tests/SMK/wf/Snakefile";
        let wf_type_version = inspect_wf_type_version(wf_loc).unwrap();
        assert_eq!(
            wf_type_version,
            Language {
                r#type: LanguageType::Smk,
                version: "1.0".to_string()
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
        assert_eq!(inspect_wf_type(wf_content).unwrap(), LanguageType::Cwl);
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
            inspect_wf_version(wf_content, &LanguageType::Cwl).unwrap(),
            "v1.2".to_string()
        );
    }
}
