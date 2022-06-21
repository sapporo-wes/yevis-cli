pub mod api;
pub mod types;

use crate::env;
use crate::gh;
use crate::metadata;

use anyhow::{anyhow, ensure, Result};
use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

pub fn upload_zenodo_and_commit_gh(
    meta_vec: &mut Vec<metadata::types::Metadata>,
    gh_token: impl AsRef<str>,
    repo: impl AsRef<str>,
    zenodo_community: &Option<impl AsRef<str>>,
) -> Result<()> {
    let host = env::zenodo_host();
    let token = env::zenodo_token()?;

    for meta in meta_vec {
        info!(
            "Uploading wf_id: {}, version: {} to Zenodo",
            meta.id, meta.version
        );
        upload_zenodo(&host, &token, meta, &repo, zenodo_community)?;
        info!("Updating workflow metadata to Zenodo URL");
        update_metadata(&host, &token, meta)?;

        // commit modified metadata file to GitHub default branch
        info!("Commit modified workflow metadata file to GitHub");
        let (owner, name) = gh::parse_repo(&repo)?;
        let default_branch = gh::api::get_default_branch(&gh_token, &owner, &name, None)?;
        let meta_path = PathBuf::from(format!("{}/yevis-metadata-{}.yml", &meta.id, &meta.version));
        let meta_content = serde_yaml::to_string(&meta)?;
        let commit_message = format!(
            "Update workflow after uploading to Zenodo, id: {} version: {}",
            &meta.id, &meta.version
        );
        gh::api::create_or_update_file(
            &gh_token,
            &owner,
            &name,
            &meta_path,
            &commit_message,
            &meta_content,
            &default_branch,
        )?;
    }
    Ok(())
}

fn upload_zenodo(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    meta: &mut metadata::types::Metadata,
    repo: impl AsRef<str>,
    zenodo_community: &Option<impl AsRef<str>>,
) -> Result<()> {
    delete_unpublished_depositions(&host, &token, meta.id.to_string())?;
    let published_deposition_ids = api::list_depositions(
        &host,
        &token,
        &meta.id.to_string(),
        types::DepositionStatus::Published,
    )?;
    ensure!(
        published_deposition_ids.len() < 2,
        "More than one published deposition for wf_id: {}",
        meta.id
    );
    let deposition_id = if published_deposition_ids.is_empty() {
        // create new deposition
        info!("Creating new deposition");
        api::create_deposition(&host, &token, meta, repo, zenodo_community)?
    } else {
        // new version deposition
        let prev_id = published_deposition_ids[0];
        let (zenodo, version) = api::retrieve_record(&host, &token, &prev_id)?;
        let new_id = if version == meta.version {
            info!("Already exist deposition with same version. So skipping.");
            meta.zenodo = Some(zenodo);
            return Ok(());
        } else {
            info!("Creating new version deposition from {}", prev_id);
            api::new_version_deposition(&host, &token, &prev_id)?
        };
        api::update_deposition(&host, &token, &new_id, meta, repo, zenodo_community)?;
        new_id
    };
    info!("Created draft deposition: {}", deposition_id);

    info!("Updating and uploading files");
    let deposition_files = api::get_files_list(&host, &token, &deposition_id)?;
    let meta_files = metadata_to_files(meta)?;
    update_deposition_files(&host, &token, &deposition_id, deposition_files, meta_files)?;

    info!("Publishing deposition {}", deposition_id);
    let zenodo = api::publish_deposition(&host, &token, &deposition_id)?;
    info!(
        "Published deposition {} as DOI {}",
        deposition_id, zenodo.doi
    );

    meta.zenodo = Some(zenodo);

    Ok(())
}

fn delete_unpublished_depositions(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    wf_id: impl AsRef<str>,
) -> Result<()> {
    let draft_deposition_ids =
        api::list_depositions(&host, &token, &wf_id, types::DepositionStatus::Draft)?;
    if !draft_deposition_ids.is_empty() {
        info!(
            "Found {} draft deposition(s), so deleting them",
            draft_deposition_ids.len()
        );
        for id in draft_deposition_ids {
            info!("Deleting draft deposition {}", id);
            api::delete_deposition(&host, &token, &id)?;
        }
    }
    Ok(())
}

