use anyhow::Result;
use std::path::Path;

pub fn pull_request(
    config_file: impl AsRef<Path>,
    repository: impl AsRef<str>,
    wes_location: &Option<impl AsRef<str>>,
    docker_host: impl AsRef<str>,
) -> Result<()> {
    println!("pull-request");
    Ok(())
}
