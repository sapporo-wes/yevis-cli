use crate::{
    remote::fetch_raw_content,
    type_config::{Config, FileType, LanguageType, TestFileType, Testing, Workflow},
    wes::{default_wes_location, get_service_info, start_wes},
};
use anyhow::{anyhow, ensure, Result};
use log::{debug, info};
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json;
use url::Url;

pub fn test(config: &Config, wes_location: &Option<Url>, docker_host: &Url) -> Result<()> {
    let default_wes_loc = Url::parse(&default_wes_location())?;
    let wes_location = match &wes_location {
        Some(wes_location) => {
            info!("Use wes_location: {} for testing", wes_location.as_str());
            wes_location
        }
        None => {
            start_wes(&docker_host)?;
            info!("Use wes_location: {} for testing", default_wes_loc.as_str());
            &default_wes_loc
        }
    };
    let service_info = get_service_info(wes_location)?;
    ensure!(
        service_info
            .supported_wes_versions
            .into_iter()
            .find(|v| v == "sapporo-wes-1.0.1")
            .is_some(),
        "yevis only supports WES version sapporo-wes-1.0.1"
    );
    for test_case in &config.workflow.testing {
        info!("Testing {}", test_case.id);
        let form = test_case_to_form(&config.workflow, &test_case)?;
        debug!("form: {:?}", form);
    }

    Ok(())
}

pub fn test_case_to_form(workflow: &Workflow, test_case: &Testing) -> Result<multipart::Form> {
    let form = multipart::Form::new()
        .text("workflow_type", wf_type(&workflow))
        .text("workflow_type_version", wf_version(&workflow))
        .text("workflow_url", wf_url(&workflow)?)
        .text("workflow_engine_name", wf_engine_name(&workflow))
        .text("workflow_params", wf_params(&test_case)?)
        .text("workflow_engine_parameters", wf_engine_params(&test_case)?)
        .text("workflow_attachment", wf_attachment(&workflow, &test_case)?);
    Ok(form)
}

fn wf_type(workflow: &Workflow) -> String {
    workflow.language.r#type.to_string()
}

fn wf_version(workflow: &Workflow) -> String {
    workflow.language.version.to_string()
}

fn wf_url(workflow: &Workflow) -> Result<String> {
    let primary_wf = workflow
        .files
        .iter()
        .find(|f| f.r#type == FileType::Primary)
        .ok_or(anyhow!("No primary workflow file"))?;
    Ok(primary_wf.url.to_string())
}

fn wf_engine_name(workflow: &Workflow) -> String {
    match &workflow.language.r#type {
        LanguageType::Cwl => "cwltool".to_string(),
        LanguageType::Wdl => "cromwell".to_string(),
        LanguageType::Nfl => "nextflow".to_string(),
        LanguageType::Smk => "snakemake".to_string(),
    }
}

fn wf_params(test_case: &Testing) -> Result<String> {
    let wf_params = test_case
        .files
        .iter()
        .find(|f| f.r#type == TestFileType::WfParams);
    match wf_params {
        Some(wf_params) => Ok(fetch_raw_content(wf_params.url.as_str())?),
        None => Ok("{}".to_string()),
    }
}

fn wf_engine_params(test_case: &Testing) -> Result<String> {
    let wf_params = test_case
        .files
        .iter()
        .find(|f| f.r#type == TestFileType::WfEngineParams);
    match wf_params {
        Some(wf_params) => Ok(fetch_raw_content(wf_params.url.as_str())?),
        None => Ok("{}".to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AttachedFile {
    file_name: String,
    file_url: Url,
}

fn wf_attachment(workflow: &Workflow, test_case: &Testing) -> Result<String> {
    let mut attachments: Vec<AttachedFile> = vec![];
    workflow.files.iter().for_each(|f| {
        if f.r#type == FileType::Secondary {
            let file_name = match f.target.to_str() {
                Some(file_name) => file_name.to_string(),
                None => f.url.path().to_string(),
            };
            attachments.push(AttachedFile {
                file_name: file_name,
                file_url: f.url.clone(),
            });
        }
    });
    test_case.files.iter().for_each(|f| {
        if f.r#type == TestFileType::Other {
            let file_name = match f.target.to_str() {
                Some(file_name) => file_name.to_string(),
                None => f.url.path().to_string(),
            };
            attachments.push(AttachedFile {
                file_name: file_name,
                file_url: f.url.clone(),
            });
        }
    });
    let attachments_json = serde_json::to_string(&attachments)?;
    Ok(attachments_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test_test_case_to_form() {
        let config = validate("tests/test_config_CWL.yml", &None::<String>).unwrap();
        let result = test_case_to_form(&config.workflow, &config.workflow.testing[0]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wf_type() {
        let config: Config = serde_yaml::from_reader(BufReader::new(
            File::open("tests/test_config_CWL.yml").unwrap(),
        ))
        .unwrap();
        assert_eq!(&wf_type(&config.workflow), "CWL");
    }

    #[test]
    fn test_wf_version() {
        let config: Config = serde_yaml::from_reader(BufReader::new(
            File::open("tests/test_config_CWL.yml").unwrap(),
        ))
        .unwrap();
        assert_eq!(&wf_version(&config.workflow), "v1.0");
    }

    #[test]
    fn test_wf_url() {
        let config: Config = serde_yaml::from_reader(BufReader::new(
            File::open("tests/test_config_CWL.yml").unwrap(),
        ))
        .unwrap();
        assert_eq!(&wf_url(&config.workflow).unwrap(), "https://raw.githubusercontent.com/ddbj/yevis-cli/645a193826bdb3f0731421d4ff1468d0736b4a06/tests/CWL/wf/trimming_and_qc.cwl");
    }

    #[test]
    fn test_wf_engine_name() {
        let config: Config = serde_yaml::from_reader(BufReader::new(
            File::open("tests/test_config_CWL.yml").unwrap(),
        ))
        .unwrap();
        assert_eq!(&wf_engine_name(&config.workflow), "cwltool");
    }

    #[test]
    fn test_wf_params() {
        let config: Config = serde_yaml::from_reader(BufReader::new(
            File::open("tests/test_config_CWL.yml").unwrap(),
        ))
        .unwrap();
        assert!(&wf_params(&config.workflow.testing[0]).is_ok());
    }

    #[test]
    fn test_wf_engine_params() {
        let config: Config = serde_yaml::from_reader(BufReader::new(
            File::open("tests/test_config_CWL.yml").unwrap(),
        ))
        .unwrap();
        assert_eq!(
            &wf_engine_params(&config.workflow.testing[0]).unwrap(),
            "{}"
        );
    }
}
