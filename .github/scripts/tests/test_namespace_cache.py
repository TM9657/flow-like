"""Exercise profile-volume compiler caching without starting external services."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest


REPOSITORY = Path(__file__).resolve().parents[3]
ACTION = REPOSITORY / ".github/actions/setup-namespace-cache/action.yml"
WORKFLOW = REPOSITORY / ".github/workflows/standalone-release.yml"


def step_script(path, name, indent):
    step = re.search(
        rf"^{re.escape(indent)}- name: {re.escape(name)}\n(.*?)(?=^{re.escape(indent)}- |\Z)",
        path.read_text(), re.MULTILINE | re.DOTALL,
    )
    if not step:
        raise AssertionError(f"Missing step: {name}")
    body = step.group(1)
    run = re.search(r"^ +run: (.*)$", body, re.MULTILINE)
    if not run:
        raise AssertionError(f"Missing command: {name}")
    return textwrap.dedent(body[run.end() + 1:]) if run.group(1) == "|" else run.group(1)


class NamespaceCacheTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.directory = Path(folder.name)
        self.commands = self.directory / "commands"
        self.commands.mkdir()
        self.environment_file = self.directory / "github-environment"
        self.environment_file.write_text("EXISTING_VALUE=kept\n")
        self.environment = {
            "PATH": str(self.commands) + os.pathsep + os.defpath,
            "GITHUB_ENV": str(self.environment_file),
        }
        self.write_command("node", '''\
            import os
            import sys

            assert sys.argv[1] == "-p"
            assert os.environ["COMPILER_CACHE_NAME"] in ("cargo", "runtime-packs")
            print(os.environ["NATIVE_CACHE_DIR"])
            sys.exit(int(os.environ.get("NODE_STATUS", "0")))
            ''')

    def write_command(self, name, body):
        command = self.commands / name
        command.write_text(f"#!{sys.executable}\n" + textwrap.dedent(body))
        command.chmod(0o755)
        return command

    def run_script(self, script, environment=None):
        return subprocess.run(
            ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", script],
            cwd=self.directory, env={**self.environment, **(environment or {})},
            capture_output=True, text=True, check=False, timeout=10,
        )

    def configure(self, directory="/home/runner/.cache/flow-like-sccache/cargo", name="cargo", status=0, runner_os="Linux"):
        return self.run_script(step_script(ACTION, "Configure compiler cache directory", "    "), {
            "NATIVE_CACHE_DIR": directory, "COMPILER_CACHE_NAME": name, "NODE_STATUS": str(status),
            "RUNNER_OS": runner_os,
        })

    def exported_environment(self):
        return dict(line.split("=", 1) for line in self.environment_file.read_text().splitlines())

    def test_exports_native_paths_and_compiler_settings_without_shell_expansion(self):
        for runner_os, directory, rustc_wrapper in (
            ("macOS", "/Users/Runner User/.cache/flow-like-sccache/cargo", "sccache"),
            ("Windows", r"C:\Users\Runner User\.cache\flow-like-sccache\cargo", ""),
            ("Linux", "/home/$(touch unexpected-command)/.cache/flow-like-sccache/cargo", "sccache"),
        ):
            with self.subTest(runner_os=runner_os, directory=directory):
                self.environment_file.write_text("EXISTING_VALUE=kept\nRUSTC_WRAPPER=existing-wrapper\n")
                result = self.configure(directory, runner_os=runner_os)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.exported_environment(), {
                    "EXISTING_VALUE": "kept", "SCCACHE_DIR": directory, "SCCACHE_CACHE_SIZE": "20G",
                    "RUSTC_WRAPPER": rustc_wrapper, "CARGO_INCREMENTAL": "0",
                    "CMAKE_C_COMPILER_LAUNCHER": "sccache", "CMAKE_CXX_COMPILER_LAUNCHER": "sccache",
                    "SCCACHE_GHA_ENABLED": "false", "SCCACHE_IDLE_TIMEOUT": "0",
                })
                self.assertEqual(result.stdout, "")
                self.assertFalse((self.directory / "unexpected-command").exists())

    def test_failed_directory_lookup_leaves_existing_environment_unchanged(self):
        result = self.configure(status=23)
        self.assertEqual(result.returncode, 23)
        self.assertEqual(self.environment_file.read_text(), "EXISTING_VALUE=kept\n")

    def test_server_uses_profile_volume_and_propagates_startup_failure(self):
        self.assertEqual(self.configure().returncode, 0)
        self.write_command("sccache", '''\
            import os
            import sys

            assert sys.argv[1:] == ["--start-server"]
            assert os.environ["SCCACHE_DIR"] == "/home/runner/.cache/flow-like-sccache/cargo"
            assert os.environ["SCCACHE_CACHE_SIZE"] == "20G"
            assert os.environ["SCCACHE_GHA_ENABLED"] == "false"
            assert os.environ["RUSTC_WRAPPER"] == "sccache"
            sys.exit(int(os.environ["SERVER_STATUS"]))
            ''')
        for status in (0, 41):
            with self.subTest(status=status):
                result = self.run_script(step_script(ACTION, "Start compiler cache", "    "), {
                    **self.exported_environment(), "SERVER_STATUS": str(status),
                })
                self.assertEqual(result.returncode, status, result.stderr)

    def run_container(self, pack, cache=False):
        self.write_command("docker", '''\
            import json
            import os
            import sys

            arguments = sys.argv[1:]
            environment = {}
            for index, argument in enumerate(arguments):
                if argument == "--env":
                    key, separator, value = arguments[index + 1].partition("=")
                    environment[key] = value if separator else os.environ.get(key)
            print(json.dumps({"arguments": arguments, "environment": environment}))
            ''')
        self.write_command("id", "import sys\nprint('1001' if sys.argv[1] == '-u' else '1002')\n")
        runner_temp = self.directory / "runner temporary files"
        runner_temp.mkdir(exist_ok=True)
        environment = {
            "RUNNER_TEMP": str(runner_temp), "RUNTIME_PACK": pack, "SOURCE_DATE_EPOCH": "1234567890",
            "RUNTIME_IMAGE": "ubuntu:22.04@sha256:test-only-digest", "UBUNTU_SNAPSHOT": "20261001T000000Z",
        }
        if cache:
            cache_dir = str(self.directory / "profile cache/runtime-packs")
            self.assertEqual(self.configure(cache_dir, name="runtime-packs").returncode, 0)
            environment.update(self.exported_environment())
            binary = self.write_command("sccache", "raise SystemExit(0)\n")
            stage = self.run_script(step_script(WORKFLOW, "Stage sccache for the frozen Linux build environment", "      "), environment)
            self.assertEqual(stage.returncode, 0, stage.stderr)
            self.assertEqual((runner_temp / "compiler-cache-bin/sccache").read_bytes(), binary.read_bytes())
        result = self.run_script(step_script(WORKFLOW, "Build and check Linux packs in the frozen Ubuntu 22.04 environment", "      "), environment)
        self.assertEqual(result.returncode, 0, result.stderr)
        call = json.loads(result.stdout)
        volumes = [call["arguments"][index + 1] for index, value in enumerate(call["arguments"]) if value == "--volume"]
        self.assertIn(f"{runner_temp}:/scratch", volumes)
        self.assertEqual(call["arguments"][-3:], [environment["RUNTIME_IMAGE"], "bash", "/work/.github/scripts/build_linux_runtime.sh"])
        return call, volumes

    def test_arm_container_mounts_profile_cache_and_preserves_host_owner(self):
        call, volumes = self.run_container("llamacpp-linux-arm64-cpu", cache=True)
        self.assertIn(f"{self.directory}/profile cache/runtime-packs:/compiler-cache", volumes)
        self.assertEqual({key: value for key, value in call["environment"].items() if key.startswith("SCCACHE_")}, {
            "SCCACHE_DIR": "/compiler-cache", "SCCACHE_CACHE_SIZE": "20G", "SCCACHE_CACHE_OWNER": "1001:1002",
            "SCCACHE_GHA_ENABLED": "false", "SCCACHE_IDLE_TIMEOUT": "0",
        })

    def test_prebuilt_x64_packs_run_without_cache_configuration(self):
        for pack in ("llamacpp-linux-x64-cpu", "llamacpp-linux-x64-vulkan"):
            with self.subTest(pack=pack):
                call, volumes = self.run_container(pack)
                self.assertFalse(any(value.endswith(":/compiler-cache") for value in volumes))
                self.assertFalse(any(key.startswith("SCCACHE_") for key in call["environment"]))

    def test_container_cleanup_restores_ownership_after_build_or_server_failure(self):
        source = (REPOSITORY / ".github/scripts/build_linux_runtime.sh").read_text()
        setup = re.search(r"^  cleanup_cache\(\) \{.*?(?=^  LLAMACPP_COMMIT=)", source, re.MULTILINE | re.DOTALL)
        self.assertIsNotNone(setup)
        command = self.write_command("sccache", '''\
            import os
            import sys

            with open(os.environ["COMMAND_LOG"], "a") as log:
                log.write(sys.argv[1] + "\\n")
            sys.exit(int(os.environ["START_STATUS" if sys.argv[1] == "--start-server" else "STOP_STATUS"]))
            ''')
        self.write_command("chown", '''\
            import json
            import os
            import sys

            with open(os.environ["COMMAND_LOG"], "a") as log:
                log.write(json.dumps(sys.argv[1:]) + "\\n")
            ''')
        log = self.directory / "cleanup.log"
        for build_status, start_status, stop_status in ((0, 0, 0), (17, 0, 0), (0, 23, 0), (0, 0, 29)):
            with self.subTest(build=build_status, start=start_status, stop=stop_status):
                log.write_text("")
                result = self.run_script(textwrap.dedent(setup.group()) + '\nexit "$BUILD_STATUS"\n', {
                    "CMAKE_C_COMPILER_LAUNCHER": str(command), "SCCACHE_DIR": "/compiler-cache",
                    "SCCACHE_CACHE_OWNER": "1001:1002", "COMMAND_LOG": str(log),
                    "BUILD_STATUS": str(build_status), "START_STATUS": str(start_status), "STOP_STATUS": str(stop_status),
                })
                self.assertEqual(result.returncode, start_status or build_status, result.stderr)
                lines = log.read_text().splitlines()
                self.assertEqual(lines[:2], ["--start-server", "--stop-server"])
                self.assertEqual(json.loads(lines[2]), ["-R", "1001:1002", "/compiler-cache"])


if __name__ == "__main__":
    unittest.main()
