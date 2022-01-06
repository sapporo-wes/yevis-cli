use anyhow::Result;
use std::path::Path;

pub fn validate(_config_file: impl AsRef<Path>) -> Result<()> {
    println!("validate");
    Ok(())
}
