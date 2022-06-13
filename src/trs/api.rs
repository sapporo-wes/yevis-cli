use crate::trs;

use anyhow::{anyhow, ensure, Result};
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
    pub fn new_from_url(url: &Url) -> Result<Self> {
        let url = Url::parse(&format!("{}/", url.as_str().trim().trim_matches('/')))?;
        Ok(TrsEndpoint { url })
    }

    pub fn new_gh_pages(owner: impl AsRef<str>, name: impl AsRef<str>) -> Result<Self> {
        let url = Url::parse(&format!(
            "https://{}.github.io/{}/",
            owner.as_ref(),
            name.as_ref()
        ))?;
        Ok(TrsEndpoint { url })
    }

    pub fn is_valid(&self) -> Result<()> {
        let service_info = get_service_info(self)?;
        ensure!(
            service_info.r#type.artifact == "yevis" && service_info.r#type.version == "2.0.1",
            "Yevis only supports yevis 2.0.1 as a TRS endpoint"
        );
        Ok(())
    }

    pub fn all_versions(&self, wf_id: impl AsRef<str>) -> Result<Vec<String>> {
        let tool = get_tool(self, wf_id.as_ref())?;
        let versions: Vec<String> = tool
            .versions
            .into_iter()
            .map(|v| {
                v.url
                    .path_segments()
                    .ok_or_else(|| anyhow!("Invalid url: {}", v.url))
                    .and_then(|segments| {
                        segments
                            .last()
                            .ok_or_else(|| anyhow!("Invalid url: {}", v.url))
                    })
                    .map(|s| s.to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(versions)
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

/// /tools/<wf_id> -> trs::types::Tool
pub fn get_tool(trs_endpoint: &TrsEndpoint, wf_id: impl AsRef<str>) -> Result<trs::types::Tool> {
    let url = Url::parse(&format!(
        "{}/tools/{}",
        trs_endpoint.url.as_str().trim().trim_matches('/'),
        wf_id.as_ref()
    ))?;
    let body = get_request(&url)?;
    let tool: trs::types::Tool = serde_json::from_str(&body)?;
    Ok(tool)
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_get_request() -> Result<()> {
        let url = Url::parse("https://suecharo.github.io/gh-pages-rest-api-hosting/foo")?;
        get_request(&url)?;
        Ok(())
    }

    #[test]
    fn test_get_request_not_found() -> Result<()> {
        let url = Url::parse("https://ddbj.github.io/yevis-cli/invalid_path")?;
        let res = get_request(&url);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("404"));
        Ok(())
    }
}
