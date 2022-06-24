# yevis-cli

[![DOI](https://zenodo.org/badge/442338046.svg)](https://zenodo.org/badge/latestdoi/442338046)
[![Apache License](https://img.shields.io/badge/license-Apache%202.0-orange.svg?style=flat&color=important)](http://www.apache.org/licenses/LICENSE-2.0)

CLI tool to support building and maintaining Yevis workflow registry.

Features include:

- Generate a workflow metadata file template
- Validate the workflow metadata file
- Execute workflow tests
- Create a Pull Request to GitHub Repository
- Upload workflow-related files to [Zenodo](https://zenodo.org/) and obtain DOI
- Generate TRS responses ([GA4GH - Tool Registry Service API](https://www.ga4gh.org/news/tool-registry-service-api-enabling-an-interoperable-library-of-genomics-analysis-tools/)) and deploy them to GitHub Pages

In addition, see the below links:

- [`ddbj/workflow-registry`](https://github.com/ddbj/workflow-registry): a workflow registry built and maintained by [DDBJ](https://www.ddbj.nig.ac.jp/) using `yevis-cli`
- [`sapporo-wes/yevis-web`](https://github.com/sapporo-wes/yevis-web): a web application to browse published workflows
- [`Yevis Getting Started`](https://sapporo-wes.github.io/yevis-cli/getting_started): the document for Yevis system installation and usage
- [`Yevis Getting Started Ja`](https://sapporo-wes.github.io/yevis-cli/getting_started_ja): 日本語での Yevis system の使い方

## Installation

**As a dependency, `yevis-cli` uses Docker to run tests.**

Use a single binary that is built without any dependencies (supports Linux only):

```bash
$ curl -fsSL -O https://github.com/sapporo-wes/yevis-cli/releases/latest/download/yevis
$ chmod +x ./yevis
$ ./yevis --help
```

Or, use the Docker environment:

```bash
$ curl -O https://raw.githubusercontent.com/sapporo-wes/yevis-cli/main/docker-compose.yml
$ docker compose up -d
$ docker compose exec app yevis --help
```

## Usage

See [Getting Started - 3. Workflow Registration](https://sapporo-wes.github.io/yevis-cli/getting_started#3-workflow-registration) for a series of usages.

This section describes some subcommands.

```bash
$ yevis --help
yevis 0.4.0
DDBJ(Bioinformatics and DDBJ Center)
CLI tool that supports building a Yevis workflow registry with automated quality control

USAGE:
    yevis <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help             Prints this message or the help of the given subcommand(s)
    make-template    Generate a template file for the Yevis metadata file
    publish          Generate TRS responses and host them on GitHub Pages. (Basically used in the CI environment
                     (`CI=true`))
    pull-request     Create a pull request based on the Yevis metadata files (after validation and testing)
    test             Test workflow based on the Yevis metadata files
    validate         Validate schema and contents of the Yevis metadata file
```

### make-template

Generate a workflow metadata file template from a primary workflow file URL.

```bash
$ yevis make-template --help
yevis-make-template 0.4.0
Generate a template file for the Yevis metadata file

USAGE:
    yevis make-template [FLAGS] [OPTIONS] <workflow-location>

FLAGS:
    -h, --help              Prints help information
        --use-commit-url    Use `<commit_hash>` instead of `<branch_name>` in generated GitHub raw contents URLs
    -V, --version           Prints version information
    -v, --verbose           Verbose mode

OPTIONS:
        --gh-token <github-token>    GitHub Personal Access Token
    -o, --output <output>            Path to the output file [default: yevis-metadata.yml]

ARGS:
    <workflow-location>    Remote location of a primary workflow document
```

Workflow location is a URL like `https://github.com/sapporo-wes/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl`, which will later be converted to a raw URL like `https://raw.githubusercontent.com/sapporo-wes/yevis-cli/main/tests/CWL/wf/trimming_and_qc.cwl`.

`yevis-cli` collects various information and generates a template for the workflow metadata file.
In particular, `workflow.files` is generated as a recursive list of files from the primary workflow location.

### validate

Validate schema and contents of the workflow metadata file.

```bash
$ yevis validate --help
yevis-validate 0.4.0
Validate schema and contents of the Yevis metadata file

USAGE:
    yevis validate [FLAGS] [OPTIONS] [metadata-locations]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
        --gh-token <github-token>    GitHub Personal Access Token

ARGS:
    <metadata-locations>...    Location of the Yevis metadata files (local file path or remote URL) [default:
                               yevis-metadata.yml]
```

Explanation of validation rules for some fields:
Several examples are provided as follows:

- [`test-metadata-CWL.yml`](https://github.com/sapporo-wes/yevis-cli/blob/main/tests/test-metadata-CWL.yml)
- [`test-metadata-WDL.yml`](https://github.com/sapporo-wes/yevis-cli/blob/main/tests/test-metadata-WDL.yml)
- [`test-metadata-NFL.yml`](https://github.com/sapporo-wes/yevis-cli/blob/main/tests/test-metadata-NFL.yml)
- [`test-metadata-SMK.yml`](https://github.com/sapporo-wes/yevis-cli/blob/main/tests/test-metadata-SMK.yml)

### test

Test workflow using [GA4GH WES](https://www.ga4gh.org/news/ga4gh-wes-api-enables-portable-genomic-analysis/).

```bash
$ yevis test --help
yevis-test 0.4.0
Test workflow based on the Yevis metadata files

USAGE:
    yevis test [FLAGS] [OPTIONS] [metadata-locations]...

FLAGS:
        --from-pr    Get modified files from a GitHub Pull Request. This option is used for pull request events in the
                     the CI environment. When using this option, specify a GitHub Pull Request URL (e.g., `${{
                     github.event.pull_request._links.html.href }}`) as `metadata_locations`
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -d, --docker-host <docker-host>      Location of the Docker host [default: unix:///var/run/docker.sock]
        --gh-token <github-token>        GitHub Personal Access Token
    -w, --wes-location <wes-location>    WES location where the test will be run. If not specified, `sapporo-service`
                                         will be started

ARGS:
    <metadata-locations>...    Location of the Yevis metadata files (local file path or remote URL) [default:
                               yevis-metadata.yml]
```

The tests are executed using WES.
If the option `--wes-location` is not specified, [`sapporo-service`](https://github.com/sapporo-wes/sapporo-service) will be started and used as WES.

An example of `workflow.testing` field is as follows:

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

| Type               | Description                                                 |
| ------------------ | ----------------------------------------------------------- |
| `wf_params`        | Workflow parameters file for the workflow execution.        |
| `wf_engine_params` | Workflow engine parameters file for the workflow execution. |
| `other`            | Other files. (e.g., data files, etc.)                       |

At WES runtime, the files specified as `wf_params` and `wf_engine_params` are placed as WES execution parameters.
In addition, the `other` files are placed in the execution directory with a `target` as a path.

The `id` field can be freely specified.

The `--from-pr` option is used within GitHub Actions.
See the GitHub Actions section.

### pull-request

Create a pull request after validation and testing.

```bash
$ yevis pull-request --help
yevis-pull-request 0.4.0
Create a pull request based on the Yevis metadata files (after validation and testing)

USAGE:
    yevis pull-request [FLAGS] [OPTIONS] --repository <repository> [metadata-locations]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -d, --docker-host <docker-host>      Location of the Docker host [default: unix:///var/run/docker.sock]
        --gh-token <github-token>        GitHub Personal Access Token
    -r, --repository <repository>        GitHub repository to which the pull request will be sent (format:
                                         <owner>/<repo>)
    -w, --wes-location <wes-location>    Location of a WES where the test will be run. If not specified, `sapporo-
                                         service` will be started

ARGS:
    <metadata-locations>...    Location of the Yevis metadata files (local file path or remote URL) [default:
                               yevis-metadata.yml]
```

A pull request is created from the forked repository as follows:

1. Fork a repository specified by the `--repository` option to your GitHub account
2. Create a new branch (named `workflow_id`) on the forked repository
3. Commit the workflow metadata file to the new branch
4. Create a pull request

### publish

Upload files to Zenodo, generate TRS responses and deploy them on GitHub Pages.

```bash
$ yevis publish --help
yevis-publish 0.4.0
Generate TRS responses and host them on GitHub Pages. (Basically used in the CI environment (`CI=true`))

USAGE:
    yevis publish [FLAGS] [OPTIONS] --repository <repository> [metadata-locations]...

FLAGS:
        --from-pr          Get modified files from GitHub Pull Request. This option is used for pull request events in
                           the CI environment. When using this option, specify GitHub Pull Request URL (e.g., `${{
                           github.event.pull_request._links.html.href }}`) as `metadata_locations`
    -h, --help             Prints help information
        --upload-zenodo    Upload dataset to Zenodo
    -V, --version          Prints version information
    -v, --verbose          Verbose mode
        --with-test        Test before publishing

OPTIONS:
    -d, --docker-host <docker-host>              Location of Docker host [default: unix:///var/run/docker.sock]
        --gh-token <github-token>                GitHub Personal Access Token
    -r, --repository <repository>                GitHub repository that publishes TRS responses (format: <owner>/<repo>)
    -w, --wes-location <wes-location>
            Location of the WES where the test will be run. If not specified, `sapporo-service` will be started

        --zenodo-community <zenodo-community>    Community set in Zenodo deposition

ARGS:
    <metadata-locations>...    Location of the Yevis metadata files (local file path or remote URL) [default:
                               yevis-metadata.yml]
```

This command is used within GitHub Actions.

Note that the following four options:

- `--from-pr`: Publish from a pull request ID
- `--upload-zenodo`: Upload workflow and dataset to Zenodo
- `--with-test`: Test before publishing

See the GitHub Actions section for more details.

## GitHub Actions

`yevis-cli` uses GitHub Actions for CI/CD.

Two actions are provided as examples:

- [`yevis-test-pr.yml`](https://github.com/sapporo-wes/yevis-cli/blob/main/actions_example/yevis-test-pr.yml): Action to automatically validate and test a pull request
- [`yevis-publish-pr.yml`](https://github.com/sapporo-wes/yevis-cli/blob/main/actions_example/yevis-publish-pr.yml): Action to upload files to Zenodo and generate TRS responses when pull requests are merged
  - `ZENODO_TOKEN` must be set as GitHub Secrets.

Examples of `yevis-cli` commands executed within each action are as follows:

```bash
# yevis-test-pr.yml
$ yevis test \
    --verbose \
    --from-pr ${{github.event.pull_request._links.html.href }}

# yevis-publish-pr.yml
$ yevis publish \
    --verbose \
    --repository ${{ github.repository }} \
    --with-test \
    --from-pr ${{github.event.pull_request._links.html.href }} \
    --upload-zenodo
```

## Development

Launch a development environment using `docker compose`:

```bash
$ docker compose -f docker-compose.dev.yml up -d --build
$ docker compose -f docker-compose.dev.yml exec app bash
# cargo run -- --help
yevis 0.4.0
...
```

### Build binary

Recommendation, build the binary using `musl``:

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

Several test workflows are prepared.
See [tests/README.md](https://github.com/sapporo-wes/yevis-cli/blob/main/tests/README.md).

### Download artifacts from building GitHub Actions

```bash
$ gh run --repo sapporo-wes/yevis-cli list --workflow build_binary --json databaseId --jq .[0].databaseId | xargs -I {} gh run --repo sapporo-wes/yevis-cli download {} -n yevis
```

### Release

Use [`release.sh`](https://github.com/sapporo-wes/yevis-cli/blob/main/release.sh) as follows:

```bash
$ bash release.sh <new_version>
```

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0).
See the [LICENSE](https://github.com/sapporo-wes/yevis-cli/blob/main/LICENSE).
