use crate::env;
use crate::file_url;
use crate::inspect;
use crate::metadata;
use crate::raw_url;

use anyhow::{anyhow, Result};
use log::{debug, info};
use std::path::Path;
use url::Url;
use uuid::Uuid;

pub fn make_template(
    wf_loc: &Url,
    gh_token: &Option<impl AsRef<str>>,
    output: impl AsRef<Path>,
    url_type: raw_url::UrlType,
) -> Result<()> {
    info!("Making a template from {}", wf_loc);

    let meta = generate_metadata(wf_loc, gh_token, url_type)?;

    debug!("template metadata file:\n{}", serde_yaml::to_string(&meta)?);

    let file_ext = metadata::io::parse_file_ext(&output)?;
    metadata::io::write_local(&meta, &output, &file_ext)?;
    Ok(())
}

fn generate_metadata(
    wf_loc: &Url,
    gh_token: &Option<impl AsRef<str>>,
    url_type: raw_url::UrlType,
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
            language: inspect::inspect_wf_type_version(&primary_wf.to_url(&url_type)?)?,
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
