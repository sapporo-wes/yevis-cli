# yevis-cli

CLI tool for registering workflows to [GitHub - ddbj/yevis-workflows](https://github.com/ddbj/yevis-workflows).

As features:

- Generating templates for registration files (called `config_file`)
- Validating registration files
- Testing workflows based on registration files
- Creating the Pull Request to [GitHub - ddbj/yevis-workflows](https://github.com/ddbj/yevis-workflows)
- Generating DOIs with [Zenodo](https://zenodo.org/)
- Generating TRS responses ([GA4GH - Tool Registry Service API](https://www.ga4gh.org/news/tool-registry-service-api-enabling-an-interoperable-library-of-genomics-analysis-tools/))

## Installation

**As a dependency, the `yevis` uses Docker to run tests.**

Use a single binary that is built without any dependencies:

```bash
$ curl -fsSL -O https://github.com/ddbj/yevis-cli/releases/latest/download/yevis
$ chmod +x ./yevis
$ ./yevis --help
```

Or, use Docker environment (also `docker-compose`):

```bash
$ docker-compose up -d --build
$ docker-compose exec app yevis --help
```

## Getting started

First, the `yevis` needs the `GitHub Personal Access Token` for various operations through GitHub REST API.
Please refer to [GitHub Docs - Creating a personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token) for how to generate the `GitHub Personal Access Token`.

The required scopes are as follows (also see ScreenShot):

- `repo - public_repo`
- `user - read:user`

<img src="https://user-images.githubusercontent.com/26019402/149902689-bfd4707d-9792-41fd-b22f-8a1631489399.png" alt="yevis-cli-img-1" width="600">

Once you have generated the `GitHub Personal Access Token`, you need to pass the `yevis` it in one of the following ways:

- env file: write the token to `.env` file like `GITHUB_TOKEN=<paste_your_token>`
- environment variable: set the `GITHUB_TOKEN` environment variable
- command-line option: use `--github-token <paste_your_token>` option

---

Use the workflow [`trimming_and_qc.cwl`](https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl) as an example.

First, generate a template of the configuration file from the GitHub location of the primary workflow file by:

```bash
$ yevis make-template https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl
```

Edit the generated `./yevis_config.yml` as [`test_config_CWL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_CWL.yml).

The main part to edit is below:

- `workflow.files`: the list of files to be included in the registration file
- `workflow.testing`: the list of tests to be run

After that, validate the config file, run a test, and create a pull request by:

```bash
$ yevis pull-request ./yevis_config.yml
...
Creating pull request to ddbj/yevis-workflows
Pull request URL: https://api.github.com/repos/ddbj/yevis-workflows/pulls/1
Finished pull-request
```

The workflow will be submitted as a pull request and checked by the administrator.

## Usage

This section describes some of the subcommands.

```bash
$ yevis --help
yevis 0.1.0
DDBJ(DNA Data Bank of Japan)

USAGE:
    yevis <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help             Prints this message or the help of the given subcommand(s)
    make-template    Generates a configuration file template for yevis from a workflow document
    pull-request     After validating and testing, create a pull request to `ddbj/yevis-workflows`
    test             Test the workflow based on the configuration file
    validate         Validate the schema and contents of the configuration file
```

### make-template

Generate a template of the configuration file from the GitHub location of the primary workflow file.

```bash
$ yevis make-template --help
yevis-make-template 0.1.0
Generates a configuration file template for yevis from a workflow document

USAGE:
    yevis make-template [FLAGS] [OPTIONS] <workflow-location>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -f, --format <format>                Format of the output file (`yaml` or `json`) [default: yaml]
    -g, --github-token <github-token>    GitHub Personal Access Token
    -o, --output <output>                Path to the output file [default: yevis_config.yml]
    -r, --repository <repository>        GitHub repository to send pull requests to [default: ddbj/yevis-workflows]
    -u, --update <update>                Update existing workflow. Please provide the workflow ID.

ARGS:
    <workflow-location>     Remote location of the workflow's main document file (only hosted on GitHub).
```

Only URLs hosted on GitHub are accepted for the `workflow-location`.
This URL is a URL like `https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl` and will be converted to a raw URL like `https://raw.githubusercontent.com/ddbj/yevis-cli/645a193826bdb3f0731421d4ff1468d0736b4a06/tests/CWL/wf/trimming_and_qc.cwl` later.

The `yevis` collects various information and generates a template for the config file.
In particular, `workflow.files` will be generated a file list from the primary workflow location recursively.

Using the `--update` option to update a workflow that has already been published.
Specifically, run `--update <workflow_id (UUID)>`, generate a template with the same ID.

### validate

Validate the schema and contents of the configuration file.

```bash
$ yevis validate --help
yevis-validate 0.1.0
Validate the schema and contents of the configuration file

USAGE:
    yevis validate [FLAGS] [OPTIONS] [config-file]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -g, --github-token <github-token>    GitHub Personal Access Token
    -r, --repository <repository>        GitHub repository to send pull requests to [default: ddbj/yevis-workflows]

ARGS:
    <config-file>    Configuration file generated by `make-template` command [default: yevis_config.yml]
```

An explanation of the validation rules for some fields in the config file:

- `id`: ID of the workflow generated by the `make-template` command. Do not change this value.
- `version`: Version in the form `x.y.z`.
- `license`: LICENSE is `CC0-1.0` only. This is because the files will be uploaded to Zenodo later.
- `authors`: Please add information for Zenodo; do not delete the ddbj account.
- `workflow.name`: Give it you like.
- `workflow.repo`: Do not change this value.
- `workflow.readme`: It is used to describe the workflow. Use any URL you like.
- `workflow.language`: `CWL`, `WDL`, `NFL`, and `SMK` are supported.
- `workflow.files`: The list of files. Files specified as `type: secondary` will be placed in the execution directory with `target` as the path at workflow execution time.
- `workflow.testing`: The list of tests. Please refer to `test` for how to write tests.

Several example are prepared. Please check:

- [`test_config_CWL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_CWL.yml)
- [`test_config_WDL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_WDL.yml)
- [`test_config_NFL.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_NFL.yml)
- [`test_config_SMK.yml`](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_SMK.yml)

### test

Test the workflow based on the configuration file.

```bash
$ yevis test --help
yevis-test 0.1.0
Test the workflow based on the configuration file

USAGE:
    yevis test [FLAGS] [OPTIONS] [config-file]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -d, --docker-host <docker-host>      Location of the docker host [default: unix:///var/run/docker.sock]
    -g, --github-token <github-token>    GitHub Personal Access Token
    -r, --repository <repository>        GitHub repository to send pull requests to [default: ddbj/yevis-workflows]
    -w, --wes-location <wes-location>    Location of WES in which to run the test. If not specified, `sapporo-service`
                                         will be started

ARGS:
    <config-file>    Configuration file generated by `make-template` command [default: yevis_config.yml]
```

The test is run using the Workflow Execution Service (WES; [GA4GH - WES API](https://www.ga4gh.org/news/ga4gh-wes-api-enables-portable-genomic-analysis/).
In particular, the `yevis` use [`sapporo-service`](https://github.com/sapporo-wes/sapporo-service) as the WES.
If the option `--wes-location` is not specified, `sapporo-service` will be stated using the default `DOCKER_HOST`.

An example of the `workflow.testing` field in the config file is shown below:

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

There are three types of file types:

- `wf_params`: The parameters for the workflow.
- `wf_engine_params`: The parameters for the workflow engine.
- `other`: Other files.

Files specified as `wf_params` and `wf_engine_params` are placed as WES execution parameters at WES runtime.
Also, `other` files will be placed in the execution directory with `target` as the path at workflow execution time.

You can freely specify the `id` field.

### pull-request

```bash
$ yevis pull-request --help
yevis-pull-request 0.1.0
After validating and testing, create a pull request to `ddbj/yevis-workflows`

USAGE:
    yevis pull-request [FLAGS] [OPTIONS] <config-file>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Verbose mode

OPTIONS:
    -d, --docker-host <docker-host>      Location of the docker host [default: unix:///var/run/docker.sock]
    -g, --github-token <github-token>    GitHub Personal Access Token
    -r, --repository <repository>        GitHub repository to send pull requests to [default: ddbj/yevis-workflows]
    -w, --wes-location <wes-location>    Location of WES in which to run the test. If not specified, `sapporo-service`
                                         will be started

ARGS:
    <config-file>    Configuration file generated by `make-template` command
```

The pull request will be created from the forked repository.
The typical flow when this command is executed is as follows:

1. Fork `ddbj/yevis-workflows` repository to your GitHub account
2. Create a new branch (named `workflow_id`) for the new workflow
3. Commit the config file to the new branch.
4. Create a new pull request to the `ddbj/yevis-workflows` repository

## Development

Launching the development environment using `docker-compose`:

```bash
$ docker-compose -f docker-compose.dev.yml up -d --build
$ docker-compose -f docker-compose.dev.yml exec app bash
# cargo run -- --help
yevis 0.1.0
...
```

If you set the environment variable `YEVIS_DEV=1`, the pull request will be created in the dev environment [`GitHub - ddbj/yevis-workflows-dev`](https://github.com/yevis/yevis-workflows-dev).

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

Several test workflows are prepared. Check [tests/README.md](https://github.com/ddbj/yevis-cli/blob/main/tests/README.md).

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0). See the [LICENSE](https://github.com/ddbj/yevis-cli/blob/main/LICENSE).
