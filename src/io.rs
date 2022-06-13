use crate::trs;
use anyhow::Result;
use log::debug;
use url::Url;

/// modified from metadata::io
pub fn find_metadata_loc_recursively_from_trs(trs_loc: impl AsRef<str>) -> Result<Vec<String>> {
    let trs_endpoint = trs::api::TrsEndpoint::new_from_url(&Url::parse(trs_loc.as_ref())?)?;
    trs_endpoint.is_valid()?;
    let metadata_locs: Vec<String> = trs::api::get_tools(&trs_endpoint)?
        .into_iter()
        .flat_map(|tool| tool.versions)
        .map(|version| version.url)
        .map(|url| format!("{}/yevis-metadata.json", url.as_str()))
        .collect();
    debug!("Found Yevis metadata file locations: {:?}", metadata_locs);
    Ok(metadata_locs)
}
