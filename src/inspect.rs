use crate::metadata;
use crate::metadata::types::LanguageType;
use crate::remote;

use anyhow::{anyhow, Result};
use colored::Colorize;
use log::warn;
use regex::Regex;
use std::collections::BTreeMap;
use url::Url;

pub fn inspect_wf_type_version(wf_loc: &Url) -> Result<metadata::types::Language> {
    let wf_content = remote::fetch_raw_content(wf_loc)?;
    let wf_type = inspect_wf_type(&wf_content);
    let wf_version = inspect_wf_version(&wf_content, &wf_type);
    Ok(metadata::types::Language {
        r#type: wf_type,
        version: wf_version,
    })
}

pub fn inspect_wf_type(wf_content: impl AsRef<str>) -> LanguageType {
    match check_by_shebang(&wf_content) {
        LanguageType::Unknown => match check_by_regexp(&wf_content) {
            Ok(wf_type) => wf_type,
            Err(e) => {
                warn!("{}: {}", "Warning".yellow(), e);
                LanguageType::Unknown
            }
        },
        wf_type => wf_type,
    }
}

pub fn check_by_shebang(wf_content: impl AsRef<str>) -> LanguageType {
    let first_line = wf_content.as_ref().lines().next().unwrap_or("");
    if first_line.starts_with("#!") {
        if first_line.contains("cwl") {
            return LanguageType::Cwl;
        } else if first_line.contains("cromwell") {
            return LanguageType::Wdl;
        } else if first_line.contains("nextflow") {
            return LanguageType::Nfl;
        } else if first_line.contains("snakemake") {
            return LanguageType::Smk;
        }
    }
    LanguageType::Unknown
}

pub fn check_by_regexp(wf_content: impl AsRef<str>) -> Result<LanguageType> {
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
    Ok(LanguageType::Unknown)
}

pub fn inspect_wf_version(wf_content: impl AsRef<str>, wf_type: &LanguageType) -> String {
    match wf_type {
        LanguageType::Cwl => match inspect_cwl_version(wf_content) {
            Ok(version) => version,
            Err(e) => {
                warn!("{}: {}", "Warning".yellow(), e);
                "v1.0".to_string()
            }
        },
        LanguageType::Wdl => match inspect_wdl_version(wf_content) {
            Ok(version) => version,
            Err(e) => {
                warn!("{}: {}", "Warning".yellow(), e);
                "1.0".to_string()
            }
        },
        LanguageType::Nfl => match inspect_nfl_version(wf_content) {
            Ok(version) => version,
            Err(e) => {
                warn!("{}: {}", "Warning".yellow(), e);
                "1.0".to_string()
            }
        },
        LanguageType::Smk => match inspect_smk_version(wf_content) {
            Ok(version) => version,
            Err(e) => {
                warn!("{}: {}", "Warning".yellow(), e);
                "1.0".to_string()
            }
        },
        LanguageType::Unknown => "1.0".to_string(),
    }
}

/// https://www.commonwl.org/v1.2/CommandLineTool.html#CWLVersion
pub fn inspect_cwl_version(wf_content: impl AsRef<str>) -> Result<String> {
    let cwl_docs: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(wf_content.as_ref())?;
    match cwl_docs.contains_key("cwlVersion") {
        true => match cwl_docs
            .get("cwlVersion")
            .ok_or_else(|| anyhow!("Failed to parse cwlVersion"))?
        {
            serde_yaml::Value::String(version) => Ok(version.to_string()),
            _ => Ok("v1.0".to_string()),
        },
        false => Ok("v1.0".to_string()),
    }
}

pub fn inspect_wdl_version(wf_content: impl AsRef<str>) -> Result<String> {
    let pattern_wdl_version = Regex::new(r"^version \d\.\d$")?;
    for line in wf_content.as_ref().lines() {
        if pattern_wdl_version.is_match(line) {
            let version = line.split_whitespace().nth(1).unwrap_or("1.0");
            return Ok(version.to_string());
        }
    }
    Ok("1.0".to_string())
}

pub fn inspect_nfl_version(wf_content: impl AsRef<str>) -> Result<String> {
    for line in wf_content.as_ref().lines() {
        if line == "nextflow.enable.dsl=2" {
            return Ok("DSL2".to_string());
        }
    }
    Ok("1.0".to_string())
}

pub fn inspect_smk_version(_wf_content: impl AsRef<str>) -> Result<String> {
    Ok("1.0".to_string())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_wf_type_version_cwl() -> Result<()> {
        let url = Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/main/tests/CWL/wf/trimming_and_qc.cwl")?;
        let wf_type_version = inspect_wf_type_version(&url)?;
        assert_eq!(wf_type_version.r#type, LanguageType::Cwl);
        assert_eq!(wf_type_version.version, "v1.0".to_string());
        Ok(())
    }

    #[test]
    fn test_inspect_wf_type_version_wdl() -> Result<()> {
        let url = Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/main/tests/WDL/wf/dockstore-tool-bamstats.wdl")?;
        let wf_type_version = inspect_wf_type_version(&url)?;
        assert_eq!(wf_type_version.r#type, LanguageType::Wdl);
        assert_eq!(wf_type_version.version, "1.0".to_string());
        Ok(())
    }

    #[test]
    fn test_inspect_wf_type_version_nfl() -> Result<()> {
        let url = Url::parse(
            "https://raw.githubusercontent.com/ddbj/yevis-cli/main/tests/NFL/wf/file_input.nf",
        )?;
        let wf_type_version = inspect_wf_type_version(&url)?;
        assert_eq!(wf_type_version.r#type, LanguageType::Nfl);
        assert_eq!(wf_type_version.version, "1.0".to_string());
        Ok(())
    }

    #[test]
    fn test_inspect_wf_type_version_smk() -> Result<()> {
        let url = Url::parse(
            "https://raw.githubusercontent.com/ddbj/yevis-cli/main/tests/SMK/wf/Snakefile",
        )?;
        let wf_type_version = inspect_wf_type_version(&url)?;
        assert_eq!(wf_type_version.r#type, LanguageType::Smk);
        assert_eq!(wf_type_version.version, "1.0".to_string());
        Ok(())
    }
}
