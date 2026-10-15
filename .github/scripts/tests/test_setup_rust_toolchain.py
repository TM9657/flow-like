"""Exercise the shared Rust setup action without downloading a compiler."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest


REPOSITORY = Path(__file__).resolve().parents[3]
ACTION = REPOSITORY / ".github/actions/setup-rust/action.yml"
ACTIVE_TOOLCHAIN = "test-beta-aarch64-apple-darwin"
TARGETS = ["aarch64-apple-darwin", "x86_64-apple-darwin"]


class SetupRustToolchainTests(unittest.TestCase):
    def run_setup(self, install_failures=0, target_failures=0, targets=TARGETS):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        directory = Path(folder.name)
        commands = directory / "commands"
        commands.mkdir()
        environment_file = directory / "environment"
        environment_file.touch()
        command_log = directory / "commands.jsonl"
        shim = f"#!{sys.executable}\n" + textwrap.dedent('''\
            import json
            import os
            from pathlib import Path
            import sys

            command = [Path(sys.argv[0]).name, *sys.argv[1:]]
            command_log = Path(os.environ["COMMAND_LOG"])
            previous = [json.loads(line) for line in command_log.read_text().splitlines()] if command_log.exists() else []
            with command_log.open("a") as log:
                log.write(json.dumps({
                    "command": command,
                    "toolchain": os.environ.get("RUSTUP_TOOLCHAIN"),
                    "exported": Path(os.environ["GITHUB_ENV"]).read_text(),
                }) + "\\n")
            if command == ["rustup", "toolchain", "install"]:
                failures = int(os.environ["INSTALL_FAILURES"])
            elif command[:3] == ["rustup", "target", "add"]:
                failures = int(os.environ["TARGET_FAILURES"])
            elif command == ["rustup", "show", "active-toolchain"]:
                print(os.environ["ACTIVE_TOOLCHAIN"] + " (overridden by rust-toolchain.toml)")
                sys.exit(0)
            elif command[0] == "sleep":
                sys.exit(0)
            elif command == ["rustc", "--version", "--verbose"]:
                print("rustc test compiler")
                sys.exit(0)
            else:
                raise AssertionError("Unexpected command: " + repr(command))
            attempt = 1 + sum(call["command"] == command for call in previous)
            if attempt <= failures:
                sys.exit(23)
            ''')
        for name in ("rustup", "rustc", "sleep"):
            executable = commands / name
            executable.write_text(shim)
            executable.chmod(0o755)
        script = textwrap.dedent(ACTION.read_text().split("      run: |\n", 1)[1])
        result = subprocess.run(
            ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", script],
            cwd=directory,
            env={
                **os.environ,
                "PATH": str(commands) + os.pathsep + os.environ["PATH"],
                "COMMAND_LOG": str(command_log),
                "GITHUB_ENV": str(environment_file),
                "RUSTUP_TOOLCHAIN": "inherited-wrong-toolchain",
                "RUST_TARGETS": ",".join(targets),
                "ACTIVE_TOOLCHAIN": ACTIVE_TOOLCHAIN,
                "INSTALL_FAILURES": str(install_failures),
                "TARGET_FAILURES": str(target_failures),
            },
            capture_output=True, text=True, check=False, timeout=10,
        )
        calls = [json.loads(line) for line in command_log.read_text().splitlines()]
        return result, calls, environment_file.read_text()

    def assert_success(self, result, calls, exported):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(exported, f"RUSTUP_TOOLCHAIN={ACTIVE_TOOLCHAIN}\n")
        self.assertTrue(all(call["toolchain"] is None for call in calls))
        self.assertTrue(all(call["exported"] == "" for call in calls))
        self.assertEqual(calls[-1]["command"], ["rustc", "--version", "--verbose"])

    def test_first_attempt_installs_both_targets_without_sleeping(self):
        result, calls, exported = self.run_setup()
        self.assert_success(result, calls, exported)
        self.assertEqual([call["command"] for call in calls], [
            ["rustup", "toolchain", "install"],
            ["rustup", "show", "active-toolchain"],
            ["rustup", "target", "add", *TARGETS],
            ["rustc", "--version", "--verbose"],
        ])

    def test_empty_targets_skip_target_installation(self):
        result, calls, exported = self.run_setup(targets=[])
        self.assert_success(result, calls, exported)
        self.assertEqual([call["command"] for call in calls], [
            ["rustup", "toolchain", "install"],
            ["rustup", "show", "active-toolchain"],
            ["rustc", "--version", "--verbose"],
        ])

    def test_transient_compiler_download_retries_before_selecting_toolchain(self):
        result, calls, exported = self.run_setup(install_failures=2)
        self.assert_success(result, calls, exported)
        self.assertEqual([call["command"] for call in calls], [
            ["rustup", "toolchain", "install"],
            ["sleep", "5"],
            ["rustup", "toolchain", "install"],
            ["sleep", "10"],
            ["rustup", "toolchain", "install"],
            ["rustup", "show", "active-toolchain"],
            ["rustup", "target", "add", *TARGETS],
            ["rustc", "--version", "--verbose"],
        ])

    def test_transient_target_download_retries_with_both_targets(self):
        result, calls, exported = self.run_setup(target_failures=2)
        self.assert_success(result, calls, exported)
        self.assertEqual([call["command"] for call in calls], [
            ["rustup", "toolchain", "install"],
            ["rustup", "show", "active-toolchain"],
            ["rustup", "target", "add", *TARGETS],
            ["sleep", "5"],
            ["rustup", "target", "add", *TARGETS],
            ["sleep", "10"],
            ["rustup", "target", "add", *TARGETS],
            ["rustc", "--version", "--verbose"],
        ])

    def test_exhausted_compiler_download_preserves_failure_and_stops_setup(self):
        result, calls, exported = self.run_setup(install_failures=5)
        self.assertEqual(result.returncode, 23, result.stdout + result.stderr)
        self.assertEqual(exported, "")
        expected = []
        for delay in ("5", "10", "20", "40"):
            expected.extend([["rustup", "toolchain", "install"], ["sleep", delay]])
        expected.append(["rustup", "toolchain", "install"])
        self.assertEqual([call["command"] for call in calls], expected)

    def test_exhausted_target_download_preserves_failure_without_exporting(self):
        result, calls, exported = self.run_setup(target_failures=5)
        self.assertEqual(result.returncode, 23, result.stdout + result.stderr)
        self.assertEqual(exported, "")
        expected = [
            ["rustup", "toolchain", "install"],
            ["rustup", "show", "active-toolchain"],
        ]
        for delay in ("5", "10", "20", "40"):
            expected.extend([["rustup", "target", "add", *TARGETS], ["sleep", delay]])
        expected.append(["rustup", "target", "add", *TARGETS])
        self.assertEqual([call["command"] for call in calls], expected)


if __name__ == "__main__":
    unittest.main()
