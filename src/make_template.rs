use crate::env;
use crate::file_url;
use crate::gh_trs;
use crate::metadata;
use crate::version;

use anyhow::{anyhow, bail, Result};
use log::{debug, info};
use std::path::Path;
use std::str::FromStr;
use url::Url;
use uuid::Uuid;

pub fn make_template(
    wf_loc: &Url,
    gh_token: &Option<impl AsRef<str>>,
    output: impl AsRef<Path>,
    url_type: gh_trs::raw_url::UrlType,
) -> Result<()> {
    info!("Making a template from {}", wf_loc);

    // let config = if update {
    //     // Retrieve metadata file from API because wf_loc is TRS ToolVersion URL
    //     let config_loc = tool_version_url_to_metadata_url(wf_loc)?;
    //     let mut config = metadata::io::read_config(&config_loc)?;
    //     let prev_version = version::Version::from_str(&config.version)?;
    //     config.version = prev_version.increment_patch().to_string();
    //     config
    // } else {
    //     generate_config(wf_loc, gh_token, url_type)?
    // };
    let config = generate_config(wf_loc, gh_token, url_type)?;

    debug!(
        "template metadata file:\n{}",
        serde_yaml::to_string(&config)?
    );

    let file_ext = metadata::io::parse_file_ext(&output)?;
    metadata::io::write_config(&config, &output, &file_ext)?;
    Ok(())
}

/// TRS ToolVersion URL: https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>
/// metadata file URL: https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>/yevis-metadata.json
fn tool_version_url_to_metadata_url(wf_loc: &Url) -> Result<Url> {
    let tool_version_url_re = regex::Regex::new(r"^https?://.+/tools/([^/]+)/versions/([^/]+)$")?;
    let (id, version) = match tool_version_url_re.captures(wf_loc.as_str()) {
        Some(caps) => (caps.get(1).unwrap().as_str(), caps.get(2).unwrap().as_str()),
        None => bail!("Invalid TRS ToolVersion URL: {}", wf_loc),
    };
    let trs_endpoint = gh_trs::trs::api::TrsEndpoint::new_from_tool_version_url(wf_loc)?;
    trs_endpoint.is_valid()?;
    let metadata_url = Url::parse(&format!(
        "{}tools/{}/versions/{}/yevis-metadata.json",
        trs_endpoint.url.as_str(),
        id,
        version
    ))?;
    Ok(metadata_url)
}

fn generate_config(
    wf_loc: &Url,
    gh_token: &Option<impl AsRef<str>>,
    url_type: gh_trs::raw_url::UrlType,
) -> Result<metadata::types::Config> {
    let gh_token = env::github_token(gh_token)?;
    let primary_wf = file_url::FileUrl::new(&gh_token, wf_loc, None, None)?;

    Ok(metadata::types::Config {
        id: Uuid::new_v4(),
        version: "1.0.0".to_string(),
        license: Some("CC0-1.0".to_string()),
        authors: vec![author_from_gh_api(&gh_token)?],
        zenodo: None,
        workflow: metadata::types::Workflow {
            name: primary_wf.file_name()?,
            readme: primary_wf.readme(&gh_token, &url_type)?,
            language: gh_trs::inspect::inspect_wf_type_version(&primary_wf.to_url(&url_type)?)?,
            files: primary_wf.wf_files(&gh_token, &url_type)?,
            testing: vec![metadata::types::Testing::default()],
        },
    })
}

fn author_from_gh_api(gh_token: impl AsRef<str>) -> Result<metadata::types::Author> {
    match metadata::types::Author::new_from_api(&gh_token) {
        Ok(mut author) => {
            author.orcid = Some("PUT YOUR ORCID OR REMOVE THIS LINE".to_string());
            Ok(author)
        }
        Err(e) => Err(anyhow!("Failed to get GitHub user with error: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_version_url_to_metadata_url() {
        let url = Url::parse(
            "https://ddbj.github.io/workflow-registry/tools/9df2332c-f51d-4752-b2bf-d4a4ed4e6760/versions/1.0.0",
        )
        .unwrap();
        let config_url = tool_version_url_to_metadata_url(&url).unwrap();
        println!("{}", config_url);
        assert_eq!(
            config_url,
            Url::parse(
                "https://ddbj.github.io/workflow-registry/tools/9df2332c-f51d-4752-b2bf-d4a4ed4e6760/versions/1.0.0/yevis-metadata.json"
            )
            .unwrap()
        );
    }

    #[test]
    fn test_tool_version_url_to_metadata_url_invalid() {
        let url = Url::parse("https://example.com/tools/1.0.0").unwrap();
        let config_url = tool_version_url_to_metadata_url(&url);
        assert!(config_url.is_err());
    }
}
