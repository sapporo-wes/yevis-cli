use crate::{
    github_api::read_github_token,
    remote::fetch_raw_content,
    type_config::{Author, Config},
    validate::read_config,
};
use anyhow::{anyhow, bail, ensure, Result};
use crypto::digest::Digest;
use crypto::md5::Md5;
use dotenv::dotenv;
use log::{debug, info};
use reqwest;
use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use url::Url;

pub fn zenodo_upload(
    config_file: impl AsRef<Path>,
    arg_github_token: &Option<impl AsRef<str>>,
    repository: impl AsRef<str>,
) -> Result<()> {
    let github_token = read_github_token(&arg_github_token)?;
    ensure!(
        !github_token.is_empty(),
        "GitHub token is empty. Please set it with --github-token option or set GITHUB_TOKEN environment variable."
    );
    let zenodo_host = zenodo_host();
    let zenodo_token = read_zenodo_token()?;

    info!("Reading config file: {}", config_file.as_ref().display());
    let config = read_config(&config_file)?;
    debug!("config:\n{:#?}", &config);

    let exist_deposition_id = find_exist_deposition(&zenodo_host, &zenodo_token, &config)?;
    let deposition_id = match exist_deposition_id {
        Some(original_deposition_id) => {
            info!(
                "Found published deposition: {}. So this deposition will be updated.",
                original_deposition_id
            );
            let latest_deposition_id =
                new_version_deposition(&zenodo_host, &zenodo_token, &original_deposition_id)?;
            info!("Latest version deposition id: {}", latest_deposition_id);
            update_deposition(&zenodo_host, &zenodo_token, &latest_deposition_id, &config)?;
            latest_deposition_id
        }
        None => {
            info!("Creating new deposition.");
            create_deposition(&zenodo_host, &zenodo_token, &config)?
        }
    };
    debug!("deposition_id: {}", deposition_id);

    let deposition_files = get_files_list(&zenodo_host, &github_token, &deposition_id)?;
    debug!("deposition_files: {:#?}", &deposition_files);
    let upload_files = config_to_upload_files(&config)?;
    debug!("upload_files: {:#?}", &upload_files);
    upload_deposition_files(
        &zenodo_host,
        &zenodo_token,
        &deposition_id,
        &deposition_files,
        &upload_files,
    )?;

    info!("Publishing deposition: {}", deposition_id);
    let doi = publish_deposition(&zenodo_host, &zenodo_token, &deposition_id)?;
    info!("DOI: {}", doi);

    Ok(())
}

fn read_zenodo_token() -> Result<String> {
    dotenv().ok();
    match env::var("ZENODO_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) => Err(anyhow!(
            "Zenodo token is empty. Please set ZENODO_TOKEN environment variable."
        )),
    }
}

fn zenodo_host() -> String {
    dotenv().ok();
    match env::var("YEVIS_DEV") {
        Ok(_) => "sandbox.zenodo.org".to_string(),
        Err(_) => "zenodo.org".to_string(),
    }
}

enum DepositionStatus {
    Draft,
    Published,
}

/// https://developers.zenodo.org/?shell#list
fn list_depositions(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    wf_id: impl AsRef<str>,
    status: DepositionStatus,
) -> Result<u64> {
    let mut url = Url::parse(&format!(
        "https://{}/api/deposit/depositions",
        zenodo_host.as_ref()
    ))?;
    url.query_pairs_mut()
        .append_pair("q", wf_id.as_ref())
        .append_pair(
            "status",
            match status {
                DepositionStatus::Draft => "draft",
                DepositionStatus::Published => "published",
            },
        );
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to list Zenodo depositions. Status: {}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_array() {
        true => {
            let depositions = body.as_array().ok_or(anyhow!(
                "Failed to parse response when listing Zenodo depositions."
            ))?;
            if depositions.len() == 0 {
                bail!("No deposition found.");
            }
            if depositions.len() > 1 {
                bail!("More than one deposition found.");
            }
            let deposition = depositions.get(0).ok_or(anyhow!(
                "Failed to parse response when listing Zenodo depositions."
            ))?;
            let id = deposition["id"].as_u64().ok_or(anyhow!(
                "Failed to parse response when listing Zenodo depositions."
            ))?;
            Ok(id)
        }
        false => bail!("Failed to parse response when listing Zenodo depositions."),
    }
}

/// https://developers.zenodo.org/?shell#create
/// https://developers.zenodo.org/?shell#representation
fn create_deposition(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    config: &Config,
) -> Result<u64> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions",
        zenodo_host.as_ref()
    ))?;
    let metadata = ZenodoMetadata::new(&config)?;
    let request_body = serde_json::to_string(&metadata)?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .body(request_body)
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to create Zenodo deposition. Status: {}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => Ok(body["id"].as_u64().ok_or(anyhow!(
            "Failed to parse response when creating Zenodo deposition."
        ))?),
        false => bail!("Failed to parse response when creating Zenodo deposition."),
    }
}

