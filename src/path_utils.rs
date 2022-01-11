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
