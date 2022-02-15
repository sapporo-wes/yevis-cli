use anyhow::{anyhow, bail, ensure, Result};
use gh_trs;
use log::{debug, info};
use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::thread;
use std::time;
use url::Url;
use uuid::Uuid;

pub fn test(
    configs: &Vec<gh_trs::config::types::Config>,
    wes_loc: &Option<Url>,
    docker_host: &Url,
) -> Result<()> {
    let wes_loc = match wes_loc {
        Some(wes_loc) => wes_loc.clone(),
        None => {
            gh_trs::wes::start_wes(&docker_host)?;
            Url::parse(&gh_trs::wes::default_wes_location())?
        }
    };
    info!("Use WES location: {} for testing", wes_loc);

    let supported_wes_versions = gh_trs::wes::get_supported_wes_versions(&wes_loc)?;
    ensure!(
        supported_wes_versions
            .into_iter()
            .find(|v| v == "sapporo-wes-1.0.1")
            .is_some(),
        "yevis only supports WES version `sapporo-wes-1.0.1`"
    );

    let in_ci = gh_trs::env::in_ci();

    for config in configs {
        info!(
            "Testing workflow_id: {}, version: {}",
            config.id, config.version
        );
        let mut test_results = vec![];
        for test_case in &config.workflow.testing {
            info!("Testing test_id: {}", test_case.id);

            let form = gh_trs::wes::test_case_to_form(&config.workflow, test_case)?;
            debug!("Form:\n{:#?}", &form);
            let run_id = gh_trs::wes::post_run(&wes_loc, form)?;
            info!("WES run_id: {}", run_id);

            let mut status = gh_trs::wes::RunStatus::Running;
            while status == gh_trs::wes::RunStatus::Running {
                status = gh_trs::wes::get_run_status(&wes_loc, &run_id)?;
                debug!("WES run status: {:?}", status);
                thread::sleep(time::Duration::from_secs(5));
            }

            let run_log =
                serde_json::to_string_pretty(&gh_trs::wes::get_run_log(&wes_loc, &run_id)?)?;
            if in_ci {
                write_test_log(&config.id, &config.version, &test_case.id, &run_log)?;
            }
            match status {
                gh_trs::wes::RunStatus::Complete => {
                    info!("Complete test_id: {}", test_case.id);
                    debug!("Run log:\n{}", run_log);
                }
                gh_trs::wes::RunStatus::Failed => {
                    info!(
                        "Failed test_id: {} with run_log:\n{}",
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
            });
        }
        match check_test_results(test_results) {
            Ok(()) => info!(
                "Passed all tests in workflow_id: {}, version: {}",
                config.id, config.version
            ),
            Err(e) => bail!(e),
        };
    }

    gh_trs::wes::stop_wes(&docker_host)?;
    Ok(())
}

struct TestResult {
    pub id: String,
    pub status: gh_trs::wes::RunStatus,
}

fn write_test_log(
    id: &Uuid,
    version: impl AsRef<str>,
    test_id: impl AsRef<str>,
    run_log: impl AsRef<str>,
) -> Result<()> {
    let test_log_file = env::current_dir()?.join(format!(
        "test-logs/{}_{}_{}.log",
        id,
        version.as_ref(),
        test_id.as_ref()
    ));
    fs::create_dir_all(
        test_log_file
            .parent()
            .ok_or(anyhow!("Failed to create dir"))?,
    )?;
    let mut buffer = BufWriter::new(fs::File::create(&test_log_file)?);
    buffer.write(run_log.as_ref().as_bytes())?;
    Ok(())
}

fn check_test_results(test_results: Vec<TestResult>) -> Result<()> {
    let failed_tests = test_results
        .iter()
        .filter(|r| r.status == gh_trs::wes::RunStatus::Failed)
        .collect::<Vec<_>>();
    if failed_tests.len() > 0 {
        bail!(
            "Some tests failed. Failed tests: {}",
            failed_tests
                .iter()
                .map(|r| r.id.clone())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
