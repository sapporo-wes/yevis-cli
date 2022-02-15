use anyhow::Result;
use colored::Colorize;
use gh_trs;
use log::{debug, info, warn};
use std::path::Path;
use url::Url;
use uuid::Uuid;

pub fn make_template(
    wf_loc: &Url,
    gh_token: &Option<impl AsRef<str>>,
    output: impl AsRef<Path>,
    _update: bool,
) -> Result<()> {
    let gh_token = gh_trs::env::github_token(gh_token)?;

    info!("Making a template from {}", wf_loc);
    let primary_wf = gh_trs::raw_url::RawUrl::new(&gh_token, &wf_loc, None, None)?;

    let id = Uuid::new_v4();
    let version = "1.0.0".to_string(); // TODO
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

    let config = gh_trs::config::types::Config {
        id,
        version,
        license: Some("CC0-1.0".to_string()),
        authors,
        workflow: gh_trs::config::types::Workflow {
            name: wf_name,
            readme,
            language,
            files,
            testing,
        },
    };
    debug!("template config: {:?}", config);

    let file_ext = gh_trs::config::io::parse_file_ext(&output)?;
    gh_trs::config::io::write_config(&config, &output, &file_ext)?;
    Ok(())
}

fn ddbj_author() -> gh_trs::config::types::Author {
    gh_trs::config::types::Author {
        github_account: "ddbj".to_string(),
        name: Some("ddbj-workflows".to_string()),
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
