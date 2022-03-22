use crate::version;

use anyhow::{anyhow, Result};
use colored::Colorize;
use gh_trs;
use log::{debug, info, warn};
use std::path::Path;
use std::str::FromStr;
use url::Url;
use uuid::Uuid;

pub fn make_template(
    wf_loc: &Url,
    gh_token: &Option<impl AsRef<str>>,
    output: impl AsRef<Path>,
    update: bool,
) -> Result<()> {
    let gh_token = gh_trs::env::github_token(gh_token)?;

    info!("Making a template from {}", wf_loc);

    let config = if update {
        // the TRS ToolVersion URL (e.g., https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>) as `workflow_location`.
        let trs_endpoint = gh_trs::trs::api::TrsEndpoint::new_from_tool_version_url(&wf_loc)?;
        trs_endpoint.is_valid()?;
        let (id, version) = parse_trs_tool_version_url(&wf_loc)?;
        let config_loc = trs_endpoint.to_config_url(&id.to_string(), &version)?;
        let mut config = gh_trs::config::io::read_config(&config_loc)?;
        let prev_version = version::Version::from_str(&version)?;
        config.version = prev_version.increment_patch().to_string();

        config
    } else {
        let primary_wf = gh_trs::raw_url::RawUrl::new(&gh_token, &wf_loc, None, None)?;

        let id = Uuid::new_v4();
        let version = "1.0.0".to_string();
        let mut authors = vec![ddbj_author()];
        match author_from_gh_api(&gh_token) {
            Ok(author) => {
                authors.push(author);
            }
            Err(e) => {
                warn!(
                    "{}: Failed to get GitHub user with error: {}",
                    "Warning".yellow(),
                    e
                );
            }
        };
        let wf_name = primary_wf.file_stem()?;
        let readme = gh_trs::raw_url::RawUrl::new(
            &gh_token,
            &gh_trs::github_api::get_readme_url(&gh_token, &primary_wf.owner, &primary_wf.name)?,
            None,
            None,
        )?
        .to_url()?;
        let language = gh_trs::inspect::inspect_wf_type_version(&primary_wf.to_url()?)?;
        let files = gh_trs::command::make_template::obtain_wf_files(&gh_token, &primary_wf)?;
        let testing = vec![gh_trs::config::types::Testing::default()];

        gh_trs::config::types::Config {
            id,
            version,
            license: Some("CC0-1.0".to_string()),
            authors,
            zenodo: None,
            workflow: gh_trs::config::types::Workflow {
                name: wf_name,
                readme,
                language,
                files,
                testing,
            },
        }
    };
    debug!("template config: {:?}", config);

    let file_ext = gh_trs::config::io::parse_file_ext(&output)?;
    gh_trs::config::io::write_config(&config, &output, &file_ext)?;
    Ok(())
}

fn ddbj_author() -> gh_trs::config::types::Author {
    gh_trs::config::types::Author {
        github_account: "ddbj".to_string(),
        name: Some("ddbj-workflow".to_string()),
        affiliation: Some("DNA Data Bank of Japan".to_string()),
        orcid: None,
    }
}

fn author_from_gh_api(gh_token: impl AsRef<str>) -> Result<gh_trs::config::types::Author> {
    match gh_trs::config::types::Author::new_from_api(&gh_token) {
        Ok(mut author) => {
            author.orcid = Some("".to_string());
            Ok(author)
        }
        Err(e) => Err(e),
    }
}

/// from: https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>
/// to: (<wf_id>, <wf_version>)
fn parse_trs_tool_version_url(url: &Url) -> Result<(Uuid, String)> {
    let mut segments = url.path_segments().ok_or(anyhow!("Invalid url: {}", url))?;
    let wf_version = segments
        .next_back()
        .ok_or(anyhow!("Invalid url: {}", url))?
        .to_string();
    segments.next_back();
    let wf_id = Uuid::parse_str(
        segments
            .next_back()
            .ok_or(anyhow!("Invalid url: {}", url))?,
    )?;
    Ok((wf_id, wf_version))
}
