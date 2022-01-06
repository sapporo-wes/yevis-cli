use anyhow::Result;
use std::path::Path;

pub fn pull_request(
    _config_file: impl AsRef<Path>,
    _repository: impl AsRef<str>,
    _wes_location: &Option<impl AsRef<str>>,
    _docker_host: impl AsRef<str>,
) -> Result<()> {
    println!("pull-request");
    Ok(())
}
