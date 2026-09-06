"""Exercise exported image inspection without Docker or executing image code."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import stat
import tarfile
import tempfile
import unittest
from unittest.mock import MagicMock, patch


SCRIPT = Path(__file__).resolve().parents[1] / "container_publication.py"
SPEC = importlib.util.spec_from_file_location("container_publication", SCRIPT)
publication = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(publication)
CONTAINER_ID = "a" * 64


def archive(entries=(), license=True):
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w") as output:
        if license:
            entries = [(publication.LICENSE, b"License text"), *entries]
        for name, content in entries:
            member = tarfile.TarInfo(name)
            if isinstance(content, tuple):
                member.type, member.linkname = content
                output.addfile(member)
            else:
                member.size = len(content)
                output.addfile(member, io.BytesIO(content))
    stream.seek(0)
    return stream


class ArchiveTests(unittest.TestCase):
    def test_elf_contents_only_are_written_with_private_permissions(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "strings"
            result = publication.inspect_archive(archive([
                ("usr/bin/service", b"\x7fELF\x00credential-value\x00abc\x00last-run"),
                ("usr/lib/library.so", b"\x7fELF\x01library-data"),
                ("usr/share/text.txt", b"not an ELF file"),
                ("etc/ssl/certs/public-ca.pem", b"public CA"),
                ("bin", (tarfile.SYMTYPE, "usr/bin")),
                ("usr/bin/absolute-link", (tarfile.SYMTYPE, "/usr/bin/service")),
                ("usr/bin/relative-link", (tarfile.SYMTYPE, "../../usr/bin/service")),
            ]), output)
            self.assertEqual(result["files"], 5)
            self.assertEqual(result["elf_files"], 2)
            self.assertEqual((output / "000001.txt").read_bytes(), b"credential-value\nlast-run\n")
            self.assertEqual((output / "000002.txt").read_bytes(), b"library-data\n")
            self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE((output / "000001.txt").stat().st_mode), 0o600)

    def test_printable_runs_survive_every_chunk_boundary(self):
        source = b"abc\x00four\x00very-long-secret-value\x01xx\x00tail"
        expected = b"four\nvery-long-secret-value\ntail\n"
        for chunk_size in range(1, len(source) + 1):
            with self.subTest(chunk_size=chunk_size):
                output = io.BytesIO()
                self.assertEqual(publication.printable_strings(io.BytesIO(source), output, chunk_size=chunk_size), len(expected))
                self.assertEqual(output.getvalue(), expected)

    def test_credential_paths_fail_without_printing_names_or_contents(self):
        for name in (
            "root/.aws/config", "root/.azure/accessTokens.json", "root/.ssh/config",
            "app/.git/config", "root/.netrc", "root/.git-credentials",
            "root/.npmrc", "root/.config/gcloud/credentials.db", "app/terraform.tfstate.backup",
            "root/.cargo/credentials", "root/.cargo/credentials.toml",
            "app/.env", "app/.env.production", "app/key.p12", "app/key.pfx", "app/private.key",
        ):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "strings"
                with self.assertRaisesRegex(publication.PublicationError, "credential file") as error:
                    publication.inspect_archive(archive([(name, b"secret-value")]), output)
                self.assertNotIn(name, str(error.exception))
                self.assertNotIn("secret-value", str(error.exception))
                self.assertFalse(output.exists())

    def test_missing_empty_and_symlinked_license_fail(self):
        for entries in ([], [(publication.LICENSE, b"")], [(publication.LICENSE, (tarfile.SYMTYPE, "/LICENSE"))]):
            with self.subTest(entries=entries), tempfile.TemporaryDirectory() as directory:
                with self.assertRaisesRegex(publication.PublicationError, "LICENSE"):
                    publication.inspect_archive(archive(entries, license=False), Path(directory) / "strings")

    def test_traversal_and_credential_links_fail(self):
        for name, content in (
            ("../escape", b"data"), ("/absolute", b"data"),
            ("app/../../escape", b"data"),
            ("app/link", (tarfile.SYMTYPE, "../../escape")),
            ("app/link", (tarfile.LNKTYPE, "../../escape")),
            ("app/link", (tarfile.SYMTYPE, "/root/.ssh/id_rsa")),
        ):
            with self.subTest(name=name, content=content), tempfile.TemporaryDirectory() as directory:
                with self.assertRaises(publication.PublicationError):
                    publication.inspect_archive(archive([(name, content)]), Path(directory) / "strings")

    def test_existing_output_directory_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "keep.txt"
            path.write_text("existing")
            with self.assertRaises(FileExistsError):
                publication.inspect_archive(archive(), Path(directory))
            self.assertEqual(path.read_text(), "existing")

    def test_duplicate_paths_cannot_replace_a_checked_license(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(publication.PublicationError, "duplicate paths"):
                publication.inspect_archive(archive([(publication.LICENSE, b"")]), Path(directory) / "strings")

    def test_malformed_archive_fails_and_removes_partial_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "strings"
            with self.assertRaises(tarfile.ReadError):
                publication.inspect_archive(io.BytesIO(b"not an archive"), output)
            self.assertFalse(output.exists())


class ConfigTests(unittest.TestCase):
    def check(self, value):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "config.json"
            path.write_text(json.dumps(value))
            publication.check_config(path)

    def test_nested_literal_credentials_fail(self):
        for key in ("password", "client_secret", "private_key", "access_token", "clientSecret", "api-key", "service_account_json"):
            with self.subTest(key=key), self.assertRaisesRegex(publication.PublicationError, "literal credential"):
                self.check({"providers": [{"nested": {key: "secret-value"}}]})

    def test_public_metadata_reference_names_and_unset_values_pass(self):
        self.check({
            "client_id": "public-client", "token": "https://example.invalid/token",
            "client_secret_env": "CLIENT_SECRET", "private_key_secret_ref": "PRIVATE_KEY",
            "password": None, "client_secret": "", "providers": [{"private_key": None}],
        })

    def test_invalid_json_duplicate_keys_and_nonstandard_numbers_fail(self):
        for content in ('{"password":"secret", "password":""}', "not JSON", '{"x":NaN}', "[]"):
            with self.subTest(content=content), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "config.json"
                path.write_text(content)
                with self.assertRaises(publication.PublicationError):
                    publication.check_config(path)


class DockerTests(unittest.TestCase):
    def test_architecture_mismatch_prevents_container_creation(self):
        with patch.object(publication, "docker", return_value="linux/amd64") as docker:
            with self.assertRaisesRegex(publication.PublicationError, "architecture"):
                publication.extract_image("local:test", "linux/arm64", Path("unused"))
            self.assertEqual(docker.call_count, 1)

    def test_exact_container_is_removed_after_archive_failure_and_never_started(self):
        with tempfile.TemporaryDirectory() as directory:
            process = MagicMock()
            process.stdout = archive([("app/.env", b"secret")])
            process.poll.return_value = 0
            with patch.object(publication, "docker", side_effect=["linux/arm64", CONTAINER_ID, ""]) as docker, patch.object(publication.subprocess, "Popen", return_value=process) as popen:
                with self.assertRaises(publication.PublicationError):
                    publication.extract_image("local:test", "linux/arm64", Path(directory) / "strings")
                self.assertEqual(docker.call_args.args, ("rm", "--", CONTAINER_ID))
                self.assertEqual(docker.call_args_list[1].args[0], "create")
                self.assertEqual(popen.call_args.args[0], ["docker", "export", "--", CONTAINER_ID])
                self.assertTrue(all(call.args[0] not in ("run", "start") for call in docker.call_args_list))

    def test_failed_export_does_not_report_success(self):
        with tempfile.TemporaryDirectory() as directory:
            process = MagicMock()
            process.stdout = archive()
            process.wait.return_value = 1
            process.poll.return_value = 1
            with patch.object(publication, "docker", side_effect=["linux/arm64", CONTAINER_ID, ""]) as docker, patch.object(publication.subprocess, "Popen", return_value=process):
                with self.assertRaisesRegex(publication.PublicationError, "complete container filesystem"):
                    publication.extract_image("local:test", "linux/arm64", Path(directory) / "strings")
                self.assertEqual(docker.call_args.args, ("rm", "--", CONTAINER_ID))

    def test_cli_fails_closed_without_echoing_unexpected_error_contents(self):
        stderr = io.StringIO()
        with patch.object(publication, "extract_image", side_effect=RuntimeError("secret-value")), contextlib.redirect_stderr(stderr):
            self.assertEqual(publication.main(["extract", "--image", "local:test", "--platform", "linux/arm64", "--output-dir", "unused"]), 1)
        self.assertNotIn("secret-value", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
