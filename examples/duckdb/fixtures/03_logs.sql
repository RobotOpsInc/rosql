-- Fixture: otel_logs — /rosout logs for the 3 navigation actions

-- Action 1: Success logs
INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, log_attributes) VALUES
('2026-03-24T10:00:01Z', 'trace-001', 'span-001-root', 'INFO', 9, 'robot_sim_001',
 'Navigation goal received: navigate to waypoint A (x=5.0, y=3.0)',
 '{"ros.node": "/bt_navigator"}'),
('2026-03-24T10:04:00Z', 'trace-001', 'span-001-ctrl', 'INFO', 9, 'robot_sim_001',
 'Following path: 12 waypoints, estimated 8.0s',
 '{"ros.node": "/controller_server"}'),
('2026-03-24T10:08:00Z', 'trace-001', 'span-001-root', 'INFO', 9, 'robot_sim_001',
 'Navigation goal reached successfully',
 '{"ros.node": "/bt_navigator"}');

-- Action 2: Battery warning + abort
INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, log_attributes) VALUES
('2026-03-24T10:09:01Z', 'trace-002', 'span-002-root', 'INFO', 9, 'robot_sim_001',
 'Navigation goal received: navigate to waypoint B (x=10.0, y=7.0)',
 '{"ros.node": "/bt_navigator"}'),
('2026-03-24T10:12:00Z', 'trace-002', 'span-002-bt', 'WARN', 13, 'robot_sim_001',
 'Battery level low: 18%. Consider returning to charging station.',
 '{"ros.node": "/battery_monitor", "battery_pct": "18"}'),
('2026-03-24T10:15:00Z', 'trace-002', 'span-002-bt', 'WARN', 13, 'robot_sim_001',
 'Battery level critical: 15%. Aborting navigation.',
 '{"ros.node": "/battery_monitor", "battery_pct": "15"}'),
('2026-03-24T10:15:01Z', 'trace-002', 'span-002-root', 'ERROR', 17, 'robot_sim_001',
 'Navigation aborted: battery critical (15%). Action goal_id=goal-002 failed.',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose"}'),
('2026-03-24T10:15:02Z', 'trace-002', 'span-002-ctrl', 'ERROR', 17, 'robot_sim_001',
 'Controller stopped: navigation aborted by behavior tree',
 '{"ros.node": "/controller_server"}');

-- Action 3: Timeout
INSERT INTO otel_logs (timestamp, trace_id, span_id, severity_text, severity_number, service_name, body, log_attributes) VALUES
('2026-03-24T10:25:01Z', 'trace-003', 'span-003-root', 'INFO', 9, 'robot_sim_001',
 'Navigation goal received: navigate to waypoint C (x=15.0, y=2.0)',
 '{"ros.node": "/bt_navigator"}'),
('2026-03-24T10:45:00Z', 'trace-003', 'span-003-ctrl', 'WARN', 13, 'robot_sim_001',
 'Controller timeout approaching: no progress for 20s',
 '{"ros.node": "/controller_server"}'),
('2026-03-24T10:55:00Z', 'trace-003', 'span-003-root', 'ERROR', 17, 'robot_sim_001',
 'Navigation timed out after 30s. Action goal_id=goal-003 failed.',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose"}');
