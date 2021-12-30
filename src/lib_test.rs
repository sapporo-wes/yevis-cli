use anyhow::Result;
use std::path::Path;

pub fn test(
    config_file: impl AsRef<Path>,
    wes_location: &Option<impl AsRef<str>>,
    docker_host: impl AsRef<str>,
) -> Result<()> {
    println!("test");
    Ok(())
}
