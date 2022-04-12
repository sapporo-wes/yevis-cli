# yevis-cli

CLI tool for sustainable workflow provisioning.
Support workflow testing, persistence, and hosting via the [GA4GH TRS (Tool Registry Service) API](https://www.ga4gh.org/news/tool-registry-service-api-enabling-an-interoperable-library-of-genomics-analysis-tools/).

Features include:

- Generate registration templates
- Validate registration files
- Test workflows based on registration files
- Create Pull Request to add workflow to GitHub repository
- Upload workflow-related files to [Zenodo](https://zenodo.org/) and obtain DOIs
- Generate TRS response ([GA4GH - Tool Registry Service API](https://www.ga4gh.org/news/tool-registry-service-api-enabling-an-interoperable-library-of-genomics-analysis-tools/)) and host on GitHub Pages

---

[`ddbj/yevis-workflows`](https://github.com/ddbj/yevis-workflows) is a workflow collection published by [DDBJ](https://www.ddbj.nig.ac.jp/) using `yevis-cli`.

A web application [`ddbj/yevis-web`](https://github.com/ddbj/yevis-web) is also available to browse the published workflows.

## Installation

**As a dependency, `yevis` uses Docker to run tests.**

Use a single binary that is built without any dependencies (supports Linux only):

```bash
$ curl -fsSL -O https://github.com/ddbj/yevis-cli/releases/latest/download/yevis
$ chmod +x ./yevis
$ ./yevis --help
```

Or, use the Docker environment (also `docker-compose`):

```bash
$ docker-compose up -d --build
$ docker-compose exec app yevis --help
```

## Getting started

First, `yevis` requires a `GitHub Personal Access Token` for various operations using the GitHub REST API.
For instructions on generating a `GitHub Personal Access Token`, see [GitHub Docs - Creating a personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token).

The required scopes are as follows (also see ScreenShot):

- `repo - public_repo`
- `user - read:user`

<img src="https://user-images.githubusercontent.com/26019402/149902689-bfd4707d-9792-41fd-b22f-8a1631489399.png" alt="yevis-cli-img-1" width="600">

Once generated, the `GitHub Personal Access Token`, need to pass it to `yevis` in one of the following ways:

- Env file: Write the token in `.env` file like `GITHUB_TOKEN=<paste_your_token>`
- Environment variable: Set the environment variable `GITHUB_TOKEN`
- Command-line option: Use option `--github-token <paste_your_token>`

---

Use the workflow [`trimming_and_qc.cwl`](https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl) as an example.

First, generate a configuration file template from the GitHub location of the primary workflow file by:

```bash
$ yevis make-template https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl
```

Edit the generated `./yevis_config.yml` as [`test_config_CWL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_CWL.yml).

The main parts to edit are as follows:

- `workflow.files`: List of workflows and related files
- `workflow.testing`: List of tests to be run

Then, validate the configuration file, run tests, and generate a pull request by:

```bash
$ yevis pull-request ./yevis_config.yml
...
Creating pull request to ddbj/yevis-workflows
Pull request URL: https://api.github.com/repos/ddbj/yevis-workflows/pulls/1
Finished pull-request
```

Workflows are submitted as Pull Requests and checked by the administrator.

## Usage

This section describes some of the subcommands.

```bash
$ yevis --help
yevis 0.1.3
DDBJ(DNA Data Bank of Japan)

USAGE:
    yevis <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help             Prints this message or the help of the given subcommand(s)
    make-template    Make a template for the yevis configuration file
    publish          Generate TRS response and host on GitHub Pages. (Basically used in a CI environment (`CI=true`))
    pull-request     Create a pull request based on the yevis configuration file (after validation and testing)
    test             Test the workflow based on the yevis configuration file
    validate         Validate the schema and contents of the yevis configuration file
```

### make-template

Generate configuration file template from the GitHub location of the primary workflow file.

```bash
$ yevis make-template --help
yevis-make-template 0.1.3
Make a template for the yevis configuration file

USAGE:
    yevis make-template [FLAGS] [OPTIONS] <workflow-location>

FLAGS:
    -h, --help       Prints help information
    -u, --update     Make a template from an existing workflow. When using this option, specify the TRS ToolVersion URL
                     (e.g., https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>) as `workflow_location`
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
        --gh-token <github-token>    GitHub Personal Access Token
    -o, --output <output>            Path to output file [default: yevis-config.yml]

ARGS:
    <workflow-location>    Location of the primary workflow document. (only hosted on GitHub)
```

Only URLs hosted on GitHub are accepted for `workflow-location`.
This URL is a URL like `https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl`, which will later be converted to a raw URL like `https://raw.githubusercontent.com/ddbj/yevis-cli/645a193826bdb3f0731421d4ff1468d0736b4a06/tests/CWL/wf/trimming_and_qc.cwl`.

`yevis` collects various information and generates a template for the configuration file.
In particular, `workflow.files` is generated as a recursive list of files from the primary workflow location.

Use the `--update` option to update an already published workflow.
Specifically, `--update https://<trs-endpoint>/tools/<wf_id>/versions/<wf_version>` will generate a template with the same ID.

### validate

Validate the schema and contents of the yevis configuration files.

```bash
$ yevis validate --help
yevis-validate 0.1.3
Validate the schema and contents of the yevis configuration file

USAGE:
    yevis validate [FLAGS] [OPTIONS] [config-locations]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
        --gh-token <github-token>    GitHub Personal Access Token
    -r, --repository <repository>    GitHub repository to send the pull requests to. (format: <owner>/<repo>) [default:
                                     ddbj/workflow-registry-dev]

ARGS:
    <config-locations>...    Location of the yevis configuration files (local file path or remote URL) [default:
                             yevis-config.yml]
```

Explanation of validation rules for some fields in the configuration file:

- `id`: The ID of the workflow generated by the `make-template` command; this value should not be changed.
- `version`: The version of the workflow, in the form of `x.y.z`.
- `license`: A license for the workflow; an example of license should be a distributable license such as `CC0-1.0`, `MIT`, or `Apache-2.0`. This is because `yevis` will later upload the workflow to Zenodo.
- `authors`: Workflow author information. `yevis` will use this information for Zenodo uploads (Please do not change the ddbj author).
  - `github_account`: GitHub account of the author.
  - `name`: Name the author in the format Family name, Given names (e.g., "Doe, John").
  - `affiliation`: Affiliation of the author (optional).
  - `orcid`: ORCID identifier of the author (optional).
- `workflow.name`: Please fill freely. Allowed characters are `a-z`, `A-Z`, `0-9`, `~!@#$%^&*()_+-={}[]|:;,.<>?`, and space.
- `workflow.readme`: It is used to describe the workflow. Specify the location of the README file.
- `workflow.language`: `CWL`, `WDL`, `NFL`, and `SMK` are supported.
- `workflow.files`: A list of files; files specified as `type: secondary` will be placed in the execution directory with `target` as a path when the workflow is executed.
- `workflow.testing`: A list of tests. See `test` for how to write tests.

Several example are provided. Please check:

- [`test_config_CWL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_CWL.yml)
- [`test_config_WDL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_WDL.yml)
- [`test_config_NFL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_NFL.yml)
- [`test_config_SMK.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_SMK.yml)

### test

Test the workflow based on the yevis configuration file.

```bash
$ yevis test --help
yevis-test 0.1.3
Test the workflow based on the yevis configuration file

USAGE:
    yevis test [FLAGS] [OPTIONS] [config-locations]...

FLAGS:
        --from-pr    Get the modified files from the GitHub PR files. This option is used for the pull request event in
                     a CI environment. When using this option, specify the GitHub PR URL (e.g., ${{
                     github.event.pull_request._links.html.href }}) as `config_locations`
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -d, --docker-host <docker-host>      Location of the docker host [default: unix:///var/run/docker.sock]
        --gh-token <github-token>        GitHub Personal Access Token
    -r, --repository <repository>        GitHub repository to send the pull requests to. (format: <owner>/<repo>)
                                         [default: ddbj/workflow-registry-dev]
    -w, --wes-location <wes-location>    WES location where the test will be run. If not specified, `sapporo-service`
                                         will be started

ARGS:
    <config-locations>...    Location of the yevis configuration files (local file path or remote URL) [default:
                             yevis-config.yml]
```

The test is run using the Workflow Execution Service (WES; [GA4GH - WES API](https://www.ga4gh.org/news/ga4gh-wes-api-enables-portable-genomic-analysis/).
In particular, `yevis` uses[`sapporo-service`](https://github.com/sapporo-wes/sapporo-service) as WES.
If the option `--wes-location` is not specified, `sapporo-service` will be stated and used as WES.

An example of the `workflow.testing` field in the configuration file is shown below:

```yaml
testing:
  - id: test_1
    files:
      - url: "https://example.com/path/to/wf_params.json"
        target: wf_params.json
        type: wf_params
      - url: "https://example.com/path/to/wf_engine_params.json"
        target: wf_engine_params.json
        type: wf_engine_params
      - url: "https://example.com/path/to/data.fq"
        target: data.fq
        type: other
```

There are three types of files:

- `wf_params`: Parameters for the workflow.
- `wf_engine_params`: Parameters for the workflow engine.
- `other`: Other files. (e.g., data files, etc.)

The files specified as `wf_params` and `wf_engine_params` are placed as WES execution parameters at WES runtime.
Also, the `other` files are placed in the execution directory with `target` as a path when the workflow is executed.

The `id` field can be freely specified.

The `--from-pr` option is used within GitHub Actions; see the GitHub Actions section.

### pull-request

Create a pull request based on the yevis configuration file (after validation and testing).

```bash
$ yevis pull-request --help
yevis-pull-request 0.1.3
Create a pull request based on the yevis configuration file (after validation and testing)

USAGE:
    yevis pull-request [FLAGS] [OPTIONS] [config-locations]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -d, --docker-host <docker-host>      Location of the docker host [default: unix:///var/run/docker.sock]
        --gh-token <github-token>        GitHub Personal Access Token
    -r, --repository <repository>        GitHub repository to send the pull requests to. (format: <owner>/<repo>)
                                         [default: ddbj/yevis-workflows]
    -w, --wes-location <wes-location>    WES location where the test will be run. If not specified, `sapporo-service`
                                         will be started

ARGS:
    <config-locations>...    Location of the yevis configuration files (local file path or remote URL) [default:
                             yevis-config.yml]
```

Pull requests are created from the forked repository.
The typical flow when this command is executed is as follows:

1. Fork the repository specified by the `--repository` option to your GitHub account
2. Create a new branch (named `workflow_id`) for the new workflow
3. Commit the configuration file to the new branch
4. Create a new pull request

The default for the `--repository` option is `ddbj/yevis-workflows`, so the Pull Request will be created in the [GitHub - ddbj/yevis-workflows](https://github.com/ddbj/yevis-workflows).
If the environment variable `YEVIS_DEV=1` is set, the default is `ddbj/workflow-registry-dev`.

### publish

Generate TRS response and host on GitHub Pages.

```bash
$ yevis publish --help
Generate TRS response and host on GitHub Pages. (Basically used in a CI environment (`CI=true`))

USAGE:
    yevis publish [FLAGS] [OPTIONS] [config-locations]...

FLAGS:
        --from-pr          Get the modified files from the GitHub PR files. This option is used for the pull request
                           event in a CI environment. When using this option, specify the GitHub PR URL (e.g., ${{
                           github.event.pull_request._links.html.href }}) as `config_locations`
        --from-trs         Recursively get the yevis configuration files from the TRS endpoint and publish them. This
                           option is used in a CI environment. When using this option, specify the TRS endpoint (e.g.,
                           https://ddbj.github.io/yevis-workflows/) as `config_locations`
    -h, --help             Prints help information
        --upload-zenodo    Upload the dataset to Zenodo
    -V, --version          Prints version information
    -v, --verbose          Verbose mode
        --with-test        Test before publishing

OPTIONS:
    -b, --branch <branch>                GitHub branch to publish the TRS response to [default: gh-pages]
    -d, --docker-host <docker-host>      Location of the docker host [default: unix:///var/run/docker.sock]
        --gh-token <github-token>        GitHub Personal Access Token
    -r, --repository <repository>        GitHub repository to publish the TRS response to. (format: <owner>/<repo>)
                                         [default: ddbj/workflow-registry-dev]
    -w, --wes-location <wes-location>    WES location where the test will be run. If not specified, `sapporo-service`
                                         will be started

ARGS:
    <config-locations>...    Location of the yevis configuration files (local file path or remote URL) [default:
                             yevis-config.yml]
```

This command is used within GitHub Actions.
Therefore, it will not run unless the environment variable `CI=true` is set.

The following four options are explained in particular:

- `--from-pr`: Publish from pull request id
- `--from-trs`: Publish all workflows contained in the TRS endpoint
- `--upload-zenodo`: Upload the workflow and dataset to Zenodo.
- `--with-test`: Test before publishing

See the GitHub Actions section for more details.

## GitHub Actions

`yevis` uses GitHub Actions for sustainable workflow provisioning.

Two actions are provided as examples:

- [`yevis-test-pr.yml`](https://github.com/ddbj/yevis-cli/blob/main/actions_example/yevis-test-pr.yml): Action to automatically validate and test pull requests
- [`yevis-publish-pr.yml`](https://github.com/ddbj/yevis-cli/blob/main/actions_example/yevis-publish-pr.yml): Action to upload to Zenodo and publish TRS response when pull requests are merged
  - `ZENODO_TOKEN` must be set as GitHub secrets.

Examples of `yevis` commands executed within each action are as follows:

```bash
# yevis-test-pr.yml
$ yevis test --from-pr ${{github.event.pull_request._links.html.href }}

# yevis-publish-pr.yml
$ yevis publish --with-test --upload-zenodo --from-pr ${{github.event.pull_request._links.html.href }}
```

## Development

Launch the development environment using `docker-compose`:

```bash
$ docker-compose -f docker-compose.dev.yml up -d --build
$ docker-compose -f docker-compose.dev.yml exec app bash
# cargo run -- --help
yevis 0.1.3
...
```

Setting the environment variable `YEVIS_DEV=1` will create a pull request in the development environment [`GitHub - ddbj/workflow-registry-dev`](https://github.com/yevis/workflow-registry-dev).

### Build binary

**Recommendation**, build binary using musl:

```bash
$ docker run --rm -it -v $PWD:/home/rust/src ekidd/rust-musl-builder cargo build --release

# No dependencies
$ ldd target/x86_64-unknown-linux-musl/release/yevis
not a dynamic executable
```

Build binary using native builder:

```bash
$ cargo build --release

# There are several packages and dependencies.
$ ldd ./target/release/yevis
linux-vdso.so.1 (0x00007ffea49d3000)
libssl.so.1.1 => /usr/lib/x86_64-linux-gnu/libssl.so.1.1 (0x00007f317cbc0000)
libcrypto.so.1.1 => /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 (0x00007f317c6f5000)
libgcc_s.so.1 => /lib/x86_64-linux-gnu/libgcc_s.so.1 (0x00007f317c4dd000)
librt.so.1 => /lib/x86_64-linux-gnu/librt.so.1 (0x00007f317c2d5000)
libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f317c0b6000)
libm.so.6 => /lib/x86_64-linux-gnu/libm.so.6 (0x00007f317bd18000)
libdl.so.2 => /lib/x86_64-linux-gnu/libdl.so.2 (0x00007f317bb14000)
libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f317b723000)
/lib64/ld-linux-x86-64.so.2 (0x00007f317d64a000)
```

### Run test

Run unit tests:

```bash
$ cargo test -- --test-threads=1 --nocapture
```

Several test workflows are prepared. See [tests/README.md](https://github.com/ddbj/yevis-cli/blob/main/tests/README.md).

### Download artifacts from build GitHub Actions

```bash
$ gh run --repo ddbj/yevis-cli list --workflow build_binary --json databaseId --jq .[0].databaseId | xargs -I {} gh run --repo ddbj/yevis-cli download {} -n yevis
```

### Release

Use [`release.sh`](https://github.com/ddbj/yevis-cli/blob/main/release.sh) as follows:

```bash
$ bash release.sh <new_version>
```

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0). See the [LICENSE](https://github.com/ddbj/yevis-cli/blob/main/LICENSE).
