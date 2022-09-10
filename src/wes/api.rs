use crate::metadata;

use anyhow::{anyhow, bail, ensure, Result};
use log::info;
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;
use std::time;
use url::Url;

pub fn get_service_info(wes_loc: &Url) -> Result<Value> {
    let url = Url::parse(&format!(
        "{}/service-info",
        wes_loc.as_str().trim().trim_end_matches('/')
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status.is_success(),
        "Failed to get service-info with status: {}. Response: {}",
        status,
        res_body
    );
    Ok(res_body)
}

pub fn sapporo_health_check(wes_loc: &Url) -> Result<()> {
    get_service_info(wes_loc)?;
    Ok(())
}

pub fn get_supported_wes_versions(wes_loc: &Url) -> Result<Vec<String>> {
    let res = get_service_info(wes_loc)?;
    let err_msg = "Failed to parse the response to get service-info";
    let supported_wes_versions = res
        .get("supported_wes_versions")
        .ok_or_else(|| anyhow!("{}", err_msg))?
        .as_array()
        .ok_or_else(|| anyhow!("{}", err_msg))?
        .iter()
        .map(|v| v.as_str().ok_or_else(|| anyhow!("{}", err_msg)))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|v| v.to_string())
        .collect();
    Ok(supported_wes_versions)
}

pub fn test_case_to_form(
    meta: &metadata::types::Metadata,
    test_case: &metadata::types::Testing,
) -> Result<multipart::Form> {
    let mut meta_cloned = meta.clone();
    meta_cloned.workflow.testing = vec![test_case.clone()];

    let form = multipart::Form::new()
        .text("workflow_type", meta.workflow.language.r#type.to_string())
        .text(
            "workflow_type_version",
            meta.workflow.language.version.clone(),
        )
        .text("workflow_url", wf_url(&meta.workflow)?)
        .text(
            "workflow_engine_name",
            match meta.workflow.language.r#type {
                metadata::types::LanguageType::Cwl => "cwltool",
                metadata::types::LanguageType::Wdl => "cromwell",
                metadata::types::LanguageType::Nfl => "nextflow",
                metadata::types::LanguageType::Smk => "snakemake",
                _ => bail!("Unsupported workflow language type"),
            },
        )
        .text("workflow_params", test_case.wf_params()?)
        .text("workflow_engine_parameters", test_case.wf_engine_params()?)
        .text(
            "workflow_attachment",
            wf_attachment(&meta.workflow, test_case)?,
        )
        .text("yevis_metadata", serde_json::to_string(&meta_cloned)?);
    Ok(form)
}

pub fn wf_url(wf: &metadata::types::Workflow) -> Result<String> {
    let primary_wf = wf.primary_wf()?;
    match wf.language.r#type {
        metadata::types::LanguageType::Nfl => {
            let file_name = match primary_wf.target.unwrap().to_str() {
                Some(file_name) => file_name.to_string(),
                None => primary_wf.url.path().to_string(),
            };
            Ok(file_name)
        }
        _ => Ok(primary_wf.url.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachedFile {
    file_name: PathBuf,
    file_url: Url,
}

impl AttachedFile {
    pub fn new_from_file(file: &metadata::types::File) -> Self {
        Self {
            file_name: file.target.clone().unwrap(),
            file_url: file.url.clone(),
        }
    }

    pub fn new_from_test_file(test_file: &metadata::types::TestFile) -> Self {
        Self {
            file_name: test_file.target.clone().unwrap(),
            file_url: test_file.url.clone(),
        }
    }
}

pub fn wf_attachment(
    wf: &metadata::types::Workflow,
    test_case: &metadata::types::Testing,
) -> Result<String> {
    let mut attachments: Vec<AttachedFile> = vec![];
    wf.files.iter().for_each(|f| match &f.r#type {
        metadata::types::FileType::Primary => {
            if wf.language.r#type == metadata::types::LanguageType::Nfl {
                attachments.push(AttachedFile::new_from_file(f));
            }
        }
        metadata::types::FileType::Secondary => {
            attachments.push(AttachedFile::new_from_file(f));
        }
    });
    test_case.files.iter().for_each(|f| {
        if f.r#type == metadata::types::TestFileType::Other {
            attachments.push(AttachedFile::new_from_test_file(f));
        }
    });
    let attachments_json = serde_json::to_string(&attachments)?;
    Ok(attachments_json)
}

