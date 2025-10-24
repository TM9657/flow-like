#!/usr/bin/env bash
set -euo pipefail

user="${POSTGRES_USER:-postgres}"
pass="${POSTGRES_PASSWORD:-postgres}"
host="${POSTGRES_HOST:-postgres}"
port="${POSTGRES_PORT:-5432}"
db="${POSTGRES_DB:-app}"

export DATABASE_URL="postgresql://${user}:${pass}@${host}:${port}/${db}"
echo "DATABASE_URL=$DATABASE_URL"
exec "$@"
