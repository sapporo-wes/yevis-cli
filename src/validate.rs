use anyhow::Result;
use std::path::Path;

pub fn validate(
    _config_file: impl AsRef<Path>,
    _github_token: &Option<impl AsRef<str>>,
) -> Result<()> {
    println!("validate");
    Ok(())
}