/// https://developers.zenodo.org/?shell#update
fn update_deposition(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
    config: &Config,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}",
        zenodo_host.as_ref(),
        deposition_id
    ))?;
    let metadata = ZenodoMetadata::new(&config)?;
    let request_body = serde_json::to_string(&metadata)?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .put(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .body(request_body)
        .send()?;
    ensure!(
        response.status() != reqwest::StatusCode::UNAUTHORIZED,
        "Failed to authenticate with Zenodo. Please check your Zenodo token."
    );
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to update Zenodo deposition. Status: {}",
            response.status()
        )
    );

    Ok(())
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct ZenodoMetadata {
    metadata: Deposition,
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Creator {
    pub name: String,
    pub affiliation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Community {
    pub identifier: String,
}

impl ZenodoMetadata {
    fn new(config: &Config) -> Result<Self> {
        let metadata = Deposition {
            upload_type: "dataset".to_string(),
            title: format!("DDBJ workflow: {}", config.id),
            creators: config
                .authors
                .iter()
                .map(|author| Creator::new(author))
                .collect(),
            description: Self::description_html(&config),
            access_right: "open".to_string(),
            license: "cc-zero".to_string(),
            keywords: vec!["ddbj-workflow".to_string()],
            communities: vec![Community {
                identifier: "ddbj-workflow".to_string(),
            }],
            version: config.version.clone(),
        };
        Ok(ZenodoMetadata { metadata })
    }

    fn description_html(config: &Config) -> String {
        // TODO Update
        // For string fields that allow HTML (e.g. description, notes), for security reasons, only the following tags are accepted: a, abbr, acronym, b, blockquote, br, code, caption, div, em, i, li, ol, p, pre, span, strike, strong, sub, table, caption, tbody, thead, th, td, tr, u, ul.
        let mut description = String::new();
        description.push_str(&format!("<h1>DDBJ Workflow: {}</h1>", config.id));
        description.push_str(&format!("<h2>Authors</h2>"));
        description.push_str(&format!("<ul>"));
        for author in &config.authors {
            description.push_str(&format!(
                "<li>{} ({})</li>",
                author.name, author.affiliation
            ));
        }
        description.push_str(&format!("</ul>"));
        description
    }
}

impl Creator {
    fn new(author: &Author) -> Self {
        let orcid = if author.name == "ddbj-workflow" {
            None
        } else {
            Some(author.orcid.clone())
        };
        Self {
            name: author.name.clone(),
            affiliation: author.affiliation.clone(),
            orcid,
        }
    }
}

/// https://developers.zenodo.org/?shell#delete
fn delete_unpublished_depositions(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}",
        zenodo_host.as_ref(),
        &deposition_id
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .delete(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to delete Zenodo deposition. Status: {}",
            response.status()
        )
    );
    Ok(())
}

fn find_exist_deposition(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    config: &Config,
) -> Result<Option<u64>> {
    match list_depositions(
        &zenodo_host,
        &zenodo_token,
        &config.id.to_string(),
        DepositionStatus::Published,
    ) {
        Ok(deposition_id) => return Ok(Some(deposition_id)),
        Err(err) => {
            if !err.to_string().contains("No deposition found.") {
                bail!("{}", err);
            }
        }
    };
    match list_depositions(
        &zenodo_host,
        &zenodo_token,
        &config.id.to_string(),
        DepositionStatus::Draft,
    ) {
        Ok(deposition_id) => {
            info!(
                "Found draft deposition: {}. So this deposition will be deleted.",
                deposition_id
            );
            delete_unpublished_depositions(&zenodo_host, &zenodo_token, &deposition_id)?;
        }
        Err(err) => {
            if !err.to_string().contains("No deposition found.") {
                bail!("{}", err);
            }
        }
    };
    Ok(None)
}

/// https://developers.zenodo.org/?shell#publish
fn publish_deposition(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/actions/publish",
        zenodo_host.as_ref(),
        &deposition_id
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to publish Zenodo deposition. Status: {}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => Ok(body["doi"]
            .as_str()
            .ok_or(anyhow!(
                "Failed to parse response when publishing deposition."
            ))?
            .to_string()),
        false => bail!("Failed to parse response when publishing deposition."),
    }
}

