use crate::version;
use anyhow::{bail, ensure, Context, Result};
use gh_trs;
use log::{debug, info};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

pub fn validate(
    config_locs: Vec<impl AsRef<str>>,
    gh_token: &Option<impl AsRef<str>>,
    repo: impl AsRef<str>,
) -> Result<Vec<gh_trs::config::types::Config>> {
    let gh_token = gh_trs::env::github_token(gh_token)?;

    let mut configs = vec![];

    for config_loc in config_locs {
        info!("Validating {}", config_loc.as_ref());
        let mut config = gh_trs::config::io::read_config(config_loc.as_ref())?;

        validate_version(&config, &repo)?;
        validate_license(&config)?;
        validate_authors(&config)?;
        validate_language(&config)?;
        validate_and_update_workflow(&mut config, &gh_token)?;

        debug!("updated config: {:?}", config);

        configs.push(config);
    }
    Ok(configs)
}

fn validate_version(config: &gh_trs::config::types::Config, repo: impl AsRef<str>) -> Result<()> {
    let version =
        version::Version::from_str(&config.version).context("Invalid version, must be x.y.z")?;

    let (owner, name) = gh_trs::github_api::parse_repo(&repo)?;
    let trs_endpoint = gh_trs::trs::api::TrsEndpoint::new_gh_pages(&owner, &name)?;
    match trs_endpoint.is_valid() {
        Ok(_) => {
            match trs_endpoint.all_versions(&config.id.to_string()) {
                Ok(versions) => {
                    let versions = versions
                        .iter()
                        .map(|v| version::Version::from_str(v))
                        .collect::<Result<Vec<version::Version>>>();
                    if versions.is_err() {
                        // versions is an error, so nothing to do
                    } else {
                        let versions = versions.unwrap();
                        let latest_version = versions.into_iter().max().unwrap();
                        ensure!(
                            version > latest_version,
                            "Version {} is less than the latest version {}",
                            version.to_string(),
                            latest_version.to_string()
                        );
                    }
                }
                Err(_) => {} // Assume that it has not been published yet.
            }
        }
        Err(_) => {} // Assume that it has not been published yet.
    };
    Ok(())
}

fn validate_license(config: &gh_trs::config::types::Config) -> Result<()> {
    match &config.license {
        Some(license) => ensure!(license == "CC0-1.0", "The `license` is not `CC0-1.0`"),
        None => bail!("The `license` is not specified. In yevis, the `license` must be `CC0-1.0`"),
    };
    Ok(())
}

fn validate_authors(config: &gh_trs::config::types::Config) -> Result<()> {
    let orcid_re = Regex::new(r"^\d{4}-\d{4}-\d{4}-\d{3}[\dX]$")?;

    let mut account_set: HashSet<&str> = HashSet::new();
    for author in &config.authors {
        ensure!(
            author.name.is_some(),
            "The `authors[].name` is not specified",
        );
        ensure!(
            author.affiliation.is_some(),
            "The `authors[].affiliation` is not specified"
        );
        match &author.orcid {
            Some(orcid) => {
                ensure!(
                    orcid_re.is_match(orcid),
                    "The `authors[].orcid` is not valid",
                );
            }
            _ => {}
        };

        if author.github_account.as_str() == "ddbj" {
            ensure!(
                author.name.as_ref().unwrap() == "ddbj-workflow",
                "The ddbj author `name` is not `ddbj-workflow`",
            );
            ensure!(
                author.affiliation.as_ref().unwrap() == "DNA Data Bank of Japan",
                "The ddbj author `affiliation` is not `DDBJ`",
            );
            ensure!(
                author.orcid.is_none(),
                "The ddbj author `orcid` is not `None`",
            );
        }

        ensure!(
            !account_set.contains(author.github_account.as_str()),
            "The `authors[].github_account` is not unique",
        );
        account_set.insert(author.github_account.as_str());
    }

    ensure!(
        account_set.contains("ddbj"),
        "The `authors[].github_account` is not contained the ddbj author",
    );
    ensure!(
        config.authors.len() > 1,
        "The `authors` must have more than one author",
    );
    Ok(())
}

fn validate_language(config: &gh_trs::config::types::Config) -> Result<()> {
    ensure!(
        config.workflow.language.r#type.is_some(),
        "The `workflow.language.type` is not specified",
    );
    ensure!(
        config.workflow.language.version.is_some(),
        "The `workflow.language.version` is not specified",
    );
    Ok(())
}

fn validate_and_update_workflow(
    config: &mut gh_trs::config::types::Config,
    gh_token: impl AsRef<str>,
) -> Result<()> {
    let mut branch_memo = HashMap::new();
    let mut commit_memo = HashMap::new();

    config.workflow.readme = match gh_trs::raw_url::RawUrl::new(
        &gh_token,
        &config.workflow.readme,
        Some(&mut branch_memo),
        Some(&mut commit_memo),
    ) {
        Ok(raw_url) => raw_url.to_url()?,
        Err(e) => {
            bail!("The `workflow.readme` is not valid with error: {}", e);
        }
    };

    ensure!(
        config.workflow.primary_wf().is_ok(),
        "One `primary` needs to be specified in the `workflow.files[].type` field",
    );

    for file in &mut config.workflow.files {
        match file.update_url(&gh_token, Some(&mut branch_memo), Some(&mut commit_memo)) {
            Ok(()) => {}
            Err(e) => bail!("The `workflow.files[].url` is not valid with error: {}", e),
        };
        file.complement_target()?;
    }

    let mut test_id_set: HashSet<&str> = HashSet::new();
    for testing in &mut config.workflow.testing {
        ensure!(
            !test_id_set.contains(testing.id.as_str()),
            "The `workflow.testing[].id` is not unique, duplicated id: {}",
            testing.id.as_str()
        );
        test_id_set.insert(testing.id.as_str());

        for file in &mut testing.files {
            match file.update_url(&gh_token, Some(&mut branch_memo), Some(&mut commit_memo)) {
                Ok(()) => {}
                Err(e) => bail!(
                    "The `workflow.testing[].files[].url` is not valid with error: {}",
                    e
                ),
            };
            file.complement_target()?;
        }
    }
    Ok(())
}
