#!/bin/sh
set -eu
test "$(/usr/local/bin/opcua-pubsub-e2e --ignored --list | grep -c ': test$')" -eq 4
exec timeout 180 /usr/local/bin/opcua-pubsub-e2e --ignored --nocapture --test-threads=1
