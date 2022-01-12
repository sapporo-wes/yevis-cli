use crate::args;
use crate::github_api;
use crate::path_utils;
use crate::type_config;
use anyhow::Result;
use serde_json;
use serde_yaml;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn validate(
    config_file: impl AsRef<Path>,
    arg_github_token: &Option<impl AsRef<str>>,
) -> Result<()> {
    let github_token = github_api::read_github_token(&arg_github_token)?;

    let file_format = path_utils::file_format(&config_file)?;
    let reader = BufReader::new(File::open(&config_file)?);
    let config: type_config::Config = match file_format {
        args::FileFormat::Yaml => serde_yaml::from_reader(reader)?,
        args::FileFormat::Json => serde_json::from_reader(reader)?,
    };
    dbg!(&config);

    Ok(())
}
