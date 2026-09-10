"""Resolve published image digests through a stubbed registry API."""
import base64
import email.message
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import urllib.error
import urllib.parse
from unittest.mock import patch

BASE = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("resolve_images", BASE / "scripts/resolve-images.py")
resolve = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(resolve)
DIGEST = "sha256:" + "c" * 64


class Response(io.BytesIO):
    def __init__(self, status, headers, body=b""):
        super().__init__(body)
        self.status = status
        self.headers = headers

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


class Registry:
    """Bearer-challenging registry: every repository under `private` needs Basic credentials at the token endpoint."""

    def __init__(self, digests, private=(), realm_host=None):
        self.digests = digests
        self.private = set(private)
        self.realm_host = realm_host
        self.requests = []

    def __call__(self, request):
        self.requests.append(request)
        url = urllib.parse.urlsplit(request.full_url)
        if url.path == "/token":
            scope = urllib.parse.parse_qs(url.query)["scope"][0]
            repository = scope.split(":")[1]
            authorized = request.get_header("Authorization", "").startswith("Basic ")
            if repository in self.private and not authorized:
                raise urllib.error.HTTPError(request.full_url, 403, "denied", email.message.Message(), io.BytesIO(b"{}"))
            return Response(200, {}, json.dumps({"token": "anon-" + repository}).encode())
        assert request.get_method() == "HEAD"
        assert "application/vnd.oci.image.index.v1+json" in request.get_header("Accept")
        repository = url.path[len("/v2/"):].rsplit("/manifests/", 1)[0]
        if not request.get_header("Authorization"):
            headers = email.message.Message()
            headers["WWW-Authenticate"] = f'Bearer realm="https://{self.realm_host or url.netloc}/token",service="{url.netloc}",scope="repository:{repository}:pull"'
            raise urllib.error.HTTPError(request.full_url, 401, "unauthorized", headers, io.BytesIO())
        tag = url.path.rsplit("/", 1)[1]
        if (repository, tag) not in self.digests:
            raise urllib.error.HTTPError(request.full_url, 404, "not found", email.message.Message(), io.BytesIO())
        return Response(200, {"Docker-Content-Digest": self.digests[(repository, tag)], "Content-Type": "application/vnd.oci.image.index.v1+json"})


def digests(prefix, tag="dev"):
    return {(f"{prefix}/{name}", tag): "sha256:" + format(index, "x").rjust(64, "0") for index, name in enumerate(resolve.REPOSITORIES.values(), start=1)}


