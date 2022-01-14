use crate::type_config::Config;
use anyhow::Result;

pub fn pull_request(
    _config: &Config,
    _github_token: &Option<impl AsRef<str>>,
    _repository: impl AsRef<str>,
    _wes_location: &Option<impl AsRef<str>>,
    _docker_host: impl AsRef<str>,
) -> Result<()> {
    println!("pull-request");
    Ok(())
}
