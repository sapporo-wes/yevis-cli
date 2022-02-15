use crate::env;

use std::path::PathBuf;
use structopt::{clap, StructOpt};
use url::Url;

#[derive(StructOpt, Debug, PartialEq, Clone)]
#[structopt(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub enum Args {
    /// Make a template for the yevis configuration file.
    MakeTemplate {
        /// Location of the primary workflow document. (only hosted on GitHub)
        workflow_location: Url,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// Path to the output file.
        #[structopt(short, long, parse(from_os_str), default_value = "yevis-config.yml")]
        output: PathBuf,

        /// Make a template from update.
        /// When using this option, specify the TRS URL (e.g., https://<trs-endpoint>/tools/<wf_id>) as the workflow location.
        #[structopt(short, long)]
        update: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    /// Validate the schema and contents of the yevis configuration file.
    Validate {
        /// Location of the yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository to send pull requests to. (format: <owner>/<repo>)
        #[structopt(short, long, default_value = env::default_pr_repo())]
        repository: String,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    /// Test the workflow based on the yevis configuration file.
    Test {
        /// Location of the yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository to send pull requests to. (format: <owner>/<repo>)
        #[structopt(short, long, default_value = env::default_pr_repo())]
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

    /// Create a pull request based on the yevis configuration file (after validating and testing.)
    PullRequest {
        /// Location of the yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository to send pull requests to. (format: <owner>/<repo>)
        #[structopt(short, long, default_value = env::default_pr_repo())]
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

    /// Publish the TRS response to GitHub. (Basically used in a CI environment (GITHUB_ACTIONS=1))
    Publish {
        /// Location of the yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository to publish the TRS response to. (format: <owner>/<repo>)
        #[structopt(short, long, default_value = env::default_pr_repo())]
        repository: String,

        /// GitHub branch to publish the TRS response to.
        #[structopt(short, long, default_value = "gh-pages")]
        branch: String,

        /// Test before publishing.
        #[structopt(long)]
        with_test: bool,

        /// Location of WES in which to run the test.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Recursively get the yevis configuration files from the TRS endpoint and publish them.
        /// This option is used to test and publish all workflows in a CI environment.
        /// If you use this option, specify the TRS endpoint for `config_locations`.
        #[structopt(long)]
        from_trs: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },
}