/// https://developers.zenodo.org/?python#edit
fn new_version_deposition(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<u64> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/actions/newversion",
        zenodo_host.as_ref(),
        &deposition_id
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to create new version Zenodo deposition. Status: {}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    match &body.is_object() {
        true => {
            let links = body["links"].as_object().ok_or(anyhow!(
                "Failed to parse response when creating new version Zenodo deposition."
            ))?;
            let latest_draft_link = links["latest_draft"].as_str().ok_or(anyhow!(
                "Failed to parse response when creating new version Zenodo deposition."
            ))?;
            let latest_draft_id = latest_draft_link.split("/").last().ok_or(anyhow!(
                "Failed to parse response when creating new version Zenodo deposition."
            ))?;
            Ok(latest_draft_id.parse::<u64>()?)
        }
        false => bail!("Failed to parse response when creating new version Zenodo deposition."),
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct DepositionFile {
    id: String,
    filename: String,
    filesize: u64,
    checksum: String,
}

/// https://developers.zenodo.org/?shell#list23
fn get_files_list(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<Vec<DepositionFile>> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/files",
        zenodo_host.as_ref(),
        &deposition_id
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to list Zenodo deposition files. Status: {}",
            response.status()
        )
    );
    let body = response.json::<Value>()?;

    let res_files = body.as_array().ok_or(anyhow!(
        "Failed to parse response when listing Zenodo deposition files."
    ))?;
    let mut files: Vec<DepositionFile> = vec![];
    for file in res_files {
        let id = file["id"].as_str().ok_or(anyhow!(
            "Failed to parse response when listing Zenodo deposition files."
        ))?;
        let filename = file["filename"].as_str().ok_or(anyhow!(
            "Failed to parse response when listing Zenodo deposition files."
        ))?;
        let filesize = file["filesize"].as_u64().ok_or(anyhow!(
            "Failed to parse response when listing Zenodo deposition files."
        ))?;
        let checksum = file["checksum"].as_str().ok_or(anyhow!(
            "Failed to parse response when listing Zenodo deposition files."
        ))?;
        files.push(DepositionFile {
            id: id.to_string(),
            filename: filename.to_string(),
            filesize,
            checksum: checksum.to_string(),
        });
    }

    Ok(files)
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct UploadFile {
    url: Url,
    file_path: PathBuf,
    filename: String,
    filesize: u64,
    checksum: String,
}

fn file_url_to_upload_file(file_url: &Url) -> Result<UploadFile> {
    let named_tempfile = NamedTempFile::new()?;
    let mut buffer = BufWriter::new(&named_tempfile);
    let content = fetch_raw_content(&file_url)?;
    buffer.write(content.as_bytes())?;
    buffer.flush()?;
    debug!(
        "File {} downloaded to {}",
        file_url,
        named_tempfile.path().display()
    );
    let file_path = named_tempfile.path().to_path_buf();
    let filename = file_url
        .path_segments()
        .ok_or(anyhow!("Failed to parse file_url: {}", file_url))?
        .last()
        .ok_or(anyhow!("Failed to parse file_url: {}", file_url))?;
    let filesize = named_tempfile.as_file().metadata()?.len();
    let mut md5 = Md5::new();
    md5.input_str(&content);
    let checksum = md5.result_str();

    Ok(UploadFile {
        url: file_url.clone(),
        file_path,
        filename: filename.to_string(),
        filesize,
        checksum,
    })
}

fn config_to_upload_files(config: &Config) -> Result<Vec<UploadFile>> {
    let mut upload_files: Vec<UploadFile> = vec![];
    upload_files.push(file_url_to_upload_file(&config.workflow.readme)?);
    for file in &config.workflow.files {
        upload_files.push(file_url_to_upload_file(&file.url)?);
    }
    for testing in &config.workflow.testing {
        for file in &testing.files {
            upload_files.push(file_url_to_upload_file(&file.url)?);
        }
    }

    Ok(upload_files)
}

/// - in deposition_files, in upload_files
///   - checksum is the same: do nothing
///   - checksum is not the same: delete and upload
/// in deposition_files, not in upload_files: delete
/// not in deposition_files, in upload_files: upload
fn upload_deposition_files(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
    deposition_files: &Vec<DepositionFile>,
    upload_files: &Vec<UploadFile>,
) -> Result<()> {
    let deposition_filenames = deposition_files
        .iter()
        .map(|f| f.filename.clone())
        .collect::<HashSet<String>>();
    let upload_filenames = upload_files
        .iter()
        .map(|f| f.filename.clone())
        .collect::<HashSet<String>>();
    let all_filenames = deposition_filenames.union(&upload_filenames);

    for filename in all_filenames {
        if deposition_filenames.contains(filename) {
            if upload_filenames.contains(filename) {
                let deposition_file = deposition_files
                    .iter()
                    .find(|f| &f.filename == filename)
                    .ok_or(anyhow!("Failed to find deposition file {}", filename))?;
                let upload_file = upload_files
                    .iter()
                    .find(|f| &f.filename == filename)
                    .ok_or(anyhow!("Failed to find upload file {}", filename))?;
                if deposition_file.checksum == upload_file.checksum {
                    debug!("File {} is same checksum, do nothing", filename);
                } else {
                    debug!(
                        "File {} is different checksum in deposition and upload. Deleting and uploading.",
                        filename
                    );
                    delete_deposition_file(
                        &zenodo_host,
                        &zenodo_token,
                        &deposition_id,
                        &deposition_file.id,
                    )?;
                    create_deposition_file(
                        &zenodo_host,
                        &zenodo_token,
                        &deposition_id,
                        filename,
                        &upload_file.file_path,
                    )?;
                }
            } else {
                let deposition_file = deposition_files
                    .iter()
                    .find(|f| &f.filename == filename)
                    .ok_or(anyhow!("Failed to find deposition file {}", filename))?;
                debug!(
                    "File {} is in deposition but not in upload. Deleting.",
                    filename
                );
                delete_deposition_file(
                    &zenodo_host,
                    &zenodo_token,
                    &deposition_id,
                    &deposition_file.id,
                )?;
            }
        } else {
            if upload_filenames.contains(filename) {
                let upload_file = upload_files
                    .iter()
                    .find(|f| &f.filename == filename)
                    .ok_or(anyhow!("Failed to find upload file {}", filename))?;
                debug!(
                    "File {} is not in deposition but in upload. Uploading.",
                    filename
                );
                create_deposition_file(
                    &zenodo_host,
                    &zenodo_token,
                    &deposition_id,
                    filename,
                    &upload_file.file_path,
                )?;
            }
        }
    }
    Ok(())
}

/// https://developers.zenodo.org/?shell#create24
fn create_deposition_file(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
    file_name: &str,
    file_path: impl AsRef<Path>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/files",
        zenodo_host.as_ref(),
        &deposition_id
    ))?;
    let form = multipart::Form::new()
        .text("name", file_name.to_string())
        .file("file", file_path.as_ref())?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .multipart(form)
        .send()?;
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to create deposition file {}: with status {}",
            file_name,
            response.status()
        )
    );

    Ok(())
}

