use anyhow::{bail, Error, Result};
use std::path::PathBuf;
use std::str::FromStr;
use structopt::{clap, StructOpt};
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub enum FileFormat {
    Yaml,
    Json,
}

impl FromStr for FileFormat {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "yaml" => Ok(FileFormat::Yaml),
            "json" => Ok(FileFormat::Json),
            _ => bail!("Invalid file format `{}`", s),
        }
    }
}

#[derive(StructOpt, Debug, PartialEq, Clone)]
#[structopt(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub enum Args {
    /// Generates a configuration file template for yevis from a workflow document.
    MakeTemplate {
        // Remote location of the workflow's main document file (only hosted on GitHub).
        workflow_location: Url,

        /// GitHub Personal Access Token.
        #[structopt(short, long)]
        github_token: Option<String>,

        /// Path to the output file.
        #[structopt(short, long, parse(from_os_str), default_value = "yevis_config.yml")]
        output: PathBuf,

        /// Format of the output file (`yaml` or `json`).
        #[structopt(short, long, default_value = "yaml")]
        format: FileFormat,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    /// Validate the schema and contents of the configuration file.
    Validate {
        /// Configuration file generated by `make-template` command.
        #[structopt(parse(from_os_str), default_value = "yevis_config.yml")]
        config_file: PathBuf,

        /// GitHub Personal Access Token.
        #[structopt(short, long)]
        github_token: Option<String>,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    /// Actually, test the workflow based on the configuration file.
    Test {
        /// Configuration file generated by `make-template` command.
        #[structopt(parse(from_os_str), default_value = "yevis_config.yml")]
        config_file: PathBuf,

        /// GitHub Personal Access Token.
        #[structopt(short, long)]
        github_token: Option<String>,

        /// Location of WES in which to run the test.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    /// After validating and testing, create a pull request to `ddbj/yevis-workflows`.
    PullRequest {
        /// Configuration file generated by `make-template` command.
        #[structopt(parse(from_os_str))]
        config_file: PathBuf,

        /// GitHub Personal Access Token.
        #[structopt(short, long)]
        github_token: Option<String>,

        /// GitHub repository to send pull requests to.
        #[structopt(short, long, default_value = "ddbj/yevis-workflows")]
        repository: String,

        /// Location of WES in which to run the test.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_template() -> Result<()> {
        let args = Args::from_iter(&[
            "yevis",
            "make-template",
            "https://github.com/ddbj/yevis-cli/path/to/workflow.yml",
        ]);
        assert_eq!(
            args,
            Args::MakeTemplate {
                workflow_location: Url::from_str(
                    "https://github.com/ddbj/yevis-cli/path/to/workflow.yml"
                )
                .unwrap(),
                github_token: None,
                output: PathBuf::from("yevis_config.yml"),
                format: FileFormat::Yaml,
                verbose: false,
            }
        );
        Ok(())
    }

    #[test]
    fn test_json_format() -> Result<()> {
        let args = Args::from_iter(&[
            "yevis",
            "make-template",
            "--format",
            "json",
            "https://github.com/ddbj/yevis-cli/path/to/workflow.yml",
        ]);
        assert_eq!(
            args,
            Args::MakeTemplate {
                workflow_location: Url::from_str(
                    "https://github.com/ddbj/yevis-cli/path/to/workflow.yml"
                )
                .unwrap(),
                github_token: None,
                output: PathBuf::from("yevis_config.yml"),
                format: FileFormat::Json,
                verbose: false,
            }
        );
        Ok(())
    }

    #[test]
    fn test_invalid_format() -> Result<()> {
        let result = Args::from_iter_safe(&[
            "yevis",
            "make-template",
            "--format",
            "toml",
            "https://github.com/ddbj/yevis-cli/path/to/workflow.yml",
        ]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_validate() -> Result<()> {
        let args = Args::from_iter(&["yevis", "validate", "yevis_config.yml"]);
        assert_eq!(
            args,
            Args::Validate {
                config_file: PathBuf::from("yevis_config.yml"),
                github_token: None,
                verbose: false,
            }
        );
        Ok(())
    }

    #[test]
    fn test_test() -> Result<()> {
        let args = Args::from_iter(&["yevis", "test", "yevis_config.yml"]);
        assert_eq!(
            args,
            Args::Test {
                config_file: PathBuf::from("yevis_config.yml"),
                github_token: None,
                wes_location: None,
                docker_host: Url::from_str("unix:///var/run/docker.sock").unwrap(),
                verbose: false,
            }
        );
        Ok(())
    }

    #[test]
    fn test_pull_request() -> Result<()> {
        let args = Args::from_iter(&["yevis", "pull-request", "yevis_config.yml"]);
        assert_eq!(
            args,
            Args::PullRequest {
                config_file: PathBuf::from("yevis_config.yml"),
                github_token: None,
                repository: "ddbj/yevis-workflows".to_string(),
                wes_location: None,
                docker_host: Url::from_str("unix:///var/run/docker.sock").unwrap(),
                verbose: false,
            }
        );
        Ok(())
    }
}
