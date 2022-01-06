use anyhow::Result;
use std::path::Path;

pub fn test(
    _config_file: impl AsRef<Path>,
    _wes_location: &Option<impl AsRef<str>>,
    _docker_host: impl AsRef<str>,
) -> Result<()> {
    println!("test");
    Ok(())
}
