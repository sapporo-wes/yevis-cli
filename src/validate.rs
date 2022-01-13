use crate::{
    args::FileFormat,
    github_api::{head_request, read_github_token, to_raw_url_from_url, WfRepoInfo},
    path_utils::file_format,
    type_config::{Author, Config, FileType, Repo, Workflow},
};
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
) -> Result<Config> {
    let github_token = read_github_token(&arg_github_token)?;

    let file_format = file_format(&config_file)?;
    let reader = BufReader::new(File::open(&config_file)?);
    let mut config: Config = match file_format {
        FileFormat::Yaml => serde_yaml::from_reader(reader)?,
        FileFormat::Json => serde_json::from_reader(reader)?,
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

fn validate_authors(authors: &Vec<Author>) -> Result<()> {
    ensure!(
        authors.len() < 3,
        "Please add at least one person and ddbj as authors.",
    );
    let mut ddbj_found = false;
    for author in authors {
        match author.github_account.as_str() {
            "ddbj" => {
                ensure!(
                    author == &Author::new_ddbj(),
                    "The value of author: ddbj has been changed."
                );
                ddbj_found = true;
            }
            _ => validate_author(&author)?,
        }
    }
    ensure!(ddbj_found, "Please add ddbj as an author.");

    Ok(())
}

fn validate_author(author: &Author) -> Result<()> {
    let re = Regex::new(r"^\d{4}-\d{4}-\d{4}-(\d{3}X|\d{4})$")?;
    ensure!(
        author.github_account != "",
        "`github_account` field in the authors is required."
    );
    ensure!(
        author.name != "",
        "`name` field in the authors is required."
    );
    if author.orcid != "" {
        ensure!(
            re.is_match(&author.orcid),
            "`orcid` field in the authors is invalid."
        );
    };

    Ok(())
}

fn validate_workflow(github_token: impl AsRef<str>, workflow: &Workflow) -> Result<Workflow> {
    let mut cloned_wf = workflow.clone();

    let primary_wf = match workflow
        .files
        .iter()
        .find(|f| f.r#type == FileType::Primary)
    {
        Some(f) => f,
        None => bail!("No primary workflow file found."),
    };
    let primary_wf_repo_info = WfRepoInfo::new(&github_token, &primary_wf.url)?;
    ensure!(
        workflow.repo == Repo::new(&primary_wf_repo_info),
        "The information for the primary workflow and values of `repo` field in the workflow are different."
    );

    let raw_readme_url = to_raw_url_from_url(&github_token, &primary_wf.url)?;
    match head_request(&raw_readme_url) {
        Ok(_) => {
            cloned_wf.readme = raw_readme_url;
        }
        Err(_) => bail!("Failed to request the readme: {}", &raw_readme_url),
    };

    for i in 0..workflow.files.len() {
        let file = &workflow.files[i];
        let raw_file_url = to_raw_url_from_url(&github_token, &file.url)?;
        match head_request(&raw_file_url) {
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
            let raw_file_url = to_raw_url_from_url(&github_token, &file.url)?;
            match head_request(&raw_file_url) {
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
