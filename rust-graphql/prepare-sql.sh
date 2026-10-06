#!/bin/sh
# Regenerates .sqlx/, the saved description of every SQL query in the code.
# Run this after changing a query or adding a migration, then commit .sqlx/.
#
# It builds a throwaway database from the migration files and lets the sqlx
# macros check each query against it. Your books.db is not touched.
set -e
cd "$(dirname "$0")"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

for migration in migrations/*.sql; do
    sqlite3 "$tmp/prepare.db" < "$migration"
done

mkdir -p "$tmp/sqlx"
# The macros only run when the crate is recompiled, so force that.
cargo clean -p rust-graphql
DATABASE_URL="sqlite:$tmp/prepare.db" SQLX_OFFLINE=false SQLX_OFFLINE_DIR="$tmp/sqlx" \
    cargo check --all-targets

rm -rf .sqlx
mv "$tmp/sqlx" .sqlx
echo "Saved $(ls .sqlx | wc -l | tr -d ' ') queries to .sqlx/"
