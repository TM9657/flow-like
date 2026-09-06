#!/usr/bin/env python3
"""Inspect an unstarted container and prepare ELF strings for secret scanning."""

import argparse
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile


_FIXTURE_SPEC = importlib.util.spec_from_file_location(
    "container_public_secret_fixtures", Path(__file__).with_name("public_secret_fixtures.py")
)
_FIXTURES = importlib.util.module_from_spec(_FIXTURE_SPEC)
_FIXTURE_SPEC.loader.exec_module(_FIXTURES)


LICENSE = "usr/share/licenses/flow-like/LICENSE"
PRINTABLE = re.compile(rb"[ -~]+")
CREDENTIAL_DIRS = {".git", ".aws", ".azure", ".ssh", ".kube"}
CREDENTIAL_NAMES = {".netrc", ".git-credentials", ".npmrc"}
CREDENTIAL_KEYS = re.compile(
    r"(?:^|_)(?:password|passwd|client_secret|private_key|access_token|refresh_token|"
    r"api_key|secret_access_key|service_account_json)$"
)


class PublicationError(Exception):
    """A safe-to-print failure that contains no inspected file content."""


def archive_path(name):
    path = PurePosixPath(name)
    if not name or path.is_absolute() or ".." in path.parts or "\x00" in name:
        raise PublicationError("container archive contains an unsafe path")
    return path


def link_path(member_path, linkname, hardlink=False):
    if not linkname or "\x00" in linkname:
        raise PublicationError("container archive contains an invalid link")
    link = PurePosixPath(linkname)
    parts = [] if hardlink or link.is_absolute() else list(member_path.parent.parts)
    for part in link.parts:
        if part in ("/", "."):
            continue
        if part == "..":
            if not parts:
                raise PublicationError("container archive contains a link outside its root")
            parts.pop()
        else:
            parts.append(part)
    return PurePosixPath(*parts)


def check_credential_path(path):
    parts = tuple(part.lower() for part in path.parts)
    for index, part in enumerate(parts):
        if (part in CREDENTIAL_DIRS or part in CREDENTIAL_NAMES
                or part == ".env" or part.startswith(".env.")
                or part.endswith((".p12", ".pfx", ".key"))
                or part.endswith(".tfstate") or ".tfstate." in part
                or (index and parts[index - 1] == ".config" and part == "gcloud")
                or (index and parts[index - 1] == ".cargo" and part in ("credentials", "credentials.toml"))):
            raise PublicationError("container contains a credential file or directory")


def printable_strings(source, destination, prefix=b"", chunk_size=1024 * 1024):
    """Write ASCII runs of at least four bytes, retaining runs across reads."""
    pending = b""
    active = False
    written = 0

    def finish_run():
        nonlocal pending, active, written
        if active:
            destination.write(b"\n")
            written += 1
        pending, active = b"", False

    chunk = prefix or source.read(chunk_size)
    while chunk:
        end = 0
        for match in PRINTABLE.finditer(chunk):
            if match.start() != end:
                finish_run()
            run = match.group()
            if active:
                destination.write(run)
                written += len(run)
            else:
                pending += run
                if len(pending) >= 4:
                    destination.write(pending)
                    written += len(pending)
                    pending, active = b"", True
            end = match.end()
        if end != len(chunk):
            finish_run()
        chunk = source.read(chunk_size)
    finish_run()
    return written


