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
use lib_test::test;
use make_template::make_template;
use pull_request::pull_request;
use structopt::StructOpt;
use validate::validate;

fn main() -> Result<()> {
    let args = Args::from_args();
    match &args {
        Args::MakeTemplate {
            workflow_location,
            github_token,
            output,
            format,
        } => {
            make_template(&workflow_location, &github_token, &output, &format)?;
        }
        Args::Validate {
            config_file,
            github_token,
        } => {
            validate(&config_file, &github_token)?;
        }
        Args::Test {
            config_file,
            github_token,
            wes_location,
            docker_host,
        } => {
            test(&config_file, &github_token, &wes_location, &docker_host)?;
        }
        Args::PullRequest {
            config_file,
            github_token,
            repository,
            wes_location,
            docker_host,
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
