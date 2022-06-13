use crate::gh_trs::config;
use crate::gh_trs::env;
use crate::gh_trs::wes;

use anyhow::{anyhow, bail, ensure, Result};
use colored::Colorize;
use log::{debug, info, warn};
use std::env as std_env;
use std::fs;
use std::io::{BufWriter, Write};
use std::thread;
use std::time;
use url::Url;

pub struct TestResult {
    pub id: String,
    pub status: wes::RunStatus,
    pub run_log: String,
}

pub fn test(
    configs: &Vec<config::types::Config>,
    wes_loc: &Option<Url>,
    docker_host: &Url,
    ignore_fail: bool,
) -> Result<()> {
    let wes_loc = match wes_loc {
        Some(wes_loc) => wes_loc.clone(),
        None => {
            wes::start_wes(docker_host)?;
            Url::parse(&wes::default_wes_location())?
        }
    };
    info!("Use WES location: {} for testing", wes_loc);

    let supported_wes_versions = wes::get_supported_wes_versions(&wes_loc)?;
    ensure!(
        supported_wes_versions
            .into_iter()
            .any(|v| &v == "sapporo-wes-1.0.1"),
        "gh-trs only supports WES version sapporo-wes-1.0.1"
    );

    let in_ci = env::in_ci();

    for config in configs {
        info!(
            "Test workflow_id: {}, version: {}",
            config.id, config.version
        );
        let mut test_results = vec![];
        for test_case in &config.workflow.testing {
            info!("Testing test case: {}", test_case.id);

            let form = wes::test_case_to_form(&config.workflow, test_case)?;
            debug!("Form:\n{:#?}", &form);
            let run_id = wes::post_run(&wes_loc, form)?;
            info!("WES run_id: {}", run_id);
            let mut status = wes::RunStatus::Running;
            while status == wes::RunStatus::Running {
                status = wes::get_run_status(&wes_loc, &run_id)?;
                debug!("WES run status: {:?}", status);
                thread::sleep(time::Duration::from_secs(5));
            }
            let run_log = serde_json::to_string_pretty(&wes::get_run_log(&wes_loc, &run_id)?)?;
            if in_ci {
                let test_log_file = std_env::current_dir()?.join(format!(
                    "test-logs/{}_{}_{}.log",
                    config.id, config.version, test_case.id
                ));
                fs::create_dir_all(
                    test_log_file
                        .parent()
                        .ok_or_else(|| anyhow!("Failed to create dir"))?,
                )?;
                let mut buffer = BufWriter::new(fs::File::create(&test_log_file)?);
                buffer.write_all(run_log.as_bytes())?;
            }
            match status {
                wes::RunStatus::Complete => {
                    info!("Complete test case: {}", test_case.id);
                    debug!("Run log:\n{}", run_log);
                }
                wes::RunStatus::Failed => {
                    info!(
                        "Failed test case: {} with run_log:\n{}",
                        test_case.id, run_log
                    );
                }
                _ => {
                    unreachable!("WES run status: {:?}", status);
                }
            }
            test_results.push(TestResult {
                id: test_case.id.clone(),
                status,
                run_log,
            });
        }
        match check_test_results(&test_results) {
            Ok(()) => {
                info!(
                    "Passed all test cases in workflow_id: {}, version: {}",
                    config.id, config.version
                );
            }
            Err(e) => {
                if ignore_fail {
                    warn!("{}, but ignore_fail is true", e);
                } else {
                    bail!(e);
                }
            }
        }
    }

    wes::stop_wes(docker_host)?;
    Ok(())
}

pub fn check_test_results(test_results: &[TestResult]) -> Result<()> {
    let failed_tests = test_results
        .iter()
        .filter(|r| r.status == wes::RunStatus::Failed)
        .collect::<Vec<_>>();
    if !failed_tests.is_empty() {
        bail!(
            "{} {} tests: {}",
            "Failed".red(),
            failed_tests.len(),
            failed_tests
                .iter()
                .map(|r| r.id.clone())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
