"""Remove only complete, content-reviewed public self-test PEMs from scanner input.

Every other byte remains eligible for the supplemental ELF secret scan. This is
not a file, library, path, or secret-rule exclusion. The metadata records hashes,
not private-key material. Images and the ordinary image scan are unchanged.
"""

from functools import lru_cache
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile


FIXTURE_FILE = Path(__file__).with_suffix(".json")
CHUNK_SIZE = 64 * 1024


@lru_cache(maxsize=1)
def fixture_fingerprints():
    """Read reviewed fingerprints and fail closed on malformed metadata."""
    metadata = json.loads(FIXTURE_FILE.read_text(encoding="utf-8"))
    fixtures = metadata["fixtures"]
    if not isinstance(fixtures, list) or not fixtures:
        raise ValueError("public fixture metadata has no reviewed fingerprints")
    fingerprints = set()
    for fixture in fixtures:
        digest, length = fixture["sha256"], fixture["length"]
        if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError("public fixture metadata has an invalid fingerprint")
        if type(length) is not int or not 64 <= length <= 8192:
            raise ValueError("public fixture metadata has an invalid length")
        if (digest, length) in fingerprints:
            raise ValueError("public fixture metadata repeats a fingerprint")
        fingerprints.add((digest, length))
    return frozenset(fingerprints)


def redact_public_fixtures(path: Path) -> int:
    """Replace exact known PEM blocks in a private strings file; return count.

    The fingerprint excludes the LF after END. Internal LF bytes are literal:
    CRLF, whitespace changes, truncated blocks, and new keys are not exempted.
    Keep only one chunk and a maximum-size PEM overlap in memory.
    """
    fingerprints = fixture_fingerprints()
    maximum_length = max(length for _, length in fingerprints)
    pattern = re.compile(
        rb"-----BEGIN (?P<label>(?:RSA |DSA |EC )?PRIVATE KEY)-----"
        rb"[A-Za-z0-9+/=\n]{1," + str(maximum_length).encode("ascii") + rb"}"
        rb"-----END (?P=label)-----"
    )
    path = Path(path)
    temporary_path = None
    redacted = 0
    try:
        with path.open("rb") as source, tempfile.NamedTemporaryFile(
            mode="wb", prefix=".public-fixtures-", dir=path.parent, delete=False
        ) as destination:
            temporary_path = Path(destination.name)
            os.chmod(temporary_path, 0o600)
            pending = b""
            while True:
                chunk = source.read(CHUNK_SIZE)
                pending += chunk
                cutoff = max(0, len(pending) - maximum_length) if chunk else len(pending)
                position = 0
                for match in pattern.finditer(pending):
                    if match.start() >= cutoff:
                        break
                    block = match.group()
                    digest = hashlib.sha256(block).hexdigest()
                    if (digest, len(block)) in fingerprints:
                        destination.write(pending[position:match.start()])
                        destination.write(b"[reviewed public GnuTLS self-test key sha256=" + digest.encode("ascii") + b"]")
                        position = match.end()
                        redacted += 1
                    cutoff = max(cutoff, match.end())
                destination.write(pending[position:cutoff])
                pending = pending[cutoff:]
                if not chunk:
                    break
        os.replace(temporary_path, path)
        temporary_path = None
        return redacted
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
