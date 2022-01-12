use crate::args;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub fn file_stem(path: impl AsRef<Path>) -> Result<String> {
    Ok(path
        .as_ref()
        .file_stem()
        .ok_or(anyhow!(
            "Could not get file stem from path: {}",
            path.as_ref().display()
        ))?
        .to_str()
        .ok_or(anyhow!(
            "Could not convert file stem to string: {}",
            path.as_ref().display()
        ))?
        .to_string())
}

pub fn dir_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    Ok(path
        .as_ref()
        .parent()
        .ok_or(anyhow!(
            "Failed to get parent path from path: {}",
            path.as_ref().display()
        ))?
        .to_path_buf())
}

pub fn file_format(path: impl AsRef<Path>) -> Result<args::FileFormat> {
    let ext = path
        .as_ref()
        .extension()
        .ok_or(anyhow!(
            "Failed to get extension from path: {}",
            path.as_ref().display()
        ))?
        .to_str()
        .ok_or(anyhow!(
            "Failed to convert extension to string: {}",
            path.as_ref().display()
        ))?;
    match ext {
        "yml" => Ok(args::FileFormat::Yaml),
        "yaml" => Ok(args::FileFormat::Yaml),
        "json" => Ok(args::FileFormat::Json),
        _ => Err(anyhow!("Invalid file format: {}", path.as_ref().display())),
    }
}
