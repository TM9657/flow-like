"""Check target coverage and reject incomplete or mixed GHCR releases."""

import contextlib
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "container_images.py"
SPEC = importlib.util.spec_from_file_location("container_images", SCRIPT)
containers = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(containers)

OWNER = "rheosoph"
SOURCE_SHA = "a" * 40
DIGEST = "sha256:" + "b" * 64
RUN_ID = "12345"
RUN_ATTEMPT = "1"


def make_records(cloud="all"):
    return [containers.record(entry["id"], OWNER, SOURCE_SHA, DIGEST, RUN_ID, RUN_ATTEMPT)
            for entry in containers.matrix(cloud)["include"]]


def merge(records, cloud="all"):
    return containers.manifest(cloud, OWNER, SOURCE_SHA, RUN_ID, RUN_ATTEMPT, records)


class MatrixTests(unittest.TestCase):
    def test_matrix_covers_portable_targets_and_real_recipes(self):
        entries = containers.matrix("all")["include"]
        self.assertEqual(len(entries), 55)
        self.assertEqual(len({entry["id"] for entry in entries}), 55)
        self.assertEqual([len(containers.matrix(cloud)["include"]) for cloud in ("aws", "gcp", "azure")], [10, 7, 8])
        self.assertIn("gcp-api", {entry["id"] for entry in entries})
        self.assertIn("azure-api", {entry["id"] for entry in entries})
        self.assertEqual({entry["cloud"] for entry in entries if entry["workload"] == "web"}, {"docker-compose", "kubernetes"})
        root = SCRIPT.parents[2]
        for entry in entries:
            with self.subTest(target=entry["id"]):
                self.assertTrue((root / entry["dockerfile"]).is_file())
                self.assertTrue((root / entry["context"]).is_dir())
                self.assertEqual(entry["context"], ".")
                self.assertEqual(entry["runner"].endswith("-arm"), entry["platform"] == "linux/arm64")

    def test_nonstandard_recipes_and_context_are_preserved(self):
        entries = {entry["id"]: entry for entry in containers.matrix("all")["include"]}
        self.assertEqual(entries["aws-executor-async"]["dockerfile"], "apps/backend/aws/executor-ecs/Dockerfile")
        self.assertEqual(entries["aws-compiler"]["dockerfile"], "apps/backend/aws/compiler-ecs/Dockerfile")
        self.assertEqual(entries["aws-signaling"]["dockerfile"], "apps/backend/docker-compose/signaling/Dockerfile")
        self.assertEqual(entries["azure-otel-collector"]["context"], ".")
        self.assertEqual(entries["aws-executor"]["platform"], "linux/amd64")
        self.assertEqual(entries["aws-api"]["platform"], "linux/arm64")

    def test_self_hosted_selectors_cover_both_native_architectures(self):
        self.assertEqual([len(containers.matrix(cloud)["include"]) for cloud in ("self-hosted", "docker-compose", "kubernetes")], [30, 18, 20])
        workloads = {}
        for entry in containers.matrix("self-hosted")["include"]:
            architecture = entry["platform"].split("/")[1]
            self.assertEqual(entry["id"], f"{entry['cloud']}-{entry['workload']}-{architecture}")
            self.assertEqual(entry["image_suffix"], f"flow-like-{entry['cloud']}-{entry['workload']}")
            workloads.setdefault(entry["image_suffix"], set()).add(entry["platform"])
        self.assertEqual(len(workloads), 15)
        self.assertTrue(all(platforms == {"linux/amd64", "linux/arm64"} for platforms in workloads.values()))

    def test_kubernetes_reuses_exact_compose_dependency_targets(self):
        compose = {entry["id"]: entry for entry in containers.matrix("docker-compose")["include"]}
        kubernetes = containers.matrix("kubernetes")["include"]
        shared = {entry["id"]: entry for entry in kubernetes if entry["cloud"] == "docker-compose"}
        expected_ids = {f"docker-compose-{workload}-{architecture}"
                        for workload in ("runtime", "compiler", "signaling", "object-store-init")
                        for architecture in ("amd64", "arm64")}
        self.assertEqual(set(shared), expected_ids)
        self.assertEqual(shared, {target_id: compose[target_id] for target_id in expected_ids})
        self.assertEqual(shared["docker-compose-object-store-init-amd64"]["dockerfile"], "apps/backend/docker-compose/object-store/Dockerfile")
        self.assertEqual(len({entry["id"] for entry in kubernetes}), len(kubernetes))


