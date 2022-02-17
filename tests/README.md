# tests

## make-template

```bash
# CWL
$ cargo run -- make-template https://github.com/ddbj/yevis-cli/blob/main/tests/CWL/wf/trimming_and_qc.cwl \
    --output ./tests/test_config_CWL.yml

# WDL
$ cargo run -- make-template https://github.com/ddbj/yevis-cli/blob/main/tests/WDL/wf/dockstore-tool-bamstats.wdl \
   --output ./tests/test_config_WDL.yml

# NFL
$ cargo run -- make-template https://github.com/ddbj/yevis-cli/blob/main/tests/NFL/wf/file_input.nf \
    --output ./tests/test_config_NFL.yml

# SMK
$ cargo run -- make-template https://github.com/ddbj/yevis-cli/blob/main/tests/SMK/wf/Snakefile \
    --output ./tests/test_config_SMK.yml
```

## validate

```bash
# All
$ cargo run -- validate ./tests/test_config_*

# CWL
$ cargo run -- validate ./tests/test_config_CWL.yml

# WDL
$ cargo run -- validate ./tests/test_config_WDL.yml

# NFL
$ cargo run -- validate ./tests/test_config_NFL.yml

# SMK
$ cargo run -- validate ./tests/test_config_SMK.yml
```

## test

```bash
# All
$ cargo run -- test ./tests/test_config_*

# CWL
$ cargo run -- test ./tests/test_config_CWL.yml

# WDL
$ cargo run -- test ./tests/test_config_WDL.yml

# NFL
$ cargo run -- test ./tests/test_config_NFL.yml

# SMK
$ cargo run -- test ./tests/test_config_SMK.yml
```

## pull-request

```bash
# CWL
$ cargo run -- pull-request ./tests/test_config_CWL.yml

# WDL
$ cargo run -- pull-request ./tests/test_config_WDL.yml

# NFL
$ cargo run -- pull-request ./tests/test_config_NFL.yml

# SMK
$ cargo run -- pull-request ./tests/test_config_SMK.yml
```
