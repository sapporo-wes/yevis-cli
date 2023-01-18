# Deploying Yevis registry without GitHub/Zenodo dependencies

For the details of the Yevis system, check out our preprint:

> Workflow sharing with automated metadata validation and test execution to improve the reusability of published workflows
Hirotaka Suetake, Tsukasa Fukusato, Takeo Igarashi, Tazro Ohta
bioRxiv 2022.07.08.499265; doi: https://doi.org/10.1101/2022.07.08.499265

## Why?

Yevis system, a toolkit to deploy workflow registry with automated testing function, strongly depends on two public web services, GitHub and Zenodo. While Zenodo is operated by CERN, GitHub is operated by a commercial company (Microsoft) and thus there is a concern for academic products relying on a single commercial platform.

In the design of the Yevis system, we aimed to develop a solution for those who don't have time to develop and manage the system to share their data analysis resources. This is why we decided to use GitHub as a core component of the Yevis system, to retrieve various information about the GitHub repositories efficiently, and to perform automated testing with its powerful CI/CD tool, GitHub actions. By using the GitHub features, Yevis users will not need to spend time configuring the system and setup the servers. More importantly, for free, at least for the time being.

However, at the same time, we recognize the importance of the academic findings and methods should be independent of a specific company's product. In this case, GitHub is a product of Microsoft company, and no one would guarantee how they treat the hosted content in the future.

Therefore, here we provide an alternate version of the Yevis system explained in our paper (see above), without any dependencies on a third-party product.

The source code and this document are hosted at https://data.dbcls.jp/~inutano/yevis/yevis_on_premise.zip . I will do my best to keep this URL and the server alive, but I cannot give a 100% guarantee.

## Prerequisites

### For both the submitter/admin

- Python 3.8 or later
  - `PyYAML`
  - `requests`
- A workflow test environment with Docker installation
- A running WES endpoint
  - try [Sapporo](https://github.com/sapporo-wes/sapporo-service) if you don't have a preference

### For registry admin

- A file server
  - for hosting the TRS responses, main entities of the registry in JSON file

## Step-by-step procedures

There are two actors *Submitter* and *Admin*. A submitter composes a metadata file to register a workflow to the Yevis registry. An admin accepts the workflow submission and hosts the registry.

### 1. Write workflow metadata [Submitter]

Use the `template.metadata.yml` and fill out the information manually.

### 2. Run workflow test locally [Submitter]

Run `test.py` with the `metadata.yml` to run a test locally.

### 3. Submit metadata to the registry admin [Submitter]

Once the test is passed, the submitter is ready to send the `metadata.yml` file to the registry admin, by any protocol.

### 4. Run workflow test on the registry side [Admin]

The registry admin needs to run a test of the workflow by `test.py` with the received YAML file. This ensures the reproducibility of the test result in a different environment.

### 5. Generate TRS response and edit [Admin]

The registry admin needs to run `generate.py` with the YAML file to generate the TRS response JSON file. The admin needs to edit the JSON file by an editor to fill the `<FIX ME>` fields according to the server to publish the file.

### 6. Publish the generated TRS response [Admin]

The main entity of the yevis registry is the generated JSON file. The admin needs to serve the JSON file on a file server so that any workflow runner can browse the TRS response.

### 7. (Optional) Deploy Yevis-web on an on-premise server [Admin]

Follow the guideline to deploy the TRS client (workflow browser), yevis-web, on a file server.

https://github.com/sapporo-wes/yevis-web#deploy-to-other-than-github-pages


## Script usage

### `test.py`

The alternative of `yevis test` but without no YAML validation. The script requires a working WES endpoint to run the workflow testing. For a local implementation, use [Sapporo-service](https://github.com/sapporo-wes/sapporo-service).

- parameters:
  - `-w`: WES endpoint (required)
- arguments:
  - metadata files

```bash=
$ python3 test.py -w http://localhost:1122 ../tests/test-metadata-CWL-validated.yml
Running test
Use WES location: http://localhost:1122 for testing
Test workflow_id: c13b6e27-a4ee-426f-8bdb-8cf5c4310bad, version: 1.0.0
Testing test case: test_1
Waiting for run fc157029-18dd-4c1d-bd40-b1601f176a85 to finish, status: RUNNING
Waiting for run fc157029-18dd-4c1d-bd40-b1601f176a85 to finish, status: RUNNING
Waiting for run fc157029-18dd-4c1d-bd40-b1601f176a85 to finish, status: COMPLETE
Complete test case: test_1
```

### `generate.py`

This script generates a TRS response as the `yevis publish` command does, while the script does not perform YAML validation, workflow testing, and submission of the files to the GitHub/Zenodo repositories.

The generated JSON file requires to be edited by the admin to fill the `<FIX ME>` fields according to the server to publish the file.

- parameters:
  - `-o`: output directory (required)
- arguments:
  - metadata files

```bash=
$ python3 generate.py -o ./trs_res ../tests/test-metadata-CWL-validated.yml
$ tree trs_res/
trs_res/
├── service-info
│   └── index.json
├── toolClasses
│   └── index.json
└── tools
    ├── c13b6e27-a4ee-426f-8bdb-8cf5c4310bad
    │   ├── index.json
    │   └── versions
    │       ├── 1.0.0
    │       │   ├── containerfile
    │       │   │   └── index.json
    │       │   ├── CWL
    │       │   │   ├── descriptor
    │       │   │   │   └── index.json
    │       │   │   ├── files
    │       │   │   │   └── index.json
    │       │   │   └── tests
    │       │   │       └── index.json
    │       │   └── index.json
    │       └── index.json
    └── index.json

11 directories, 10 files
```

## Run scripts via Docker

Launch Sapporo WES instance locally:

```
$ git clone https://github.com/sapporo-wes/sapporo-service
$ cd sapporo-service
$ docker compose up -d
```

Build the yevis-no-gh Docker image and run:

```
$ docker build -t yevis-no-gh .
$ docker run --network="host" -it --rm -v $(pwd):/work -w /work yevis-no-gh python3 test.py -w http://127.0.0.1:1122 test.metadata.yml
Running test
Use WES location: http://127.0.0.1:1122 for testing
Test workflow_id: c13b6e27-a4ee-426f-8bdb-8cf5c4310bad, version: 1.0.0
Testing test case: test_1
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: RUNNING
Waiting for run 26cee039-7a37-4647-924f-ee2187304ef3 to finish, status: COMPLETE
Complete test case: test_1
$ docker run -it --rm -v $(pwd):/work -w /work yevis-no-gh python3 generate.py -o trs_res test.metadata.yml
$ tree trs_res
trs_res
├── service-info
│   └── index.json
├── toolClasses
│   └── index.json
└── tools
    ├── c13b6e27-a4ee-426f-8bdb-8cf5c4310bad
    │   ├── index.json
    │   └── versions
    │       ├── 1.0.0
    │       │   ├── CWL
    │       │   │   ├── descriptor
    │       │   │   │   └── index.json
    │       │   │   ├── files
    │       │   │   │   └── index.json
    │       │   │   └── tests
    │       │   │       └── index.json
    │       │   ├── containerfile
    │       │   │   └── index.json
    │       │   └── index.json
    │       └── index.json
    └── index.json

11 directories, 10 files
```
