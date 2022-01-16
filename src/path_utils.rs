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
    use super::*;

    #[test]
    fn test_file_stem() -> Result<()> {
        assert_eq!(file_stem("/path/to/file.yml")?, "file");
        assert_eq!(file_stem("path/to/file.yaml")?, "file");
        assert_eq!(file_stem("file.json")?, "file");
        assert_eq!(file_stem("/path/to/file")?, "file");
        Ok(())
    }

    #[test]
    fn test_dir_path() -> Result<()> {
        assert_eq!(dir_path("/path/to/file.yml")?, PathBuf::from("/path/to"));
        assert_eq!(dir_path("path/to/file.yaml")?, PathBuf::from("path/to"));
        assert_eq!(dir_path("file.json")?, PathBuf::from(""));
        assert_eq!(dir_path("/path/to/file")?, PathBuf::from("/path/to"));
        Ok(())
    }

    #[test]
    fn test_file_format() -> Result<()> {
        assert_eq!(file_format("/path/to/file.yml")?, FileFormat::Yaml);
        assert_eq!(file_format("path/to/file.yaml")?, FileFormat::Yaml);
        assert_eq!(file_format("file.json")?, FileFormat::Json);
        assert!(file_format("/path/to/file").is_err(),);
        Ok(())
    }

    #[test]
    fn test_file_name() -> Result<()> {
        assert_eq!(file_name("/path/to/file.yml")?, "file.yml");
        assert_eq!(file_name("path/to/file.yaml")?, "file.yaml");
        assert_eq!(file_name("file.json")?, "file.json");
        assert_eq!(file_name("/path/to/file")?, "file");
        Ok(())
    }
}
