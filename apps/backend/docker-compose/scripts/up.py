#!/usr/bin/env python3
"""Check deployment prerequisites, then start the selected Compose services."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
from preflight import IMAGE_WORKLOADS, ROOT, read_env, run, write_env


def local_tags(values):
    return {key: f"flow-like-{workload}:local" for key, workload in IMAGE_WORKLOADS.items() if not values.get(key)}


def compose_command(values, env_file, build):
    pinned = [key for key in IMAGE_WORKLOADS if "@" in values.get(key, "")]
    if build and pinned:
        raise ValueError(f"--build cannot rebuild digest-pinned images ({', '.join(pinned)}); run scripts/prepare-images.py for local runtime images or clear the pins first")
    return ["docker", "compose", "--env-file", str(env_file), "up", "-d", "--build" if build else "--no-build"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--env-file", type=Path, default=ROOT / ".env")
    parser.add_argument("--build", action="store_true", help="Build images locally instead of pulling published ones")
    args = parser.parse_args()
    args.env_file = args.env_file.absolute()
    args.config_only = False
    try:
        values = read_env(args.env_file)
        command = compose_command(values, args.env_file, args.build)
        assigned = local_tags(values) if args.build else {}
        if assigned:
            write_env(args.env_file, assigned)
            values.update(assigned)
            print(f"Set {', '.join(assigned)} to local tags so the build does not shadow the published images.")
        run(args)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"Startup refused: {error}", file=sys.stderr)
        return 1
    env = os.environ.copy()
    for key in values:
        env.pop(key, None)
    subprocess.run(command, cwd=ROOT, env=env, check=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
