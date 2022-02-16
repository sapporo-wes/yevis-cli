use crate::env;
use crate::pull_request;

use anyhow::{anyhow, ensure, Result};
use crypto::digest::Digest;
use crypto::md5::Md5;
use gh_trs;
use log::info;
use reqwest::{self, blocking::multipart};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use tempfile;
use url::Url;

pub fn upload_and_commit_zenodo(
    configs: &mut Vec<gh_trs::config::types::Config>,
    gh_token: &Option<impl AsRef<str>>,
    repo: impl AsRef<str>,
) -> Result<()> {
    let host = env::zenodo_host();
    let token = env::zenodo_token()?;

    for config in configs {
        upload_zenodo(&host, &token, config)?;
        // update config with zenodo url
        info!("Updating config with Zenodo URL");
        update_config_files(&host, &token, config)?;

        // push modified config to GitHub default branch
        info!("Pushing modified config to GitHub");
        let gh_token = gh_trs::env::github_token(gh_token)?;
        let (owner, name) = gh_trs::github_api::parse_repo(&repo)?;
        let default_branch =
            gh_trs::github_api::get_default_branch(&gh_token, &owner, &name, None)?;
        let config_path = PathBuf::from(format!(
            "{}/yevis-config-{}.yml",
            &config.id, &config.version
        ));
        let config_content = serde_yaml::to_string(&config)?;
        let commit_message = format!(
            "Update a workflow with Zenodo URL, id: {} version: {}",
            &config.id, &config.version
        );
        pull_request::create_or_update_file(
            &gh_token,
            &owner,
            &name,
            &config_path,
            &commit_message,
            &config_content,
            &default_branch,
        )?;
    }
    Ok(())
}

fn upload_zenodo(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    config: &mut gh_trs::config::types::Config,
) -> Result<()> {
    info!(
        "Uploading wf_id: {}, version: {} to Zenodo",
        config.id, config.version
    );

    delete_unpublished_depositions(&host, &token, &config)?;
    let published_deposition_ids = list_depositions(
        &host,
        &token,
        &config.id.to_string(),
        DepositionStatus::Published,
    )?;
    ensure!(
        published_deposition_ids.len() < 2,
        "More than one published deposition for wf_id: {}",
        config.id
    );
    let deposition_id = if published_deposition_ids.len() == 0 {
        // create new deposition
        info!("Creating new deposition");
        create_deposition(&host, &token, &config)?
    } else {
        // new version deposition
        let prev_id = published_deposition_ids[0];
        info!("Creating new version deposition from {}", prev_id);
        let new_id = new_version_deposition(&host, &token, &prev_id)?;
        update_deposition(&host, &token, &new_id, &config)?;
        new_id
    };
    info!("Created draft deposition: {}", deposition_id);

    let deposition_files = get_files_list(&host, &token, &deposition_id)?;
    let config_files = config_to_files(&config)?;
    update_deposition_files(
        &host,
        &token,
        &deposition_id,
        deposition_files,
        config_files,
    )?;

    info!("Publishing deposition: {}", deposition_id);
    let zenodo = publish_deposition(&host, &token, &deposition_id)?;
    info!(
        "Published deposition: {} as DOI: {}",
        deposition_id, zenodo.doi
    );

    config.zenodo = Some(zenodo.clone());

    Ok(())
}

fn get_request(zenodo_token: impl AsRef<str>, url: &Url, query: &[(&str, &str)]) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .query(query)
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        status.is_success(),
        "Failed to get request to {}. Status: {}. Response: {}",
        url,
        status,
        res_body
    );
    Ok(res_body)
}

fn post_request(zenodo_token: impl AsRef<str>, url: &Url, body: &Value) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .json(body)
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        status.is_success(),
        "Failed to post request to {}. Status: {}. Response: {}",
        url,
        status,
        res_body
    );
    Ok(res_body)
}

fn put_request(zenodo_token: impl AsRef<str>, url: &Url, body: &Value) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .put(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .json(body)
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        status.is_success(),
        "Failed to put request to {}. Status: {}. Response: {}",
        url,
        status,
        res_body
    );
    Ok(res_body)
}

