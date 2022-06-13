use crate::env;
use crate::metadata;

use anyhow::{anyhow, bail, ensure, Context, Result};
use log::info;
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env as std_env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time;
use url::Url;

const SAPPORO_SERVICE_IMAGE: &str = "ghcr.io/sapporo-wes/sapporo-service:1.2.4";
const SAPPORO_SERVICE_NAME: &str = "yevis-sapporo-service";

pub fn inside_docker_container() -> bool {
    Path::new("/.dockerenv").exists()
}

pub fn default_wes_location() -> String {
    if inside_docker_container() {
        format!("http://{}:1122", SAPPORO_SERVICE_NAME)
    } else {
        "http://localhost:1122".to_string()
    }
}

pub fn start_wes(docker_host: &Url) -> Result<()> {
    let status = check_wes_running(docker_host)?;
    if status {
        info!("sapporo-service is already running. So skip starting it.");
        return Ok(());
    }

    info!(
        "Starting sapporo-service using docker_host: {}",
        docker_host.as_str()
    );
    let sapporo_run_dir = &env::sapporo_run_dir()?;
    let arg_socket_val = &format!("{}:/var/run/docker.sock", docker_host.path());
    let arg_tmp_val = &format!(
        "{}:/tmp",
        std_env::temp_dir()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid path"))?
    );
    let arg_run_dir_val = &format!("{}:{}", sapporo_run_dir, sapporo_run_dir);
    let (arg_network, arg_network_val) = if inside_docker_container() {
        ("--network", "yevis-network")
    } else {
        ("-p", "1122:1122")
    };
    let process = Command::new("docker")
        .args(&[
            "-H",
            docker_host.as_str(),
            "run",
            "-d",
            "--rm",
            "-v",
            arg_socket_val,
            "-v",
            arg_tmp_val,
            "-v",
            arg_run_dir_val,
            arg_network,
            arg_network_val,
            "--name",
            SAPPORO_SERVICE_NAME,
            SAPPORO_SERVICE_IMAGE,
            "sapporo",
            "--run-dir",
            sapporo_run_dir,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Please make sure that the docker command is present in your PATH")?;
    let output = process.wait_with_output()?;
    ensure!(
        output.status.success(),
        "Failed to start sapporo-service:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    info!(
        "Stdout from docker:\n{}",
        String::from_utf8_lossy(&output.stdout).trim()
    );

    // health check
    let mut retry = 0;
    while retry < 5 {
        match sapporo_health_check() {
            Ok(_) => break,
            Err(_) => thread::sleep(time::Duration::from_secs(2)),
        }
        retry += 1;
    }
    ensure!(
        retry < 5,
        "Failed to start sapporo-service:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

pub fn stop_wes(docker_host: &Url) -> Result<()> {
    let status = check_wes_running(docker_host)?;
    if !status {
        info!("sapporo-service is not running. So skip stopping it.");
        return Ok(());
    }

    info!("Stopping sapporo-service");
    let process = Command::new("docker")
        .args(&["-H", docker_host.as_str(), "kill", SAPPORO_SERVICE_NAME])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Please make sure that the docker command is present in your PATH")?;
    let output = process.wait_with_output()?;
    ensure!(
        output.status.success(),
        "Failed to stop the sapporo-service:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    info!(
        "Stdout from docker:\n{}",
        String::from_utf8_lossy(&output.stdout).trim()
    );
    thread::sleep(time::Duration::from_secs(3));
    Ok(())
}

pub fn check_wes_running(docker_host: &Url) -> Result<bool> {
    let process = Command::new("docker")
        .args(&[
            "-H",
            docker_host.as_str(),
            "ps",
            "-f",
            &format!("name={}", SAPPORO_SERVICE_NAME),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Please make sure that the docker command is present in your PATH")?;
    let output = process.wait_with_output()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains(SAPPORO_SERVICE_NAME) {
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        bail!(
            "Failed to check sapporo-service status:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

pub fn sapporo_health_check() -> Result<()> {
    let wes_loc = Url::parse(&default_wes_location())?;
    let url = Url::parse(&format!(
        "{}/service-info",
        wes_loc.as_str().trim().trim_end_matches('/')
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to get service-info with status: {} from {}",
        response.status(),
        url.as_str()
    );
    Ok(())
}

pub fn get_supported_wes_versions(wes_loc: &Url) -> Result<Vec<String>> {
    let url = Url::parse(&format!(
        "{}/service-info",
        wes_loc.as_str().trim().trim_end_matches('/')
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to get service-info with status: {} from {}",
        response.status(),
        url.as_str()
    );
    let res_body = response.json::<Value>()?;
    let err_msg = "Failed to parse the response to get service-info";
    let supported_wes_versions = res_body
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
    wf: &metadata::types::Workflow,
    test_case: &metadata::types::Testing,
) -> Result<multipart::Form> {
    let form = multipart::Form::new()
        .text(
            "workflow_type",
            wf.language.r#type.clone().unwrap().to_string(),
        )
        .text(
            "workflow_type_version",
            wf.language.version.clone().unwrap(),
        )
        .text("workflow_url", wf_url(wf)?)
        .text(
            "workflow_engine_name",
            match wf.language.r#type.clone().unwrap() {
                metadata::types::LanguageType::Cwl => "cwltool",
                metadata::types::LanguageType::Wdl => "cromwell",
                metadata::types::LanguageType::Nfl => "nextflow",
                metadata::types::LanguageType::Smk => "snakemake",
            },
        )
        .text("workflow_params", test_case.wf_params()?)
        .text("workflow_engine_parameters", test_case.wf_engine_params()?)
        .text("workflow_attachment", wf_attachment(wf, test_case)?);
    Ok(form)
}

pub fn wf_url(wf: &metadata::types::Workflow) -> Result<String> {
    let primary_wf = wf.primary_wf()?;
    match wf.language.r#type.clone().unwrap() {
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
            if wf.language.r#type.clone().unwrap() == metadata::types::LanguageType::Nfl {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_wes() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        assert!(start_wes(&docker_host).is_ok());
        stop_wes(&docker_host)?;
        Ok(())
    }

    #[test]
    fn test_stop_wes() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        start_wes(&docker_host)?;
        assert!(stop_wes(&docker_host).is_ok());
        Ok(())
    }

    #[test]
    fn test_check_wes_running() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        start_wes(&docker_host)?;
        assert!(check_wes_running(&docker_host)?);
        Ok(())
    }

    #[test]
    fn test_check_wes_running_with_invalid_docker_host() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/invalid")?;
        let result = check_wes_running(&docker_host);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot connect to the Docker daemon"));
        Ok(())
    }

    #[test]
    fn test_get_supported_wes_versions() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        start_wes(&docker_host)?;
        let wes_loc = Url::parse(&default_wes_location())?;
        let supported_wes_versions = get_supported_wes_versions(&wes_loc)?;
        assert!(!supported_wes_versions.is_empty());
        stop_wes(&docker_host)?;
        Ok(())
    }

    #[test]
    fn test_post_run() -> Result<()> {
        let docker_host = Url::parse("unix:///var/run/docker.sock")?;
        start_wes(&docker_host)?;
        let wes_loc = Url::parse(&default_wes_location())?;
        let config = metadata::io::read_config("./tests/test_config_CWL_validated.yml")?;
        let form = test_case_to_form(&config.workflow, &config.workflow.testing[0])?;
        let run_id = post_run(&wes_loc, form)?;
        assert!(!run_id.is_empty());
        stop_wes(&docker_host)?;
        Ok(())
    }
}
