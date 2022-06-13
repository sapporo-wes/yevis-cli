use crate::metadata;
use crate::remote;
use crate::trs;

use anyhow::{bail, Result};
use log::debug;
use serde_json;
use serde_yaml;
use std::fs;
use std::io::BufReader;
use std::io::{BufWriter, Write};
use std::path::Path;
use url::Url;

pub enum FileExt {
    Yaml,
    Json,
}

pub fn parse_file_ext(path: impl AsRef<Path>) -> Result<FileExt> {
    match path.as_ref().extension() {
        Some(ext) => match ext.to_str() {
            Some("yml") => Ok(FileExt::Yaml),
            Some("yaml") => Ok(FileExt::Yaml),
            Some("json") => Ok(FileExt::Json),
            Some(ext) => bail!("Unsupported output file extension: {}", ext),
            None => bail!("Unsupported output file extension"),
        },
        None => Ok(FileExt::Yaml),
    }
}

pub fn write_config(
    config: &metadata::types::Config,
    path: impl AsRef<Path>,
    ext: &FileExt,
) -> Result<()> {
    let content = match ext {
        FileExt::Yaml => serde_yaml::to_string(&config)?,
        FileExt::Json => serde_json::to_string_pretty(&config)?,
    };
    let mut buffer = BufWriter::new(fs::File::create(path)?);
    buffer.write_all(content.as_bytes())?;

    Ok(())
}

pub fn read_config(location: impl AsRef<str>) -> Result<metadata::types::Config> {
    match Url::parse(location.as_ref()) {
        Ok(url) => {
            // as remote url
            // Even json can be read with yaml reader
            let content = remote::fetch_json_content(&url)?;
            Ok(serde_yaml::from_str(&content)?)
        }
        Err(_) => {
            // as local file path
            let reader = BufReader::new(fs::File::open(location.as_ref())?);
            Ok(serde_yaml::from_reader(reader)?)
        }
    }
}

pub fn find_config_loc_recursively_from_trs(trs_loc: impl AsRef<str>) -> Result<Vec<String>> {
    let trs_endpoint = trs::api::TrsEndpoint::new_from_url(&Url::parse(trs_loc.as_ref())?)?;
    trs_endpoint.is_valid()?;
    let config_locs: Vec<String> = trs::api::get_tools(&trs_endpoint)?
        .into_iter()
        .flat_map(|tool| tool.versions)
        .map(|version| version.url)
        .map(|url| format!("{}/gh-trs-config.json", url.as_str()))
        .collect();
    debug!("Found config locations: {:?}", config_locs);
    Ok(config_locs)
}