fn delete_request(zenodo_token: impl AsRef<str>, url: &Url) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .delete(url.as_str())
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .send()?;
    let status = response.status();
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        status.is_success(),
        "Failed to delete request to {}. Status: {}.",
        url,
        status,
    );
    Ok(())
}

enum DepositionStatus {
    Draft,
    Published,
}

impl fmt::Display for DepositionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DepositionStatus::Draft => write!(f, "draft"),
            DepositionStatus::Published => write!(f, "published"),
        }
    }
}

/// https://developers.zenodo.org/?shell#list
/// same id but different version: -> return only oldest version
fn list_depositions(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    wf_id: impl AsRef<str>,
    status: DepositionStatus,
) -> Result<Vec<u64>> {
    let mut url = Url::parse(&format!(
        "https://{}/api/deposit/depositions",
        host.as_ref()
    ))?;
    url.query_pairs_mut()
        .append_pair("q", &format!("DDBJ workflow: {}", wf_id.as_ref()))
        .append_pair("status", &status.to_string());
    let res = get_request(&token, &url, &[])?;
    let err_msg = "Failed to parse the response when listing depositions";
    let ids = res
        .as_array()
        .ok_or(anyhow!(err_msg))?
        .into_iter()
        .map(|d| {
            d.as_object()
                .ok_or(anyhow!(err_msg))
                .and_then(|d| d.get("id").ok_or(anyhow!(err_msg)))
                .and_then(|id| id.as_u64().ok_or(anyhow!(err_msg)))
        })
        .collect::<Result<Vec<u64>>>()?;
    Ok(ids)
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Deposition {
    pub upload_type: String,
    pub title: String,
    pub creators: Vec<Creator>,
    pub description: String,
    pub access_right: String,
    pub license: String,
    pub keywords: Vec<String>,
    pub communities: Vec<Community>,
    pub version: String,
}

impl Deposition {
    fn new(config: &gh_trs::config::types::Config) -> Result<Self> {
        Ok(Self {
            upload_type: "dataset".to_string(),
            title: format!("DDBJ workflow: {}", config.id),
            creators: config
                .authors
                .iter()
                .map(|author| Creator::new(author))
                .collect(),
            description: r#"This dataset was created by the <a href="https://github.com/ddbj/yevis-cli">GitHub - ddbj/yevis-cli</a>."#.to_string(),
            access_right: "open".to_string(),
            license: "cc-zero".to_string(),
            keywords: vec!["ddbj-workflow".to_string()],
            communities: vec![Community {
                identifier: "ddbj-workflow".to_string(),
            }],
            version: config.version.clone(),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Creator {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
}

impl Creator {
    fn new(author: &gh_trs::config::types::Author) -> Self {
        let name = match author.name.clone() {
            Some(name) => name.to_string(),
            None => author.github_account.clone(),
        };
        Self {
            name,
            affiliation: author.affiliation.clone(),
            orcid: author.orcid.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Community {
    pub identifier: String,
}

/// https://developers.zenodo.org/?shell#create
/// https://developers.zenodo.org/?shell#representation
fn create_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    config: &gh_trs::config::types::Config,
) -> Result<u64> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions",
        host.as_ref()
    ))?;
    let deposition = Deposition::new(&config)?;
    let body = json!({
        "metadata": deposition,
    });
    let res = post_request(&token, &url, &body)?;
    let err_msg = "Failed to parse the response when creating a deposition";
    let id = res
        .as_object()
        .ok_or(anyhow!(err_msg))?
        .get("id")
        .ok_or(anyhow!(err_msg))?
        .as_u64()
        .ok_or(anyhow!(err_msg))?;
    Ok(id)
}

/// https://developers.zenodo.org/?shell#update
fn update_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
    config: &gh_trs::config::types::Config,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}",
        host.as_ref(),
        deposition_id
    ))?;
    let deposition = Deposition::new(&config)?;
    let body = json!({
        "metadata": deposition,
    });
    put_request(&token, &url, &body)?;
    Ok(())
}