class ManifestTests(unittest.TestCase):
    def test_complete_release_is_sorted_and_references_digests(self):
        records = make_records()
        manifest = merge(list(reversed(records)))
        self.assertEqual(manifest["images"], records)
        self.assertEqual(manifest["source_sha"], SOURCE_SHA)
        self.assertTrue(all(entry["image"] == f"{entry['repository']}@{DIGEST}" for entry in manifest["images"]))

    def test_selected_cloud_is_complete(self):
        for cloud in ("aws", "gcp", "azure", "docker-compose"):
            with self.subTest(cloud=cloud):
                result = merge(make_records(cloud), cloud)
                self.assertEqual({entry["cloud"] for entry in result["images"]}, {cloud})

    def test_kubernetes_manifest_requires_shared_dependencies(self):
        records = make_records("kubernetes")
        self.assertEqual(len(merge(records, "kubernetes")["images"]), 20)
        own_records = [entry for entry in records if entry["cloud"] == "kubernetes"]
        with self.assertRaisesRegex(ValueError, "missing image records"):
            merge(own_records, "kubernetes")

    def test_architectures_share_a_repository_but_not_records_or_tags(self):
        records = make_records("self-hosted")
        self.assertEqual(len(merge(records, "self-hosted")["images"]), 30)
        entries = {entry["id"]: entry for entry in records}
        amd64 = entries["docker-compose-api-amd64"]
        arm64 = entries["docker-compose-api-arm64"]
        self.assertEqual(amd64["repository"], arm64["repository"])
        self.assertNotEqual(amd64["tag"], arm64["tag"])
        self.assertEqual(amd64["tag"], f"sha-{SOURCE_SHA}-amd64-run-{RUN_ID}-{RUN_ATTEMPT}")
        self.assertEqual(arm64["tag"], f"sha-{SOURCE_SHA}-arm64-run-{RUN_ID}-{RUN_ATTEMPT}")
        arm64["platform"] = amd64["platform"]
        with self.assertRaisesRegex(ValueError, "does not match the release: platform"):
            merge(records, "self-hosted")

    def test_missing_duplicate_and_unexpected_targets_fail(self):
        records = make_records()
        for variant, message in (
            (records[:-1], "missing image records"),
            ([], "missing image records"),
            (records + [records[0]], "duplicate image record"),
            ([dict(records[0], id="unknown-api")] + records[1:], "unexpected target"),
        ):
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                merge(variant)
        with self.assertRaisesRegex(ValueError, "unexpected target"):
            merge(records, "aws")

    def test_metadata_mismatches_and_unknown_fields_fail(self):
        for field, replacement in (
            ("source_sha", "c" * 40),
            ("build", {"run_id": "98765", "run_attempt": RUN_ATTEMPT}),
            ("build", {"run_id": RUN_ID, "run_attempt": "2"}),
            ("build_inputs", {"dockerfile_sha256": "0" * 64}),
            ("platform", "linux/s390x"),
            ("repository", "ghcr.io/another-owner/flow-like-aws-api"),
            ("image", "ghcr.io/rheosoph/flow-like-aws-api:latest"),
            ("dockerfile", "unexpected/Dockerfile"),
            ("context", ".."),
            ("schema_version", 2),
            ("schema_version", True),
            ("schema_version", 1.0),
            ("tag", "latest"),
            ("extra", "unexpected content"),
        ):
            records = copy.deepcopy(make_records())
            records[0][field] = replacement
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "does not match the release"):
                merge(records)

    def test_invalid_digest_or_record_shape_fails(self):
        for digest in ("latest", "sha256:abc", "sha256:" + "B" * 64, None):
            records = make_records()
            records[0]["digest"] = digest
            with self.subTest(digest=digest), self.assertRaisesRegex(ValueError, "SHA-256"):
                merge(records)
        for entry in ([], None, "image", {"id": []}):
            with self.subTest(entry=entry), self.assertRaisesRegex(ValueError, "object with a target ID"):
                merge([entry])

    def test_record_identity_is_normalized_and_validated(self):
        result = containers.record("aws-api", "Rheosoph", SOURCE_SHA, DIGEST, RUN_ID, RUN_ATTEMPT)
        self.assertEqual(result["repository"], "ghcr.io/rheosoph/flow-like-aws-api")
        self.assertEqual(result["tag"], f"sha-{SOURCE_SHA}-arm64-run-{RUN_ID}-{RUN_ATTEMPT}")
        for owner, sha, run_id, attempt in (
            ("rheosoph/other", SOURCE_SHA, RUN_ID, RUN_ATTEMPT),
            (OWNER, "dev", RUN_ID, RUN_ATTEMPT),
            (OWNER, "0" * 40, RUN_ID, RUN_ATTEMPT),
            (OWNER, SOURCE_SHA, "0", RUN_ATTEMPT),
            (OWNER, SOURCE_SHA, RUN_ID, "-1"),
        ):
            with self.subTest(owner=owner, sha=sha, run_id=run_id, attempt=attempt), self.assertRaises(ValueError):
                containers.record("aws-api", owner, sha, DIGEST, run_id, attempt)

    def test_build_inputs_bind_recipe_and_default_aws_configuration(self):
        entries = {entry["id"]: entry for entry in make_records()}
        for entry in entries.values():
            recipe = containers.REPOSITORY_ROOT / entry["dockerfile"]
            self.assertEqual(entry["build_inputs"]["dockerfile_sha256"], hashlib.sha256(recipe.read_bytes()).hexdigest())
        config = containers.REPOSITORY_ROOT / "flow-like.config.json"
        for cloud in ("aws", "gcp", "azure"):
            inputs = entries[f"{cloud}-api"]["build_inputs"]
            self.assertEqual(inputs["flow_like_config_sha256"], hashlib.sha256(config.read_bytes()).hexdigest())
            self.assertEqual(inputs["runtime_config"], "full-document-v1")
            self.assertEqual(inputs["variant"], "runtime-config-public-default")
        for target_id in ("aws-api", "aws-file-tracker"):
            self.assertEqual(entries[target_id]["build_inputs"]["database_tls"], "dsql-no-custom-ca")
        self.assertNotIn("database_tls", entries["gcp-executor"]["build_inputs"])
        self.assertNotIn("flow_like_config_sha256", entries["aws-file-tracker"]["build_inputs"])

    def test_self_hosted_api_records_bind_each_example_configuration(self):
        entries = {entry["id"]: entry for entry in make_records("self-hosted")}
        for cloud in ("docker-compose", "kubernetes"):
            config = containers.REPOSITORY_ROOT / f"apps/backend/{cloud}/flow-like.config.example.json"
            for architecture in ("amd64", "arm64"):
                build_inputs = entries[f"{cloud}-api-{architecture}"]["build_inputs"]
                self.assertEqual(build_inputs["flow_like_config_sha256"], hashlib.sha256(config.read_bytes()).hexdigest())
                self.assertEqual(build_inputs["variant"], "runtime-config-self-hosted-default")
                self.assertEqual(build_inputs["runtime_config"], "full-document-v1")
                self.assertNotIn("database_tls", build_inputs)
        self.assertNotIn("variant", entries["docker-compose-runtime-amd64"]["build_inputs"])

    def test_self_hosted_web_records_identify_runtime_configuration(self):
        entries = {entry["id"]: entry for entry in make_records("self-hosted")}
        for cloud in ("docker-compose", "kubernetes"):
            for architecture in ("amd64", "arm64"):
                build_inputs = entries[f"{cloud}-web-{architecture}"]["build_inputs"]
                self.assertEqual(build_inputs["variant"], "runtime-public-config")
                self.assertNotIn("flow_like_config_sha256", build_inputs)


