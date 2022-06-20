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

            sub_cmd::validate(meta_locs, &gh_token);
        }
        args::Args::PullRequest {
            metadata_locations,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            unimplemented!()
            // info!("{} validate", "Running".green());
            // let meta_vec = match sub_cmd::validate(metadata_locations, &github_token, &repository) {
            //     Ok(meta_vec) => {
            //         info!("{} validate", "Success".green());
            //         meta_vec
            //     }
            //     Err(e) => {
            //         error!("{} to validate with error: {}", "Failed".red(), e);
            //         exit(1);
            //     }
            // };

            // info!("{} test", "Running".green());
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

            // info!("{} pull-request", "Running".green());
            // match sub_cmd::pull_request(&meta_vec, &github_token, &repository) {
            //     Ok(()) => info!("{} pull-request", "Success".green()),
            //     Err(e) => {
            //         error!("{} to pull-request with error: {}", "Failed".red(), e);
            //         exit(1);
            //     }
            // };
        }
        args::Args::Publish {
            metadata_locations,
            repository,
            with_test,
            wes_location,
            docker_host,
            from_trs,
            from_pr,
            upload_zenodo,
            zenodo_community,
            ..
        } => {
            unimplemented!()
            //     if !env::in_ci() {
            //         info!("yevis-cli publish is only available in the CI environment (GitHub Actions). Aborting.");
            //         exit(1);
            //     }

            //     let metadata_locations = if from_pr {
            //         info!("Run yevis-cli test in from_pr mode");
            //         info!("GitHub Pull Request URL: {}", metadata_locations[0]);
            //         match gh::pr::list_modified_files(&github_token, &metadata_locations[0]) {
            //             Ok(files) => files,
            //             Err(e) => {
            //                 error!(
            //                     "{} to get modified files from GitHub Pull Request URL with error: {}",
            //                     "Failed".red(),
            //                     e
            //                 );
            //                 exit(1);
            //             }
            //         }
            //     } else {
            //         metadata_locations
            //     };

            //     let metadata_locations = if from_trs {
            //         info!("Run yevis-cli publish in from_trs mode");
            //         info!("TRS endpoint: {}", metadata_locations[0]);
            //         match metadata::io::find_metadata_loc_recursively_from_trs(&metadata_locations[0]) {
            //             Ok(metadata_locations) => metadata_locations,
            //             Err(e) => {
            //                 error!(
            //                     "{} to find metadata file locations from TRS endpoint with error: {}",
            //                     "Failed".red(),
            //                     e
            //                 );
            //                 exit(1);
            //             }
            //         }
            //     } else {
            //         metadata_locations
            //     };

            //     info!("{} validate", "Running".green());
            //     let mut meta_vec =
            //         match sub_cmd::validate(metadata_locations, &github_token, &repository) {
            //             Ok(meta_vec) => {
            //                 info!("{} validate", "Success".green());
            //                 meta_vec
            //             }
            //             Err(e) => {
            //                 error!("{} to validate with error: {}", "Failed".red(), e);
            //                 exit(1);
            //             }
            //         };

            //     if upload_zenodo {
            //         info!("{} upload_zenodo", "Running".green());
            //         match zenodo::upload_and_commit_zenodo(
            //             &mut meta_vec,
            //             &github_token,
            //             &repository,
            //             &zenodo_community,
            //         ) {
            //             Ok(()) => info!("{} upload_zenodo", "Success".green()),
            //             Err(e) => {
            //                 error!("{} to upload_zenodo with error: {}", "Failed".red(), e);
            //                 exit(1);
            //             }
            //         }
            //     }

            //     let verified = if with_test {
            //         info!("{} test", "Running".green());
            //         match sub_cmd::test(&meta_vec, &wes_location, &docker_host) {
            //             Ok(()) => info!("{} test", "Success".green()),
            //             Err(e) => {
            //                 match wes::stop_wes(&docker_host) {
            //                     Ok(_) => {}
            //                     Err(e) => {
            //                         error!("{} to stop the WES with error: {}", "Failed".red(), e)
            //                     }
            //                 }
            //                 error!("{} to test with error: {}", "Failed".red(), e);
            //                 exit(1);
            //             }
            //         }
            //         true
            //     } else {
            //         false
            //     };

            //     info!("{} publish", "Running".green());
            //     match sub_cmd::publish(&meta_vec, &github_token, &repository, verified) {
            //         Ok(()) => info!("{} publish", "Success".green()),
            //         Err(e) => {
            //             error!("{} to publish with error: {}", "Failed".red(), e);
            //             exit(1);
            //         }
            //     };
        }
    };
    Ok(())
}