/// https://developers.zenodo.org/?shell#delete
/// can delete only draft
fn delete_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}",
        host.as_ref(),
        &deposition_id
    ))?;
    delete_request(&token, &url)?;
    Ok(())
}

/// https://developers.zenodo.org/?shell#publish
fn publish_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<gh_trs::config::types::Zenodo> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/actions/publish",
        host.as_ref(),
        &deposition_id
    ))?;
    let res = post_request(&token, &url, &json!({}))?;
    let err_msg = "Failed to parse the response when publishing a deposition";
    let res_obj = res.as_object().ok_or(anyhow!(err_msg))?;
    let id = res_obj
        .get("id")
        .ok_or(anyhow!(err_msg))?
        .as_u64()
        .ok_or(anyhow!(err_msg))?;
    let doi = res_obj
        .get("doi")
        .ok_or(anyhow!(err_msg))?
        .as_str()
        .ok_or(anyhow!(err_msg))?;
    let concept_doi = res_obj
        .get("conceptdoi")
        .ok_or(anyhow!(err_msg))?
        .as_str()
        .ok_or(anyhow!(err_msg))?;
    let url = Url::parse(&format!("https://{}/record/{}", host.as_ref(), &id))?;
    Ok(gh_trs::config::types::Zenodo {
        url,
        id,
        doi: doi.to_string(),
        concept_doi: concept_doi.to_string(),
    })
}

/// https://developers.zenodo.org/?shell#edit
fn new_version_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<u64> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/actions/newversion",
        host.as_ref(),
        &deposition_id
    ))?;
    let res = post_request(&token, &url, &json!({}))?;
    let err_msg = "Failed to parse the response when creating a new version of a deposition";
    let latest_draft = res
        .as_object()
        .ok_or(anyhow!(err_msg))?
        .get("links")
        .ok_or(anyhow!(err_msg))?
        .as_object()
        .ok_or(anyhow!(err_msg))?
        .get("latest_draft")
        .ok_or(anyhow!(err_msg))?
        .as_str()
        .ok_or(anyhow!(err_msg))?;
    let latest_draft_id: u64 = latest_draft
        .split("/")
        .last()
        .ok_or(anyhow!(err_msg))?
        .parse()?;
    Ok(latest_draft_id)
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct DepositionFile {
    id: String,
    filename: String,
    filesize: u64,
    checksum: String,
}

impl DepositionFile {
    fn download_url(&self, host: impl AsRef<str>, id: &u64) -> Result<Url> {
        Ok(Url::parse(&format!(
            "https://{}/record/{}/files/{}",
            host.as_ref(),
            id,
            self.filename
        ))?)
    }
}

/// https://developers.zenodo.org/?shell#list23
fn get_files_list(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<Vec<DepositionFile>> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/files",
        host.as_ref(),
        &deposition_id
    ))?;
    let res = get_request(&token, &url, &[])?;
    let files: Vec<DepositionFile> = serde_json::from_value(res)?;
    Ok(files)
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ConfigFile {
    filename: String,
    file_path: PathBuf,
    checksum: String,
}

impl ConfigFile {
    fn new(file_url: &Url, target: impl AsRef<Path>) -> Result<Self> {
        let (file, file_path) = tempfile::NamedTempFile::new()?.keep()?;
        let mut buffer = BufWriter::new(file);
        let content = gh_trs::remote::fetch_raw_content(&file_url)?;
        io::copy(&mut content.as_bytes(), &mut buffer)?;
        let filename = target.as_ref().to_string_lossy().to_string();
        let mut md5 = Md5::new();
        md5.input_str(&content);
        let checksum = md5.result_str();
        Ok(Self {
            filename,
            file_path,
            checksum,
        })
    }

    fn new_from_str(content: impl AsRef<str>, target: impl AsRef<Path>) -> Result<Self> {
        let (file, file_path) = tempfile::NamedTempFile::new()?.keep()?;
        let mut buffer = BufWriter::new(file);
        io::copy(&mut content.as_ref().as_bytes(), &mut buffer)?;
        let filename = target.as_ref().to_string_lossy().to_string();
        let mut md5 = Md5::new();
        md5.input_str(content.as_ref());
        let checksum = md5.result_str();
        Ok(Self {
            filename,
            file_path,
            checksum,
        })
    }
}