pub fn post_run(wes_loc: &Url, form: multipart::Form) -> Result<String> {
    let url = Url::parse(&format!(
        "{}/runs",
        wes_loc.as_str().trim().trim_end_matches('/')
    ))?;
    let client = reqwest::blocking::Client::builder()
        .timeout(time::Duration::from_secs(300))
        .build()?;
    let response = client
        .post(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
        .multipart(form)
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to post run with status: {} from {}",
        response.status(),
        url.as_str()
    );
    let res_body = response.json::<Value>()?;
    let err_msg = "Failed to parse the response to post a run";
    let run_id = res_body
        .get("run_id")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?
        .to_string();
    Ok(run_id)
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Complete,
    Failed,
}

impl FromStr for RunStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "QUEUED" => Ok(RunStatus::Running),
            "INITIALIZING" => Ok(RunStatus::Running),
            "RUNNING" => Ok(RunStatus::Running),
            "PAUSED" => Ok(RunStatus::Running),
            "COMPLETE" => Ok(RunStatus::Complete),
            "EXECUTOR_ERROR" => Ok(RunStatus::Failed),
            "SYSTEM_ERROR" => Ok(RunStatus::Failed),
            "CANCELED" => Ok(RunStatus::Failed),
            "CANCELING" => Ok(RunStatus::Failed),
            "UNKNOWN" => bail!("Unknown run status: {}", s),
            _ => Err(anyhow!("Failed to parse run status")),
        }
    }
}

pub fn get_run_status(wes_loc: &Url, run_id: impl AsRef<str>) -> Result<RunStatus> {
    let url = Url::parse(&format!(
        "{}/runs/{}/status",
        wes_loc.as_str().trim().trim_end_matches('/'),
        run_id.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let mut retry_count = 0;
    let response = loop {
        match client.get(url.as_str()).send() {
            Ok(response) => break response,
            Err(e) => {
                retry_count += 1;
                if retry_count > 3 {
                    bail!("Failed to get run status: {}", e);
                }
                thread::sleep(time::Duration::from_secs(5));
            }
        }
    };
    ensure!(
        response.status().is_success(),
        "Failed to get run status with status: {} from {}",
        response.status(),
        url.as_str()
    );
    let err_msg = "Failed to parse the response to get run status";
    let res_body = response.json::<Value>()?;
    RunStatus::from_str(
        res_body
            .get("state")
            .ok_or_else(|| anyhow!(err_msg))?
            .as_str()
            .ok_or_else(|| anyhow!(err_msg))?,
    )
}

pub fn get_run_log(wes_loc: &Url, run_id: impl AsRef<str>) -> Result<Value> {
    let url = Url::parse(&format!(
        "{}/runs/{}",
        wes_loc.as_str().trim().trim_end_matches('/'),
        run_id.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to get run log with status: {} from {}",
        response.status(),
        url.as_str()
    );
    let res_body = response.json::<Value>()?;
    Ok(res_body)
}

pub fn fetch_ro_crate(wes_loc: &Url, run_id: impl AsRef<str>) -> Result<Value> {
    let url = Url::parse(&format!(
        "{}/runs/{}/data/ro-crate-metadata.json",
        wes_loc.as_str().trim().trim_end_matches('/'),
        run_id.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let mut retry = 0;
    while retry < 12 {
        let response = client.get(url.as_str()).send()?;
        if response.status().is_success() {
            let res_body = response.json::<Value>()?;
            return Ok(res_body);
        } else {
            retry += 1;
            info!("Waiting for the RO-Crate to be ready");
            thread::sleep(time::Duration::from_secs(20));
        }
    }
    bail!("Failed to fetch the RO-Crate");
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::env;
    use crate::wes;

    #[test]
    fn test_get_supported_wes_versions() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        wes::instance::start_wes(&docker_host)?;
        let wes_loc = wes::instance::default_wes_location();
        let supported_wes_versions = get_supported_wes_versions(&wes_loc)?;
        assert!(!supported_wes_versions.is_empty());
        wes::instance::stop_wes(&docker_host)?;
        Ok(())
    }

    #[test]
    fn test_post_run() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        wes::instance::start_wes(&docker_host)?;
        let wes_loc = wes::instance::default_wes_location();
        let gh_token = env::github_token(&None::<String>)?;
        let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml", &gh_token)?;
        let form = test_case_to_form(&meta, &meta.workflow.testing[0])?;
        let run_id = post_run(&wes_loc, form)?;
        assert!(!run_id.is_empty());
        wes::instance::stop_wes(&docker_host)?;
        Ok(())
    }
}
