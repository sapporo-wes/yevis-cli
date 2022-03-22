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

        /// Path to output file.
        #[structopt(short, long, parse(from_os_str), default_value = "yevis-config.yml")]
        output: PathBuf,

        /// Make a template from an existing workflow.
        /// When using this option, specify the TRS ToolVersion URL (e.g., https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>) as `workflow_location`.
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

        /// GitHub repository to send the pull requests to. (format: <owner>/<repo>)
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

        /// GitHub repository to send the pull requests to. (format: <owner>/<repo>)
        #[structopt(short, long, default_value = env::default_pr_repo())]
        repository: String,

        /// WES location where the test will be run.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Get the modified files from the GitHub PR files.
        /// This option is used for the pull request event in a CI environment.
        /// When using this option, specify the GitHub PR URL (e.g., ${{ github.event.pull_request._links.html.href }}) as `config_locations`.
        #[structopt(long)]
        from_pr: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    /// Create a pull request based on the yevis configuration file (after validation and testing)
    PullRequest {
        /// Location of the yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository to send the pull requests to. (format: <owner>/<repo>)
        #[structopt(short, long, default_value = env::default_pr_repo())]
        repository: String,

        /// WES location where the test will be run.
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

    /// Generate TRS response and host on GitHub Pages. (Basically used in a CI environment (`CI=true`))
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

        /// WES location where the test will be run.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Recursively get the yevis configuration files from the TRS endpoint and publish them.
        /// This option is used in a CI environment.
        /// When using this option, specify the TRS endpoint (e.g., https://ddbj.github.io/yevis-workflows/) as `config_locations`.
        #[structopt(long)]
        from_trs: bool,

        /// Get the modified files from the GitHub PR files.
        /// This option is used for the pull request event in a CI environment.
        /// When using this option, specify the GitHub PR URL (e.g., ${{ github.event.pull_request._links.html.href }}) as `config_locations`.
        #[structopt(long)]
        from_pr: bool,

        /// Upload the dataset to Zenodo.
        #[structopt(long)]
        upload_zenodo: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },
}
