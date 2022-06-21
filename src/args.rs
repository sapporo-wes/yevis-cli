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
/// CLI tool that supports building a Yevis workflow registry with automated quality control.
pub enum Args {
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Generate a template file for the Yevis metadata file.
    MakeTemplate {
        /// Remote location of a primary workflow document.
        workflow_location: Url,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// Path to the output file.
        #[structopt(short, long, parse(from_os_str), default_value = "yevis-metadata.yml")]
        output: PathBuf,

        /// Use `<commit_hash>` instead of `<branch_name>` in generated GitHub raw contents URLs.
        #[structopt(long)]
        use_commit_url: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Validate schema and contents of the Yevis metadata file.
    Validate {
        /// Location of the Yevis metadata files (local file path or remote URL).
        #[structopt(default_value = "yevis-metadata.yml")]
        metadata_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Test workflow based on the Yevis metadata files.
    Test {
        /// Location of the Yevis metadata files (local file path or remote URL).
        #[structopt(default_value = "yevis-metadata.yml")]
        metadata_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// WES location where the test will be run.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the Docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Get modified files from a GitHub Pull Request.
        /// This option is used for pull request events in the the CI environment.
        /// When using this option, specify a GitHub Pull Request URL (e.g., `${{ github.event.pull_request._links.html.href }}`) as `metadata_locations`.
        #[structopt(long)]
        from_pr: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Create a pull request based on the Yevis metadata files (after validation and testing).
    PullRequest {
        /// Location of the Yevis metadata files (local file path or remote URL).
        #[structopt(default_value = "yevis-metadata.yml")]
        metadata_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository to which the pull request will be sent (format: <owner>/<repo>).
        #[structopt(short, long)]
        repository: String,

        /// Location of a WES where the test will be run.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the Docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Generate TRS responses and host them on GitHub Pages. (Basically used in the CI environment (`CI=true`))
    Publish {
        /// Location of the Yevis metadata files (local file path or remote URL).
        #[structopt(default_value = "yevis-metadata.yml")]
        metadata_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository that publishes TRS responses (format: <owner>/<repo>).
        #[structopt(short, long)]
        repository: String,

        /// Test before publishing.
        #[structopt(long)]
        with_test: bool,

        /// Location of the WES where the test will be run.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of Docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Get modified files from GitHub Pull Request.
        /// This option is used for pull request events in the CI environment.
        /// When using this option, specify GitHub Pull Request URL (e.g., `${{ github.event.pull_request._links.html.href }}`) as `metadata_locations`.
        #[structopt(long)]
        from_pr: bool,

        /// Upload dataset to Zenodo.
        #[structopt(long)]
        upload_zenodo: bool,

        /// Community set in Zenodo deposition.
        #[structopt(long)]
        zenodo_community: Option<String>,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },
}

impl Args {
    pub fn verbose(&self) -> bool {
        match self {
            Args::MakeTemplate { verbose, .. } => *verbose,
            Args::Validate { verbose, .. } => *verbose,
            Args::Test { verbose, .. } => *verbose,
            Args::PullRequest { verbose, .. } => *verbose,
            Args::Publish { verbose, .. } => *verbose,
        }
    }

    pub fn gh_token(&self) -> Option<String> {
        match self {
            Args::MakeTemplate { github_token, .. } => github_token.clone(),
            Args::Validate { github_token, .. } => github_token.clone(),
            Args::Test { github_token, .. } => github_token.clone(),
            Args::PullRequest { github_token, .. } => github_token.clone(),
            Args::Publish { github_token, .. } => github_token.clone(),
        }
    }
}
