# yevis-cli

CLI tool for registering workflows to [GitHub - ddbj/yevis-workflows](https://github.com/ddbj/yevis-workflows).

As features:

- Generating templates for registration files (called `config_file`)
- Validating registration files
- Testing workflows based on registration files
- Creating the Pull Request to [GitHub - ddbj/yevis-workflows](https://github.com/ddbj/yevis-workflows)
- Generating DOIs with [Zenodo](https://zenodo.org/)
- Generating TRS responses ([GA4GH - Tool Registry Service API)[https://www.ga4gh.org/news/tool-registry-service-api-enabling-an-interoperable-library-of-genomics-analysis-tools/])

## Installation

**As a dependency, `yevis` uses Docker to run tests.**

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

First of all, `yevis` needs the `GitHub Personal Access Token` for various operations through GitHub REST API.
Please refer to [GitHub Docs - Creating a personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token) for how to generate the `GitHub Personal Access Token`.

The required scopes are as follows (also see ScreenShot):

- `repo - public_repo`
- `user - read:user`

![yevis-cli-img-1](https://user-images.githubusercontent.com/26019402/149902689-bfd4707d-9792-41fd-b22f-8a1631489399.png)

Once you have generated the `GitHub Personal Access Token`, you need to pass `yevis` it in one of the following ways:

- env file: write the token to `.env` file like `GITHUB_TOKEN=<paste_your_token>`
- environment variable: set the `GITHUB_TOKEN` environment variable
- command line option: use `--github-token <paste_your_token>` option

---

Use the workflow [trimming_and_qc.cwl](https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl) as an example.

First, generate a template of configuration file from the GitHub location of the primary workflow file by:

```bash
$ yevis make-template https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl
```

Edit the generated `./yevis_config.yml` as [test_config_CWL.yml](https://github.com/ddbj/yevis-cli/blob/main/tests/test_config_CWL.yml).

The main part to edit is bellow:

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

The workflow will be submitted as a pull request and will be checked by the administrator.

## Usage

This section describes some of the sub commands.

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
    test             Actually, test the workflow based on the configuration file
    validate         Validate the schema and contents of the configuration file
```

### `make-template`

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
    -u, --update <update>                Update existing workflow

ARGS:
    <workflow-location>
```

### `validate`

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

### `test`

```bash
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

### `pull-request`

```bash
yevis-test 0.1.0
Actually, test the workflow based on the configuration file

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

## Development

Launching the development environment using `docker-compose`:

```bash
$ docker-compose -f docker-compose.dev.yml up -d --build
$ docker-compose -f docker-compose.dev.yml exec app bash
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
