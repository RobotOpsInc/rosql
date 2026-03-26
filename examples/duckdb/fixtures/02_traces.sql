-- Fixture: otel_traces — 3 navigation actions for robot_sim_001
--
-- Action 1 (trace-001): /navigate_to_pose to waypoint A → SUCCEEDS (~8s)
-- Action 2 (trace-002): /navigate_to_pose to waypoint B → ABORTS (low battery, ~12s)
-- Action 3 (trace-003): /navigate_to_pose to waypoint C → TIMES OUT (~30s)
--
-- Each action has a span chain: root → bt_navigator → controller_server → local_costmap_node
-- parent_span_id links create the causality chain for MESSAGE JOURNEY queries.

-- ============================================================================
-- Action 1: SUCCESS — waypoint A (10:00:00 – 10:08:00)
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name_col, service_name, duration, status_code, span_attributes) VALUES
('2026-03-24T10:00:00Z', 'trace-001', 'span-001-root', '', '/navigate_to_pose', 'robot_sim_001', 8000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-001"}'),
('2026-03-24T10:00:00.1Z', 'trace-001', 'span-001-bt', 'span-001-root', '/bt_navigator/navigate', 'robot_sim_001', 7900000000, 'OK',
 '{"ros.node": "/bt_navigator"}'),
('2026-03-24T10:00:00.2Z', 'trace-001', 'span-001-ctrl', 'span-001-bt', '/controller_server/follow_path', 'robot_sim_001', 7800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}'),
('2026-03-24T10:00:00.3Z', 'trace-001', 'span-001-costmap', 'span-001-ctrl', '/local_costmap_node/update', 'robot_sim_001', 500000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}');

-- ============================================================================
-- Action 2: ABORTED — waypoint B, low battery (10:09:00 – 10:21:00)
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name_col, service_name, duration, status_code, span_attributes) VALUES
('2026-03-24T10:09:00Z', 'trace-002', 'span-002-root', '', '/navigate_to_pose', 'robot_sim_001', 12000000000, 'ERROR',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "aborted", "ros.action.goal_id": "goal-002"}'),
('2026-03-24T10:09:00.1Z', 'trace-002', 'span-002-bt', 'span-002-root', '/bt_navigator/navigate', 'robot_sim_001', 11900000000, 'ERROR',
 '{"ros.node": "/bt_navigator"}'),
('2026-03-24T10:09:00.2Z', 'trace-002', 'span-002-ctrl', 'span-002-bt', '/controller_server/follow_path', 'robot_sim_001', 11800000000, 'ERROR',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}'),
('2026-03-24T10:09:00.3Z', 'trace-002', 'span-002-costmap', 'span-002-ctrl', '/local_costmap_node/update', 'robot_sim_001', 600000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}');

-- ============================================================================
-- Action 3: TIMED OUT — waypoint C (10:25:00 – 10:55:00)
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name_col, service_name, duration, status_code, span_attributes) VALUES
('2026-03-24T10:25:00Z', 'trace-003', 'span-003-root', '', '/navigate_to_pose', 'robot_sim_001', 30000000000, 'ERROR',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "aborted", "ros.action.goal_id": "goal-003"}'),
('2026-03-24T10:25:00.1Z', 'trace-003', 'span-003-bt', 'span-003-root', '/bt_navigator/navigate', 'robot_sim_001', 29900000000, 'ERROR',
 '{"ros.node": "/bt_navigator"}'),
('2026-03-24T10:25:00.2Z', 'trace-003', 'span-003-ctrl', 'span-003-bt', '/controller_server/follow_path', 'robot_sim_001', 29800000000, 'ERROR',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}');
