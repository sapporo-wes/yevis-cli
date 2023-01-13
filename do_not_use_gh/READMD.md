# Scripts for testing workflow and generating TRS responses without GitHub / Zenodo

## Why?

Yevis system, a toolkit to deploy workflow registry with automated testing function, strongly depends on two public web services, GitHub and Zenodo. While Zenodo is operated by CERN, GitHub is operated by a commercial company (Microsoft) and thus there is a concern for academic products relying on a single commercial platform.

In the design of the Yevis system, we aimed to develop a solution for those who don't have time to develop and manage the system to share their data analysis resources. This is why we decided to use GitHub as a core component of the Yevis system, to retrieve various information about the GitHub repositories efficiently, and to perform automated testing with its powerful CI/CD tool, GitHub actions. By using the GitHub features, Yevis users will not need to spend time configuring the system and setup the servers. More importantly, for free, at least for the time being.

However, at the same time, we recognize the importance of the academic findings and methods should be independent of a specific company's product. In this case, GitHub is a product of Microsoft company, and no one would guarantee how they treat the hosted content in the future.

Therefore, here we provide some scripts without any dependencies on a third-party product. But we cannot give a 100% guarantee. Sadly, this is one downside of a perfect-commercial-free solution.

## Requirements

Python 3.8 or later.

External libraries:

- `PyYAML`
- `requests`

## `test.py`

Same as the `yevis test` command. However, unlike that command, no prior validation (`yevis validate`) is performed. The WES system must be running before running this script. Please refer to the [GitHub - sapporo-wes/sapporo-service](https://github.com/sapporo-wes/sapporo-service) repository for the WES service.

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

## `generate.py`

Same as the `yevis publish` command, it generates a TRS response. However, unlike that command, no prior validation (`yevis validate`) and testing (`yevis test`) is performed. In addition, Zenodo uploads and deployments to GitHub Pages are also not performed.

The admin need to edit the JSON file by an editor to fill the `<FIX ME>` fields according to the server to publish the file.

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
