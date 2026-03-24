# ROSQL MySQL Examples

MySQL / MariaDB support is available via the `mysql` feature flag.

## Status

There is no official OTel Collector MySQL exporter. The OTel ecosystem treats MySQL as a metrics *source* (via the MySQL receiver), not a telemetry storage destination.

MySQL is useful when your organization already has MySQL infrastructure and wants to store OTel data via a custom pipeline.

## Quick start

```sh
# Build with MySQL support
cargo build --features server,mysql --bin rosql

# Compile a query for MySQL dialect
rosql compile "FROM traces WHERE status = 'ERROR'" --backend mysql

# Execute against a MySQL database
rosql query "FROM traces WHERE status = 'ERROR'" \
  --backend mysql --url mysql://user:pass@localhost:3306/telemetry
```

## Schema setup

Create your MySQL database with the same OTel schema as the PostgreSQL example (lowercase columns). See [../postgres/fixtures/schema.sql](../postgres/fixtures/schema.sql) for the table definitions — adapt PostgreSQL-specific types:
- `JSONB` → `JSON`
- `TIMESTAMPTZ` → `TIMESTAMP`
- `TEXT[]` → `JSON` (for array columns)

## Fixture data

The query files in [../queries/](../queries/) are backend-agnostic and work with any database that has the OTel schema populated.
