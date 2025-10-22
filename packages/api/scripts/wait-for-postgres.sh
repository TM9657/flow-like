#!/usr/bin/env bash
set -euo pipefail

host="${PGHOST:-postgres}"
port="${PGPORT:-5432}"
user="${PGUSER:-${POSTGRES_USER:-postgres}}"
db="${PGDATABASE:-${POSTGRES_DB:-app}}"

echo "Waiting for Postgres at $host:$port (db: $db, user: $user)..."
for i in {1..60}; do
  if PGPASSWORD="${PGPASSWORD:-${POSTGRES_PASSWORD:-postgres}}"
     pg_isready -h "$host" -p "$port" -U "$user" -d "$db" >/dev/null 2>&1; then
    echo "Postgres is ready."
    exit 0
  fi
  sleep 1
done

echo "Postgres did not become ready in time."
exit 1
