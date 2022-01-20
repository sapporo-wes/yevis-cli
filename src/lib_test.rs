use crate::{
    remote::fetch_raw_content,
    type_config::{Config, FileType, LanguageType, TestFileType, Testing, Workflow},
    wes::{
        default_wes_location, get_run_log, get_run_status, get_service_info, post_run, start_wes,
        stop_wes, RunStatus,
    },
};
use anyhow::{anyhow, bail, ensure, Result};
use log::{debug, info};
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::thread;
use std::time;
use url::Url;

pub fn test(
    config: &Config,
    wes_location: &Option<Url>,
    docker_host: &Url,
    in_ci: bool,
) -> Result<()> {
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

    let mut failed_tests = vec![];
    for test_case in &config.workflow.testing {
        info!("Testing {}", &test_case.id);
        let form = test_case_to_form(&config.workflow, &test_case)?;
        debug!("form:\n{:#?}", &form);
        let run_id = post_run(&wes_location, form)?;
        info!("WES run_id: {}", &run_id);
        let mut status = RunStatus::Running;
        while status == RunStatus::Running {
            status = get_run_status(&wes_location, &run_id)?;
            debug!("status: {:#?}", &status);
            thread::sleep(time::Duration::from_secs(5));
        }
        let run_log = get_run_log(&wes_location, &run_id)?;
        let run_log_str = serde_json::to_string_pretty(&run_log)?;
        if in_ci {
            let test_log_file =
                env::current_dir()?.join(format!("test-logs/{}_log.json", &test_case.id));
            fs::create_dir_all(
                &test_log_file
                    .parent()
                    .ok_or(anyhow!("Failed to create dir"))?,
            )?;
            let mut buffer = BufWriter::new(fs::File::create(&test_log_file)?);
            buffer.write(run_log_str.as_bytes())?;
        }
        match status {
            RunStatus::Complete => {
                info!("Complete {}", &test_case.id);
                debug!("Test result is:\n{}", &run_log_str);
            }
            RunStatus::Failed => {
                if in_ci {
                    failed_tests.push(&test_case.id);
                    info!("Failed {}.", &test_case.id);
                } else {
                    bail!("Failed {}. Log is:\n{}", &test_case.id, &run_log_str);
                }
            }
            _ => {}
        }
    }

    if failed_tests.len() > 0 {
        bail!("Test failed: {:#?}", &failed_tests);
    }

    stop_wes(&docker_host)?;

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
    match workflow.language.r#type {
        LanguageType::Nfl => {
            let file_name = match primary_wf.target.to_str() {
                Some(file_name) => file_name.to_string(),
                None => primary_wf.url.path().to_string(),
            };
            Ok(file_name)
        }
        _ => Ok(primary_wf.url.to_string()),
    }
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
    workflow.files.iter().for_each(|f| match &f.r#type {
        FileType::Primary => match workflow.language.r#type {
            LanguageType::Nfl => {
                let file_name = match f.target.to_str() {
                    Some(file_name) => file_name.to_string(),
                    None => f.url.path().to_string(),
                };
                attachments.push(AttachedFile {
                    file_name: file_name,
                    file_url: f.url.clone(),
                })
            }
            _ => {}
        },
        FileType::Secondary => {
            let file_name = match f.target.to_str() {
                Some(file_name) => file_name.to_string(),
                None => f.url.path().to_string(),
            };
            attachments.push(AttachedFile {
                file_name: file_name,
                file_url: f.url.clone(),
            })
        }
    });
    test_case.files.iter().for_each(|f| match &f.r#type {
        TestFileType::Other => {
            let file_name = match f.target.to_str() {
                Some(file_name) => file_name.to_string(),
                None => f.url.path().to_string(),
            };
            attachments.push(AttachedFile {
                file_name: file_name,
                file_url: f.url.clone(),
            });
        }
        _ => {}
    });
    let attachments_json = serde_json::to_string(&attachments)?;
    Ok(attachments_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::default_ddbj_workflows;
    use crate::validate::validate;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test_test_cwl() -> Result<()> {
        let config = validate(
            "tests/test_config_CWL.yml",
            &None::<String>,
            default_ddbj_workflows(),
        )?;
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        test(&config, &None::<Url>, &docker_host, false)?;
        Ok(())
    }

    #[test]
    fn test_test_wdl() -> Result<()> {
        let config = validate(
            "tests/test_config_WDL.yml",
            &None::<String>,
            default_ddbj_workflows(),
        )?;
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        test(&config, &None::<Url>, &docker_host, false)?;
        Ok(())
    }

    #[test]
    fn test_test_nfl() -> Result<()> {
        let config = validate(
            "tests/test_config_NFL.yml",
            &None::<String>,
            default_ddbj_workflows(),
        )?;
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        test(&config, &None::<Url>, &docker_host, false)?;
        Ok(())
    }

    #[test]
    fn test_test_smk() -> Result<()> {
        let config = validate(
            "tests/test_config_SMK.yml",
            &None::<String>,
            default_ddbj_workflows(),
        )?;
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        test(&config, &None::<Url>, &docker_host, false)?;
        Ok(())
    }

    #[test]
    fn test_test_case_to_form() -> Result<()> {
        let config = validate(
            "tests/test_config_CWL.yml",
            &None::<String>,
            default_ddbj_workflows(),
        )?;
        let result = test_case_to_form(&config.workflow, &config.workflow.testing[0]);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_wf_type() -> Result<()> {
        let config: Config =
            serde_yaml::from_reader(BufReader::new(File::open("tests/test_config_CWL.yml")?))?;
        assert_eq!(&wf_type(&config.workflow), "CWL");
        Ok(())
    }

    #[test]
    fn test_wf_version() -> Result<()> {
        let config: Config =
            serde_yaml::from_reader(BufReader::new(File::open("tests/test_config_CWL.yml")?))?;
        assert_eq!(&wf_version(&config.workflow), "v1.0");
        Ok(())
    }

    #[test]
    fn test_wf_url() -> Result<()> {
        let config: Config =
            serde_yaml::from_reader(BufReader::new(File::open("tests/test_config_CWL.yml")?))?;
        assert_eq!(&wf_url(&config.workflow)?, "https://raw.githubusercontent.com/ddbj/yevis-cli/645a193826bdb3f0731421d4ff1468d0736b4a06/tests/CWL/wf/trimming_and_qc.cwl");
        Ok(())
    }

    #[test]
    fn test_wf_engine_name() -> Result<()> {
        let config: Config =
            serde_yaml::from_reader(BufReader::new(File::open("tests/test_config_CWL.yml")?))?;
        assert_eq!(&wf_engine_name(&config.workflow), "cwltool");
        Ok(())
    }

    #[test]
    fn test_wf_params() -> Result<()> {
        let config: Config =
            serde_yaml::from_reader(BufReader::new(File::open("tests/test_config_CWL.yml")?))?;
        assert!(&wf_params(&config.workflow.testing[0]).is_ok());
        Ok(())
    }

    #[test]
    fn test_wf_engine_params() -> Result<()> {
        let config: Config =
            serde_yaml::from_reader(BufReader::new(File::open("tests/test_config_CWL.yml")?))?;
        assert_eq!(&wf_engine_params(&config.workflow.testing[0])?, "{}");
        Ok(())
    }
}
