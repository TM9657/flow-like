#!/bin/bash
# Generate ES256 (P-256) keypair for execution JWTs and export as base64-encoded PEM
#
# This script generates a keypair that can be used for signing/verifying
# execution JWTs. The output is base64-encoded PEM format suitable for
# environment variables.
#
# Usage:
#   ./gen-execution-keys.sh           # Generate new keypair
#   ./gen-execution-keys.sh --export  # Export to .env format
#   ./gen-execution-keys.sh --verify  # Verify current env vars are valid
#
# Environment variables set:
#   EXECUTION_KEY - Base64-encoded PEM private key
#   EXECUTION_PUB - Base64-encoded PEM public key
#   EXECUTION_KID - Key ID for JWKS

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEY_FILE="${SCRIPT_DIR}/.execution-key.pem"
PUB_FILE="${SCRIPT_DIR}/.execution-pub.pem"

generate_keypair() {
    echo "Generating ES256 (P-256) keypair..."

    # Generate private key
    openssl ecparam -genkey -name prime256v1 -noout -out "$KEY_FILE" 2>/dev/null

    # Convert to PKCS#8 format (required by jsonwebtoken crate)
    openssl pkcs8 -topk8 -nocrypt -in "$KEY_FILE" -out "${KEY_FILE}.pkcs8"
    mv "${KEY_FILE}.pkcs8" "$KEY_FILE"

    # Extract public key
    openssl ec -in "$KEY_FILE" -pubout -out "$PUB_FILE" 2>/dev/null

    echo "✓ Generated keypair"
    echo "  Private key: $KEY_FILE"
    echo "  Public key:  $PUB_FILE"
}

export_env() {
    if [[ ! -f "$KEY_FILE" ]] || [[ ! -f "$PUB_FILE" ]]; then
        echo "No keypair found. Generating new one..."
        generate_keypair
    fi

    # Base64 encode the PEM files
    local key_b64=$(base64 < "$KEY_FILE" | tr -d '\n')
    local pub_b64=$(base64 < "$PUB_FILE" | tr -d '\n')
    local kid="execution-es256-$(date +%Y%m%d)"

    echo ""
    echo "# Execution JWT Configuration"
    echo "# Add these to your .env file or deployment config"
    echo ""
    echo "EXECUTION_KEY=${key_b64}"
    echo ""
    echo "EXECUTION_PUB=${pub_b64}"
    echo ""
    echo "EXECUTION_KID=${kid}"
    echo ""
    echo "# For API callback URL (adjust for your deployment)"
    echo "# API_BASE_URL=https://api.your-domain.com"
}

export_shell() {
    if [[ ! -f "$KEY_FILE" ]] || [[ ! -f "$PUB_FILE" ]]; then
        echo "No keypair found. Generating new one..."
        generate_keypair
    fi

    # Base64 encode the PEM files
    local key_b64=$(base64 < "$KEY_FILE" | tr -d '\n')
    local pub_b64=$(base64 < "$PUB_FILE" | tr -d '\n')
    local kid="execution-es256-$(date +%Y%m%d)"

    echo "# Run this to set environment variables for current shell:"
    echo "export EXECUTION_KEY='${key_b64}'"
    echo "export EXECUTION_PUB='${pub_b64}'"
    echo "export EXECUTION_KID='${kid}'"
}

verify_env() {
    echo "Verifying execution JWT configuration..."

    local errors=0

    if [[ -z "${EXECUTION_KEY:-}" ]]; then
        echo "✗ EXECUTION_KEY not set"
        errors=$((errors + 1))
    else
        # Try to decode and verify
        local decoded=$(echo "$EXECUTION_KEY" | base64 -d 2>/dev/null || true)
        if [[ "$decoded" == *"BEGIN PRIVATE KEY"* ]]; then
            echo "✓ EXECUTION_KEY is valid PKCS#8 private key"
        else
            echo "✗ EXECUTION_KEY is not a valid base64-encoded PEM private key"
            errors=$((errors + 1))
        fi
    fi

    if [[ -z "${EXECUTION_PUB:-}" ]]; then
        echo "✗ EXECUTION_PUB not set"
        errors=$((errors + 1))
    else
        local decoded=$(echo "$EXECUTION_PUB" | base64 -d 2>/dev/null || true)
        if [[ "$decoded" == *"BEGIN PUBLIC KEY"* ]]; then
            echo "✓ EXECUTION_PUB is valid public key"
        else
            echo "✗ EXECUTION_PUB is not a valid base64-encoded PEM public key"
            errors=$((errors + 1))
        fi
    fi

    if [[ -z "${EXECUTION_KID:-}" ]]; then
        echo "⚠ EXECUTION_KID not set (will use default)"
    else
        echo "✓ EXECUTION_KID is set: $EXECUTION_KID"
    fi

    if [[ $errors -gt 0 ]]; then
        echo ""
        echo "Run '$0 --export' to generate configuration"
        exit 1
    fi

    echo ""
    echo "All checks passed!"
}

show_help() {
    echo "Generate and manage ES256 keypair for execution JWTs"
    echo ""
    echo "Usage:"
    echo "  $0              Generate new keypair (if not exists)"
    echo "  $0 --export     Export as .env format"
    echo "  $0 --shell      Export as shell commands"
    echo "  $0 --verify     Verify current environment"
    echo "  $0 --force      Force regenerate keypair"
    echo "  $0 --help       Show this help"
    echo ""
    echo "Environment variables:"
    echo "  EXECUTION_KEY   Base64-encoded PKCS#8 private key"
    echo "  EXECUTION_PUB   Base64-encoded public key"
    echo "  EXECUTION_KID   Key identifier for JWKS"
}

# Parse arguments
case "${1:-}" in
    --export)
        export_env
        ;;
    --shell)
        export_shell
        ;;
    --verify)
        verify_env
        ;;
    --force)
        rm -f "$KEY_FILE" "$PUB_FILE"
        generate_keypair
        export_env
        ;;
    --help|-h)
        show_help
        ;;
    *)
        if [[ ! -f "$KEY_FILE" ]] || [[ ! -f "$PUB_FILE" ]]; then
            generate_keypair
        else
            echo "Keypair already exists. Use --force to regenerate."
        fi
        export_env
        ;;
esac
