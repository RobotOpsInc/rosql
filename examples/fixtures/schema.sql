-- ROSQL example schema — standard OTel tables for PostgreSQL.
-- Matches the column names expected by the ROSQL SQL compiler.

CREATE TABLE IF NOT EXISTS otel_traces (
    "Timestamp"        TIMESTAMPTZ NOT NULL,
    "TraceId"          TEXT NOT NULL,
    "SpanId"           TEXT NOT NULL,
    "ParentSpanId"     TEXT NOT NULL DEFAULT '',
    "SpanName"         TEXT NOT NULL,
    "SpanKind"         TEXT NOT NULL DEFAULT 'INTERNAL',
    "ServiceName"      TEXT NOT NULL DEFAULT '',
    "Duration"         BIGINT NOT NULL,  -- nanoseconds
    "StatusCode"       TEXT NOT NULL DEFAULT 'OK',
    "SpanAttributes"   JSONB NOT NULL DEFAULT '{}',
    "ResourceAttributes" JSONB NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS otel_logs (
    "Timestamp"        TIMESTAMPTZ NOT NULL,
    "TraceId"          TEXT NOT NULL DEFAULT '',
    "SpanId"           TEXT NOT NULL DEFAULT '',
    "SeverityText"     TEXT NOT NULL DEFAULT 'INFO',
    "SeverityNumber"   INTEGER NOT NULL DEFAULT 9,
    "ServiceName"      TEXT NOT NULL DEFAULT '',
    "Body"             TEXT NOT NULL DEFAULT '',
    "ResourceAttributes" JSONB NOT NULL DEFAULT '{}',
    "LogAttributes"    JSONB NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS otel_metrics (
    "Timestamp"        TIMESTAMPTZ NOT NULL,
    "MetricName"       TEXT NOT NULL,
    "Value"            DOUBLE PRECISION NOT NULL,
    "Attributes"       JSONB NOT NULL DEFAULT '{}',
    "ServiceName"      TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS topic_messages (
    robot_id           TEXT NOT NULL,
    topic_name         TEXT NOT NULL,
    "timestamp"        TIMESTAMPTZ NOT NULL,
    fields             JSONB NOT NULL DEFAULT '{}',
    message_type       TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS mcap_metadata (
    robot_id           TEXT NOT NULL,
    session_id         TEXT NOT NULL,
    start_time         TIMESTAMPTZ NOT NULL,
    end_time           TIMESTAMPTZ NOT NULL,
    s3_key             TEXT NOT NULL,
    topics             TEXT[] NOT NULL DEFAULT '{}'
);
