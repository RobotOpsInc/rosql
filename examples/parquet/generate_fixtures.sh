#!/usr/bin/env bash
# Generate Parquet fixture files from the existing DuckDB SQL fixtures.
#
# Requires: duckdb CLI (https://duckdb.org/docs/installation/)
#   brew install duckdb  # macOS
#
# Usage: ./examples/parquet/generate_fixtures.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SQL_FIXTURES="$REPO_ROOT/examples/duckdb/fixtures"
OUT="$SCRIPT_DIR/fixtures"

# Clean previous output
rm -rf "$OUT"
mkdir -p "$OUT/traces" "$OUT/logs" "$OUT/metrics" "$OUT/topic_messages" "$OUT/mcap_metadata" "$OUT/robot_joint_map"

# Build a single DuckDB SQL script that loads all fixtures and exports to Parquet
TEMP_SQL=$(mktemp)
trap 'rm -f "$TEMP_SQL"' EXIT

for f in "$SQL_FIXTURES"/0[1-9]_*.sql; do
  cat "$f" >> "$TEMP_SQL"
  echo ";" >> "$TEMP_SQL"
done

cat >> "$TEMP_SQL" <<SQL
COPY otel_traces     TO '$OUT/traces/otel_traces.parquet'                (FORMAT PARQUET);
COPY otel_logs       TO '$OUT/logs/otel_logs.parquet'                    (FORMAT PARQUET);
COPY otel_metrics    TO '$OUT/metrics/otel_metrics.parquet'              (FORMAT PARQUET);
COPY topic_messages  TO '$OUT/topic_messages/topic_messages.parquet'     (FORMAT PARQUET);
COPY mcap_metadata   TO '$OUT/mcap_metadata/mcap_metadata.parquet'       (FORMAT PARQUET);
COPY robot_joint_map TO '$OUT/robot_joint_map/robot_joint_map.parquet'   (FORMAT PARQUET);
SQL

echo "Generating Parquet fixtures from SQL..."
duckdb < "$TEMP_SQL"
echo "Done. Fixtures written to $OUT/"
ls -lR "$OUT"