def inspect_archive(stream, output_dir):
    """Read Docker's tar stream without extracting member paths or links."""
    output_dir = Path(output_dir)
    output_dir.mkdir(mode=0o700)
    os.chmod(output_dir, 0o700)
    outputs = []
    seen_paths = set()
    counts = {"files": 0, "elf_files": 0, "printable_bytes": 0, "public_fixture_blocks": 0}
    license_found = False
    try:
        with tarfile.open(fileobj=stream, mode="r|") as archive:
            for member in archive:
                path = archive_path(member.name)
                if path in seen_paths:
                    raise PublicationError("container archive contains duplicate paths")
                seen_paths.add(path)
                check_credential_path(path)
                if member.issym() or member.islnk():
                    check_credential_path(link_path(path, member.linkname, member.islnk()))
                if not member.isfile():
                    continue
                counts["files"] += 1
                if str(path) == LICENSE and member.size > 0:
                    license_found = True
                source = archive.extractfile(member)
                if source is None:
                    raise PublicationError("container archive has an unreadable file")
                with source:
                    prefix = source.read(4)
                    if prefix != b"\x7fELF":
                        continue
                    counts["elf_files"] += 1
                    output = output_dir / f"{counts['elf_files']:06d}.txt"
                    descriptor = os.open(output, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
                    outputs.append(output)
                    with os.fdopen(descriptor, "wb") as destination:
                        counts["printable_bytes"] += printable_strings(source, destination, prefix)
                    # Remove only exact, independently verified public crypto
                    # self-test keys from scan text, never from the image itself.
                    # Unknown or modified keys retain their complete scan coverage.
                    counts["public_fixture_blocks"] += _FIXTURES.redact_public_fixtures(output)
        if not license_found:
            raise PublicationError("container is missing its nonempty Flow-Like LICENSE file")
        return counts
    except BaseException:
        for output in outputs:
            output.unlink()
        output_dir.rmdir()
        raise


def docker(*arguments):
    try:
        return subprocess.run(["docker", *arguments], check=True, capture_output=True, text=True, timeout=120).stdout.strip()
    except (OSError, subprocess.SubprocessError) as error:
        raise PublicationError("Docker command failed") from error


def extract_image(image, platform, output_dir):
    if not re.fullmatch(r"[a-z0-9][a-zA-Z0-9_./:@-]*", image):
        raise PublicationError("invalid image reference")
    if docker("image", "inspect", "--format", "{{.Os}}/{{.Architecture}}", "--", image) != platform:
        raise PublicationError("local image operating system or architecture differs from the build platform")
    container_id = docker("create", "--platform", platform, "--entrypoint", "/bin/true", "--", image)
    if not re.fullmatch(r"[a-f0-9]{64}", container_id):
        raise PublicationError("Docker did not return a valid created container ID")
    try:
        process = subprocess.Popen(["docker", "export", "--", container_id], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        try:
            counts = inspect_archive(process.stdout, output_dir)
            process.stdout.close()
            if process.wait(timeout=120) != 0:
                raise PublicationError("Docker could not export the complete container filesystem")
            return counts
        finally:
            process.stdout.close()
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=30)
    finally:
        # Only the exact container created above is removed. It was never run.
        docker("rm", "--", container_id)


def check_config(path):
    def unique_keys(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise PublicationError("configuration contains a duplicate JSON key")
            result[key] = value
        return result

    def invalid_constant(_value):
        raise PublicationError("configuration contains a nonstandard JSON constant")

    try:
        config = json.loads(Path(path).read_text(encoding="utf-8"), object_pairs_hook=unique_keys, parse_constant=invalid_constant)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise PublicationError("configuration is not readable valid JSON") from error
    if not isinstance(config, dict):
        raise PublicationError("configuration must be a JSON object")

    def visit(value):
        if isinstance(value, dict):
            for key, child in value.items():
                normalized = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", key).lower().replace("-", "_")
                reference = normalized.endswith(("_env", "_secret_ref"))
                if not reference and (CREDENTIAL_KEYS.search(normalized) or normalized == "secret") and child not in (None, "", [], {}):
                    raise PublicationError("configuration contains a literal credential field")
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(config)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    extract = commands.add_parser("extract", help="inspect an unstarted local image and collect ELF strings")
    extract.add_argument("--image", required=True)
    extract.add_argument("--platform", required=True, choices=("linux/arm64", "linux/amd64"))
    extract.add_argument("--output-dir", type=Path, required=True)
    config = commands.add_parser("check-config", help="reject literal credential fields in the embedded API config")
    config.add_argument("--path", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "extract":
            print(json.dumps(extract_image(args.image, args.platform, args.output_dir), sort_keys=True))
        else:
            check_config(args.path)
            print("Configuration credential-field check passed.")
    except PublicationError as error:
        print(f"container publication: {error}", file=sys.stderr)
        return 1
    except Exception as error:
        # Unexpected parser or Docker errors fail closed without echoing image
        # contents, Docker stderr, configuration values, or traceback locals.
        print(f"container publication: inspection failed ({type(error).__name__})", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
