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

```bash
yevis --make-template https://github.com/path/to/workflow-file
```

Once you have generated the `GitHub Personal Access Token`, you need to pass `yevis` it in one of the following ways:

- env file: write the token to `.env` file like `GITHUB_TOKEN=<paste_your_token>`
- environment variable: set the `GITHUB_TOKEN` environment variable
- command line option: use `--github-token <paste_your_token>` option

## Usage

TODO write usage.

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

Several test workflows are available. Check [tests/README.md](https://github.com/ddbj/yevis-cli/blob/main/tests/README.md).

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0). See the [LICENSE](https://github.com/ddbj/yevis-cli/blob/main/LICENSE).