/// in deposition_files, in config_files
///   - checksum is the same: do nothing
///   - checksum is not the same: delete and create
/// in deposition_files, not in config_files: delete
/// not in deposition_files, in config_files: create
fn update_deposition_files(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
    deposition_files: Vec<DepositionFile>,
    config_files: Vec<ConfigFile>,
) -> Result<()> {
    let deposition_files_map: HashMap<String, DepositionFile> = deposition_files
        .into_iter()
        .map(|f| (f.filename.clone(), f))
        .collect();
    let config_files_map: HashMap<String, ConfigFile> = config_files
        .into_iter()
        .map(|f| (f.filename.clone(), f))
        .collect();

    for (filename, deposition_file) in deposition_files_map.iter() {
        match config_files_map.get(filename) {
            Some(config_file) => {
                if deposition_file.checksum == config_file.checksum {
                    // do nothing
                    continue;
                } else {
                    // delete and create
                    delete_deposition_file(&host, &token, deposition_id, &deposition_file.id)?;
                    create_deposition_file(
                        &host,
                        &token,
                        deposition_id,
                        &config_file.filename,
                        &config_file.file_path,
                    )?;
                }
            }
            None => {
                // delete
                delete_deposition_file(&host, &token, deposition_id, &deposition_file.id)?;
            }
        }
    }
    for (filename, config_file) in config_files_map.iter() {
        match deposition_files_map.get(filename) {
            Some(_) => {
                // do nothing (already done)
                continue;
            }
            None => {
                // create
                create_deposition_file(
                    &host,
                    &token,
                    deposition_id,
                    &config_file.filename,
                    &config_file.file_path,
                )?;
            }
        }
    }
    Ok(())
}

fn config_to_files(config: &gh_trs::config::types::Config) -> Result<Vec<ConfigFile>> {
    let mut files = vec![];
    files.push(ConfigFile::new_from_str(
        serde_yaml::to_string(&config)?,
        PathBuf::from(format!("yevis-config-{}.yml", config.version)),
    )?);
    files.push(ConfigFile::new(
        &config.workflow.readme,
        PathBuf::from("README.md"),
    )?);
    for file in &config.workflow.files {
        files.push(ConfigFile::new(&file.url, file.target.as_ref().unwrap())?); // validated
    }
    for testing in &config.workflow.testing {
        for file in &testing.files {
            files.push(ConfigFile::new(&file.url, file.target.as_ref().unwrap())?);
            // validated
        }
    }
    Ok(files)
}

/// https://developers.zenodo.org/?shell#create24
fn create_deposition_file(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
    file_name: impl AsRef<str>,
    file_path: impl AsRef<Path>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/files",
        host.as_ref(),
        &deposition_id
    ))?;
    let form = multipart::Form::new()
        .text("name", file_name.as_ref().to_string())
        .file("file", file_path.as_ref())?;

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token.as_ref()),
        )
        .multipart(form)
        .send()?;
    let status = response.status();
    let res_body = response.json::<Value>()?;
    ensure!(
        status != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        status.is_success(),
        "Failed to post request to {}. Status: {}. Response: {}",
        url,
        status,
        res_body
    );
    Ok(())
}

/// https://developers.zenodo.org/?shell#delete28
fn delete_deposition_file(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
    file_id: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/files/{}",
        host.as_ref(),
        &deposition_id,
        file_id.as_ref()
    ))?;
    delete_request(&token, &url)?;
    Ok(())
}

fn delete_unpublished_depositions(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    config: &gh_trs::config::types::Config,
) -> Result<()> {
    let draft_deposition_ids = list_depositions(
        &host,
        &token,
        &config.id.to_string(),
        DepositionStatus::Draft,
    )?;
    if draft_deposition_ids.len() > 0 {
        info!(
            "Found {} draft deposition(s), so deleting them",
            draft_deposition_ids.len()
        );
        for id in draft_deposition_ids {
            info!("Deleting draft deposition {}", id);
            delete_deposition(&host, &token, &id)?;
        }
    }
    Ok(())
}

