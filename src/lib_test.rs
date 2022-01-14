use crate::{
    type_config::Config,
    wes::{start_wes, DEFAULT_WES_LOCATION},
};
use anyhow::Result;
use log::info;
use url::Url;

pub fn test(
    _config: &Config,
    _github_token: &Option<impl AsRef<str>>,
    wes_location: &Option<Url>,
    docker_host: &Url,
) -> Result<()> {
    let default_wes_loc = Url::parse(DEFAULT_WES_LOCATION)?;
    let wes_location = match &wes_location {
        Some(wes_location) => {
            info!("Use wes_location: {} for testing", wes_location.as_str());
            wes_location
        }
        None => {
            start_wes(&docker_host)?;
            info!("Use wes_location: {} for testing", DEFAULT_WES_LOCATION);
            &default_wes_loc
        }
    };

    Ok(())
}
