"""Check reviewed fixture handling and preservation of unknown scan content."""

import hashlib
import importlib.util
import json
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "public_secret_fixtures.py"
SPEC = importlib.util.spec_from_file_location("public_secret_fixtures", SCRIPT)
fixtures = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixtures)
PEM = b"-----BEGIN EC PRIVATE KEY-----\n" + b"QUJD" * 24 + b"\n-----END EC PRIVATE KEY-----"
FINGERPRINT = (hashlib.sha256(PEM).hexdigest(), len(PEM))


class FixtureTests(unittest.TestCase):
    def redact(self, content, fingerprints=(FINGERPRINT,), chunk_size=64 * 1024):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "strings.txt"
            path.write_bytes(content)
            with patch.object(fixtures, "fixture_fingerprints", return_value=frozenset(fingerprints)), patch.object(fixtures, "CHUNK_SIZE", chunk_size):
                count = fixtures.redact_public_fixtures(path)
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertEqual(list(Path(directory).iterdir()), [path])
            return path.read_bytes(), count

    def test_public_literals_gain_only_a_separator_and_preserve_other_tokens(self):
        before = b"xoxb-" + b"syntheticTokenBeforeFixture"
        after = b"xoxp-" + b"syntheticTokenAfterFixture"
        literal_run = b"-----END " + b"xoxp-" + b"clientsecretclient_secretclient-secret"
        separated = b"-----END " + b"xoxp-\n" + b"clientsecretclient_secretclient-secret"
        content = before + literal_run + after
        output, count = self.redact(content)
        self.assertEqual(count, 1)
        self.assertEqual(output, before + separated + after)
        self.assertEqual(output.replace(b"\n", b""), content)
        self.assertEqual(self.redact(output), (output, 0))

    def test_public_literals_and_pems_survive_every_chunk_boundary(self):
        content = b"padding" + fixtures.COPILOT_LITERAL_RUN + PEM + fixtures.COPILOT_LITERAL_RUN + b"tail"
        expected, count = self.redact(content)
        self.assertEqual(count, 3)
        for chunk_size in range(1, len(content) + 2):
            with self.subTest(chunk_size=chunk_size):
                self.assertEqual(self.redact(content, chunk_size=chunk_size), (expected, 3))

    def test_changed_or_partial_public_literals_are_not_normalized(self):
        literal_run = fixtures.COPILOT_LITERAL_RUN
        variants = [literal_run[1:], literal_run[:-1], literal_run.replace(b"xoxp-", b"xoxb-")]
        variants.extend(literal_run[:i] + b"?" + literal_run[i + 1:] for i in range(len(literal_run)))
        for content in variants:
            with self.subTest(content=content):
                self.assertEqual(self.redact(content, chunk_size=7), (content, 0))

    def test_only_complete_exact_reviewed_content_is_replaced(self):
        unknown = PEM.replace(b"QUJD", b"REVG", 1)
        content = b"before\n" + PEM + b"\nother-credential\n" + unknown + b"\nafter"
        output, count = self.redact(content)
        self.assertEqual(count, 1)
        self.assertNotIn(PEM, output)
        self.assertIn(unknown, output)
        self.assertTrue(output.startswith(b"before\n[reviewed public"))
        self.assertTrue(output.endswith(b"\nother-credential\n" + unknown + b"\nafter"))

    def test_chunk_boundaries_preserve_matches_and_unrelated_bytes(self):
        content = b"padding\n" + PEM + b"\n" + PEM + b"\nlast-byte"
        expected, _ = self.redact(content)
        for chunk_size in range(1, len(content) + 2):
            with self.subTest(chunk_size=chunk_size):
                self.assertEqual(self.redact(content, chunk_size=chunk_size), (expected, 2))

    def test_changed_incomplete_and_wrong_length_content_is_not_exempted(self):
        for content in (
            PEM.replace(b"QUJD", b"REVG", 1),
            PEM.replace(b"\n", b"\r\n"),
            PEM.replace(b"END EC", b"END RSA"),
            PEM[:-1], PEM[1:], PEM + b"new-secret\n",
        ):
            with self.subTest(content_length=len(content)):
                if content.startswith(PEM):
                    self.assertTrue(self.redact(content)[0].endswith(b"new-secret\n"))
                else:
                    self.assertEqual(self.redact(content), (content, 0))
        self.assertEqual(self.redact(PEM, [(FINGERPRINT[0], FINGERPRINT[1] + 1)]), (PEM, 0))
        self.assertEqual(self.redact(PEM, [("0" * 64, len(PEM))]), (PEM, 0))

    def test_unknown_nested_and_oversized_pems_are_preserved(self):
        for content in (
            b"-----BEGIN PRIVATE KEY-----" + b"A" * 9000 + b"-----END PRIVATE KEY-----",
            b"-----BEGIN OPENSSH PRIVATE KEY-----\nQUJD\n-----END OPENSSH PRIVATE KEY-----",
            b"-----BEGIN PRIVATE KEY-----\nmalformed\n" + PEM + b"\n-----END PRIVATE KEY-----",
        ):
            output, count = self.redact(content, chunk_size=5)
            if PEM in content:
                self.assertEqual(count, 1)
                self.assertTrue(output.startswith(b"-----BEGIN PRIVATE KEY-----\nmalformed\n"))
                self.assertTrue(output.endswith(b"\n-----END PRIVATE KEY-----"))
            else:
                self.assertEqual((output, count), (content, 0))

    def test_reviewed_metadata_has_twelve_unique_bounded_fingerprints(self):
        fixtures.fixture_fingerprints.cache_clear()
        self.assertEqual(len(fixtures.fixture_fingerprints()), 12)
        metadata = json.loads(fixtures.FIXTURE_FILE.read_text())
        self.assertIn("477a733247460b94cd2b37a10579c27ca6fc196f", metadata["source"])
        self.assertEqual(metadata["source_version"], "3.8.9")

    def test_malformed_metadata_fails_before_modifying_input(self):
        for entry in ({"sha256": "bad", "length": len(PEM)}, {"sha256": FINGERPRINT[0], "length": True}, {"sha256": FINGERPRINT[0], "length": 100000000}):
            with self.subTest(entry=entry), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "metadata.json"
                path.write_text(json.dumps({"fixtures": [entry]}))
                source = Path(directory) / "strings.txt"
                source.write_bytes(PEM)
                fixtures.fixture_fingerprints.cache_clear()
                with patch.object(fixtures, "FIXTURE_FILE", path), self.assertRaises(ValueError):
                    fixtures.redact_public_fixtures(source)
                self.assertEqual(source.read_bytes(), PEM)
                fixtures.fixture_fingerprints.cache_clear()


if __name__ == "__main__":
    unittest.main()
