use crate::gh;
use crate::metadata;
use crate::remote;

use anyhow::Context;
use anyhow::{anyhow, bail, ensure, Result};
use log::debug;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use url::Url;

pub fn validate(
    meta_loc: impl AsRef<str>,
    gh_token: impl AsRef<str>,
) -> Result<metadata::types::Metadata> {
    let mut meta = metadata::io::read(meta_loc.as_ref(), &gh_token)?;
    validate_version(&meta.version)?;
    validate_license(&mut meta, &gh_token)?;
    validate_authors(&meta)?;
    validate_language(&meta)?;
    validate_wf_name(&meta.workflow.name)?;
    validate_and_update_workflow(&mut meta, &gh_token)?;
    debug!("updated metadata file:\n{}", serde_yaml::to_string(&meta)?);
    Ok(meta)
}

/// allow characters
/// - alphabet
/// - number
/// - ~!@#$%^&()_+-={}[];,.
/// - space
pub fn validate_version(version: impl AsRef<str>) -> Result<()> {
    let version_re = regex::Regex::new(r"^[a-zA-Z0-9\~!@\#\$%\^\&\(\)_\+\-=\{\}\[\];,\. ]+$")?;
    ensure!(
        version_re.is_match(version.as_ref()),
        "The version field contains invalid characters, only alphanumeric, space and ~!@#$%^&()_+-={{}}[];,. are allowed"
    );
    Ok(())
}

/// Validate the license of the metadata file.
/// Contact GitHub API and Zenodo API to confirm.
/// Change the license to `spdx_id`
/// e.g., `apache-2.0` -> `Apache-2.0`
fn validate_license(meta: &mut metadata::types::Metadata, gh_token: impl AsRef<str>) -> Result<()> {
    let spdx_id: String = validate_with_github_license_api(gh_token, &meta.license)?;
    validate_with_zenodo_license_api(&spdx_id)?;
    meta.license = spdx_id;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct LicenseResponse {
    permissions: Vec<String>,
    spdx_id: String,
}

/// https://docs.github.com/ja/rest/reference/licenses#get-a-license
/// Ensure that `distribution` is included in `permissions` field.
fn validate_with_github_license_api(
    gh_token: impl AsRef<str>,
    license: impl AsRef<str>,
) -> Result<String> {
    let url = Url::parse(&format!(
        "https://api.github.com/licenses/{}",
        license.as_ref()
    ))?;
    let res = gh::get_request(gh_token, &url, &[])?;
    let res: LicenseResponse =
        serde_json::from_value(res).context("Failed to parse GitHub license API response")?;
    ensure!(
        res.permissions.contains(&String::from("distribution")),
        "GitHub license API response does not contain `distribution` in `permissions` field"
    );
    Ok(res.spdx_id)
}

/// https://developers.zenodo.org/?shell#retrieve41
fn validate_with_zenodo_license_api(license: impl AsRef<str>) -> Result<()> {
    let url = Url::parse(&format!(
        "https://zenodo.org/api/vocabularies/licenses/{}",
        license.as_ref()
    ))?;
    // Return the path for this URL, as a percent-encoded ASCII string
    let response = reqwest::blocking::get(url.as_str())?;
    let status = response.status();
    ensure!(
        status.is_success(),
        "`license` is not valid from Zenodo license API"
    );
    Ok(())
}

fn validate_authors(meta: &metadata::types::Metadata) -> Result<()> {
    let orcid_re = Regex::new(r"^\d{4}-\d{4}-\d{4}-\d{3}[\dX]$")?;
    let mut account_set: HashSet<&str> = HashSet::new();
    for author in &meta.authors {
        if let Some(orcid) = &author.orcid {
            ensure!(orcid_re.is_match(orcid), "`authors[].orcid` is not valid",);
        };
        ensure!(
            !account_set.contains(author.github_account.as_str()),
            "`authors[].github_account` is not unique",
        );
        account_set.insert(author.github_account.as_str());
    }
    ensure!(
        !meta.authors.is_empty(),
        "`authors` must have more than one author",
    );
    Ok(())
}

fn validate_language(meta: &metadata::types::Metadata) -> Result<()> {
    match meta.workflow.language.r#type {
        metadata::types::LanguageType::Unknown => {
            bail!("`language.type` is not specified. Please specify `CWL`, `WDL`, `NFL` or `SMK`")
        }
        _ => Ok(()),
    }
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

fn update_url(
    url: &Url,
    gh_token: impl AsRef<str>,
    branch_memo: Option<&mut HashMap<String, String>>,
    commit_memo: Option<&mut HashMap<String, String>>,
) -> Result<Url> {
    let remote = remote::Remote::new(url, gh_token, branch_memo, commit_memo)?;
    remote.to_typed_url(&remote::UrlType::Commit)
}

fn validate_and_update_workflow(
    meta: &mut metadata::types::Metadata,
    gh_token: impl AsRef<str>,
) -> Result<()> {
    let mut branch_memo = HashMap::new();
    let mut commit_memo = HashMap::new();

    meta.workflow.readme = update_url(
        &meta.workflow.readme,
        &gh_token,
        Some(&mut branch_memo),
        Some(&mut commit_memo),
    )
    .map_err(|e| anyhow!("Invalid `workflow.readme`: {}", e))?;

    ensure!(
        meta.workflow.primary_wf().is_ok(),
        "One `primary` needs to be specified in the `workflow.files[].type` field",
    );

    for file in &mut meta.workflow.files {
        file.url = update_url(
            &file.url,
            &gh_token,
            Some(&mut branch_memo),
            Some(&mut commit_memo),
        )
        .map_err(|e| anyhow!("Invalid `workflow.files[].url`: {}", e))?;
        file.complement_target()?;
    }

    let mut test_id_set: HashSet<&str> = HashSet::new();
    for testing in &mut meta.workflow.testing {
        ensure!(
            !test_id_set.contains(testing.id.as_str()),
            "`workflow.testing[].id` is not unique, duplicated id: {}",
            testing.id.as_str()
        );
        test_id_set.insert(testing.id.as_str());

        for file in &mut testing.files {
            file.url = update_url(
                &file.url,
                &gh_token,
                Some(&mut branch_memo),
                Some(&mut commit_memo),
            )
            .map_err(|e| anyhow!("Invalid `workflow.testing[].files[].url`: {}", e))?;
            file.complement_target()?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::env;

    #[test]
    fn test_validate_with_github_license_api() -> Result<()> {
        let gh_token = env::github_token(&None::<String>)?;
        validate_with_github_license_api(&gh_token, "cc0-1.0")?;
        validate_with_github_license_api(&gh_token, "mit")?;
        validate_with_github_license_api(&gh_token, "MIT")?;
        validate_with_github_license_api(&gh_token, "apache-2.0")?;
        Ok(())
    }

    #[test]
    fn test_validate_with_zenodo_license_api() -> Result<()> {
        validate_with_zenodo_license_api("cc0-1.0")?;
        validate_with_zenodo_license_api("mit")?;
        validate_with_zenodo_license_api("apache-2.0")?;
        Ok(())
    }

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
