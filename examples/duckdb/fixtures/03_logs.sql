-- Fixture: otel_logs — /rosout logs for all 3 robots across all 9 missions
--
-- Every trace_id here exists in otel_traces (referential integrity requirement).
-- Severity distribution: mostly INFO, WARN/ERROR only for trace-amr02-m3 failure.
-- Timestamps match their parent trace windows (within same NOW()::TIMESTAMP-relative frame).

-- ============================================================================
-- robot-amr-01 — missions 1-3 (all OK, INFO only)
-- ============================================================================

INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, resource_attributes, log_attributes) VALUES
-- Mission 1
(NOW()::TIMESTAMP - INTERVAL '57 minutes' + INTERVAL '1 second',
 'trace-amr01-m1', 'span-a01-m1-root', 'INFO', 9, 'robot-amr-01',
 'Navigation goal received: navigate to zone A (x=8.0, y=4.0)',
 '{"robot.id": "robot-amr-01"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '57 minutes' + INTERVAL '3 seconds',
 'trace-amr01-m1', 'span-a01-m1-ctrl', 'INFO', 9, 'robot-amr-01',
 'Following planned path: 14 waypoints, estimated 7.0s',
 '{"robot.id": "robot-amr-01"}',
 '{"ros.node": "/controller_server"}'),
(NOW()::TIMESTAMP - INTERVAL '57 minutes' + INTERVAL '7 seconds',
 'trace-amr01-m1', 'span-a01-m1-root', 'INFO', 9, 'robot-amr-01',
 'Navigation goal reached successfully',
 '{"robot.id": "robot-amr-01"}',
 '{"ros.node": "/bt_navigator"}'),
-- Mission 2
(NOW()::TIMESTAMP - INTERVAL '43 minutes' + INTERVAL '1 second',
 'trace-amr01-m2', 'span-a01-m2-root', 'INFO', 9, 'robot-amr-01',
 'Navigation goal received: navigate to zone B (x=12.0, y=6.0)',
 '{"robot.id": "robot-amr-01"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '43 minutes' + INTERVAL '8 seconds',
 'trace-amr01-m2', 'span-a01-m2-root', 'INFO', 9, 'robot-amr-01',
 'Navigation goal reached successfully',
 '{"robot.id": "robot-amr-01"}',
 '{"ros.node": "/bt_navigator"}'),
-- Mission 3
(NOW()::TIMESTAMP - INTERVAL '27 minutes' + INTERVAL '1 second',
 'trace-amr01-m3', 'span-a01-m3-root', 'INFO', 9, 'robot-amr-01',
 'Navigation goal received: navigate to zone C (x=5.0, y=9.0)',
 '{"robot.id": "robot-amr-01"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '27 minutes' + INTERVAL '9 seconds',
 'trace-amr01-m3', 'span-a01-m3-root', 'INFO', 9, 'robot-amr-01',
 'Navigation goal reached successfully',
 '{"robot.id": "robot-amr-01"}',
 '{"ros.node": "/bt_navigator"}');

-- ============================================================================
-- robot-amr-02 — Mission 1 (OK, INFO only)
-- ============================================================================

INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, resource_attributes, log_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '56 minutes' + INTERVAL '1 second',
 'trace-amr02-m1', 'span-a02-m1-root', 'INFO', 9, 'robot-amr-02',
 'Navigation goal received: navigate to drop-off A (x=10.0, y=5.0)',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '56 minutes' + INTERVAL '7 seconds',
 'trace-amr02-m1', 'span-a02-m1-root', 'INFO', 9, 'robot-amr-02',
 'Navigation goal reached successfully',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/bt_navigator"}');

-- ============================================================================
-- robot-amr-02 — Mission 2 (OK, INFO only)
-- ============================================================================

INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, resource_attributes, log_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '42 minutes' + INTERVAL '1 second',
 'trace-amr02-m2', 'span-a02-m2-root', 'INFO', 9, 'robot-amr-02',
 'Navigation goal received: navigate to pick-up B (x=14.0, y=3.0)',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '42 minutes' + INTERVAL '8 seconds',
 'trace-amr02-m2', 'span-a02-m2-root', 'INFO', 9, 'robot-amr-02',
 'Navigation goal reached successfully',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/bt_navigator"}');

-- ============================================================================
-- robot-amr-02 — Mission 3 (FAILURE) — ERROR and WARN logs  ← ENRICH target
--
-- These logs are what "TRACE 'trace-amr02-m3' ENRICH WITH logs LIMIT 5" returns.
-- severity_number: 9=INFO, 13=WARN, 17=ERROR
-- ============================================================================

INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, resource_attributes, log_attributes) VALUES
-- Initial goal accepted
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '1 second',
 'trace-amr02-m3', 'span-a02-m3-root', 'INFO', 9, 'robot-amr-02',
 'Navigation goal received: navigate to dock C (x=18.0, y=8.0)',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/bt_navigator"}'),
