pub mod make_template;
pub mod publish;
pub mod pull_request;
pub mod test;
pub mod validate;

use make_template::make_template as make_template_process;
use publish::publish as publish_process;
use pull_request::pull_request as pull_request_process;
use test::test as test_process;
use validate::validate as validate_process;

use crate::env;
use crate::metadata;
use crate::wes;

use colored::Colorize;
use log::{error, info};
use std::path::Path;
use std::process::exit;
use url::Url;

pub fn make_template(
    wf_loc: &Url,
    gh_token: impl AsRef<str>,
    output: impl AsRef<Path>,
    use_commit_url: &bool,
) {
    info!("{} make-template", "Running".green());
    match make_template_process(wf_loc, &gh_token, &output, use_commit_url) {
        Ok(()) => info!("{} make-template", "Success".green()),
        Err(e) => {
            error!("{} to make-template with error: {}", "Failed".red(), e);
            exit(1);
        }
    }
}

pub fn validate(
    meta_locs: Vec<impl AsRef<str>>,
    gh_token: impl AsRef<str>,
) -> Vec<metadata::types::Metadata> {
    info!("{} validate", "Running".green());
    let mut meta_vec = vec![];
    for meta_loc in meta_locs {
        info!("Validating {}", meta_loc.as_ref());
        let meta = match validate_process(meta_loc, &gh_token) {
            Ok(meta) => meta,
            Err(e) => {
                error!("{} to validate with error: {}", "Failed".red(), e);
                exit(1);
            }
        };
        meta_vec.push(meta);
    }
    info!("{} validate", "Success".green());
    meta_vec
}

pub fn test(meta_vec: &Vec<metadata::types::Metadata>, wes_loc: &Option<Url>, docker_host: &Url) {
    info!("{} test", "Running".green());
    let wes_loc = match wes_loc {
        Some(wes_loc) => wes_loc.clone(),
        None => match wes::instance::start_wes(docker_host) {
            Ok(_) => wes::instance::default_wes_location(),
            Err(e) => {
                error!("{} to start WES instance with error: {}", "Failed".red(), e);
                wes::instance::stop_wes_no_result(docker_host);
                exit(1);
            }
        },
    };
    info!("Use WES location: {} for testing", wes_loc);
    match wes::api::get_supported_wes_versions(&wes_loc) {
        Ok(supported_wes_versions) => {
            if !supported_wes_versions
                .into_iter()
                .any(|v| v == "sapporo-wes-1.0.1")
            {
                error!(
                    "{}: Yevis only supports WES version `sapporo-wes-1.0.1`",
                    "Failed".red()
                );
                wes::instance::stop_wes_no_result(docker_host);
                exit(1);
            }
        }
        Err(e) => {
            error!(
                "{} to get supported WES versions with error: {}",
                "Failed".red(),
                e
            );
            wes::instance::stop_wes_no_result(docker_host);
            exit(1);
        }
    };
    let write_log = env::in_ci();
    for meta in meta_vec {
        info!("Test workflow_id: {}, version: {}", meta.id, meta.version);
        match test_process(meta, &wes_loc, write_log) {
            Ok(()) => {
                info!("{} test", "Success".green());
            }
            Err(e) => {
                error!("{} to test with error: {}", "Failed".red(), e);
                wes::instance::stop_wes_no_result(docker_host);
                exit(1);
            }
        };
    }
}

pub fn pull_request(
    meta_vec: &Vec<metadata::types::Metadata>,
    gh_token: impl AsRef<str>,
    repo: impl AsRef<str>,
) {
    info!("{} pull-request", "Running".green());
    match pull_request_process(meta_vec, &gh_token, &repo) {
        Ok(()) => info!("{} pull-request", "Success".green()),
        Err(e) => {
            error!("{} to pull-request with error: {}", "Failed".red(), e);
            exit(1);
        }
    };
}

pub fn publish(
    meta_vec: &Vec<metadata::types::Metadata>,
    gh_token: impl AsRef<str>,
    repo: impl AsRef<str>,
    verified: bool,
) {
    info!("{} publish", "Running".green());
    match publish_process(meta_vec, &gh_token, &repo, verified) {
        Ok(()) => info!("{} publish", "Success".green()),
        Err(e) => {
            error!("{} to publish with error: {}", "Failed".red(), e);
            exit(1);
        }
    };
}
