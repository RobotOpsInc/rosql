-- Fixture: otel_metrics — system.cpu.utilization metrics for 3 robots
--
-- Metric name: system.cpu.utilization (ROSQL canonical: cpu_usage)
-- resource_attributes includes robot.id for FACET robot_id queries.
-- service_name mirrors robot_id for convenience.
--
-- Time window: NOW()::TIMESTAMP-47min to NOW()::TIMESTAMP-3min at 2-minute intervals (23 points/robot)
-- Covers SINCE 45 min ago (for TIMESERIES 2 min FACET robot_id SINCE 45 min ago)
--
-- Story:
--   robot-amr-01: steady ~28-32% CPU throughout (reliable)
--   robot-amr-02: normal during missions 1-2, then spikes to 92% at NOW()::TIMESTAMP-26min
--                 (during the failed mission 3 costmap timeout)
--   robot-amr-03: gradual climb from 30% to 45% (aging hardware)

-- ============================================================================
-- robot-amr-01 CPU — steady baseline
-- ============================================================================

INSERT INTO otel_metrics (timestamp, metric_name, value, attributes, service_name, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '47 minutes', 'system.cpu.utilization', 28.5, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '45 minutes', 'system.cpu.utilization', 29.2, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '43 minutes', 'system.cpu.utilization', 31.0, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '41 minutes', 'system.cpu.utilization', 30.5, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '39 minutes', 'system.cpu.utilization', 28.8, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '37 minutes', 'system.cpu.utilization', 29.5, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '35 minutes', 'system.cpu.utilization', 30.1, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '33 minutes', 'system.cpu.utilization', 31.2, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '31 minutes', 'system.cpu.utilization', 29.7, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '29 minutes', 'system.cpu.utilization', 28.3, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '27 minutes', 'system.cpu.utilization', 32.0, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'system.cpu.utilization', 30.8, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '23 minutes', 'system.cpu.utilization', 29.0, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '21 minutes', 'system.cpu.utilization', 31.5, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '19 minutes', 'system.cpu.utilization', 28.9, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '17 minutes', 'system.cpu.utilization', 30.3, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '15 minutes', 'system.cpu.utilization', 29.6, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '13 minutes', 'system.cpu.utilization', 31.8, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '11 minutes', 'system.cpu.utilization', 30.0, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '9 minutes',  'system.cpu.utilization', 28.7, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '7 minutes',  'system.cpu.utilization', 29.9, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '5 minutes',  'system.cpu.utilization', 30.5, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}'),
(NOW()::TIMESTAMP - INTERVAL '3 minutes',  'system.cpu.utilization', 28.2, '{}', 'robot-amr-01', '{"robot.id": "robot-amr-01"}');

-- ============================================================================
-- robot-amr-02 CPU — normal, then spikes during mission 3 failure at NOW()::TIMESTAMP-26min
-- ============================================================================

INSERT INTO otel_metrics (timestamp, metric_name, value, attributes, service_name, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '47 minutes', 'system.cpu.utilization', 30.2, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '45 minutes', 'system.cpu.utilization', 31.5, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '43 minutes', 'system.cpu.utilization', 29.8, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '41 minutes', 'system.cpu.utilization', 32.0, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '39 minutes', 'system.cpu.utilization', 30.5, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '37 minutes', 'system.cpu.utilization', 28.9, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '35 minutes', 'system.cpu.utilization', 31.2, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '33 minutes', 'system.cpu.utilization', 29.0, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '31 minutes', 'system.cpu.utilization', 30.8, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
-- Mission 3 starts at NOW()::TIMESTAMP-26min — CPU climbs as costmap retry loops
(NOW()::TIMESTAMP - INTERVAL '29 minutes', 'system.cpu.utilization', 38.5, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '27 minutes', 'system.cpu.utilization', 55.0, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
-- *** SPIKE: costmap timeout + recovery loop running
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'system.cpu.utilization', 92.3, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '23 minutes', 'system.cpu.utilization', 78.6, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
-- Recovery/abort — CPU drops
(NOW()::TIMESTAMP - INTERVAL '21 minutes', 'system.cpu.utilization', 45.2, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '19 minutes', 'system.cpu.utilization', 32.1, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '17 minutes', 'system.cpu.utilization', 29.5, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '15 minutes', 'system.cpu.utilization', 28.0, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '13 minutes', 'system.cpu.utilization', 30.2, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '11 minutes', 'system.cpu.utilization', 29.8, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '9 minutes',  'system.cpu.utilization', 31.0, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '7 minutes',  'system.cpu.utilization', 28.5, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '5 minutes',  'system.cpu.utilization', 30.3, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}'),
(NOW()::TIMESTAMP - INTERVAL '3 minutes',  'system.cpu.utilization', 29.1, '{}', 'robot-amr-02', '{"robot.id": "robot-amr-02"}');

-- ============================================================================
-- robot-amr-03 CPU — gradual upward drift (aging hardware)
-- ============================================================================

INSERT INTO otel_metrics (timestamp, metric_name, value, attributes, service_name, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '47 minutes', 'system.cpu.utilization', 30.1, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '45 minutes', 'system.cpu.utilization', 31.0, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '43 minutes', 'system.cpu.utilization', 31.5, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '41 minutes', 'system.cpu.utilization', 32.2, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '39 minutes', 'system.cpu.utilization', 33.0, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '37 minutes', 'system.cpu.utilization', 33.5, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '35 minutes', 'system.cpu.utilization', 34.2, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '33 minutes', 'system.cpu.utilization', 35.0, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '31 minutes', 'system.cpu.utilization', 35.8, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '29 minutes', 'system.cpu.utilization', 36.5, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '27 minutes', 'system.cpu.utilization', 37.2, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'system.cpu.utilization', 38.0, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '23 minutes', 'system.cpu.utilization', 38.8, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '21 minutes', 'system.cpu.utilization', 39.5, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '19 minutes', 'system.cpu.utilization', 40.2, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '17 minutes', 'system.cpu.utilization', 41.0, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '15 minutes', 'system.cpu.utilization', 41.8, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '13 minutes', 'system.cpu.utilization', 42.5, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '11 minutes', 'system.cpu.utilization', 43.2, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '9 minutes',  'system.cpu.utilization', 43.8, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '7 minutes',  'system.cpu.utilization', 44.5, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '5 minutes',  'system.cpu.utilization', 45.0, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}'),
(NOW()::TIMESTAMP - INTERVAL '3 minutes',  'system.cpu.utilization', 45.5, '{}', 'robot-amr-03', '{"robot.id": "robot-amr-03"}');
