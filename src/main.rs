mod args;
mod github_api;
mod lib_test;
mod make_template;
mod path_utils;
mod pull_request;
mod remote;
mod type_config;
mod validate;
mod workflow_type_version;
use anyhow::Result;
use args::Args;
use env_logger;
use lib_test::test;
use log::{debug, error, info};
use make_template::make_template;
use pull_request::pull_request;
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
    env_logger::init_from_env(env_logger::Env::default().filter_or(
        env_logger::DEFAULT_FILTER_ENV,
        if verbose { "debug" } else { "info" },
    ));

    info!("Start yevis {}", env!("CARGO_PKG_VERSION"));
    debug!("args: {:?}", args);

    match &args {
        Args::MakeTemplate {
            workflow_location,
            github_token,
            output,
            format,
            ..
        } => {
            info!("Running make-template");
            match make_template(&workflow_location, &github_token, &output, &format) {
                Ok(()) => info!("Successfully make-template"),
                Err(e) => error!("{}", e),
            };
        }
        Args::Validate {
            config_file,
            github_token,
            ..
        } => {
            validate(&config_file, &github_token)?;
        }
        Args::Test {
            config_file,
            github_token,
            wes_location,
            docker_host,
            ..
        } => {
            test(&config_file, &github_token, &wes_location, &docker_host)?;
        }
        Args::PullRequest {
            config_file,
            github_token,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            validate(&config_file, &github_token)?;
            test(&config_file, &github_token, &wes_location, &docker_host)?;
            pull_request(
                &config_file,
                &github_token,
                &repository,
                &wes_location,
                &docker_host,
            )?;
        }
    }
    Ok(())
}
