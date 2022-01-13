use crate::args;
use crate::github_api;
use crate::path_utils;
use crate::type_config;
use anyhow::{bail, ensure, Result};
use regex::Regex;
use serde_json;
use serde_yaml;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn validate(
    config_file: impl AsRef<Path>,
    arg_github_token: &Option<impl AsRef<str>>,
) -> Result<type_config::Config> {
    let github_token = github_api::read_github_token(&arg_github_token)?;

    let file_format = path_utils::file_format(&config_file)?;
    let reader = BufReader::new(File::open(&config_file)?);
    let mut config: type_config::Config = match file_format {
        args::FileFormat::Yaml => serde_yaml::from_reader(reader)?,
        args::FileFormat::Json => serde_json::from_reader(reader)?,
    };

    validate_version(&config.version)?;
    validate_license(&config.license)?;
    validate_authors(&config.authors)?;
    config.workflow = validate_workflow(&github_token, &config.workflow)?;

    Ok(config)
}

fn validate_version(version: impl AsRef<str>) -> Result<()> {
    // TODO validate version using github api
    let re = Regex::new(r"([0-9]+)\.([0-9]+)\.([0-9]+)$")?;
    ensure!(
        re.is_match(version.as_ref()),
        "Invalid sematic version: {}",
        version.as_ref()
    );
    Ok(())
}

fn validate_license(license: impl AsRef<str>) -> Result<()> {
    ensure!(
        license.as_ref() == "CC0-1.0",
        "Invalid license: {}, expected only `CC0-1.0`",
        license.as_ref()
    );
    Ok(())
}

fn validate_authors(authors: &Vec<type_config::Author>) -> Result<()> {
    ensure!(
        authors.len() < 3,
        "Please add at least one person and ddbj as authors.",
    );
    let mut ddbj_found = false;
    for author in authors {
        match author.github_account.as_str() {
            "ddbj" => {
                ensure!(
                    author == &type_config::Author::new_ddbj(),
                    "The value of author: ddbj has been changed."
                );
                ddbj_found = true;
            }
            _ => author.validate()?,
        }
    }
    ensure!(ddbj_found, "Please add ddbj as an author.");

    Ok(())
}

fn validate_workflow(
    github_token: impl AsRef<str>,
    workflow: &type_config::Workflow,
) -> Result<type_config::Workflow> {
    let mut cloned_wf = workflow.clone();

    let primary_wf = match workflow
        .files
        .iter()
        .find(|f| f.r#type == type_config::FileType::Primary)
    {
        Some(f) => f,
        None => bail!("No primary workflow file found."),
    };
    let primary_wf_repo_info = github_api::WfRepoInfo::new(&github_token, &primary_wf.url)?;
    ensure!(
        workflow.repo == type_config::Repo::new(&primary_wf_repo_info),
        "The information for the primary workflow and values of `repo` field in the workflow are different."
    );

    let raw_readme_url = github_api::to_raw_url_from_url(&github_token, &primary_wf.url)?;
    match github_api::head_request(&raw_readme_url) {
        Ok(_) => {
            cloned_wf.readme = raw_readme_url;
        }
        Err(_) => bail!("Failed to request the readme: {}", &raw_readme_url),
    };

    for i in 0..workflow.files.len() {
        let file = &workflow.files[i];
        let raw_file_url = github_api::to_raw_url_from_url(&github_token, &file.url)?;
        match github_api::head_request(&raw_file_url) {
            Ok(_) => {
                cloned_wf.files[i].url = raw_file_url;
            }
            Err(_) => bail!("Failed to request the file: {}", &raw_file_url),
        };
    }

    let mut test_id_set: HashSet<&str> = HashSet::new();
    for i in 0..workflow.testing.len() {
        let testing = &workflow.testing[i];
        for j in 0..testing.files.len() {
            let file = &testing.files[j];
            let raw_file_url = github_api::to_raw_url_from_url(&github_token, &file.url)?;
            match github_api::head_request(&raw_file_url) {
                Ok(_) => {
                    cloned_wf.testing[i].files[j].url = raw_file_url;
                }
                Err(_) => bail!("Failed to request the file: {}", &raw_file_url),
            };
        }
        match test_id_set.insert(testing.id.as_str()) {
            true => {}
            false => bail!("Duplicated test id: {}", testing.id.as_str()),
        }
    }

    Ok(cloned_wf)
}
