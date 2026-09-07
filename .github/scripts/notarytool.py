#!/usr/bin/env python3
"""Keep Apple's notarization diagnostics visible to the Tauri bundler."""

import os
import signal
import subprocess
import sys
import time


XCRUN = "/usr/bin/xcrun"
MAX_ATTEMPTS = 3
RETRY_DELAY_SECONDS = 10
SUBMIT_TIMEOUT_SECONDS = 1800
HISTORY_TIMEOUT_SECONDS = 120
CRASH_SIGNALS = {signal.SIGABRT, signal.SIGBUS, signal.SIGSEGV}


def report_failure(args: list[str], reason: str, stdout: bytes, stderr: bytes) -> None:
    # Tauri 2.9.6 reports only stderr when notarytool exits unsuccessfully.
    # Include stdout too, since notarytool can put its error in JSON there.
    message = f"notarytool {args[1]} {reason}.\n"
    for label, output in (("stdout", stdout), ("stderr", stderr)):
        if output:
            message += f"{label}:\n{output.decode('utf-8', errors='replace')}\n"
    for index, arg in enumerate(args):
        password = ""
        if arg == "--password" and index + 1 < len(args):
            password = args[index + 1]
        elif arg.startswith("--password="):
            password = arg.split("=", 1)[1]
        if password:
            message = message.replace(password, "***")
    print(message, file=sys.stderr, end="", flush=True)


def main(args: list[str]) -> int:
    if args[:2] not in (["notarytool", "submit"], ["notarytool", "history"]):
        os.execv(XCRUN, [XCRUN, *args])
        return 0

    submit = args[1] == "submit"
    timeout = SUBMIT_TIMEOUT_SECONDS if submit else HISTORY_TIMEOUT_SECONDS
    attempts = MAX_ATTEMPTS if submit else 1
    command_args = list(args)

    for attempt in range(1, attempts + 1):
        try:
            result = subprocess.run(
                [XCRUN, *command_args],
                stdin=subprocess.DEVNULL,
                capture_output=True,
                timeout=timeout,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            report_failure(
                args,
                f"timed out after {timeout}s; check Apple's submission history before retrying",
                error.stdout or b"",
                error.stderr or b"",
            )
            return 124
        except OSError as error:
            report_failure(args, f"could not start ({error})", b"", b"")
            return 1

        if result.returncode == 0:
            # Preserve the JSON protocol, including Invalid results. Tauri must
            # still check Accepted, retrieve rejection logs, and staple the app.
            sys.stdout.buffer.write(result.stdout)
            sys.stderr.buffer.write(result.stderr)
            return 0

        crashed = result.returncode < 0 and -result.returncode in CRASH_SIGNALS
        if result.returncode < 0:
            reason = f"terminated by signal {-result.returncode}"
        else:
            reason = f"exited with status {result.returncode}"
        report_failure(args, f"{reason} (attempt {attempt}/{attempts})", result.stdout, result.stderr)

        if not crashed or attempt == attempts:
            return 128 - result.returncode if result.returncode < 0 else result.returncode

        # Reuse the signed archive inside Tauri's temporary directory. Apple's
        # standard S3 endpoint is an alternative when the accelerated upload
        # path fails: https://developer.apple.com/documentation/security/customizing-the-notarization-workflow
        if not any(arg in command_args for arg in ("--s3-acceleration", "--no-s3-acceleration")):
            command_args.append("--no-s3-acceleration")
        print(f"Retrying crashed notarytool in {RETRY_DELAY_SECONDS}s.", file=sys.stderr, flush=True)
        time.sleep(RETRY_DELAY_SECONDS)

    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
