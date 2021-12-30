mod args;
mod lib_test;
mod make_template;
mod pull_request;
mod validate;
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
            output,
            format,
        } => {
            make_template(&workflow_location, &output, &format);
        }
        Args::Validate { config_file } => {
            validate(&config_file);
        }
        Args::Test {
            config_file,
            wes_location,
            docker_host,
        } => {
            test(&config_file, &wes_location, &docker_host);
        }
        Args::PullRequest {
            config_file,
            repository,
            wes_location,
            docker_host,
        } => {
            pull_request(&config_file, &repository, &wes_location, &docker_host);
        }
    }
}
