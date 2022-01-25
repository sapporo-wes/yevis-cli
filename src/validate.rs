use crate::{
    args::FileFormat,
    github_api::{head_request, read_github_token, to_raw_url_from_url, WfRepoInfo},
    make_template::{find_latest_version, Version},
    path_utils::file_format,
    pull_request::parse_repo,
    type_config::{Author, Config, FileType, Repo, TestFileType, Workflow},
};
use anyhow::{bail, ensure, Context, Result};
use colored::Colorize;
use log::{debug, info};
use regex::Regex;
use serde_json;
use serde_yaml;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;
use std::string::ToString;
use uuid::Uuid;

pub fn validate(
    config_file: impl AsRef<Path>,
    arg_github_token: &Option<impl AsRef<str>>,
    repository: impl AsRef<str>,
) -> Result<Config> {
    let github_token = read_github_token(&arg_github_token)?;
    ensure!(
        !github_token.is_empty(),
        "GitHub token is empty. Please set it with --github-token option or set GITHUB_TOKEN environment variable."
    );
    let (repo_owner, repo_name) = parse_repo(&repository)?;

    info!("Reading config file: {}", config_file.as_ref().display());
    let mut config = read_config(&config_file)?;
    debug!("config:\n{:#?}", &config);

    validate_version(
        &github_token,
        &repo_owner,
        &repo_name,
        &config.id,
        &config.version,
    )?;
    validate_license(&config.license)?;
    validate_authors(&config.authors)?;
    config.workflow = validate_workflow(&github_token, &config.workflow)?;

    Ok(config)
}

pub fn read_config(config_file: impl AsRef<Path>) -> Result<Config> {
    let file_format = file_format(&config_file)?;
    let reader = BufReader::new(File::open(&config_file).context(format!(
        "Failed to open inputted config file: {}",
        config_file.as_ref().display()
    ))?);
    let config: Config = match file_format {
        FileFormat::Yaml => match serde_yaml::from_reader(reader) {
            Ok(config) => config,
            Err(err) => bail!("Failed to parse YAML because it does not conform to the expected schema. Error: {}", err),
        },
        FileFormat::Json => match serde_json::from_reader(reader) {
            Ok(config) => config,
            Err(err) => bail!("Failed to parse JSON because it does not conform to the expected schema. Error: {}", err),
        },
    };
    Ok(config)
}

fn validate_version(
    github_token: impl AsRef<str>,
    owner: impl AsRef<str>,
    name: impl AsRef<str>,
    wf_id: &Uuid,
    version: impl AsRef<str>,
) -> Result<()> {
    let re = Regex::new(r"^([0-9]+)\.([0-9]+)\.([0-9]+)$")?;
    ensure!(
        re.is_match(version.as_ref()),
        "Invalid version: {}. It should be in the format of `x.y.z`",
        version.as_ref()
    );
    let inputted_version = Version::from_str(version.as_ref())?;
    match find_latest_version(github_token, owner.as_ref(), name.as_ref(), wf_id) {
        Ok(latest_version) => {
            ensure!(
                inputted_version > latest_version,
                "Version {} is less than the latest version {}",
                inputted_version.to_string(),
                latest_version.to_string()
            )
        }
        Err(err) => {
            ensure!(
                inputted_version == Version::from_str("1.0.0")?,
                "Failed to find latest version. Error: {}",
                err
            )
        }
    };
    Ok(())
}

fn validate_license(license: impl AsRef<str>) -> Result<()> {
    ensure!(
        license.as_ref() == "CC0-1.0",
        "Invalid license: {}, expected only `CC0-1.0`. Since yevis uploads all data to Zenodo, it needs to use the CC0-1.0 license.",
        license.as_ref()
    );
    Ok(())
}