class ResolveTest(unittest.TestCase):
    def test_anonymous_public_resolution_pins_every_first_party_image(self):
        registry = Registry(digests("rheosoph"))
        resolved = resolve.resolve_all("ghcr.io/rheosoph", "dev", opener=registry)
        self.assertEqual(set(resolved), set(resolve.REPOSITORIES))
        self.assertEqual(resolved["executor"][0], "ghcr.io/rheosoph/flow-like-kubernetes-executor")
        values = resolve.image_values(resolved, "dev")
        executor = resolved["executor"][1]
        self.assertEqual(values["executionManager"]["sandbox"]["image"], "ghcr.io/rheosoph/flow-like-kubernetes-executor@" + executor)
        self.assertEqual(values["executor"]["image"], {"repository": "ghcr.io/rheosoph/flow-like-kubernetes-executor", "tag": "dev", "digest": executor, "pullPolicy": "IfNotPresent"})
        self.assertEqual(values["executorPool"]["image"], values["executor"]["image"])
        self.assertEqual(values["executionManager"]["image"]["digest"], resolved["execution-manager"][1])
        self.assertEqual(values["executionManager"]["queueBridge"]["image"]["repository"], "ghcr.io/rheosoph/flow-like-docker-compose-runtime")
        self.assertEqual(values["database"]["migration"]["image"]["repository"], "ghcr.io/rheosoph/flow-like-kubernetes-migration")
        self.assertEqual(values["sinkServices"]["image"]["repository"], "ghcr.io/rheosoph/flow-like-kubernetes-sink-trigger")
        self.assertEqual(values["rustfs"]["bootstrap"]["image"]["repository"], "ghcr.io/rheosoph/flow-like-docker-compose-object-store-init")
        self.assertEqual(values["global"], {"imageRegistry": ""})
        self.assertNotIn("imagePullSecrets", values["global"])
        self.assertNotIn("nodeSelector", values["api"])
        self.assertFalse(any(r.get_header("Authorization", "").startswith("Basic") for r in registry.requests))

    def test_private_packages_use_environment_credentials_never_arguments(self):
        registry = Registry(digests("fork"), private={"fork/flow-like-kubernetes-api"})
        with patch.dict(os.environ, {}, clear=True), self.assertRaisesRegex(ValueError, "GHCR_TOKEN"):
            resolve.resolve_all("ghcr.io/fork", "dev", opener=registry)
        registry = Registry(digests("fork"), private={"fork/flow-like-kubernetes-api"})
        with patch.dict(os.environ, {"GHCR_TOKEN": "secret-value"}, clear=True):
            resolved = resolve.resolve_all("ghcr.io/fork", "dev", opener=registry)
        self.assertEqual(resolved["api"][1], registry.digests[("fork/flow-like-kubernetes-api", "dev")])
        token_requests = [r for r in registry.requests if urllib.parse.urlsplit(r.full_url).path == "/token"]
        self.assertTrue(token_requests)
        for request in token_requests:
            user, password = base64.b64decode(request.get_header("Authorization").split(" ", 1)[1]).decode().split(":", 1)
            self.assertEqual(password, "secret-value")
            self.assertEqual(user, "token")
        result = subprocess.run(["python3", str(BASE / "scripts/resolve-images.py"), "--token", "x"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)

    def test_credentials_never_follow_a_challenge_to_another_host(self):
        registry = Registry(digests("rheosoph"), realm_host="auth.attacker.example")
        with patch.dict(os.environ, {"GHCR_TOKEN": "secret-value"}, clear=True):
            resolved = resolve.resolve_all("ghcr.io/rheosoph", "dev", opener=registry)
        self.assertEqual(set(resolved), set(resolve.REPOSITORIES))
        token_requests = [r for r in registry.requests if urllib.parse.urlsplit(r.full_url).path == "/token"]
        self.assertTrue(token_requests)
        for request in token_requests:
            self.assertEqual(urllib.parse.urlsplit(request.full_url).hostname, "auth.attacker.example")
            self.assertFalse(request.has_header("Authorization"))
        registry = Registry(digests("rheosoph"), private={"rheosoph/flow-like-kubernetes-api"}, realm_host="auth.attacker.example")
        with patch.dict(os.environ, {"GHCR_TOKEN": "secret-value"}, clear=True), self.assertRaisesRegex(ValueError, "HTTP 403"):
            resolve.resolve_all("ghcr.io/rheosoph", "dev", opener=registry)

    def test_missing_tag_and_bad_digest_fail_with_repository_context(self):
        registry = Registry(digests("rheosoph", "1.2.3"))
        with self.assertRaisesRegex(ValueError, r"flow-like-kubernetes-api:dev returned HTTP 404"):
            resolve.resolve_all("ghcr.io/rheosoph", "dev", opener=registry)
        broken = Registry({("rheosoph/flow-like-kubernetes-api", "dev"): "md5:nope"})
        with self.assertRaisesRegex(ValueError, "no sha256 manifest digest"):
            resolve.resolve_digest("ghcr.io", "rheosoph/flow-like-kubernetes-api", "dev", opener=broken)
        for registry_argument in ("https://ghcr.io/rheosoph", ""):
            with self.assertRaises(ValueError):
                resolve.split_registry(registry_argument)
        self.assertEqual(resolve.split_registry("registry.example.com"), ("registry.example.com", ""))
        self.assertEqual(resolve.split_registry("ghcr.io/rheosoph/"), ("ghcr.io", "rheosoph"))

    def test_mirror_registry_immutable_tag_pull_secrets_and_arch(self):
        tag = "sha-" + "f" * 40 + "-run-123-1"
        registry = Registry(digests("mirror/flow-like", tag))
        resolved = resolve.resolve_all("registry.example.com/mirror/flow-like", tag, opener=registry)
        values = resolve.image_values(resolved, tag, pull_secrets=["mirror-pull"], arch="arm64")
        self.assertEqual(values["api"]["image"]["repository"], "registry.example.com/mirror/flow-like/flow-like-kubernetes-api")
        self.assertEqual(values["api"]["image"]["tag"], tag)
        self.assertEqual(values["global"]["imagePullSecrets"], [{"name": "mirror-pull"}])
        for path in resolve.NODE_SELECTOR_PATHS:
            target = values
            for key in path:
                target = target[key]
            self.assertEqual(target["nodeSelector"]["kubernetes.io/arch"], "arm64", path)

    def test_output_merges_existing_values_and_replaces_local_images(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "values-images.yaml"
            output.write_text(json.dumps({"global": {"imageRegistry": "old/"}, "api": {"image": {"repository": "flow-like-kubernetes-api", "tag": "local"}, "nodeSelector": {"pool": "web"}}, "monitoring": {"grafana": {"adminPassword": "keep"}}}))
            resolved = resolve.resolve_all("ghcr.io/rheosoph", "dev", opener=Registry(digests("rheosoph")))
            existing = json.loads(output.read_text())
            resolve.write_values(output, resolve.image_values(resolved, "dev", existing, arch="amd64"))
            values = json.loads(output.read_text())
            self.assertEqual(values["api"]["image"]["tag"], "dev")
            self.assertEqual(values["api"]["nodeSelector"], {"pool": "web", "kubernetes.io/arch": "amd64"})
            self.assertEqual(values["monitoring"]["grafana"]["adminPassword"], "keep")
            self.assertEqual(values["global"]["imageRegistry"], "")

    def test_command_line_validates_tag_before_any_request(self):
        for tag in ("-bad", "a" * 129, "with space"):
            result = subprocess.run(["python3", str(BASE / "scripts/resolve-images.py"), "--tag=" + tag, "--registry", "registry.invalid"], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("--tag must match", result.stderr)
        result = subprocess.run(["python3", str(BASE / "scripts/resolve-images.py"), "--pull-secret", "Bad Name", "--registry", "registry.invalid"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--pull-secret", result.stderr)


if __name__ == "__main__":
    unittest.main()
