mod args;
mod env;
mod gh;
mod inspect;
mod logger;
mod metadata;
mod remote;
mod sub_cmd;
mod trs;
mod wes;
mod zenodo;

use anyhow::{anyhow, Result};
use colored::Colorize;
use log::{debug, error, info};
use std::process::exit;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args = args::Args::from_args();
    logger::init_logger(args.verbose());

    info!("{} yevis", "Start".green());
    debug!("args: {:?}", args);

    let gh_token = env::github_token(&args.gh_token())?;

    match args {
        args::Args::MakeTemplate {
            workflow_location,
            output,
            use_commit_url,
            ..
        } => {
            sub_cmd::make_template(&workflow_location, &gh_token, &output, &use_commit_url);
        }
        args::Args::Validate {
            metadata_locations, ..
        } => {
            sub_cmd::validate(metadata_locations, &gh_token);
        }
        args::Args::Test {
            metadata_locations,
            wes_location,
            docker_host,
            from_pr,
            fetch_ro_crate,
            ..
        } => {
            let meta_locs = if from_pr {
                info!("Run yevis-cli test in from_pr mode");
                let pr_url = metadata_locations.get(0).ok_or_else(|| {
                    anyhow!(
                        "GitHub PR url is required as `workflow_locations` when from_pr is true"
                    )
                })?;
                info!("GitHub Pull Request URL: {}", pr_url);
                match gh::pr::list_modified_files(&gh_token, &pr_url) {
                    Ok(files) => files,
                    Err(e) => {
                        error!(
                            "{} to get modified files from a GitHub Pull Request URL with error: {}",
                            "Failed".red(),
                            e
                        );
                        exit(1);
                    }
                }
            } else {
                metadata_locations
            };

            let meta_vec = sub_cmd::validate(meta_locs, &gh_token);
            sub_cmd::test(&meta_vec, &wes_location, &docker_host, fetch_ro_crate);
        }
        args::Args::PullRequest {
            metadata_locations,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            let meta_vec = sub_cmd::validate(metadata_locations, &gh_token);
            sub_cmd::test(&meta_vec, &wes_location, &docker_host, false);
            sub_cmd::pull_request(&meta_vec, &gh_token, &repository);
        }
        args::Args::Publish {
            metadata_locations,
            repository,
            with_test,
            wes_location,
            docker_host,
            from_pr,
            upload_zenodo,
            zenodo_community,
            ..
        } => {
            if !env::in_ci() {
                info!("yevis-cli publish is only available in the CI environment (GitHub Actions). Aborting.");
                exit(1);
            }

            let meta_locs = if from_pr {
                info!("Run yevis-cli publish in from_pr mode");
                let pr_url = metadata_locations.get(0).ok_or_else(|| {
                    anyhow!(
                        "GitHub PR url is required as `workflow_locations` when from_pr is true"
                    )
                })?;
                info!("GitHub Pull Request URL: {}", pr_url);
                match gh::pr::list_modified_files(&gh_token, &pr_url) {
                    Ok(files) => files,
                    Err(e) => {
                        error!(
                            "{} to get modified files from a GitHub Pull Request URL with error: {}",
                            "Failed".red(),
                            e
                        );
                        exit(1);
                    }
                }
            } else {
                metadata_locations
            };

            let mut meta_vec = sub_cmd::validate(meta_locs, &gh_token);

            if upload_zenodo {
                info!("{} upload_zenodo", "Running".green());
                match zenodo::upload_zenodo_and_commit_gh(
                    &mut meta_vec,
                    &gh_token,
                    &repository,
                    &zenodo_community,
                ) {
                    Ok(()) => info!("{} upload_zenodo", "Success".green()),
                    Err(e) => {
                        error!("{} to upload_zenodo with error: {}", "Failed".red(), e);
                        exit(1);
                    }
                }
            }

            if with_test {
                sub_cmd::test(&meta_vec, &wes_location, &docker_host, false);
            };

            sub_cmd::publish(&meta_vec, &gh_token, &repository, with_test);
        }
    };
    Ok(())
}