fn metadata_to_files(meta: &metadata::types::Metadata) -> Result<Vec<types::MetaFile>> {
    let mut files = vec![];
    files.push(types::MetaFile::new_from_str(
        serde_yaml::to_string(&meta)?,
        PathBuf::from(format!("yevis-metadata-{}.yml", meta.version)),
    )?);
    files.push(types::MetaFile::new_from_url(
        &meta.workflow.readme,
        PathBuf::from("README.md"),
    )?);
    for file in &meta.workflow.files {
        files.push(types::MetaFile::new_from_url(
            &file.url,
            file.target.as_ref().unwrap(),
        )?); // validated
    }
    for testing in &meta.workflow.testing {
        for file in &testing.files {
            files.push(types::MetaFile::new_from_url(
                &file.url,
                file.target.as_ref().unwrap(),
            )?); // validated
        }
    }
    Ok(files)
}

/// in deposition_files, in meta_files
///   - checksum is the same: do nothing
///   - checksum is not the same: delete and create
/// in deposition_files, not in meta_files: delete
/// not in deposition_files, in meta_files: create
fn update_deposition_files(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    deposition_id: &u64,
    deposition_files: Vec<types::DepositionFile>,
    meta_files: Vec<types::MetaFile>,
) -> Result<()> {
    let deposition_files_map: HashMap<String, types::DepositionFile> = deposition_files
        .into_iter()
        .map(|f| (f.filename.clone(), f))
        .collect();
    let meta_files_map: HashMap<String, types::MetaFile> = meta_files
        .into_iter()
        .map(|f| (f.filename.clone(), f))
        .collect();

    for (filename, deposition_file) in deposition_files_map.iter() {
        match meta_files_map.get(filename) {
            Some(meta_file) => {
                if deposition_file.checksum == meta_file.checksum {
                    // do nothing
                    continue;
                } else {
                    // delete and create
                    api::delete_deposition_file(&host, &token, deposition_id, &deposition_file.id)?;
                    api::create_deposition_file(
                        &host,
                        &token,
                        deposition_id,
                        &meta_file.filename,
                        &meta_file.file_path,
                    )?;
                }
            }
            None => {
                // delete
                api::delete_deposition_file(&host, &token, deposition_id, &deposition_file.id)?;
            }
        }
    }
    for (filename, meta_file) in meta_files_map.iter() {
        match deposition_files_map.get(filename) {
            Some(_) => {
                // do nothing (already done)
                continue;
            }
            None => {
                // create
                api::create_deposition_file(
                    &host,
                    &token,
                    deposition_id,
                    &meta_file.filename,
                    &meta_file.file_path,
                )?;
            }
        }
    }
    Ok(())
}