-- Plan computed OK
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '2 seconds',
 'trace-amr02-m3', 'span-a02-m3-plan', 'INFO', 9, 'robot-amr-02',
 'Global plan computed: 22 waypoints, 18.0m estimated distance',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/global_planner"}'),
-- *** First WARNING: costmap update is taking longer than expected
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '3 seconds',
 'trace-amr02-m3', 'span-a02-m3-costmap', 'WARN', 13, 'robot-amr-02',
 'Costmap update taking longer than expected: 1.5s elapsed (threshold: 1.0s)',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/local_costmap_node", "elapsed_ms": "1500"}'),
-- *** Second WARNING: recovery behavior triggered
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '7 seconds',
 'trace-amr02-m3', 'span-a02-m3-bt', 'WARN', 13, 'robot-amr-02',
 'Recovery behavior triggered: ClearCostmapService (costmap stale, timeout=8.0s)',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/bt_navigator", "recovery_type": "ClearCostmapService"}'),
-- *** ERROR: costmap update timed out — the root cause
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '8300 milliseconds',
 'trace-amr02-m3', 'span-a02-m3-costmap', 'ERROR', 17, 'robot-amr-02',
 'Costmap update timed out after 8.0s — /scan topic not publishing (subscriber lag). This may indicate a node lifecycle issue following the v2.4.0 firmware upgrade.',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/local_costmap_node", "timeout_ms": "8000", "topic": "/scan"}'),
-- *** ERROR: navigation aborted
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '17 seconds',
 'trace-amr02-m3', 'span-a02-m3-root', 'ERROR', 17, 'robot-amr-02',
 'Navigation aborted: controller failed to compute velocity after costmap timeout. Goal goal-a02-m3 failed.',
 '{"robot.id": "robot-amr-02"}',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose"}');

-- ============================================================================
-- robot-amr-03 — missions 1-3 (all OK, some INFO/WARN for slow mission 3)
-- ============================================================================

INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, resource_attributes, log_attributes) VALUES
-- Mission 1
(NOW()::TIMESTAMP - INTERVAL '55 minutes' + INTERVAL '1 second',
 'trace-amr03-m1', 'span-a03-m1-root', 'INFO', 9, 'robot-amr-03',
 'Navigation goal received: navigate to shelf A (x=6.0, y=3.0)',
 '{"robot.id": "robot-amr-03"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '55 minutes' + INTERVAL '10 seconds',
 'trace-amr03-m1', 'span-a03-m1-root', 'INFO', 9, 'robot-amr-03',
 'Navigation goal reached successfully',
 '{"robot.id": "robot-amr-03"}',
 '{"ros.node": "/bt_navigator"}'),
-- Mission 2
(NOW()::TIMESTAMP - INTERVAL '41 minutes' + INTERVAL '1 second',
 'trace-amr03-m2', 'span-a03-m2-root', 'INFO', 9, 'robot-amr-03',
 'Navigation goal received: navigate to shelf B (x=11.0, y=7.0)',
 '{"robot.id": "robot-amr-03"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '41 minutes' + INTERVAL '11 seconds',
 'trace-amr03-m2', 'span-a03-m2-root', 'INFO', 9, 'robot-amr-03',
 'Navigation goal reached successfully',
 '{"robot.id": "robot-amr-03"}',
 '{"ros.node": "/bt_navigator"}'),
-- Mission 3 (slow — elevated costmap lag)
(NOW()::TIMESTAMP - INTERVAL '25 minutes' + INTERVAL '1 second',
 'trace-amr03-m3', 'span-a03-m3-root', 'INFO', 9, 'robot-amr-03',
 'Navigation goal received: navigate to charging station (x=2.0, y=1.0)',
 '{"robot.id": "robot-amr-03"}',
 '{"ros.node": "/bt_navigator"}'),
(NOW()::TIMESTAMP - INTERVAL '25 minutes' + INTERVAL '2 seconds',
 'trace-amr03-m3', 'span-a03-m3-costmap', 'WARN', 13, 'robot-amr-03',
 'Costmap update slower than nominal: 1.8s (hardware may be degrading)',
 '{"robot.id": "robot-amr-03"}',
 '{"ros.node": "/local_costmap_node", "elapsed_ms": "1800"}'),
(NOW()::TIMESTAMP - INTERVAL '25 minutes' + INTERVAL '13 seconds',
 'trace-amr03-m3', 'span-a03-m3-root', 'INFO', 9, 'robot-amr-03',
 'Navigation goal reached successfully (slow path due to elevated costmap lag)',
 '{"robot.id": "robot-amr-03"}',
 '{"ros.node": "/bt_navigator"}');