fn update_config_files(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    config: &mut gh_trs::config::types::Config,
) -> Result<()> {
    let deposition_id = config
        .zenodo
        .as_ref()
        .ok_or(anyhow!("No Zenodo deposition ID"))?
        .id;
    let deposition_files = get_files_list(&host, &token, &deposition_id)?;
    let files_map: HashMap<PathBuf, DepositionFile> = deposition_files
        .into_iter()
        .map(|f| (PathBuf::from(f.filename.clone()), f))
        .collect();

    let err_msg = "Failed to update config files.";
    config.workflow.readme = files_map
        .get(&PathBuf::from("README.md"))
        .ok_or(anyhow!(err_msg))?
        .download_url(&host, &deposition_id)?;
    for file in &mut config.workflow.files {
        file.url = files_map
            .get(&PathBuf::from(file.target.as_ref().unwrap())) // already validated
            .ok_or(anyhow!(err_msg))?
            .download_url(&host, &deposition_id)?;
    }
    for testing in &mut config.workflow.testing {
        for file in &mut testing.files {
            file.url = files_map
                .get(&PathBuf::from(file.target.as_ref().unwrap())) // already validated
                .ok_or(anyhow!(err_msg))?
                .download_url(&host, &deposition_id)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_list_depositions() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        let ids = list_depositions(
            &host,
            &token,
            &config.id.to_string(),
            DepositionStatus::Draft,
        )?;
        dbg!(&ids);
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_new_deposition() -> Result<()> {
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        Deposition::new(&config)?;
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_create_deposition() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        create_deposition(&host, &token, &config)?;
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_delete_draft_deposition() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        let ids = list_depositions(
            &host,
            &token,
            &config.id.to_string(),
            DepositionStatus::Draft,
        )?;
        if ids.len() > 0 {
            let id = ids[0];
            delete_deposition(&host, &token, &id)?;
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_update_deposition() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        let ids = list_depositions(
            &host,
            &token,
            &config.id.to_string(),
            DepositionStatus::Draft,
        )?;
        if ids.len() > 0 {
            let id = ids[0];
            update_deposition(&host, &token, &id, &config)?;
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_update_deposition_new_version() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let mut config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        config.version = "1.0.1".to_string();
        let id = 1015788;
        update_deposition(&host, &token, &id, &config)?;
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_config_to_files() -> Result<()> {
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        config_to_files(&config)?;
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_update_deposition_files() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        let mut config_files = config_to_files(&config)?;
        config_files.pop();
        // let config_files = vec![];
        let ids = list_depositions(
            &host,
            &token,
            &config.id.to_string(),
            DepositionStatus::Draft,
        )?;
        if ids.len() > 0 {
            // let id = ids[0];
            let id = 1015788;
            let deposition_files = get_files_list(&host, &token, &id)?;
            // let deposition_files = vec![];
            update_deposition_files(&host, &token, &id, deposition_files, config_files)?;
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_get_files_list() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        let ids = list_depositions(
            &host,
            &token,
            &config.id.to_string(),
            DepositionStatus::Published,
        )?;
        if ids.len() > 0 {
            let id = ids[0];
            let list = get_files_list(&host, &token, &id)?;
            dbg!(&list);
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_publish_deposition() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        let ids = list_depositions(
            &host,
            &token,
            &config.id.to_string(),
            DepositionStatus::Draft,
        )?;
        if ids.len() > 0 {
            // let id = ids[0];
            let id = 1015788;
            publish_deposition(&host, &token, &id)?;
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_new_version_deposition() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        let ids = list_depositions(
            &host,
            &token,
            &config.id.to_string(),
            DepositionStatus::Published,
        )?;
        if ids.len() > 0 {
            let id = ids[0];
            new_version_deposition(&host, &token, &id)?;
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_upload_zenodo() -> Result<()> {
        let host = env::zenodo_host();
        let token = env::zenodo_token()?;
        let mut config = gh_trs::config::io::read_config("./tests/test_config_CWL.yml")?;
        config.version = "1.0.2".to_string();
        upload_zenodo(&host, &token, &mut config)?;
        Ok(())
    }
}
