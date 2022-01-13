use crate::args::FileFormat;
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

pub fn file_format(path: impl AsRef<Path>) -> Result<FileFormat> {
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
        "yml" => Ok(FileFormat::Yaml),
        "yaml" => Ok(FileFormat::Yaml),
        "json" => Ok(FileFormat::Json),
        _ => Err(anyhow!(
            "Invalid file format: {}. Only `.yml`, `.yaml` and `.json` are supported",
            ext
        )),
    }
}

pub fn file_name(path: impl AsRef<Path>) -> Result<String> {
    Ok(path
        .as_ref()
        .file_name()
        .ok_or(anyhow!(
            "Could not get file name from path: {}",
            path.as_ref().display()
        ))?
        .to_str()
        .ok_or(anyhow!(
            "Could not convert file name to string: {}",
            path.as_ref().display()
        ))?
        .to_string())
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_file_stem() {
        assert_eq!(file_stem("/path/to/file.yml").unwrap(), "file");
        assert_eq!(file_stem("path/to/file.yaml").unwrap(), "file");
        assert_eq!(file_stem("file.json").unwrap(), "file");
        assert_eq!(file_stem("/path/to/file").unwrap(), "file");
    }

    #[test]
    fn test_dir_path() {
        assert_eq!(
            dir_path("/path/to/file.yml").unwrap(),
            PathBuf::from("/path/to")
        );
        assert_eq!(
            dir_path("path/to/file.yaml").unwrap(),
            PathBuf::from("path/to")
        );
        assert_eq!(dir_path("file.json").unwrap(), PathBuf::from(""));
        assert_eq!(
            dir_path("/path/to/file").unwrap(),
            PathBuf::from("/path/to")
        );
    }

    #[test]
    fn test_file_format() {
        assert_eq!(file_format("/path/to/file.yml").unwrap(), FileFormat::Yaml);
        assert_eq!(file_format("path/to/file.yaml").unwrap(), FileFormat::Yaml);
        assert_eq!(file_format("file.json").unwrap(), FileFormat::Json);
        assert!(file_format("/path/to/file").is_err(),);
    }

    #[test]
    fn test_file_name() {
        assert_eq!(file_name("/path/to/file.yml").unwrap(), "file.yml");
        assert_eq!(file_name("path/to/file.yaml").unwrap(), "file.yaml");
        assert_eq!(file_name("file.json").unwrap(), "file.json");
        assert_eq!(file_name("/path/to/file").unwrap(), "file");
    }
}
