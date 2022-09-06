use crate::env;
use crate::wes;

use anyhow::{anyhow, bail, ensure, Context, Result};
use colored::Colorize;
use log::{error, info};
use std::env as std_env;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time;
use url::Url;

pub const SAPPORO_SERVICE_IMAGE: &str = "ghcr.io/sapporo-wes/sapporo-service:latest";
pub const SAPPORO_SERVICE_NAME: &str = "yevis-sapporo-service";

pub fn inside_docker_container() -> bool {
    Path::new("/.dockerenv").exists()
}

pub fn default_wes_location() -> Url {
    if inside_docker_container() {
        Url::parse(&format!("http://{}:1122", SAPPORO_SERVICE_NAME)).unwrap()
    } else {
        Url::parse("http://localhost:1122").unwrap()
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
        match wes::api::sapporo_health_check(&default_wes_location()) {
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

pub fn stop_wes_no_result(docker_host: &Url) {
    match stop_wes(docker_host) {
        Ok(_) => {}
        Err(e) => {
            error!("{} to stop WES instance with error: {}", "Failed".red(), e);
        }
    };
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

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
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
}
