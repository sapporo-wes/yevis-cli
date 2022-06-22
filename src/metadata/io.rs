use crate::metadata;
use crate::remote;

use anyhow::{bail, Result};
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

pub fn write_local(
    meta: &metadata::types::Metadata,
    path: impl AsRef<Path>,
    ext: &FileExt,
) -> Result<()> {
    let content = match ext {
        FileExt::Yaml => serde_yaml::to_string(&meta)?,
        FileExt::Json => serde_json::to_string_pretty(&meta)?,
    };
    let mut buffer = BufWriter::new(fs::File::create(path)?);
    buffer.write_all(content.as_bytes())?;

    Ok(())
}

pub fn read(
    location: impl AsRef<str>,
    gh_token: impl AsRef<str>,
) -> Result<metadata::types::Metadata> {
    match Url::parse(location.as_ref()) {
        Ok(url) => {
            // as remote url
            let remote = remote::Remote::new(&url, &gh_token, None, None)?;
            let url = remote.to_url()?;
            let content = remote::fetch_json_content(&url)?;
            // Even json can be read with yaml reader
            Ok(serde_yaml::from_str(&content)?)
        }
        Err(_) => {
            // as local file path
            let reader = BufReader::new(fs::File::open(location.as_ref())?);
            Ok(serde_yaml::from_reader(reader)?)
        }
    }
}
