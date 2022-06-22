use crate::trs;

use anyhow::{ensure, Result};
use reqwest;
use url::Url;

pub fn get_request(url: &Url) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url.as_str())
        .header(reqwest::header::ACCEPT, "application/json")
        .send()?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "Failed to get request to {} with status: {}",
        url,
        status
    );
    let body = response.text()?;
    Ok(body)
}

#[derive(Debug, PartialEq, Clone)]
pub struct TrsEndpoint {
    pub url: Url,
}

impl TrsEndpoint {
    pub fn new_gh_pages(owner: impl AsRef<str>, name: impl AsRef<str>) -> Result<Self> {
        let url = Url::parse(&format!(
            "https://{}.github.io/{}/",
            owner.as_ref(),
            name.as_ref()
        ))?;
        Ok(TrsEndpoint { url })
    }
}

/// /service-info -> trs::types::ServiceInfo
pub fn get_service_info(trs_endpoint: &TrsEndpoint) -> Result<trs::types::ServiceInfo> {
    let url = Url::parse(&format!(
        "{}/service-info",
        trs_endpoint.url.as_str().trim().trim_matches('/')
    ))?;
    let body = get_request(&url)?;
    let service_info: trs::types::ServiceInfo = serde_json::from_str(&body)?;
    Ok(service_info)
}

/// /toolClasses -> trs::types::ToolClass[]
pub fn get_tool_classes(trs_endpoint: &TrsEndpoint) -> Result<Vec<trs::types::ToolClass>> {
    let url = Url::parse(&format!(
        "{}/toolClasses",
        trs_endpoint.url.as_str().trim().trim_matches('/')
    ))?;
    let body = get_request(&url)?;
    let tool_classes: Vec<trs::types::ToolClass> = serde_json::from_str(&body)?;
    Ok(tool_classes)
}

/// /tools -> trs::types::Tool[]
pub fn get_tools(trs_endpoint: &TrsEndpoint) -> Result<Vec<trs::types::Tool>> {
    let url = Url::parse(&format!(
        "{}/tools",
        trs_endpoint.url.as_str().trim().trim_matches('/')
    ))?;
    let body = get_request(&url)?;
    let tools: Vec<trs::types::Tool> = serde_json::from_str(&body)?;
    Ok(tools)
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_get_request() -> Result<()> {
        let url = Url::parse("https://ddbj.github.io/workflow-registry/service-info")?;
        get_request(&url)?;
        Ok(())
    }

    #[test]
    fn test_get_request_not_found() -> Result<()> {
        let url = Url::parse("https://ddbj.github.io/workflow-registry/invalid_path")?;
        let res = get_request(&url);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("404"));
        Ok(())
    }
}