class CLITests(unittest.TestCase):
    def test_record_and_manifest_files_round_trip(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            common = ["--owner", OWNER, "--source-sha", SOURCE_SHA, "--run-id", RUN_ID, "--run-attempt", RUN_ATTEMPT]
            for entry in containers.matrix("gcp")["include"]:
                output = root / "records" / entry["id"] / "image.json"
                self.assertEqual(containers.main(["record", "--target", entry["id"], "--digest", DIGEST, "--output", str(output), *common]), 0)
            output = root / "release.json"
            self.assertEqual(containers.main(["manifest", "--cloud", "gcp", "--records-dir", str(root / "records"), "--output", str(output), *common]), 0)
            self.assertEqual(json.loads(output.read_text()), merge(make_records("gcp"), "gcp"))

    def test_invalid_json_does_not_echo_artifact_contents(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "image.json"
            path.write_text("sensitive-invalid-content")
            with self.assertRaisesRegex(ValueError, "cannot read image record image.json") as error:
                containers.read_records(path.parent)
            self.assertNotIn("sensitive-invalid-content", str(error.exception))

    def test_matrix_cli_emits_one_json_line(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(containers.main(["matrix", "--cloud", "aws"]), 0)
        self.assertEqual(len(output.getvalue().splitlines()), 1)
        self.assertEqual(json.loads(output.getvalue()), containers.matrix("aws"))


if __name__ == "__main__":
    unittest.main()
