#!/usr/bin/env python3
"""Delete superseded Actions cache generations so live archives keep their quota.

rust-cache restores by a restore-key prefix and GitHub returns only the newest
match, so every older generation of a prefix is dead weight until the seven-day
sweep removes it. Pruning them immediately keeps peak residency near the size of
the working set instead of a week of accumulated pushes.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timedelta, timezone


# Generational suffix appended by rust-cache (lockfile hash) and by the container
# Cargo cache (commit sha). Content-addressed BuildKit blobs and scope indexes do
# not match, so they are never grouped or pruned here.
GENERATION = re.compile(r"^(?P<prefix>.+)-(?P<generation>[0-9a-f]{8,40})$")
IN_FLIGHT = timedelta(hours=2)


def parse_time(value):
    if not value:
        return None
    return datetime.fromisoformat(value.replace("Z", "+00:00").replace(" ", "T"))


def plan_deletions(entries, now, keep=1, in_flight=IN_FLIGHT):
    """Return the entries to delete, newest generation per (ref, prefix) retained.

    An entry touched within `in_flight` is always kept: a job may still be
    restoring it, and deleting it mid-run turns a hit into a silent miss.
    """
    groups = {}
    for entry in entries:
        match = GENERATION.match(entry["key"])
        if not match:
            continue
        groups.setdefault((entry["ref"], match.group("prefix")), []).append(entry)

    doomed = []
    for (_, _), members in sorted(groups.items()):
        members.sort(key=lambda item: (parse_time(item["last_accessed_at"]) or parse_time(item["created_at"])),
                     reverse=True)
        for entry in members[keep:]:
            touched = parse_time(entry["last_accessed_at"]) or parse_time(entry["created_at"])
            if touched and now - touched < in_flight:
                continue
            doomed.append(entry)
    return doomed


def list_caches(repository):
    entries, page = [], 1
    while True:
        payload = json.loads(subprocess.run(
            ["gh", "api", f"repos/{repository}/actions/caches?per_page=100&page={page}"],
            check=True, capture_output=True, text=True).stdout)
        batch = payload.get("actions_caches", [])
        entries.extend(batch)
        if len(batch) < 100:
            return entries
        page += 1


def delete_cache(repository, cache_id):
    subprocess.run(["gh", "api", "--method", "DELETE", f"repos/{repository}/actions/caches/{cache_id}"],
                   check=True, capture_output=True, text=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY"))
    parser.add_argument("--keep", type=int, default=1, help="generations to retain per restore-key prefix")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    if not args.repository:
        parser.error("--repository or GITHUB_REPOSITORY is required")

    entries = list_caches(args.repository)
    doomed = plan_deletions(entries, datetime.now(timezone.utc), keep=args.keep)
    reclaimed = sum(entry["size_in_bytes"] for entry in doomed)

    for entry in doomed:
        print(f"{'would delete' if args.dry_run else 'deleting'} "
              f"{entry['size_in_bytes'] / 2**30:6.2f} GiB  {entry['ref']}  {entry['key']}")
        if not args.dry_run:
            delete_cache(args.repository, entry["id"])

    total = sum(entry["size_in_bytes"] for entry in entries)
    print(f"\n{len(doomed)} of {len(entries)} entries superseded; "
          f"{reclaimed / 2**30:.2f} GiB of {total / 2**30:.2f} GiB reclaimed")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write(f"Pruned {len(doomed)} superseded cache entries, "
                         f"reclaiming {reclaimed / 2**30:.2f} GiB of {total / 2**30:.2f} GiB.\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
