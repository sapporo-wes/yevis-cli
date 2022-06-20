use crate::metadata;
use crate::remote;

use anyhow::Result;
use log::{debug, info};
use std::path::Path;
use url::Url;

pub fn make_template(
    wf_loc: &Url,
    gh_token: impl AsRef<str>,
    output: impl AsRef<Path>,
    use_commit_url: &bool,
) -> Result<()> {
    info!("Making a template from {}", wf_loc);
    let url_type = match use_commit_url {
        true => remote::UrlType::Commit,
        false => remote::UrlType::Branch,
    };
    let metadata = metadata::types::Metadata::new(wf_loc, gh_token, &url_type)?;
    debug!(
        "template metadata file:\n{}",
        serde_yaml::to_string(&metadata)?
    );
    let file_ext = metadata::io::parse_file_ext(&output)?;
    metadata::io::write_local(&metadata, &output, &file_ext)?;
    Ok(())
}
