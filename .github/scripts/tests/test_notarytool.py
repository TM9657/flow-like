"""Exercise notarization failure handling without Apple credentials or network access."""

import importlib.util
import io
from pathlib import Path
import signal
import subprocess
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "notarytool.py"
SPEC = importlib.util.spec_from_file_location("notarytool", SCRIPT)
notarytool = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(notarytool)


class NotarytoolTests(unittest.TestCase):
    submit_args = [
        "notarytool", "submit", "/tmp/Flow Like.zip", "--wait",
        "--output-format", "json", "--apple-id", "release@example.com",
        "--password", "fixture-password", "--team-id", "TEAM123456",
    ]

    def invoke(self, args, results):
        stdout = io.TextIOWrapper(io.BytesIO(), encoding="utf-8")
        stderr = io.TextIOWrapper(io.BytesIO(), encoding="utf-8")
        with (
            mock.patch.object(notarytool.subprocess, "run", side_effect=results) as run,
            mock.patch.object(notarytool.time, "sleep") as sleep,
            mock.patch.object(notarytool.sys, "stdout", stdout),
            mock.patch.object(notarytool.sys, "stderr", stderr),
        ):
            result = notarytool.main(args)
            stdout.flush()
            stderr.flush()
            return result, stdout.buffer.getvalue(), stderr.buffer.getvalue(), run, sleep

    @staticmethod
    def completed(code=0, stdout=b"", stderr=b""):
        return subprocess.CompletedProcess([], code, stdout, stderr)

    def test_crash_then_acceptance_preserves_json_and_disables_s3_acceleration(self):
        accepted = b'{\n  "id": "request-id", "status": "Accepted"\n}\n'
        args = list(self.submit_args)
        result, stdout, stderr, run, sleep = self.invoke(args, [
            self.completed(-signal.SIGBUS, b"partial upload", b"crash detail"),
            self.completed(stdout=accepted),
        ])

        self.assertEqual(result, 0)
        self.assertEqual(stdout, accepted)
        self.assertEqual(args, self.submit_args)
        self.assertEqual(run.call_count, 2)
        self.assertEqual(run.call_args_list[0].args[0], [notarytool.XCRUN, *args])
        self.assertEqual(
            run.call_args_list[1].args[0],
            [notarytool.XCRUN, *args, "--no-s3-acceleration"],
        )
        for call in run.call_args_list:
            self.assertTrue(call.kwargs["capture_output"])
            self.assertEqual(call.kwargs["stdin"], subprocess.DEVNULL)
            self.assertEqual(call.kwargs["timeout"], notarytool.SUBMIT_TIMEOUT_SECONDS)
        sleep.assert_called_once_with(notarytool.RETRY_DELAY_SECONDS)
        self.assertIn(b"partial upload", stderr)
        self.assertIn(b"crash detail", stderr)
        self.assertNotIn(b"fixture-password", stderr)

    def test_repeated_supported_crashes_stop_at_attempt_limit(self):
        for crash in (signal.SIGBUS, signal.SIGSEGV, signal.SIGABRT):
            with self.subTest(signal=crash):
                result, _, stderr, run, sleep = self.invoke(self.submit_args, [
                    self.completed(-crash, b"last output", b"native crash")
                    for _ in range(notarytool.MAX_ATTEMPTS)
                ])
                self.assertNotEqual(result, 0)
                self.assertEqual(run.call_count, notarytool.MAX_ATTEMPTS)
                self.assertEqual(sleep.call_count, notarytool.MAX_ATTEMPTS - 1)
                self.assertRegex(stderr.decode(), r"(?i)(signal|SIGBUS|SIGSEGV|SIGABRT)")
                self.assertIn(b"last output", stderr)
                self.assertIn(b"native crash", stderr)
                for call in run.call_args_list[1:]:
                    self.assertEqual(call.args[0].count("--no-s3-acceleration"), 1)

    def test_explicit_acceleration_choice_is_preserved_on_retry(self):
        for flag in ("--s3-acceleration", "--no-s3-acceleration"):
            with self.subTest(flag=flag):
                args = [*self.submit_args, flag]
                result, _, _, run, _ = self.invoke(args, [
                    self.completed(-signal.SIGBUS), self.completed(stdout=b"{}\n"),
                ])
                self.assertEqual(result, 0)
                self.assertEqual(run.call_args_list[1].args[0], [notarytool.XCRUN, *args])

    def test_stdout_only_auth_error_is_visible_and_not_retried(self):
        result, _, stderr, run, sleep = self.invoke(self.submit_args, [
            self.completed(1, b"Error: HTTP status code 401. Invalid credentials.\n"),
        ])
        self.assertNotEqual(result, 0)
        self.assertIn(b"HTTP status code 401", stderr)
        self.assertRegex(stderr.decode(), r"(?i)(exit|status|code).*1")
        run.assert_called_once()
        sleep.assert_not_called()

    def test_unrecognized_signal_is_not_retried(self):
        result, _, _, run, sleep = self.invoke(self.submit_args, [
            self.completed(-signal.SIGTERM),
        ])
        self.assertNotEqual(result, 0)
        run.assert_called_once()
        sleep.assert_not_called()

    def test_failure_diagnostics_redact_password_in_both_argument_forms(self):
        for password_args in (["--password", "fixture-secret"], ["--password=fixture-secret"]):
            with self.subTest(arguments=password_args):
                result, _, stderr, run, _ = self.invoke([
                    "notarytool", "submit", "/tmp/app.zip", *password_args,
                ], [self.completed(
                    1, b"stdout: fixture-secret was rejected\n",
                    b"stderr: fixture-secret could not authenticate\n",
                )])
                self.assertNotEqual(result, 0)
                self.assertNotIn(b"fixture-secret", stderr)
                self.assertIn(b"was rejected", stderr)
                self.assertIn(b"could not authenticate", stderr)
                run.assert_called_once()

    def test_invalid_result_is_returned_unchanged_for_tauri_to_reject(self):
        invalid = b'{"id":"request-id","status":"Invalid"}\n'
        result, stdout, _, run, sleep = self.invoke(self.submit_args, [
            self.completed(stdout=invalid),
        ])
        self.assertEqual(result, 0)
        self.assertEqual(stdout, invalid)
        run.assert_called_once()
        sleep.assert_not_called()

    def test_submit_timeout_is_not_retried(self):
        result, _, stderr, run, sleep = self.invoke(self.submit_args, [
            subprocess.TimeoutExpired(
                [notarytool.XCRUN, *self.submit_args],
                notarytool.SUBMIT_TIMEOUT_SECONDS,
                output=b"upload started", stderr=b"network waiting",
            ),
        ])
        self.assertNotEqual(result, 0)
        self.assertRegex(stderr.decode(), r"(?i)(timeout|timed out)")
        self.assertIn(b"upload started", stderr)
        self.assertIn(b"network waiting", stderr)
        self.assertNotIn(b"fixture-password", stderr)
        run.assert_called_once()
        sleep.assert_not_called()

    def test_history_failure_uses_short_timeout_without_retry(self):
        result, _, stderr, run, sleep = self.invoke([
            "notarytool", "history", "--password", "fixture-password",
        ], [self.completed(-signal.SIGBUS, b"history output", b"history crash")])
        self.assertNotEqual(result, 0)
        self.assertEqual(run.call_args.kwargs["timeout"], notarytool.HISTORY_TIMEOUT_SECONDS)
        self.assertIn(b"history output", stderr)
        self.assertIn(b"history crash", stderr)
        run.assert_called_once()
        sleep.assert_not_called()

    def test_other_xcrun_commands_are_forwarded_with_original_arguments(self):
        for args in (
            ["stapler", "staple", "Flow Like.app"],
            ["notarytool", "log", "request-id"],
            ["--sdk", "macosx", "--show-sdk-version"],
            [],
        ):
            with (
                self.subTest(arguments=args),
                mock.patch.object(notarytool.os, "execv", side_effect=SystemExit(0)) as execv,
                mock.patch.object(notarytool.subprocess, "run") as run,
            ):
                with self.assertRaises(SystemExit):
                    notarytool.main(args)
                execv.assert_called_once_with(notarytool.XCRUN, [notarytool.XCRUN, *args])
                run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
