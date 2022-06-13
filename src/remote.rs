use anyhow::{ensure, Result};
use url::Url;

pub fn fetch_raw_content(remote_loc: &Url) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(remote_loc.as_str())
        .header(reqwest::header::ACCEPT, "plain/text")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to fetch raw content from {} with status code {}",
        remote_loc.as_str(),
        response.status()
    );

    Ok(response.text()?)
}

pub fn fetch_json_content(remote_loc: &Url) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(remote_loc.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    ensure!(
        response.status().is_success(),
        "Failed to fetch json content from {} with status code {}",
        remote_loc.as_str(),
        response.status()
    );

    Ok(response.text()?)
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_fetch_raw_content() -> Result<()> {
        let remote_loc =
            Url::parse("https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md")?;
        let content = fetch_raw_content(&remote_loc)?;
        assert!(content.contains("yevis-cli"));
        Ok(())
    }
}