fn validate_authors(authors: &Vec<Author>) -> Result<()> {
    ensure!(
        authors.len() > 1,
        "Please add at least one person and ddbj as authors.",
    );

    let mut account_set: HashSet<&str> = HashSet::new();
    ensure!(
        authors
            .iter()
            .all(|author| account_set.insert(author.github_account.as_ref())),
        "Duplicated GitHub account found: {}",
        authors
            .iter()
            .filter(|author| !account_set.insert(author.github_account.as_ref()))
            .map(|author| author.github_account.as_ref())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut ddbj_found = false;
    for author in authors {
        match author.github_account.as_str() {
            "ddbj" => {
                let ddbj_author = Author::new_ddbj();
                ensure!(
                    author == &ddbj_author,
                    "The ddbj author in authors field has been changed. Please update it to the correct one like: {:#?}",
                    ddbj_author
                );
                ddbj_found = true;
            }
            _ => validate_author(&author)?,
        }
    }
    ensure!(ddbj_found, "Please add ddbj as an author.");

    Ok(())
}

fn validate_author(author: &Author) -> Result<()> {
    let re = Regex::new(r"^\d{4}-\d{4}-\d{4}-(\d{3}X|\d{4})$")?;
    ensure!(
        author.github_account != "",
        "Please specify github account for author: {:#?}",
        &author
    );
    ensure!(
        author.name != "",
        "Please specify name for author: {:#?}",
        &author
    );
    if author.orcid != "" {
        ensure!(
            re.is_match(&author.orcid),
            "Invalid orcid: {}. It should be in the format of `0000-0000-0000-0000`",
            &author.orcid
        );
    };

    Ok(())
}

fn validate_workflow(github_token: impl AsRef<str>, workflow: &Workflow) -> Result<Workflow> {
    let mut cloned_wf = workflow.clone();

    let primary_wf_num = workflow
        .files
        .iter()
        .filter(|f| f.r#type == FileType::Primary)
        .count();
    ensure!(
        primary_wf_num == 1,
        "Please specify only one primary workflow."
    );
    let primary_wf = match workflow
        .files
        .iter()
        .find(|f| f.r#type == FileType::Primary)
    {
        Some(f) => f,
        None => bail!(
            "The primary workflow file is not found. Please add it to the `workflow.files` field."
        ),
    };
    let primary_wf_repo_info = WfRepoInfo::new(&github_token, &primary_wf.url)?;
    ensure!(
        workflow.repo == Repo::new(&primary_wf_repo_info),
        "The repository information in the primary workflow file does not match the `workflow.repo` field. Please update it to the correct one like: {:#?}",
        &primary_wf_repo_info
    );

    let raw_readme_url = to_raw_url_from_url(&github_token, &primary_wf.url)?;
    match head_request(&raw_readme_url, 0) {
        Ok(_) => {
            info!(
                "{}: Readme URL is not raw URL. It will be converted to raw URL: {}",
                "Warning".yellow(),
                raw_readme_url.as_str()
            );
            cloned_wf.readme = raw_readme_url;
        }
        Err(e) => bail!(
            "Failed to head request to the readme file: {}, error: {}",
            &raw_readme_url,
            e
        ),
    };

    for i in 0..workflow.files.len() {
        let file = &workflow.files[i];
        let raw_file_url = to_raw_url_from_url(&github_token, &file.url)?;
        match head_request(&raw_file_url, 0) {
            Ok(_) => {
                info!(
                    "{}: File URL is not raw URL. It will be converted to raw URL: {}",
                    "Warning".yellow(),
                    raw_file_url.as_str()
                );
                cloned_wf.files[i].url = raw_file_url;
            }
            Err(e) => bail!(
                "Failed to head request to the file: {}, error: {}",
                &raw_file_url,
                e
            ),
        };
    }

    ensure!(
        workflow.testing.len() > 0,
        "Please specify at least one testing for workflow."
    );
    let mut test_id_set: HashSet<&str> = HashSet::new();
    for i in 0..workflow.testing.len() {
        let testing = &workflow.testing[i];
        let wf_params_num = testing
            .files
            .iter()
            .filter(|f| f.r#type == TestFileType::WfParams)
            .count();
        ensure!(
            wf_params_num < 2,
            "Please specify only one workflow parameters file in test id: {}",
            testing.id
        );
        let wf_engine_params_num = testing
            .files
            .iter()
            .filter(|f| f.r#type == TestFileType::WfEngineParams)
            .count();
        ensure!(
            wf_engine_params_num < 2,
            "Please specify only one workflow engine parameters file in test id: {}",
            testing.id
        );

        for j in 0..testing.files.len() {
            let file = &testing.files[j];
            let raw_file_url = to_raw_url_from_url(&github_token, &file.url)?;
            match head_request(&raw_file_url, 0) {
                Ok(_) => {
                    info!(
                        "{}: Test file URL is not raw URL. It will be converted to raw URL: {}",
                        "Warning".yellow(),
                        raw_file_url.as_str()
                    );
                    cloned_wf.testing[i].files[j].url = raw_file_url;
                }
                Err(e) => bail!(
                    "Failed to head request to the file: {}, error: {}",
                    &raw_file_url,
                    e
                ),
            };
        }
        match test_id_set.insert(testing.id.as_str()) {
            true => {}
            false => bail!("Duplicated test id: {}", testing.id.as_str()),
        }
    }

    Ok(cloned_wf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::default_ddbj_workflows;
    use crate::type_config::{File as WfFile, TestFile};
    use anyhow::anyhow;
    use std::path::PathBuf;
    use url::Url;

    #[test]
    fn test_validate_cwl_config() -> Result<()> {
        let config_file = "tests/test_config_CWL.yml";
        validate(config_file, &None::<String>, default_ddbj_workflows())?;
        Ok(())
    }

    #[test]
    fn test_validate_wdl_config() -> Result<()> {
        let config_file = "tests/test_config_WDL.yml";
        validate(config_file, &None::<String>, default_ddbj_workflows())?;
        Ok(())
    }

    #[test]
    fn test_validate_nfl_config() -> Result<()> {
        let config_file = "tests/test_config_NFL.yml";
        validate(config_file, &None::<String>, default_ddbj_workflows())?;
        Ok(())
    }

    #[test]
    fn test_validate_smk_config() -> Result<()> {
        let config_file = "tests/test_config_SMK.yml";
        validate(config_file, &None::<String>, default_ddbj_workflows())?;
        Ok(())
    }

    #[test]
    fn test_validate_broken_config() -> Result<()> {
        let config_file = "tests/test_config_broken.yml";
        let result = validate(config_file, &None::<String>, default_ddbj_workflows());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse YAML because it does not conform to the expected schema."));
        Ok(())
    }

    #[test]
    fn test_validate_with_invalid_file_format() -> Result<()> {
        let config_file = "tests/yevis.foobar";
        let result = validate(config_file, &None::<String>, default_ddbj_workflows());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid file format"));
        Ok(())
    }

    #[test]
    fn test_validate_with_not_found_config_file() -> Result<()> {
        let config_file = "foobar.yml";
        let result = validate(config_file, &None::<String>, default_ddbj_workflows());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to open inputted config file"));
        Ok(())
    }

    // #[test]
    // fn test_validate_version() -> Result<()> {
    //     assert!(validate_version("0.1.0").is_ok());
    //     assert!(validate_version("v0.1.0").is_err());
    //     assert!(validate_version("0.1.0-alpha").is_err());
    //     Ok(())
    // }

    #[test]
    fn test_validate_license() -> Result<()> {
        assert!(validate_license("CC0-1.0").is_ok());
        assert!(validate_license("MIT").is_err());
        Ok(())
    }

    #[test]
    fn test_validate_authors_ok() -> Result<()> {
        let authors = vec![
            Author {
                github_account: "suecharo".to_string(),
                name: "Example Name".to_string(),
                affiliation: "Example Affiliation".to_string(),
                orcid: "0000-0003-2765-0049".to_string(),
            },
            Author {
                github_account: "ddbj".to_string(),
                name: "ddbj-workflow".to_string(),
                affiliation: "DNA Data Bank of Japan".to_string(),
                orcid: "DO NOT ENTER".to_string(),
            },
        ];
        assert!(validate_authors(&authors).is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_authors_with_only_ddbj() -> Result<()> {
        let authors = vec![Author {
            github_account: "ddbj".to_string(),
            name: "ddbj-workflow".to_string(),
            affiliation: "DNA Data Bank of Japan".to_string(),
            orcid: "DO NOT ENTER".to_string(),
        }];
        let result = validate_authors(&authors);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Please add at least one person and ddbj as authors."));
        Ok(())
    }

    #[test]
    fn test_validate_authors_with_ddbj_fixed() -> Result<()> {
        let authors = vec![
            Author {
                github_account: "suecharo".to_string(),
                name: "Example Name".to_string(),
                affiliation: "Example Affiliation".to_string(),
                orcid: "0000-0003-2765-0049".to_string(),
            },
            Author {
                github_account: "ddbj".to_string(),
                name: "ddbj fixed".to_string(),
                affiliation: "DNA".to_string(),
                orcid: "".to_string(),
            },
        ];
        let result = validate_authors(&authors);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("The ddbj author in authors field has been changed."));
        Ok(())
    }

    #[test]
    fn test_validate_authors_no_ddbj() -> Result<()> {
        let authors = vec![
            Author {
                github_account: "suecharo".to_string(),
                name: "Example Name".to_string(),
                affiliation: "Example Affiliation".to_string(),
                orcid: "0000-0003-2765-0049".to_string(),
            },
            Author {
                github_account: "suecharo_".to_string(),
                name: "Example Name".to_string(),
                affiliation: "Example Affiliation".to_string(),
                orcid: "0000-0003-2765-0049".to_string(),
            },
        ];
        let result = validate_authors(&authors);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Please add ddbj as an author."
        );
        Ok(())
    }

    #[test]
    fn test_validate_author() -> Result<()> {
        let author = Author {
            github_account: "suecharo".to_string(),
            name: "Example Name".to_string(),
            affiliation: "Example Affiliation".to_string(),
            orcid: "0000-0003-2765-0049".to_string(),
        };
        assert!(validate_author(&author).is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_workflow_with_no_primary_wf() -> Result<()> {
        let github_token = read_github_token(&None::<String>)?;
        let reader = BufReader::new(File::open("./tests/test_config_CWL.yml")?);
        let mut config: Config = serde_yaml::from_reader(reader)?;
        config.workflow.files = vec![];
        let result = validate_workflow(&github_token, &config.workflow);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Please specify only one primary workflow."));
        Ok(())
    }

    #[test]
    fn test_validate_workflow_with_invalid_repo_info() -> Result<()> {
        let github_token = read_github_token(&None::<String>)?;
        let reader = BufReader::new(File::open("./tests/test_config_CWL.yml")?);
        let mut config: Config = serde_yaml::from_reader(reader)?;
        config.workflow.repo = Repo {
            owner: "ddbj".to_string(),
            name: "yevis-cli".to_string(),
            commit: "invalid".to_string(),
        };
        let result = validate_workflow(&github_token, &config.workflow);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("The repository information in the primary workflow file does not match the `workflow.repo` field."));
        Ok(())
    }

    #[test]
    fn test_validate_workflow_update_raw_url() -> Result<()> {
        let github_token = read_github_token(&None::<String>)?;
        let reader = BufReader::new(File::open("./tests/test_config_CWL.yml")?);
        let mut config: Config = serde_yaml::from_reader(reader)?;
        config.workflow.files.push(WfFile {
            url: Url::parse("https://github.com/ddbj/yevis-cli/blob/main/README.md")?,
            target: PathBuf::from("README.md"),
            r#type: FileType::Secondary,
        });
        config.workflow.testing[0].files.push(TestFile {
            url: Url::parse("https://github.com/ddbj/yevis-cli/blob/main/README.md")?,
            target: PathBuf::from("README.md"),
            r#type: TestFileType::Other,
        });

        let new_wf = validate_workflow(&github_token, &config.workflow)?;
        let new_file = new_wf
            .files
            .iter()
            .find(|f| f.target == PathBuf::from("README.md"))
            .ok_or(anyhow!("File not found"))?;
        assert_eq!(new_file.url.host_str(), Some("raw.githubusercontent.com"));
        let new_test_file = new_wf.testing[0]
            .files
            .iter()
            .find(|f| f.target == PathBuf::from("README.md"))
            .ok_or(anyhow!("File not found"))?;
        assert_eq!(
            new_test_file.url.host_str(),
            Some("raw.githubusercontent.com")
        );
        Ok(())
    }
}
