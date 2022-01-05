use anyhow::{ensure, Result};
use reqwest;

fn fetch_raw_content(remote_location: impl AsRef<str>) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(remote_location.as_ref())
        .header(reqwest::header::USER_AGENT, "yevis")
        .send()?;
    ensure!(response.status().is_success(), "Failed to get raw content");
    Ok(response.text()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_wf_content() {
        let wf_content =
            fetch_raw_content("https://raw.githubusercontent.com/ddbj/yevis-cli/main/README.md")
                .unwrap();
        assert!(wf_content.contains("yevis-cli"));
    }
}
