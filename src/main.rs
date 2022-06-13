mod args;
mod env;
mod file_url;
mod github_api;
mod inspect;
mod io;
mod logger;
mod make_template;
mod metadata;
mod pr;
mod publish;
mod pull_request;
mod raw_url;
mod remote;
mod test;
mod trs;
mod validate;
mod version;
mod wes;
mod zenodo;

use anyhow::Result;
use colored::Colorize;
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
        args::Args::Publish { verbose, .. } => verbose,
    };
    logger::init_logger(verbose);

    info!("{} yevis", "Start".green());
    debug!("args: {:?}", args);

    match args {
        args::Args::MakeTemplate {
            workflow_location,
            github_token,
            output,
            use_commit_url,
            ..
        } => {
            info!("{} make-template", "Running".green());
            match make_template::make_template(
                &workflow_location,
                &github_token,
                &output,
                match use_commit_url {
                    true => raw_url::UrlType::Commit,
                    false => raw_url::UrlType::Branch,
                },
            ) {
                Ok(()) => info!("{} make-template", "Success".green()),
                Err(e) => {
                    error!("{} to make-template with error: {}", "Failed".red(), e);
                    exit(1);
                }
            }
        }
        args::Args::Validate {
            metadata_locations,
            github_token,
            repository,
            ..
        } => {
            info!("{} validate", "Running".green());
            match validate::validate(metadata_locations, &github_token, &repository) {
                Ok(_) => info!("{} validate", "Success".green()),
                Err(e) => {
                    error!("{} to validate with error: {}", "Failed".red(), e);
                    exit(1);
                }
            }
        }
        args::Args::Test {
            metadata_locations,
            github_token,
            repository,
            wes_location,
            docker_host,
            from_pr,
            ..
        } => {
            let metadata_locations = if from_pr {
                info!("Run yevis-cli test in from_pr mode");
                info!("GitHub Pull Request URL: {}", metadata_locations[0]);
                match pr::list_modified_files(&github_token, &metadata_locations[0]) {
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

            info!("{} validate", "Running".green());
            let configs = match validate::validate(metadata_locations, &github_token, &repository) {
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
                    match wes::stop_wes(&docker_host) {
                        Ok(_) => {}
                        Err(e) => error!("{} to stop the WES with error: {}", "Failed".red(), e),
                    }
                    error!("{} to test with error: {}", "Failed".red(), e);
                    exit(1);
                }
            };
        }
        args::Args::PullRequest {
            metadata_locations,
            github_token,
            repository,
            wes_location,
            docker_host,
            ..
        } => {
            info!("{} validate", "Running".green());
            let configs = match validate::validate(metadata_locations, &github_token, &repository) {
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
                    match wes::stop_wes(&docker_host) {
                        Ok(_) => {}
                        Err(e) => error!("{} to stop the WES with error: {}", "Failed".red(), e),
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
        args::Args::Publish {
            metadata_locations,
            github_token,
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
            if !env::in_ci() {
                info!("yevis-cli publish is only available in the CI environment (GitHub Actions). Aborting.");
                exit(1);
            }

            let metadata_locations = if from_pr {
                info!("Run yevis-cli test in from_pr mode");
                info!("GitHub Pull Request URL: {}", metadata_locations[0]);
                match pr::list_modified_files(&github_token, &metadata_locations[0]) {
                    Ok(files) => files,
                    Err(e) => {
                        error!(
                            "{} to get modified files from GitHub Pull Request URL with error: {}",
                            "Failed".red(),
                            e
                        );
                        exit(1);
                    }
                }
            } else {
                metadata_locations
            };

            let metadata_locations = if from_trs {
                info!("Run yevis-cli publish in from_trs mode");
                info!("TRS endpoint: {}", metadata_locations[0]);
                match io::find_config_loc_recursively_from_trs(&metadata_locations[0]) {
                    Ok(metadata_locations) => metadata_locations,
                    Err(e) => {
                        error!(
                            "{} to find metadata file locations from TRS endpoint with error: {}",
                            "Failed".red(),
                            e
                        );
                        exit(1);
                    }
                }
            } else {
                metadata_locations
            };

            info!("{} validate", "Running".green());
            let mut configs =
                match validate::validate(metadata_locations, &github_token, &repository) {
                    Ok(configs) => {
                        info!("{} validate", "Success".green());
                        configs
                    }
                    Err(e) => {
                        error!("{} to validate with error: {}", "Failed".red(), e);
                        exit(1);
                    }
                };

            if upload_zenodo {
                info!("{} upload_zenodo", "Running".green());
                match zenodo::upload_and_commit_zenodo(
                    &mut configs,
                    &github_token,
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

            let verified = if with_test {
                info!("{} test", "Running".green());
                match test::test(&configs, &wes_location, &docker_host) {
                    Ok(()) => info!("{} test", "Success".green()),
                    Err(e) => {
                        match wes::stop_wes(&docker_host) {
                            Ok(_) => {}
                            Err(e) => {
                                error!("{} to stop the WES with error: {}", "Failed".red(), e)
                            }
                        }
                        error!("{} to test with error: {}", "Failed".red(), e);
                        exit(1);
                    }
                }
                true
            } else {
                false
            };

            info!("{} publish", "Running".green());
            match publish::publish(&configs, &github_token, &repository, verified) {
                Ok(()) => info!("{} publish", "Success".green()),
                Err(e) => {
                    error!("{} to publish with error: {}", "Failed".red(), e);
                    exit(1);
                }
            };
        }
    }
    Ok(())
}
