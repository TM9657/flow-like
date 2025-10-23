#!/usr/bin/env bash
set -euo pipefail

# Generate EC P-256 private key
openssl ecparam -name prime256v1 -genkey -noout -out ec-private.pem

# Derive corresponding public key (PEM)
openssl ec -in ec-private.pem -pubout -out ec-public.pem

# Detect whether GNU or BSD base64 is available
if base64 --help 2>&1 | grep -q "\-w"; then
  # GNU coreutils (Linux)
  PRIV_B64=$(base64 -w 0 ec-private.pem)
  PUB_B64=$(base64 -w 0 ec-public.pem)
else
  # BSD base64 (macOS)
  PRIV_B64=$(base64 -b 0 < ec-private.pem)
  PUB_B64=$(base64 -b 0 < ec-public.pem)
fi

# Output environment-ready variables
echo "REALTIME_KEY=$PRIV_B64"
echo "REALTIME_PUB=$PUB_B64"

# Cleanup
rm -f ec-private.pem ec-public.pem
