use crate::metadata;
use crate::zenodo;

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time;
use url::Url;

fn get_request(zenodo_token: impl AsRef<str>, url: &Url, query: &[(&str, &str)]) -> Result<Value> {
    // timeout is set to 10 minutes
    let client = reqwest::blocking::Client::builder()
        .timeout(time::Duration::from_secs(600))
        .build()?;
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
    // timeout is set to 60 minutes
    let client = reqwest::blocking::Client::builder()
        .timeout(time::Duration::from_secs(3600))
        .build()?;
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
    // timeout is set to 60 minutes
    let client = reqwest::blocking::Client::builder()
        .timeout(time::Duration::from_secs(3600))
        .build()?;
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
    // timeout is set to 10 minutes
    let client = reqwest::blocking::Client::builder()
        .timeout(time::Duration::from_secs(600))
        .build()?;
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

/// https://developers.zenodo.org/?shell#list
/// same id but different version: -> return only oldest version
pub fn list_depositions(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    wf_id: impl AsRef<str>,
    status: zenodo::types::DepositionStatus,
) -> Result<Vec<u64>> {
    let mut url = Url::parse(&format!(
        "https://{}/api/deposit/depositions",
        host.as_ref()
    ))?;
    url.query_pairs_mut()
        .append_pair("q", wf_id.as_ref())
        .append_pair("status", &status.to_string());
    let res = get_request(&token, &url, &[])?;
    let err_msg = "Failed to parse the response when listing depositions";
    let ids = res
        .as_array()
        .ok_or_else(|| anyhow!(err_msg))?
        .iter()
        .map(|d| {
            d.as_object()
                .ok_or_else(|| anyhow!(err_msg))
                .and_then(|d| d.get("id").ok_or_else(|| anyhow!(err_msg)))
                .and_then(|id| id.as_u64().ok_or_else(|| anyhow!(err_msg)))
        })
        .collect::<Result<Vec<u64>>>()?;
    Ok(ids)
}

/// https://developers.zenodo.org/?shell#create
/// https://developers.zenodo.org/?shell#representation
pub fn create_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    meta: &metadata::types::Metadata,
    repo: impl AsRef<str>,
    zenodo_community: &Option<impl AsRef<str>>,
) -> Result<u64> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions",
        host.as_ref()
    ))?;
    let deposition = zenodo::types::Deposition::new(meta, repo, zenodo_community);
    let body = json!({
        "metadata": deposition,
    });
    let res = post_request(&token, &url, &body)?;
    let err_msg = "Failed to parse the response when creating a deposition";
    let id = res
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("id")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_u64()
        .ok_or_else(|| anyhow!(err_msg))?;
    Ok(id)
}

/// https://developers.zenodo.org/?shell#update
pub fn update_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
    meta: &metadata::types::Metadata,
    repo: impl AsRef<str>,
    zenodo_community: &Option<impl AsRef<str>>,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}",
        host.as_ref(),
        deposition_id
    ))?;
    let deposition = zenodo::types::Deposition::new(meta, repo, zenodo_community);
    let body = json!({
        "metadata": deposition,
    });
    put_request(&token, &url, &body)?;
    Ok(())
}

/// https://developers.zenodo.org/?shell#delete
/// can delete only draft
pub fn delete_deposition(
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
pub fn publish_deposition(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<metadata::types::Zenodo> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/actions/publish",
        host.as_ref(),
        &deposition_id
    ))?;
    let res = post_request(&token, &url, &json!({}))?;
    let err_msg = "Failed to parse the response when publishing a deposition";
    let res_obj = res.as_object().ok_or_else(|| anyhow!(err_msg))?;
    let id = res_obj
        .get("id")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_u64()
        .ok_or_else(|| anyhow!(err_msg))?;
    let doi = res_obj
        .get("doi")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let concept_doi = res_obj
        .get("conceptdoi")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let url = Url::parse(&format!("https://{}/record/{}", host.as_ref(), &id))?;
    Ok(metadata::types::Zenodo {
        url,
        id,
        doi: doi.to_string(),
        concept_doi: concept_doi.to_string(),
    })
}

/// https://developers.zenodo.org/?shell#edit
pub fn new_version_deposition(
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
        .ok_or_else(|| anyhow!(err_msg))?
        .get("links")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("latest_draft")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let latest_draft_id: u64 = latest_draft
        .split('/')
        .last()
        .ok_or_else(|| anyhow!(err_msg))?
        .parse()?;
    Ok(latest_draft_id)
}

