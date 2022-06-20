pub mod make_template;
pub mod publish;
pub mod pull_request;
pub mod test;
pub mod validate;

use make_template::make_template as make_template_process;
// use publish::publish as publish_process;
// use pull_request::pull_request as pull_request_process;
use test::test as test_process;
use validate::validate as validate_process;

use crate::metadata;
use crate::wes;

use anyhow::ensure;
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
                exit(1);
            }
        },
    };
    info!("Use WES location: {} for testing", wes_loc);
    let supported_wes_versions = wes::api::get_supported_wes_versions(&wes_loc)?;
    ensure!(
        supported_wes_versions
            .into_iter()
            .any(|v| &v == "sapporo-wes-1.0.1"),
        "yevis only supports WES version `sapporo-wes-1.0.1`"
    );
}

// match sub_cmd::test(&meta_vec, &wes_location, &docker_host) {
//     Ok(()) => info!("{} test", "Success".green()),
//     Err(e) => {
//         match wes::stop_wes(&docker_host) {
//             Ok(_) => {}
//             Err(e) => error!("{} to stop the WES with error: {}", "Failed".red(), e),
//         }
//         error!("{} to test with error: {}", "Failed".red(), e);
//         exit(1);
//     }
// };
