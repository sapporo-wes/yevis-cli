#!/usr/bin/env python3
# coding: utf-8

"""\
python3 test.py -w http://localhost:1122 ../tests/test-metadata-CWL-validated.yml
"""

import argparse
import json
from time import sleep
from typing import Any

import requests
import yaml


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("-w", required=True, help="WES location where the test will be run.")
    parser.add_argument("metadata_locations", nargs="+", help="Location of the Yevis metadata files (only local file)")
    args = parser.parse_args()
    return args


def get_service_info(wes_loc: str) -> None:
    res = requests.get(f"{wes_loc}/service-info")
    if res.status_code != 200:
        raise RuntimeError(f"Failed to get service-info: {res.status_code} {res.text}")


TYPE_ENGINE = {
    "CWL": "cwltool",
    "WDL": "cromwell",
    "Nextflow": "nextflow",
    "Snakemake": "snakemake",
}


def fetch_raw_content(url: str) -> str:
    res = requests.get(url)
    if res.status_code != 200:
        raise RuntimeError(f"Failed to get {url}: {res.status_code} {res.text}")
    return res.text


def wf_params(test_case: Any) -> str:
    for f in test_case["files"]:
        if f["type"] == "wf_params":
            return fetch_raw_content(f["url"])
    return "{}"


def wf_engine_params(test_case: Any) -> str:
    for f in test_case["files"]:
        if f["type"] == "wf_engine_params":
            return fetch_raw_content(f["url"])
    return "{}"


def wf_attachment(meta: Any, test_case: Any) -> str:
    attachment = []
    for f in meta["workflow"]["files"]:
        if f["type"] == "primary":
            if meta["workflow"]["language"]["type"] == "Nextflow":
                attachment.append({"file_name": f["target"], "file_url": f["url"]})
        elif f["type"] == "secondary":
            attachment.append({"file_name": f["target"], "file_url": f["url"]})
    for f in test_case["files"]:
        if f["type"] == "other":
            attachment.append({"file_name": f["target"], "file_url": f["url"]})
    return json.dumps(attachment)


def post_runs(wes_loc: str, meta: Any, test_case: Any) -> str:
    data = {
        "workflow_type": meta["workflow"]["language"]["type"],
        "workflow_type_version": meta["workflow"]["language"]["version"],
        "workflow_url": [f for f in meta["workflow"]["files"] if f["type"] == "primary"][0]["url"],
        "workflow_engine_name": TYPE_ENGINE[meta["workflow"]["language"]["type"]],
        "workflow_params": wf_params(test_case),
        "workflow_engine_parameters": wf_engine_params(test_case),
        "workflow_attachment": wf_attachment(meta, test_case),
        "yevis_metadata": json.dumps(meta),
    }
    r = requests.post(f"{wes_loc}/runs", data=data)
    if r.status_code != 200:
        raise RuntimeError(f"Failed to post runs: {r.status_code} {r.text}")
    return str(r.json()["run_id"])


def get_run_status(wes_loc: str, run_id: str) -> str:
    r = requests.get(f"{wes_loc}/runs/{run_id}/status")
    if r.status_code != 200:
        raise RuntimeError(f"Failed to get run status: {r.status_code} {r.text}")
    return str(r.json()["state"])


running = ["QUEUED", "INITIALIZING", "RUNNING", "PAUSED"]
failed = ["EXECUTOR_ERROR", "SYSTEM_ERROR", "CANCELED", "CANCELING", "UNKNOWN"]
complete = ["COMPLETE"]


def our_sleep(iter_num: int) -> None:
    if iter_num < 6:
        sleep(10)
    elif iter_num < 15:
        sleep(30)
    elif iter_num < 69:
        sleep(60)
    else:
        sleep(120)


def main() -> None:
    args = parse_args()
    wes_loc = args.w
    metadata_locs = args.metadata_locations
    meta_vec = []
    for metadata_loc in metadata_locs:
        with open(metadata_loc, "r") as f:
            metadata = yaml.safe_load(f)
        meta_vec.append(metadata)

    print("Running test")
    get_service_info(wes_loc)
    print(f"Use WES location: {wes_loc} for testing")
    for meta in meta_vec:
        print(f"Test workflow_id: {meta['id']}, version: {meta['version']}")
        for test_case in meta["workflow"]["testing"]:
            print(f"Testing test case: {test_case['id']}")
            run_id = post_runs(wes_loc, meta, test_case)
            status = get_run_status(wes_loc, run_id)
            iter_num = 0
            while status in running:
                our_sleep(iter_num)
                status = get_run_status(wes_loc, run_id)
                print(f"Waiting for run {run_id} to finish, status: {status}")
                iter_num += 1
            if status in complete:
                print(f"Complete test case: {test_case['id']}")
            elif status in failed:
                print(f"Failed test case: {test_case['id']}")


if __name__ == "__main__":
    main()
