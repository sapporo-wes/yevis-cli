use crate::env;
use crate::gh_trs::config;
use crate::gh_trs::raw_url;

use anyhow::{ensure, Context, Result};
use log::{debug, info};
use std::collections::{HashMap, HashSet};

#[cfg(not(tarpaulin_include))]
pub fn validate(
    config_locs: Vec<impl AsRef<str>>,
    gh_token: &Option<impl AsRef<str>>,
) -> Result<Vec<config::types::Config>> {
    let gh_token = env::github_token(gh_token)?;

    let mut configs = Vec::new();

    for config_loc in config_locs {
        info!("Validating {}", config_loc.as_ref());
        let mut config = config::io::read_config(config_loc.as_ref())?;

        validate_authors(&config.authors)?;
        validate_language(&config.workflow.language)?;
        validate_wf_name(&config.workflow.name)?;
        validate_and_update_workflow(&gh_token, &mut config)?;

        debug!("updated config: {:?}", config);

        configs.push(config);
    }
    Ok(configs)
}

pub fn validate_authors(authors: &[config::types::Author]) -> Result<()> {
    ensure!(!authors.is_empty(), "No authors found in config file");
    ensure!(
        authors.len()
            == authors
                .iter()
                .map(|a| a.github_account.clone())
                .collect::<HashSet<_>>()
                .len(),
        "Duplicate github accounts found in config file"
    );
    Ok(())
}

pub fn validate_language(language: &config::types::Language) -> Result<()> {
    ensure!(
        language.r#type.is_some(),
        "Language type not specified in config file"
    );
    ensure!(
        language.version.is_some(),
        "Language version not specified in config file"
    );
    Ok(())
}

/// allow characters
/// - alphabet
/// - number
/// - ~!@#$%^&*()_+-={}[]|:;,.<>?
/// - space
pub fn validate_wf_name(wf_name: impl AsRef<str>) -> Result<()> {
    let wf_name_re =
        regex::Regex::new(r"^[a-zA-Z0-9\~!@\#\$%\^\&\*\(\)_\+\-=\{\}\[\]\|:;,\.<>\? ]+$")?;
    ensure!(
        wf_name_re.is_match(wf_name.as_ref()),
        "Workflow name contains invalid characters, only alphanumeric, space and ~!@#$%^&*()_+-={{}}[]|:;,.<>? are allowed"
    );
    Ok(())
}

pub fn validate_and_update_workflow(
    gh_token: &impl AsRef<str>,
    config: &mut config::types::Config,
) -> Result<()> {
    let mut branch_memo = HashMap::new();
    let mut commit_memo = HashMap::new();

    config.workflow.readme = raw_url::RawUrl::new(
        gh_token,
        &config.workflow.readme,
        Some(&mut branch_memo),
        Some(&mut commit_memo),
    )
    .context("Failed to convert readme url to raw url")?
    .to_url(&raw_url::UrlType::Commit)?;

    ensure!(
        config.workflow.primary_wf().is_ok(),
        "Expected to contain one primary workflow file."
    );

    for file in &mut config.workflow.files {
        file.update_url(gh_token, Some(&mut branch_memo), Some(&mut commit_memo))?;
        file.complement_target()?;
    }

    let mut test_id_set: HashSet<&str> = HashSet::new();
    for testing in &mut config.workflow.testing {
        ensure!(
            !test_id_set.contains(testing.id.as_str()),
            "Duplicate test id: {}",
            testing.id.as_str()
        );
        test_id_set.insert(testing.id.as_str());

        for file in &mut testing.files {
            file.update_url(gh_token, Some(&mut branch_memo), Some(&mut commit_memo))?;
            file.complement_target()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_wf_name() -> Result<()> {
        validate_wf_name("abc")?;
        validate_wf_name("abcABC123")?;
        validate_wf_name("abcABC123~!@#$%^&*()_+-={{}}[]|:;,.<>? ")?;
        validate_wf_name("Workflow name: example_workflow-123.cwl (for example)")?;
        let err = validate_wf_name("`");
        assert!(err.is_err());
        Ok(())
    }
}