/// https://developers.zenodo.org/?shell#delete28
fn delete_deposition_file(
    zenodo_host: impl AsRef<str>,
    zenodo_token: impl AsRef<str>,
    deposition_id: &u64,
    file_id: impl AsRef<str>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/files/{}",
        zenodo_host.as_ref(),
        &deposition_id,
        file_id.as_ref()
    ))?;
    let client = reqwest::blocking::Client::new();
    let response = client
        .delete(url.as_str())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", zenodo_token.as_ref()),
        )
        .send()?;
    ensure!(
        response.status().is_success(),
        format!(
            "Failed to delete file {} from deposition {}: {}",
            file_id.as_ref(),
            deposition_id,
            response.status()
        )
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_depositions() -> Result<()> {
        let zenodo_host = zenodo_host();
        let zenodo_token = read_zenodo_token()?;
        let config = read_config("./tests/test_config_CWL.yml")?;
        match list_depositions(
            &zenodo_host,
            &zenodo_token,
            &config.id.to_string(),
            DepositionStatus::Draft,
        ) {
            Ok(_) => Ok(()),
            Err(err) => {
                assert!(err.to_string().contains("No deposition found."));
                Ok(())
            }
        }
    }

    #[test]
    fn test_config_to_zenodo_metadata() -> Result<()> {
        ZenodoMetadata::new(&read_config("./tests/test_config_CWL.yml")?)?;
        Ok(())
    }

    #[test]
    fn test_create_deposition() -> Result<()> {
        let zenodo_host = zenodo_host();
        let zenodo_token = read_zenodo_token()?;
        let deposition_id = create_deposition(
            &zenodo_host,
            &zenodo_token,
            &read_config("./tests/test_config_CWL.yml")?,
        )?;
        delete_unpublished_depositions(&zenodo_host, &zenodo_token, &deposition_id)?;
        Ok(())
    }

    #[test]
    fn test_delete_unpublished_depositions() -> Result<()> {
        let zenodo_host = zenodo_host();
        let zenodo_token = read_zenodo_token()?;
        let config = read_config("./tests/test_config_CWL.yml")?;
        match list_depositions(
            &zenodo_host,
            &zenodo_token,
            &config.id.to_string(),
            DepositionStatus::Draft,
        ) {
            Ok(deposition_id) => Ok(delete_unpublished_depositions(
                &zenodo_host,
                &zenodo_token,
                &deposition_id,
            )?),
            Err(err) => {
                assert!(err.to_string().contains("No deposition found."));
                Ok(())
            }
        }
    }
}
