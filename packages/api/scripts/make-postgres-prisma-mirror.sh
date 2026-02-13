#!/usr/bin/env bash
set -euo pipefail

SRC_ROOT="prisma/schema"                     # your actual schema directory
DST_ROOT="prisma-postgres-mirror/schema"     # mirror lives here

if [ ! -d "$SRC_ROOT" ]; then
  echo "No '$SRC_ROOT' directory found. Aborting."
  exit 1
fi

rm -rf "prisma-postgres-mirror"
mkdir -p "$DST_ROOT"
cp -R "$SRC_ROOT"/. "$DST_ROOT"/

# For every .prisma file in the tree, only change provider inside datasource blocks
# This keeps generator blocks (e.g., prisma-client-js) intact.
while IFS= read -r -d '' file; do
  awk '
    BEGIN { in_ds = 0 }
    {
      # Detect start of a datasource block (e.g.,: datasource db { )
      if ($0 ~ /^[[:space:]]*datasource[[:space:]]+[A-Za-z0-9_]+[[:space:]]*\{[[:space:]]*$/) {
        in_ds = 1
      }
      # Inside datasource, rewrite provider "cockroachdb" -> "postgresql"
      if (in_ds && $0 ~ /provider[[:space:]]*=/) {
        gsub(/provider[[:space:]]*=[[:space:]]*"cockroachdb"/, "provider = \"postgresql\"")
      }
      print
      # Close brace ends the block
      if (in_ds && $0 ~ /^[[:space:]]*\}[[:space:]]*$/) {
        in_ds = 0
      }
    }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done < <(find "$DST_ROOT" -type f -name "*.prisma" -print0)

# Optional: format/validate the mirrored schema directory
if command -v bunx >/dev/null 2>&1; then
  bunx prisma format   --schema="$DST_ROOT" || true
  bunx prisma validate --schema="$DST_ROOT" || true
fi

echo "Postgres mirror created at 'prisma-postgres-mirror/schema'."
