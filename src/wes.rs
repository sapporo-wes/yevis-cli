use anyhow::{anyhow, bail, ensure, Context, Result};
use log::info;
use reqwest;
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use std::{thread, time};
use url::Url;

const SAPPORO_SERVICE_IMAGE: &str = "ghcr.io/sapporo-wes/sapporo-service:1.1.0";
const SAPPORO_SERVICE_NAME: &str = "yevis-sapporo-service";

pub fn inside_docker_container() -> bool {
    Path::new("/.dockerenv").exists()
}

pub fn default_wes_location() -> String {
    if inside_docker_container() {
        "http://yevis-sapporo-service:1122".to_string()
    } else {
        "http://localhost:1122".to_string()
    }
}

pub fn sapporo_run_dir() -> Result<String> {
    match env::var("SAPPORO_RUN_DIR") {
        Ok(run_dir) => Ok(run_dir),
        Err(e) => bail!("SAPPORO_RUN_DIR is not set: {}", e),
    }
}

pub fn start_wes(docker_host: &Url) -> Result<()> {
    let status = check_wes_running(docker_host)?;
    dbg!(status);
    if status {
        info!("The sapporo-service for yevis is already running. So skip starting it.");
        return Ok(());
    }

    info!(
        "Starting the sapporo-service for yevis using docker_host: {}",
        docker_host.as_str()
    );
    let arg_socket_val = &format!("{}:/var/run/docker.sock", docker_host.path());
    let sapporo_run_dir = &sapporo_run_dir()?;
    let arg_run_dir_val = &format!("{}:{}", sapporo_run_dir, sapporo_run_dir);
    let (arg_network, arg_network_val) = if inside_docker_container() {
        ("--network", "yevis-cli_default")
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
            "/tmp:/tmp",
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
        "Failed to start the sapporo-service: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    info!("{}", String::from_utf8_lossy(&output.stdout));
    thread::sleep(time::Duration::from_secs(3));
    Ok(())
}

pub fn stop_wes(docker_host: &Url) -> Result<()> {
    let status = check_wes_running(docker_host)?;
    if !status {
        info!("The sapporo-service for yevis is not running. So skip stopping it.");
        return Ok(());
    }

    info!("Stopping the sapporo-service for yevis");
    let process = Command::new("docker")
        .args(&["-H", docker_host.as_str(), "kill", SAPPORO_SERVICE_NAME])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Please make sure that the docker command is present in your PATH")?;
    let output = process.wait_with_output()?;
    ensure!(
        output.status.success(),
        "Failed to stop the sapporo-service: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    info!("{}", String::from_utf8_lossy(&output.stdout));
    thread::sleep(time::Duration::from_secs(3));
    Ok(())
}

fn check_wes_running(docker_host: &Url) -> Result<bool> {
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to check yevis's sapporo-service status: {}", stderr);
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub supported_wes_versions: Vec<String>,
}

pub fn get_service_info(wes_loc: &Url) -> Result<ServiceInfo> {
    let url = wes_loc.join("/service-info")?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to get the service info with status: {} from {}",
        response.status(),
        url.as_str()
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => {
            let supported_wes_versions = body["supported_wes_versions"]
                .as_array()
                .ok_or(anyhow!(
                    "Failed to parse response when getting service info"
                ))?
                .iter()
                .map(|v| -> Result<&str> {
                    v.as_str().ok_or(anyhow!(
                        "Failed to parse response when getting service info"
                    ))
                })
                .collect::<Result<Vec<&str>>>()?
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>();
            Ok(ServiceInfo {
                supported_wes_versions,
            })
        }
        false => bail!("The service info is not an object"),
    }
}

fn post_runs(wes_loc: &Url, form: multipart::Form) -> Result<String> {
    let url = wes_loc.join("/runs")?;
    let client = reqwest::blocking::Client::new();
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
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => Ok(body["run_id"]
            .as_str()
            .ok_or(anyhow!("Failed to parse response when posting run"))?
            .to_string()),
        false => bail!("Response from posting run is not an object"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lib_test::test_case_to_form;
    use crate::validate::validate;

    #[test]
    fn test_start_wes() {
        let docker_host = Url::parse("unix:///var/run/docker.sock").unwrap();
        assert!(start_wes(&docker_host).is_ok());
        stop_wes(&docker_host).unwrap();
    }

    #[test]
    fn test_stop_wes() {
        let docker_host = Url::parse("unix:///var/run/docker.sock").unwrap();
        start_wes(&docker_host).unwrap();
        assert!(stop_wes(&docker_host).is_ok());
    }

    #[test]
    fn test_check_wes_running() {
        let docker_host = Url::parse("unix:///var/run/docker.sock").unwrap();
        start_wes(&docker_host).unwrap();
        assert!(check_wes_running(&docker_host).unwrap());
    }

    #[test]
    fn test_check_wes_running_with_invalid_docker_host() {
        let docker_host = Url::parse("unix:///var/run/invalid").unwrap();
        let result = check_wes_running(&docker_host);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot connect to the Docker daemon at unix:///var/run/invalid. Is the docker daemon running?"));
    }

    #[test]
    fn test_get_service_info() {
        let docker_host = Url::parse("unix:///var/run/docker.sock").unwrap();
        start_wes(&docker_host).unwrap();
        let wf_loc = Url::parse(&default_wes_location()).unwrap();
        let service_info = get_service_info(&wf_loc).unwrap();
        assert_eq!(
            service_info,
            ServiceInfo {
                supported_wes_versions: vec!["sapporo-wes-1.0.1".to_string()],
            }
        );
        stop_wes(&docker_host).unwrap();
    }

    #[test]
    fn test_post_runs() {
        let docker_host = Url::parse("unix:///var/run/docker.sock").unwrap();
        start_wes(&docker_host).unwrap();
        let wf_loc = Url::parse(&default_wes_location()).unwrap();
        let config = validate("tests/test_config_CWL.yml", &None::<String>).unwrap();
        let form = test_case_to_form(&config.workflow, &config.workflow.testing[0]).unwrap();
        assert!(post_runs(&wf_loc, form).is_ok());
        stop_wes(&docker_host).unwrap();
    }
}
