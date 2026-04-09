-- ROSQL example schema — standard OTel tables for PostgreSQL.
-- Uses lowercase column names matching the OTel Collector PostgreSQL exporter.

CREATE TABLE IF NOT EXISTS otel_traces (
    timestamp          TIMESTAMPTZ NOT NULL,
    trace_id           TEXT NOT NULL,
    span_id            TEXT NOT NULL,
    parent_span_id     TEXT NOT NULL DEFAULT '',
    span_name_col      TEXT NOT NULL,
    span_kind          TEXT NOT NULL DEFAULT 'INTERNAL',
    service_name       TEXT NOT NULL DEFAULT '',
    duration           BIGINT NOT NULL,  -- nanoseconds
    status_code        TEXT NOT NULL DEFAULT 'OK',
    span_attributes    JSONB NOT NULL DEFAULT '{}',
    resource_attributes JSONB NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS otel_logs (
    timestamp          TIMESTAMPTZ NOT NULL,
    trace_id           TEXT NOT NULL DEFAULT '',
    span_id            TEXT NOT NULL DEFAULT '',
    severity_text      TEXT NOT NULL DEFAULT 'INFO',
    severity_number    INTEGER NOT NULL DEFAULT 9,
    service_name       TEXT NOT NULL DEFAULT '',
    body               TEXT NOT NULL DEFAULT '',
    resource_attributes JSONB NOT NULL DEFAULT '{}',
    log_attributes     JSONB NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS otel_metrics (
    timestamp          TIMESTAMPTZ NOT NULL,
    metric_name        TEXT NOT NULL,
    value              DOUBLE PRECISION NOT NULL,
    attributes         JSONB NOT NULL DEFAULT '{}',
    service_name       TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS topic_messages (
    robot_id           TEXT NOT NULL,
    topic_name         TEXT NOT NULL,
    timestamp          TIMESTAMPTZ NOT NULL,
    fields             JSONB NOT NULL DEFAULT '{}',
    message_type       TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS mcap_metadata (
    robot_id           TEXT NOT NULL,
    session_id         TEXT NOT NULL,
    start_time         TIMESTAMPTZ NOT NULL,
    end_time           TIMESTAMPTZ NOT NULL,
    s3_key             TEXT NOT NULL,
    topics             TEXT[] NOT NULL DEFAULT '{}',
    message_types      JSONB NOT NULL DEFAULT '{}'  -- topic → message_type map
);

-- Optional: URDF-derived joint map for SHOW JOINTS / JOINT DEVIATION (v0.4.3+)
CREATE TABLE IF NOT EXISTS robot_joint_map (
    robot_model        TEXT NOT NULL,
    valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_to           TIMESTAMPTZ,                          -- NULL = currently active
    version            TEXT NOT NULL DEFAULT '',
    robot_ids          TEXT[] NOT NULL DEFAULT '{}',
    joint_map          JSONB NOT NULL DEFAULT '[]'
);
