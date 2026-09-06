#!/usr/bin/env python3
"""Select portable container targets and validate complete GHCR build manifests."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys


SCHEMA_VERSION = 1
CLOUDS = ("all", "aws", "gcp", "azure", "docker-compose", "kubernetes", "self-hosted")
REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SELF_HOSTED_CLOUDS = ("docker-compose", "kubernetes")
KUBERNETES_SHARED_WORKLOADS = {"runtime", "compiler", "signaling", "object-store-init"}


def target(cloud, workload, platform="linux/amd64", recipe=None, context=".", architecture_id=False):
    target_id = f"{cloud}-{workload}"
    if architecture_id:
        target_id += f"-{platform.split('/')[1]}"
    return {
        "id": target_id,
        "cloud": cloud,
        "workload": workload,
        "dockerfile": recipe or f"apps/backend/{cloud}/{workload}/Dockerfile",
        "context": context,
        "platform": platform,
        "runner": "ubuntu-24.04-arm" if platform == "linux/arm64" else "ubuntu-24.04",
        "image_suffix": f"flow-like-{cloud}-{workload}",
    }


# APIs embed reviewed defaults and accept a full configuration at startup.
# Publication records identify the fallback separately from the runtime contract.
TARGETS = tuple(sorted([
    target("aws", "api", "linux/arm64"),
    target("aws", "executor"),
    target("aws", "executor-async", recipe="apps/backend/aws/executor-ecs/Dockerfile"),
    target("aws", "compiler", "linux/arm64", recipe="apps/backend/aws/compiler-ecs/Dockerfile"),
    target("aws", "file-tracker", "linux/arm64"),
    target("aws", "media-transformer", "linux/arm64"),
    target("aws", "event-bridge", "linux/arm64"),
    target("aws", "maintenance", "linux/arm64"),
    target("aws", "signaling", "linux/arm64", recipe="apps/backend/docker-compose/signaling/Dockerfile"),
    target("aws", "migration", "linux/arm64"),
    *[target("gcp", workload) for workload in (
        "api", "queue-worker", "executor", "signaling", "migration", "scheduler", "maintenance",
    )],
    *[target("azure", workload) for workload in (
        "api", "queue-worker", "executor", "maintenance", "scheduler", "migration", "signaling",
    )],
    target("azure", "otel-collector"),
    *[target("docker-compose", workload, f"linux/{architecture}", architecture_id=True,
             recipe="apps/backend/docker-compose/object-store/Dockerfile" if workload == "object-store-init" else None)
      for workload in ("api", "runtime", "compiler", "execution-manager", "sink-services", "signaling", "db-init", "object-store-init", "web")
      for architecture in ("amd64", "arm64")],
    *[target("kubernetes", workload, f"linux/{architecture}", architecture_id=True)
      for workload in ("api", "executor", "execution-manager", "sink-trigger", "migration", "web")
      for architecture in ("amd64", "arm64")],
], key=lambda entry: entry["id"]))
TARGET_BY_ID = {entry["id"]: entry for entry in TARGETS}


def matrix(cloud):
    if cloud not in CLOUDS:
        raise ValueError("unsupported cloud")
    def selected(entry):
        if cloud == "all":
            return True
        if cloud == "self-hosted":
            return entry["cloud"] in SELF_HOSTED_CLOUDS
        # The Helm chart uses the same Compose packages for these services.
        if cloud == "kubernetes" and entry["cloud"] == "docker-compose":
            return entry["workload"] in KUBERNETES_SHARED_WORKLOADS
        return cloud == entry["cloud"]
    return {"include": [dict(entry) for entry in TARGETS if selected(entry)]}


def build_identity(owner, source_sha, run_id, run_attempt):
    owner = owner.lower()
    if not re.fullmatch(r"[a-z0-9](?:[a-z0-9-]{0,37}[a-z0-9])?", owner):
        raise ValueError("owner must be a GitHub user or organization name")
    if not re.fullmatch(r"[0-9a-f]{40}", source_sha) or source_sha == "0" * 40:
        raise ValueError("source SHA must be a nonzero 40-character lowercase Git commit SHA")
    for name, value in (("run ID", run_id), ("run attempt", run_attempt)):
        if not re.fullmatch(r"[1-9][0-9]*", value):
            raise ValueError(f"{name} must be a positive decimal integer")
    return owner, {"run_id": run_id, "run_attempt": run_attempt}


def record(target_id, owner, source_sha, digest, run_id, run_attempt):
    owner, build = build_identity(owner, source_sha, run_id, run_attempt)
    if target_id not in TARGET_BY_ID:
        raise ValueError("unknown container target")
    if not isinstance(digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        raise ValueError("digest must be a lowercase SHA-256 image digest")
    entry = TARGET_BY_ID[target_id]
    repository = f"ghcr.io/{owner}/{entry['image_suffix']}"
    architecture = entry["platform"].split("/")[1]
    build_inputs = {
        "dockerfile_sha256": hashlib.sha256((REPOSITORY_ROOT / entry["dockerfile"]).read_bytes()).hexdigest(),
    }
    if entry["workload"] == "api":
        build_inputs["runtime_config"] = "full-document-v1"
    if entry["workload"] == "api" and entry["cloud"] not in SELF_HOSTED_CLOUDS:
        build_inputs["flow_like_config_sha256"] = hashlib.sha256((REPOSITORY_ROOT / "flow-like.config.json").read_bytes()).hexdigest()
        build_inputs["variant"] = "runtime-config-public-default"
    if entry["cloud"] in SELF_HOSTED_CLOUDS and entry["workload"] == "api":
        config = REPOSITORY_ROOT / f"apps/backend/{entry['cloud']}/flow-like.config.example.json"
        build_inputs["flow_like_config_sha256"] = hashlib.sha256(config.read_bytes()).hexdigest()
        build_inputs["variant"] = "runtime-config-self-hosted-default"
    if entry["cloud"] in SELF_HOSTED_CLOUDS and entry["workload"] == "web":
        build_inputs["variant"] = "runtime-public-config"
    if target_id in ("aws-api", "aws-file-tracker"):
        build_inputs["database_tls"] = "dsql-no-custom-ca"
    return {
        "schema_version": SCHEMA_VERSION,
        "id": entry["id"],
        "cloud": entry["cloud"],
        "workload": entry["workload"],
        "dockerfile": entry["dockerfile"],
        "context": entry["context"],
        "platform": entry["platform"],
        "source_sha": source_sha,
        "build": build,
        "build_inputs": build_inputs,
        "repository": repository,
        "tag": f"sha-{source_sha}-{architecture}-run-{run_id}-{run_attempt}",
        "digest": digest,
        "image": f"{repository}@{digest}",
    }


def manifest(cloud, owner, source_sha, run_id, run_attempt, records):
    owner, build = build_identity(owner, source_sha, run_id, run_attempt)
    expected_ids = {entry["id"] for entry in matrix(cloud)["include"]}
    images = {}
    for entry in records:
        if not isinstance(entry, dict) or not isinstance(entry.get("id"), str):
            raise ValueError("image record must be an object with a target ID")
        target_id = entry["id"]
        if target_id not in expected_ids:
            raise ValueError("image record has an unexpected target ID")
        if target_id in images:
            raise ValueError(f"duplicate image record for {target_id}")
        expected = record(target_id, owner, source_sha, entry.get("digest"), run_id, run_attempt)
        # Reject extra fields too: release output contains only this reviewed
        # schema, and cannot silently carry arbitrary artifact contents.
        if json.dumps(entry, sort_keys=True) != json.dumps(expected, sort_keys=True):
            fields = sorted(set(entry) ^ set(expected) | {
                key for key in expected if key in entry
                and json.dumps(entry[key], sort_keys=True) != json.dumps(expected[key], sort_keys=True)
            })
            raise ValueError(f"image record for {target_id} does not match the release: {', '.join(fields)}")
        images[target_id] = expected
    missing = sorted(expected_ids - images.keys())
    if missing:
        raise ValueError(f"missing image records: {', '.join(missing)}")
    return {
        "schema_version": SCHEMA_VERSION,
        "cloud": cloud,
        "registry": "ghcr.io",
        "owner": owner,
        "source_sha": source_sha,
        "build": build,
        "images": [images[target_id] for target_id in sorted(images)],
    }


def read_records(directory):
    if not directory.is_dir():
        raise ValueError("records directory does not exist")
    records = []
    for path in sorted(directory.rglob("*.json")):
        try:
            records.append(json.loads(path.read_text(encoding="utf-8")))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise ValueError(f"cannot read image record {path.name}") from error
    return records


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    matrix_parser = commands.add_parser("matrix", help="print the GitHub Actions build matrix")
    matrix_parser.add_argument("--cloud", choices=CLOUDS, default="all")
    record_parser = commands.add_parser("record", help="record one successfully pushed image digest")
    record_parser.add_argument("--target", choices=sorted(TARGET_BY_ID), required=True)
    record_parser.add_argument("--digest", required=True)
    manifest_parser = commands.add_parser("manifest", help="validate and merge a complete set of image records")
    manifest_parser.add_argument("--cloud", choices=CLOUDS, default="all")
    manifest_parser.add_argument("--records-dir", type=Path, required=True)
    for command in (record_parser, manifest_parser):
        command.add_argument("--owner", required=True)
        command.add_argument("--source-sha", required=True)
        command.add_argument("--run-id", required=True)
        command.add_argument("--run-attempt", required=True)
        command.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "matrix":
            result = matrix(args.cloud)
        elif args.command == "record":
            result = record(args.target, args.owner, args.source_sha, args.digest, args.run_id, args.run_attempt)
        else:
            result = manifest(args.cloud, args.owner, args.source_sha, args.run_id, args.run_attempt, read_records(args.records_dir))
        output = json.dumps(result, sort_keys=True, separators=(",", ":")) + "\n"
        if getattr(args, "output", None):
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(output, encoding="utf-8")
        else:
            print(output, end="")
    except (OSError, ValueError) as error:
        print(f"container images: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
