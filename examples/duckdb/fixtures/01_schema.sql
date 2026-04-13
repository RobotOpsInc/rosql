-- ROSQL example schema — DuckDB-compatible version of the OTel tables.
-- Adapted from examples/postgres/fixtures/01_schema.sql
-- Changes: JSONB → JSON, TEXT[] → VARCHAR[]
-- v2: added trace_id to topic_messages (for PATH DEVIATION FOR TRACE),
--     added ros2_events table (for SHOW DEPLOYMENTS and node lifecycle events)

CREATE TABLE IF NOT EXISTS otel_traces (
    timestamp           TIMESTAMPTZ NOT NULL,
    trace_id            TEXT NOT NULL,
    span_id             TEXT NOT NULL,
    parent_span_id      TEXT NOT NULL DEFAULT '',
    span_name_col       TEXT NOT NULL,
    span_kind           TEXT NOT NULL DEFAULT 'INTERNAL',
    service_name        TEXT NOT NULL DEFAULT '',
    duration            BIGINT NOT NULL,
    status_code         TEXT NOT NULL DEFAULT 'OK',
    span_attributes     JSON NOT NULL DEFAULT '{}',
    resource_attributes JSON NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS otel_logs (
    timestamp           TIMESTAMPTZ NOT NULL,
    trace_id            TEXT NOT NULL DEFAULT '',
    span_id             TEXT NOT NULL DEFAULT '',
    severity_text       TEXT NOT NULL DEFAULT 'INFO',
    severity_number     INTEGER NOT NULL DEFAULT 9,
    service_name        TEXT NOT NULL DEFAULT '',
    body                TEXT NOT NULL DEFAULT '',
    resource_attributes JSON NOT NULL DEFAULT '{}',
    log_attributes      JSON NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS otel_metrics (
    timestamp    TIMESTAMPTZ NOT NULL,
    metric_name  TEXT NOT NULL,
    value        DOUBLE PRECISION NOT NULL,
    attributes   JSON NOT NULL DEFAULT '{}',
    service_name TEXT NOT NULL DEFAULT '',
    resource_attributes JSON NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS topic_messages (
    robot_id     TEXT NOT NULL,
    topic_name   TEXT NOT NULL,
    timestamp    TIMESTAMPTZ NOT NULL,
    trace_id     TEXT NOT NULL DEFAULT '',
    fields       JSON NOT NULL DEFAULT '{}',
    message_type TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS mcap_metadata (
    robot_id   TEXT NOT NULL,
    session_id TEXT NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time   TIMESTAMPTZ NOT NULL,
    s3_key     TEXT NOT NULL,
    topics     VARCHAR[] NOT NULL DEFAULT []
);

CREATE TABLE IF NOT EXISTS ros2_events (
    timestamp    TIMESTAMPTZ NOT NULL,
    robot_id     TEXT NOT NULL,
    event_type   TEXT NOT NULL,
    node_name    TEXT NOT NULL DEFAULT '',
    version      TEXT NOT NULL DEFAULT '',
    payload      JSON NOT NULL DEFAULT '{}'
);
