use anyhow::{bail, ensure, Context, Result};
use log::info;
use std::process::{Command, Stdio};
use url::Url;

pub const DEFAULT_WES_LOCATION: &str = "http://yevis-sapporo-service:1122";
const YEVIS_NETWORK_NAME: &str = "yevis-cli_default";
const SAPPORO_SERVICE_IMAGE: &str = "ghcr.io/sapporo-wes/sapporo-service:1.1.0";
const SAPPORO_SERVICE_NAME: &str = "yevis-sapporo-service";

pub fn start_wes(docker_host: &Url) -> Result<()> {
    let status = check_wes_running(docker_host)?;
    if status {
        info!("The sapporo-service for yevis is already running. So skip starting it.");
        return Ok(());
    }

    info!(
        "Starting the sapporo-service for yevis using docker_host: {}",
        docker_host.as_str()
    );
    let process = Command::new("docker")
        .args(&[
            "-H",
            docker_host.as_str(),
            "run",
            "-d",
            "--rm",
            "--name",
            SAPPORO_SERVICE_NAME,
            "--network",
            YEVIS_NETWORK_NAME,
            SAPPORO_SERVICE_IMAGE,
            "sapporo",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()
        .context("Please make sure that the docker command is present in your PATH")?;
    let output = process.wait_with_output()?;
    ensure!(
        output.status.success(),
        "Failed to start the sapporo-service: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
