#!/bin/sh
set -eu
count="$(dds-e2e --list --ignored | awk '/: test$/ {count++} END {print count+0}')"
if [ "$count" -ne 9 ]; then
    echo "Expected 9 DDS E2E tests, found $count" >&2
    exit 1
fi
exec timeout 180 dds-e2e --ignored --test-threads=1 --nocapture
