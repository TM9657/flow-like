"""Check that cache pruning keeps the newest generation and never touches live entries."""

import importlib.util
from datetime import datetime, timedelta, timezone
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "prune_caches.py"
SPEC = importlib.util.spec_from_file_location("prune_caches", SCRIPT)
prune = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prune)

NOW = datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc)


def entry(key, hours_ago, cache_id=0, ref="refs/heads/dev", size=2**30):
    stamp = (NOW - timedelta(hours=hours_ago)).isoformat().replace("+00:00", "Z")
    return {"id": cache_id, "key": key, "ref": ref, "size_in_bytes": size,
            "created_at": stamp, "last_accessed_at": stamp}


class PlanDeletionsTests(unittest.TestCase):
    def test_keeps_newest_generation_per_prefix(self):
        prefix = "v0-rust-workspace-v2-X64-tests-ci-Linux-x64-7a658b95"
        entries = [entry(f"{prefix}-28c32be1", 100, 1),
                   entry(f"{prefix}-39d43cf2", 50, 2),
                   entry(f"{prefix}-4ae54d03", 10, 3)]
        doomed = prune.plan_deletions(entries, NOW)
        self.assertEqual([item["id"] for item in doomed], [2, 1])

    def test_distinct_prefixes_and_refs_are_independent(self):
        entries = [entry("v0-rust-a-Linux-x64-aaaaaaaa-11111111", 100, 1),
                   entry("v0-rust-a-Linux-x64-aaaaaaaa-22222222", 10, 2),
                   entry("v0-rust-b-Linux-x64-aaaaaaaa-33333333", 100, 3),
                   entry("v0-rust-a-Linux-x64-aaaaaaaa-44444444", 100, 4, ref="refs/heads/alpha")]
        self.assertEqual([item["id"] for item in prune.plan_deletions(entries, NOW)], [1])

    def test_in_flight_entries_are_never_deleted(self):
        prefix = "v0-rust-workspace-v2-X64-clippy-check-Linux-x64-7a658b95"
        entries = [entry(f"{prefix}-11111111", 0.5, 1), entry(f"{prefix}-22222222", 0.1, 2)]
        self.assertEqual(prune.plan_deletions(entries, NOW), [])

    def test_content_addressed_and_ungenerational_keys_are_ignored(self):
        entries = [entry("buildkit-blob-1-sha256:" + "a" * 64, 100, 1),
                   entry("index-container-v1-aws-api-1-5a70b200#1", 100, 2),
                   entry("cache-apt-pkgs_7e22ac393005b1660dc34800c960f954", 100, 3),
                   entry("buildkit-blob-1-sha256:" + "b" * 64, 100, 4)]
        self.assertEqual(prune.plan_deletions(entries, NOW), [])

    def test_keep_more_than_one_generation(self):
        prefix = "container-cargo-v1-gcp-api-abcdef01"
        entries = [entry(f"{prefix}-{index:08x}", 100 - index, index) for index in range(1, 5)]
        doomed = prune.plan_deletions(entries, NOW, keep=2)
        self.assertEqual(sorted(item["id"] for item in doomed), [1, 2])


if __name__ == "__main__":
    unittest.main()
