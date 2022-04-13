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
/// CLI tool to support building and maintaining a workflow registry.
pub enum Args {
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Make a template for the Yevis configuration file.
    MakeTemplate {
        /// Location of a primary workflow document (only hosted on GitHub).
        workflow_location: Url,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// Path to the output file.
        #[structopt(short, long, parse(from_os_str), default_value = "yevis-config.yml")]
        output: PathBuf,

        /// Make a template from an existing workflow.
        /// When using this option, specify a TRS ToolVersion URL (e.g., `https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>`) as `workflow_location`.
        #[structopt(short, long)]
        update: bool,

        /// Use `<commit_hash>` instead of `<branch_name>` in generated GitHub raw contents URLs.
        #[structopt(long)]
        use_commit_url: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Validate schema and contents of the Yevis configuration file.
    Validate {
        /// Location of the Yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository that published TRS responses (format: <owner>/<repo>, this option is used for version validation).
        #[structopt(short, long)]
        repository: String,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Test workflow based on the Yevis configuration files.
    Test {
        /// Location of the Yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

        /// GitHub Personal Access Token.
        #[structopt(long = "gh-token")]
        github_token: Option<String>,

        /// GitHub repository to which the pull request will be sent (format: <owner>/<repo>).
        #[structopt(short, long)]
        repository: String,

        /// WES location where the test will be run.
        /// If not specified, `sapporo-service` will be started.
        #[structopt(short, long)]
        wes_location: Option<Url>,

        /// Location of the Docker host.
        #[structopt(short, long, default_value = "unix:///var/run/docker.sock")]
        docker_host: Url,

        /// Get modified files from a GitHub Pull Request.
        /// This option is used for pull request events in the the CI environment.
        /// When using this option, specify a GitHub Pull Request URL (e.g., `${{ github.event.pull_request._links.html.href }}`) as `config_locations`.
        #[structopt(long)]
        from_pr: bool,

        /// Verbose mode.
        #[structopt(short, long)]
        verbose: bool,
    },

    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    /// Create a pull request based on the Yevis configuration files (after validation and testing).
    PullRequest {
        /// Location of the Yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

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
        /// Location of the Yevis configuration files (local file path or remote URL).
        #[structopt(default_value = "yevis-config.yml")]
        config_locations: Vec<String>,

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

        /// Recursively get the Yevis configuration files from the TRS endpoint and publish them.
        /// This option is used in the CI environment.
        /// When using this option, specify the TRS endpoint (e.g., https://ddbj.github.io/yevis-workflows/) as `config_locations`.
        #[structopt(long)]
        from_trs: bool,

        /// Get modified files from GitHub Pull Request.
        /// This option is used for pull request events in the CI environment.
        /// When using this option, specify GitHub Pull Request URL (e.g., `${{ github.event.pull_request._links.html.href }}`) as `config_locations`.
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
