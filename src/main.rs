mod args;
mod github_api;
mod lib_test;
mod make_template;
mod path_utils;
mod pull_request;
mod remote;
mod type_config;
mod validate;
mod wes;
mod workflow_type_version;

use crate::wes::stop_wes;
use anyhow::Result;
use args::Args;
use colored::Colorize;
use env_logger;
use lib_test::test;
use log::{debug, error, info};
use make_template::make_template;
use pull_request::pull_request;
use std::io::Write;
use std::process::exit;
use structopt::StructOpt;
use validate::validate;

fn main() -> Result<()> {
    let args = Args::from_args();
    let verbose = match &args {
        Args::Test { verbose, .. } => *verbose,
        Args::Validate { verbose, .. } => *verbose,
        Args::MakeTemplate { verbose, .. } => *verbose,
        Args::PullRequest { verbose, .. } => *verbose,
    };

    let env = env_logger::Env::default().filter_or(
        env_logger::DEFAULT_FILTER_ENV,
        if verbose { "debug" } else { "info" },
    );
    let mut builder = env_logger::Builder::from_env(env);
    if !verbose {
        builder.format(|buf, record| writeln!(buf, "{}", record.args()));
    }
    builder.init();

    info!("{} yevis {}", "Start".green(), env!("CARGO_PKG_VERSION"));
    debug!("args:\n{:#?}", args);

    match &args {
        Args::MakeTemplate {
            workflow_location,
            github_token,
            repository,
            output,
            format,
            update,
            ..
        } => {
            info!("{} make-template", "Running".green());
            match make_template(
                &workflow_location,
                &github_token,
                &repository,
                &output,
                &format,
                &update,
            ) {
                Ok(()) => info!("{} make-template", "Finished".green()),
                Err(e) => {
                    error!("{}: {}", "Error".red(), e);
                    exit(1);
                }
            };
        }
        Args::Validate {
            config_file,
            github_token,
            repository,
            ..
        } => {
            info!("{} validate", "Running".green());
            match validate(&config_file, &github_token, &repository) {
                Ok(_) => info!("{} validate", "Finished".green()),
                Err(e) => {
                    error!("{}: {}", "Error".red(), e);
                    exit(1);
                }
            };
        }
        Args::Test {
            config_file,
            github_token,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            info!("{} validate", "Running".green());
            let config = match validate(&config_file, &github_token, &repository) {
                Ok(config) => {
                    info!("{} validate", "Finished".green());
                    config
                }
                Err(e) => {
                    error!("{}: {}", "Error".red(), e);
                    exit(1);
                }
            };
            info!("{} test", "Running".green());
            match test(&config, &wes_location, &docker_host) {
                Ok(_) => info!("{} test", "Finished".green()),
                Err(e) => {
                    match stop_wes(&docker_host) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{}: {}", "Error".red(), e);
                            exit(1);
                        }
                    };
                    error!("{}: {}", "Error".red(), e);
                    exit(1);
                }
            };
        }
        Args::PullRequest {
            config_file,
            github_token,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            info!("{} validate", "Running".green());
            let config = match validate(&config_file, &github_token, &repository) {
                Ok(config) => {
                    info!("{} validate", "Finished".green());
                    config
                }
                Err(e) => {
                    error!("{}: {}", "Error".red(), e);
                    exit(1);
                }
            };
            info!("{} test", "Running".green());
            match test(&config, &wes_location, &docker_host) {
                Ok(_) => info!("{} test", "Finished".green()),
                Err(e) => {
                    match stop_wes(&docker_host) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{}: {}", "Error".red(), e);
                            exit(1);
                        }
                    };
                    error!("{}: {}", "Error".red(), e);
                    exit(1);
                }
            };
            info!("{} pull-request", "Running".green());
            match pull_request(&config, &github_token, &repository) {
                Ok(_) => info!("{} pull-request", "Finished".green()),
                Err(e) => {
                    error!("{}: {}", "Error".red(), e);
                    exit(1);
                }
            };
        }
    }
    Ok(())
}
