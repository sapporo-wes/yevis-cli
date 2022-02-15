mod args;
mod env;
mod make_template;
mod pull_request;
mod test;
mod validate;

use anyhow::Result;
use colored::Colorize;
use gh_trs;
use log::{debug, error, info};
use std::process::exit;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args = args::Args::from_args();
    let verbose = match args {
        args::Args::MakeTemplate { verbose, .. } => verbose,
        args::Args::Validate { verbose, .. } => verbose,
        args::Args::Test { verbose, .. } => verbose,
        args::Args::PullRequest { verbose, .. } => verbose,
    };
    gh_trs::logger::init_logger(verbose);

    info!("{} yevis", "Start".green());
    debug!("args: {:?}", args);

    match args {
        args::Args::MakeTemplate {
            workflow_location,
            github_token,
            output,
            update,
            ..
        } => {
            info!("{} make-template", "Running".green());
            match make_template::make_template(&workflow_location, &github_token, &output, update) {
                Ok(()) => info!("{} make-template", "Success".green()),
                Err(e) => {
                    error!("{} to make-template with error: {}", "Failed".red(), e);
                    exit(1);
                }
            }
        }
        args::Args::Validate {
            config_locations,
            github_token,
            repository,
            ..
        } => {
            info!("{} validate", "Running".green());
            match validate::validate(config_locations, &github_token, &repository) {
                Ok(_) => info!("{} validate", "Success".green()),
                Err(e) => {
                    error!("{} to validate with error: {}", "Failed".red(), e);
                    exit(1);
                }
            }
        }
        args::Args::Test {
            config_locations,
            github_token,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            info!("{} validate", "Running".green());
            let configs = match validate::validate(config_locations, &github_token, &repository) {
                Ok(configs) => {
                    info!("{} validate", "Success".green());
                    configs
                }
                Err(e) => {
                    error!("{} to validate with error: {}", "Failed".red(), e);
                    exit(1);
                }
            };

            info!("{} test", "Running".green());
            match test::test(&configs, &wes_location, &docker_host) {
                Ok(()) => info!("{} test", "Success".green()),
                Err(e) => {
                    match gh_trs::wes::stop_wes(&docker_host) {
                        Ok(_) => {}
                        Err(e) => error!("{} to stop WES with error: {}", "Failed".red(), e),
                    }
                    error!("{} to test with error: {}", "Failed".red(), e);
                    exit(1);
                }
            };
        }
        args::Args::PullRequest {
            config_locations,
            github_token,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            info!("{} validate", "Running".green());
            let configs = match validate::validate(config_locations, &github_token, &repository) {
                Ok(configs) => {
                    info!("{} validate", "Success".green());
                    configs
                }
                Err(e) => {
                    error!("{} to validate with error: {}", "Failed".red(), e);
                    exit(1);
                }
            };

            info!("{} test", "Running".green());
            match test::test(&configs, &wes_location, &docker_host) {
                Ok(()) => info!("{} test", "Success".green()),
                Err(e) => {
                    match gh_trs::wes::stop_wes(&docker_host) {
                        Ok(_) => {}
                        Err(e) => error!("{} to stop WES with error: {}", "Failed".red(), e),
                    }
                    error!("{} to test with error: {}", "Failed".red(), e);
                    exit(1);
                }
            };

            info!("{} pull-request", "Running".green());
            match pull_request::pull_request(&configs, &github_token, &repository) {
                Ok(()) => info!("{} pull-request", "Success".green()),
                Err(e) => {
                    error!("{} to pull-request with error: {}", "Failed".red(), e);
                    exit(1);
                }
            };
        }
    }
    Ok(())
}
