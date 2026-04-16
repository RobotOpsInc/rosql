-- Fixture: historical baseline data for ANOMALY COMPARED TO last week
--
-- ANOMALY(duration) COMPARED TO last week:
--   Baseline window: NOW()::TIMESTAMP-14 days to NOW()::TIMESTAMP-7 days (compiler-generated filter)
--   Current window:  all current data (no SINCE clause in showcase query)
--
-- For robot-amr-02 to be flagged as anomalous (is_anomalous = true):
--   baseline avg duration: ~7.0s ± 0.5s stddev → z_score = (11.0 - 7.0) / 0.5 = 8.0 → anomalous
--   current avg duration:  7s + 8s + 18s = 33s / 3 = 11.0s (pulled up by failed mission)
--
-- For robot-amr-01 to NOT be anomalous:
--   baseline avg duration: ~8.0s ± 0.5s stddev
--   current avg duration:  7s + 8s + 9s = 8.0s → z_score ≈ 0 → not anomalous
--
-- For robot-amr-03 to NOT be anomalous:
--   baseline avg duration: ~11.0s ± 1.0s stddev
--   current avg duration:  10s + 11s + 13s = 11.3s → z_score ≈ 0.3 → not anomalous
--
-- Baseline is stored as root spans only (one span per mission = one duration value).
-- Using NOW()::TIMESTAMP - INTERVAL arithmetic to land in the 7-14 days ago window.

-- ============================================================================
-- Baseline traces: robot-amr-01 (7-14 days ago, ~8s missions, normal)
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name_col, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '10 days',
 'trace-baseline-a01-w1', 'span-baseline-a01-w1-1', '',
 '/navigate_to_pose', 'robot-amr-01', 7500000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '9 days' - INTERVAL '12 hours',
 'trace-baseline-a01-w1', 'span-baseline-a01-w1-2', '',
 '/navigate_to_pose', 'robot-amr-01', 8000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '8 days' - INTERVAL '6 hours',
 'trace-baseline-a01-w1', 'span-baseline-a01-w1-3', '',
 '/navigate_to_pose', 'robot-amr-01', 8500000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}');

-- ============================================================================
-- Baseline traces: robot-amr-02 (7-14 days ago, ~7s missions, normal — pre v2.4.0)
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name_col, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '10 days',
 'trace-baseline-a02-w1', 'span-baseline-a02-w1-1', '',
 '/navigate_to_pose', 'robot-amr-02', 6500000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '9 days' - INTERVAL '12 hours',
 'trace-baseline-a02-w1', 'span-baseline-a02-w1-2', '',
 '/navigate_to_pose', 'robot-amr-02', 7000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '8 days' - INTERVAL '6 hours',
 'trace-baseline-a02-w1', 'span-baseline-a02-w1-3', '',
 '/navigate_to_pose', 'robot-amr-02', 7500000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.3.1"}');

-- ============================================================================
-- Baseline traces: robot-amr-03 (7-14 days ago, ~11s missions, consistent)
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name_col, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '10 days',
 'trace-baseline-a03-w1', 'span-baseline-a03-w1-1', '',
 '/navigate_to_pose', 'robot-amr-03', 10000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '9 days' - INTERVAL '12 hours',
 'trace-baseline-a03-w1', 'span-baseline-a03-w1-2', '',
 '/navigate_to_pose', 'robot-amr-03', 11000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '8 days' - INTERVAL '6 hours',
 'trace-baseline-a03-w1', 'span-baseline-a03-w1-3', '',
 '/navigate_to_pose', 'robot-amr-03', 12000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}');