fn update_metadata(
    host: impl AsRef<str>,
    token: impl AsRef<str>,
    meta: &mut metadata::types::Metadata,
) -> Result<()> {
    let deposition_id = meta
        .zenodo
        .as_ref()
        .ok_or_else(|| anyhow!("No Zenodo deposition ID"))?
        .id;
    let files_map: HashMap<String, Url> =
        api::get_files_download_urls(&host, &token, &deposition_id)?;

    let err_msg = "Failed to update workflow metadata files.";
    meta.workflow.readme = files_map
        .get("README.md")
        .ok_or_else(|| anyhow!(err_msg))?
        .clone();
    for file in &mut meta.workflow.files {
        file.url = files_map
            .get(
                &file
                    .target
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|x| x.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("_"),
            )
            .ok_or_else(|| anyhow!(err_msg))?
            .clone();
    }
    for testing in &mut meta.workflow.testing {
        for file in &mut testing.files {
            file.url = files_map
                .get(
                    &file
                        .target
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| x.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("_"),
                )
                .ok_or_else(|| anyhow!(err_msg))?
                .clone();
        }
    }
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::sub_cmd::validate;
//     // use uuid::Uuid;

//     #[test]
//     #[ignore]
//     fn test_list_depositions() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(&host, &token, &meta.id.to_string(), DepositionStatus::Draft)?;
//         dbg!(&ids);
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_new_deposition() -> Result<()> {
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         Deposition::new(&meta, "ddbj/workflow-registry-dev", &None::<String>)?;
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_create_deposition() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         create_deposition(
//             &host,
//             &token,
//             &meta,
//             "ddbj/workflow-registry-dev",
//             &None::<String>,
//         )?;
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_delete_draft_deposition() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(&host, &token, &meta.id.to_string(), DepositionStatus::Draft)?;
//         if !ids.is_empty() {
//             let id = ids[0];
//             delete_deposition(&host, &token, &id)?;
//         }
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_update_deposition() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(&host, &token, &meta.id.to_string(), DepositionStatus::Draft)?;
//         if !ids.is_empty() {
//             let id = ids[0];
//             update_deposition(
//                 &host,
//                 &token,
//                 &id,
//                 &meta,
//                 "ddbj/workflow-registry-dev",
//                 &None::<String>,
//             )?;
//         }
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_update_deposition_new_version() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let mut meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         meta.version = "1.0.1".to_string();
//         let id = 1018767;
//         update_deposition(
//             &host,
//             &token,
//             &id,
//             &meta,
//             "ddbj/workflow-registry-dev",
//             &None::<String>,
//         )?;
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_metadata_to_files() -> Result<()> {
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let files = metadata_to_files(&meta)?;
//         dbg!(&files);
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_get_bucket_url() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let deposition_id = 1064212;
//         let bucket_url = get_bucket_url(&host, &token, &deposition_id)?;
//         dbg!(&bucket_url);
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_update_deposition_files() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = validate::validate(
//             vec!["./yevis-metadata_gatk-workflows_mitochondria-pipeline.yml"],
//             &None::<String>,
//             "ddbj/workflow-registry",
//         )?;
//         // let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let meta_files = metadata_to_files(&meta[0])?;
//         // let meta_files = vec![];
//         // let ids = list_depositions(
//         //     &host,
//         //     &token,
//         //     &meta.id.to_string(),
//         //     DepositionStatus::Draft,
//         // )?;
//         // let id = ids[0];
//         let id = 1064212;
//         let deposition_files = get_files_list(&host, &token, &id)?;
//         // let deposition_files = vec![];
//         update_deposition_files(&host, &token, &id, deposition_files, meta_files)?;
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_get_files_list() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(
//             &host,
//             &token,
//             &meta.id.to_string(),
//             DepositionStatus::Published,
//         )?;
//         if !ids.is_empty() {
//             // let id = ids[0];
//             let id = 1018767;
//             let list = get_files_list(&host, &token, &id)?;
//             dbg!(&list);
//         }
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_publish_deposition() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(&host, &token, &meta.id.to_string(), DepositionStatus::Draft)?;
//         if !ids.is_empty() {
//             let id = ids[0];
//             publish_deposition(&host, &token, &id)?;
//         }
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_new_version_deposition() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(
//             &host,
//             &token,
//             &meta.id.to_string(),
//             DepositionStatus::Published,
//         )?;
//         if !ids.is_empty() {
//             let id = ids[0];
//             new_version_deposition(&host, &token, &id)?;
//         }
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_upload_zenodo() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let mut meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         // meta.id = Uuid::new_v4();
//         upload_zenodo(
//             &host,
//             &token,
//             &mut meta,
//             "ddbj/workflow-registry-dev",
//             &None::<String>,
//         )?;
//         // update_metadata(&host, &token, &mut meta)?;
//         // println!("{}", serde_yaml::to_string(&meta)?);
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_retrieve_record() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(
//             &host,
//             &token,
//             &meta.id.to_string(),
//             DepositionStatus::Published,
//         )?;
//         if !ids.is_empty() {
//             let id = ids[0];
//             let (zenodo, version) = retrieve_record(&host, &token, &id)?;
//             dbg!(&zenodo);
//             dbg!(&version);
//         }
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_update_metadata() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let mut meta = validate::validate(
//             vec!["./tests/test-metadata-SMK.yml"],
//             &None::<String>,
//             "ddbj/workflow-registry-dev",
//         )?[0]
//             .clone();
//         let zenodo = metadata::types::Zenodo {
//             concept_doi: "10.5072/zenodo.1018220".to_string(),
//             doi: "10.5072/zenodo.1018220".to_string(),
//             id: 1018220,
//             url: Url::parse("https://sandbox.zenodo.org/record/1018220")?,
//         };
//         meta.zenodo = Some(zenodo);
//         update_metadata(&host, &token, &mut meta)?;
//         Ok(())
//     }

//     #[test]
//     #[ignore]
//     fn test_get_files_download_urls() -> Result<()> {
//         let host = env::zenodo_host();
//         let token = env::zenodo_token()?;
//         let meta = metadata::io::read("./tests/test-metadata-CWL-validated.yml")?;
//         let ids = list_depositions(
//             &host,
//             &token,
//             &meta.id.to_string(),
//             DepositionStatus::Published,
//         )?;
//         if !ids.is_empty() {
//             let id = ids[0];
//             let files_map = get_files_download_urls(&host, &token, &id)?;
//             dbg!(&files_map);
//         }
//         Ok(())
//     }
// }
