"""Prepare reviewed public PEMs and adjacent source literals for secret scanning.

Only fingerprinted self-test PEMs are removed. A known public literal sequence
gets a line break so adjacent Rust strings cannot form a spurious token. Every
other byte remains eligible for the supplemental ELF secret scan. Images and
the ordinary image scan are unchanged.
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

# Older builds placed these literals from packages/core/editor/src/flow/copilot/stream.rs
# next to each other in the AWS API binary: redact_private_key_blocks (END),
# redact_known_secret_tokens (xoxp-), and redact_inline_secret_values (markers).
# Trivy mistakes the prefix plus the following markers for a Slack token.
# Match the full reviewed context and preserve every byte, adding only a LF at
# the source-literal boundary. Other token prefixes and contents stay untouched.
COPILOT_LITERAL_RUN = (
    b"-----END " b"xoxp-" b"clientsecret" b"client_secret" b"client-secret"
)
COPILOT_LITERAL_SCAN_TEXT = (
    b"-----END " b"xoxp-\n" b"clientsecret" b"client_secret" b"client-secret"
)


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
    """Normalize exact reviewed fixtures in a private strings file; return count.

    The fingerprint excludes the LF after END. Internal LF bytes are literal:
    CRLF, whitespace changes, truncated blocks, and new keys are not exempted.
    The public source-literal sequence only gains a separator. Keep one chunk
    and a maximum-size fixture overlap in memory.
    """
    fingerprints = fixture_fingerprints()
    maximum_length = max(len(COPILOT_LITERAL_RUN), *(length for _, length in fingerprints))
    pattern = re.compile(
        rb"-----BEGIN (?P<label>(?:RSA |DSA |EC )?PRIVATE KEY)-----"
        rb"[A-Za-z0-9+/=\n]{1," + str(maximum_length).encode("ascii") + rb"}"
        rb"-----END (?P=label)-----|" + re.escape(COPILOT_LITERAL_RUN)
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
                    if block == COPILOT_LITERAL_RUN:
                        destination.write(pending[position:match.start()])
                        destination.write(COPILOT_LITERAL_SCAN_TEXT)
                        position = match.end()
                        redacted += 1
                    elif (digest, len(block)) in fingerprints:
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
