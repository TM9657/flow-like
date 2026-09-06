#!/usr/bin/env python3
"""Write only the web application's allowlisted, public runtime settings."""

import argparse
import ipaddress
import json
import os
from pathlib import Path
import re
import sys
import tempfile
from urllib.parse import urlsplit


def public_url(value, name, *, required=False, api=False):
    if not value:
        if required:
            raise ValueError(f"{name} is required")
        return None
    # Reject ambiguous browser URL parsing and credentials, including URL queries.
    if any(character.isspace() or ord(character) < 32 or ord(character) == 127 for character in value) or "\\" in value:
        raise ValueError(f"{name} must be an absolute HTTP(S) URL without whitespace")
    try:
        parsed = urlsplit(value)
        hostname = parsed.hostname
        if not value.startswith(("http://", "https://")) or not hostname:
            raise ValueError()
        if parsed.username is not None or parsed.password is not None or "?" in value or "#" in value:
            raise ValueError()
        # Accessing port also validates malformed and out-of-range ports.
        if parsed.port == 0 or parsed.netloc.endswith(":"):
            raise ValueError()
        if ":" in hostname:
            if "%" in hostname:
                raise ValueError()
            ipaddress.IPv6Address(hostname)
        else:
            ascii_host = hostname.encode("idna").decode("ascii").rstrip(".")
            if len(ascii_host) > 253 or not all(
                re.fullmatch(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?", label)
                for label in ascii_host.split(".")
            ):
                raise ValueError()
            # Browsers parse numeric final labels as IPv4, not DNS names.
            final_label = ascii_host.split(".")[-1]
            if final_label.isdigit() or re.fullmatch(r"0[xX][0-9a-fA-F]+", final_label):
                ipaddress.IPv4Address(ascii_host)
    except (ValueError, UnicodeError):
        # Never echo a potentially secret or malicious value into container logs.
        raise ValueError(f"{name} must be an absolute HTTP(S) URL without credentials, query, or fragment") from None
    return value.rstrip("/") if api else value


def build_config(environment):
    config = {
        "version": 1,
        "apiUrl": public_url(environment.get("FLOW_LIKE_WEB_API_URL"), "FLOW_LIKE_WEB_API_URL", required=True, api=True),
    }
    for key, name in (
        ("redirectUrl", "FLOW_LIKE_WEB_REDIRECT_URL"),
        ("logoutUrl", "FLOW_LIKE_WEB_LOGOUT_URL"),
    ):
        value = public_url(environment.get(name), name)
        if value is not None:
            config[key] = value
    return config


def render_script(config):
    # Safe even if a proxy later embeds this script in HTML. ensure_ascii also
    # escapes U+2028/U+2029; JSON handles quotes and backslashes without shell eval.
    payload = json.dumps(config, ensure_ascii=True, separators=(",", ":"))
    for character, escaped in (("<", "\\u003c"), (">", "\\u003e"), ("&", "\\u0026")):
        payload = payload.replace(character, escaped)
    return f"window.__FLOW_LIKE_PUBLIC_CONFIG__=Object.freeze({payload});\n"


def write_config(output, environment):
    script = render_script(build_config(environment))
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=output.parent, prefix=".runtime-config-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(script)
        temporary.chmod(0o644)
        temporary.replace(output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("/tmp/flow-like-web/runtime-config.js"))
    arguments = parser.parse_args()
    try:
        write_config(arguments.output, os.environ)
    except (ValueError, OSError) as error:
        print(f"Web runtime configuration failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