pub fn get_bucket_url(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}",
        host.as_ref(),
        deposition_id
    ))?;
    let res = get_request(&token, &url, &[])?;
    let err_msg = "Failed to parse the response when getting bucket url";
    let res_obj = res.as_object().ok_or_else(|| anyhow!(err_msg))?;
    let bucket_url = res_obj
        .get("links")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("bucket")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    Ok(bucket_url.to_string())
}

/// https://developers.zenodo.org/?shell#delete28
pub fn delete_deposition_file(
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

/// https://developers.zenodo.org/?shell#retrieve
pub fn retrieve_record(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    record_id: &u64,
) -> Result<(metadata::types::Zenodo, String)> {
    let url = Url::parse(&format!(
        "https://{}/api/records/{}",
        host.as_ref(),
        record_id
    ))?;
    let res = get_request(&token, &url, &[])?;
    let err_msg = "Failed to parse the response when retrieving a deposition";
    let res_obj = res.as_object().ok_or_else(|| anyhow!(err_msg))?;
    let id = res_obj
        .get("id")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_u64()
        .ok_or_else(|| anyhow!(err_msg))?;
    let doi = res_obj
        .get("doi")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let concept_doi = res_obj
        .get("conceptdoi")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;
    let url = Url::parse(&format!("https://{}/record/{}", host.as_ref(), &id))?;
    let version = res_obj
        .get("metadata")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("version")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_str()
        .ok_or_else(|| anyhow!(err_msg))?;

    Ok((
        metadata::types::Zenodo {
            url,
            id,
            doi: doi.to_string(),
            concept_doi: concept_doi.to_string(),
        },
        version.to_string(),
    ))
}

/// https://github.com/zenodo/zenodo/issues/2246
pub fn get_files_download_urls(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    record_id: &u64,
) -> Result<HashMap<String, Url>> {
    let url = Url::parse(&format!(
        "https://{}/api/records/{}",
        host.as_ref(),
        record_id
    ))?;
    let res = get_request(&token, &url, &[])?;
    let err_msg = "Failed to parse the response when retrieving a deposition";
    let files_arr = res
        .as_object()
        .ok_or_else(|| anyhow!(err_msg))?
        .get("files")
        .ok_or_else(|| anyhow!(err_msg))?
        .as_array()
        .ok_or_else(|| anyhow!(err_msg))?;
    let mut files_map: HashMap<String, Url> = HashMap::new();
    for file_obj in files_arr {
        let filename = file_obj
            .as_object()
            .ok_or_else(|| anyhow!(err_msg))?
            .get("key")
            .ok_or_else(|| anyhow!(err_msg))?
            .as_str()
            .ok_or_else(|| anyhow!(err_msg))?;
        let download_url = file_obj
            .as_object()
            .ok_or_else(|| anyhow!(err_msg))?
            .get("links")
            .ok_or_else(|| anyhow!(err_msg))?
            .as_object()
            .ok_or_else(|| anyhow!(err_msg))?
            .get("self")
            .ok_or_else(|| anyhow!(err_msg))?
            .as_str()
            .ok_or_else(|| anyhow!(err_msg))?;
        files_map.insert(filename.to_string(), Url::parse(download_url)?);
    }

    Ok(files_map)
}

/// https://developers.zenodo.org/?shell#list23
pub fn get_files_list(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
) -> Result<Vec<zenodo::types::DepositionFile>> {
    let url = Url::parse(&format!(
        "https://{}/api/deposit/depositions/{}/files",
        host.as_ref(),
        &deposition_id
    ))?;
    let res = get_request(&token, &url, &[])?;
    let files: Vec<zenodo::types::DepositionFile> = serde_json::from_value(res)?;
    Ok(files)
}

/// https://developers.zenodo.org/?shell#create24
pub fn create_deposition_file(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
    file_name: impl AsRef<str>,
    file_path: impl AsRef<Path>,
) -> Result<()> {
    let bucket_url = get_bucket_url(&host, &token, deposition_id)?;
    let url = Url::parse(&format!("{}/{}", bucket_url, file_name.as_ref()))?;
    // timeout is set to 60 * 60 seconds
    let client = reqwest::blocking::Client::builder()
        .timeout(time::Duration::from_secs(3600))
        .build()?;
    let response = client
        .put(url.as_ref())
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token.as_ref()),
        )
        .body(fs::File::open(file_path)?)
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
    Ok(())
}
