#!/usr/bin/env python3
"""Build and pin the local runner/helper images without changing deployment secrets."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from preflight import ROOT, SANDBOX_SOURCES, ensure_private, read_env, write_env

LOCAL_TAGS = {"RUNTIME_IMAGE": "flow-like-runtime:local", "EXECUTION_MANAGER_IMAGE": "flow-like-execution-manager:local"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--env-file", type=Path, default=ROOT / ".env")
    args = parser.parse_args()
    path = args.env_file.absolute()
    try:
        ensure_private(path)
        values = read_env(path)
    except (ValueError, OSError) as error:
        parser.error(str(error))
    local = {key: tag for key, tag in LOCAL_TAGS.items() if not values.get(key) or "@" in values[key]}
    if local:
        write_env(path, local)
        values.update(local)
        print(f"Set {', '.join(local)} to local build tags; published digests no longer apply to these services.")
    env = os.environ.copy()
    for key in values:
        env.pop(key, None)
    compose = ["docker", "compose", "--env-file", str(path)]
    subprocess.run(compose + ["build", "runtime", "execution-manager"], cwd=ROOT, env=env, check=True)
    pins = {}
    for key, image_key in SANDBOX_SOURCES.items():
        result = subprocess.run(["docker", "image", "inspect", "--format", "{{json .Id}}", values[image_key]], capture_output=True, text=True, check=True)
        digest = json.loads(result.stdout)
        if not re.fullmatch(r"sha256:[a-f0-9]{64}", digest):
            raise RuntimeError("Docker did not return an immutable image ID")
        pins[key] = digest
    write_env(path, pins)
    print("Pinned the locally built runner and gateway images. Deployment secrets were preserved.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
