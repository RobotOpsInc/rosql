-- Fixture: otel_traces — 3-robot AMR warehouse fleet, 9 missions + topology spans
--
-- Robot roster:
--   robot-amr-01 (v2.3.1) — reliable, 3 successful missions (~7-9s each)
--   robot-amr-02 (v2.4.0) — recently upgraded, 2 successes + 1 FAILURE
--     trace-amr02-m3: costmap timeout → path deviation → navigation abort
--     local_costmap_node/update blows out to 8s (normal: 0.5s)
--   robot-amr-03 (v2.3.0) — aging unit, 3 successes, mission 3 is slow (~13s)
--
-- Timestamps use NOW()::TIMESTAMP - INTERVAL so data is always fresh for SINCE queries.
-- Current window:  NOW()::TIMESTAMP-57min to NOW()::TIMESTAMP-8min  (missions 1-3 for all robots)
-- Topology spans:  NOW()::TIMESTAMP-15min  (for SHOW NODE GRAPH / MESSAGE FLOW)
-- Baseline window: NOW()::TIMESTAMP-14 days to NOW()::TIMESTAMP-7 days (for ANOMALY COMPARED TO last week)
--
-- Span attribute keys used by compiler queries:
--   ros.node           → compile_show_nodes, SHOW TOPICS, scope filters
--   ros.topic          → MESSAGE FLOW seed filter
--   ros.publisher_node → SHOW NODE GRAPH, SHOW TOPICS
--   ros.subscriber_node→ SHOW NODE GRAPH, SHOW TOPICS
--   ros.action.name    → action summary queries
--   ros.action.status  → action success rate
--
-- resource_attributes keys:
--   robot.id           → FOR ROBOT scope filter, FACET robot_id
--   service.version    → SHOW DEPLOYMENTS, COMPARE TO VERSION

-- ============================================================================
-- robot-amr-01 — Mission 1 (SUCCESS, 7s) at NOW()::TIMESTAMP-57min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '57 minutes',
 'trace-amr01-m1', 'span-a01-m1-root', '',
 '/navigate_to_pose', 'robot-amr-01', 7000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a01-m1"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '57 minutes' + INTERVAL '100 milliseconds',
 'trace-amr01-m1', 'span-a01-m1-bt', 'span-a01-m1-root',
 '/bt_navigator/navigate', 'robot-amr-01', 6900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '57 minutes' + INTERVAL '200 milliseconds',
 'trace-amr01-m1', 'span-a01-m1-ctrl', 'span-a01-m1-bt',
 '/controller_server/follow_path', 'robot-amr-01', 6800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel", "ros.publisher_node": "/controller_server", "ros.subscriber_node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '57 minutes' + INTERVAL '300 milliseconds',
 'trace-amr01-m1', 'span-a01-m1-costmap', 'span-a01-m1-ctrl',
 '/local_costmap_node/update', 'robot-amr-01', 500000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan", "ros.publisher_node": "/lidar_node", "ros.subscriber_node": "/local_costmap_node"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}');

-- ============================================================================
-- robot-amr-01 — Mission 2 (SUCCESS, 8s) at NOW()::TIMESTAMP-43min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '43 minutes',
 'trace-amr01-m2', 'span-a01-m2-root', '',
 '/navigate_to_pose', 'robot-amr-01', 8000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a01-m2"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '43 minutes' + INTERVAL '100 milliseconds',
 'trace-amr01-m2', 'span-a01-m2-bt', 'span-a01-m2-root',
 '/bt_navigator/navigate', 'robot-amr-01', 7900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '43 minutes' + INTERVAL '200 milliseconds',
 'trace-amr01-m2', 'span-a01-m2-ctrl', 'span-a01-m2-bt',
 '/controller_server/follow_path', 'robot-amr-01', 7800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '43 minutes' + INTERVAL '300 milliseconds',
 'trace-amr01-m2', 'span-a01-m2-costmap', 'span-a01-m2-ctrl',
 '/local_costmap_node/update', 'robot-amr-01', 480000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}');

-- ============================================================================
-- robot-amr-01 — Mission 3 (SUCCESS, 9s) at NOW()::TIMESTAMP-27min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '27 minutes',
 'trace-amr01-m3', 'span-a01-m3-root', '',
 '/navigate_to_pose', 'robot-amr-01', 9000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a01-m3"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '27 minutes' + INTERVAL '100 milliseconds',
 'trace-amr01-m3', 'span-a01-m3-bt', 'span-a01-m3-root',
 '/bt_navigator/navigate', 'robot-amr-01', 8900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '27 minutes' + INTERVAL '200 milliseconds',
 'trace-amr01-m3', 'span-a01-m3-ctrl', 'span-a01-m3-bt',
 '/controller_server/follow_path', 'robot-amr-01', 8800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}'),
(NOW()::TIMESTAMP - INTERVAL '27 minutes' + INTERVAL '300 milliseconds',
 'trace-amr01-m3', 'span-a01-m3-costmap', 'span-a01-m3-ctrl',
 '/local_costmap_node/update', 'robot-amr-01', 520000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}',
 '{"robot.id": "robot-amr-01", "service.version": "v2.3.1"}');

-- ============================================================================
-- robot-amr-02 — Mission 1 (SUCCESS, 7s) at NOW()::TIMESTAMP-56min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '56 minutes',
 'trace-amr02-m1', 'span-a02-m1-root', '',
 '/navigate_to_pose', 'robot-amr-02', 7000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a02-m1"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
(NOW()::TIMESTAMP - INTERVAL '56 minutes' + INTERVAL '100 milliseconds',
 'trace-amr02-m1', 'span-a02-m1-bt', 'span-a02-m1-root',
 '/bt_navigator/navigate', 'robot-amr-02', 6900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
(NOW()::TIMESTAMP - INTERVAL '56 minutes' + INTERVAL '200 milliseconds',
 'trace-amr02-m1', 'span-a02-m1-ctrl', 'span-a02-m1-bt',
 '/controller_server/follow_path', 'robot-amr-02', 6800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
(NOW()::TIMESTAMP - INTERVAL '56 minutes' + INTERVAL '300 milliseconds',
 'trace-amr02-m1', 'span-a02-m1-costmap', 'span-a02-m1-ctrl',
 '/local_costmap_node/update', 'robot-amr-02', 490000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}');

-- ============================================================================
-- robot-amr-02 — Mission 2 (SUCCESS, 8s) at NOW()::TIMESTAMP-42min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '42 minutes',
 'trace-amr02-m2', 'span-a02-m2-root', '',
 '/navigate_to_pose', 'robot-amr-02', 8000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a02-m2"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
(NOW()::TIMESTAMP - INTERVAL '42 minutes' + INTERVAL '100 milliseconds',
 'trace-amr02-m2', 'span-a02-m2-bt', 'span-a02-m2-root',
 '/bt_navigator/navigate', 'robot-amr-02', 7900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
(NOW()::TIMESTAMP - INTERVAL '42 minutes' + INTERVAL '200 milliseconds',
 'trace-amr02-m2', 'span-a02-m2-ctrl', 'span-a02-m2-bt',
 '/controller_server/follow_path', 'robot-amr-02', 7800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
(NOW()::TIMESTAMP - INTERVAL '42 minutes' + INTERVAL '300 milliseconds',
 'trace-amr02-m2', 'span-a02-m2-costmap', 'span-a02-m2-bt',
 '/local_costmap_node/update', 'robot-amr-02', 510000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}');

-- ============================================================================
-- robot-amr-02 — Mission 3 (FAILURE, 18s) at NOW()::TIMESTAMP-26min  ← THE KEY TRACE
--
-- Investigation story:
--   /local_costmap_node/update times out (8s vs normal 0.5s) due to a stale
--   /scan subscription after the v2.4.0 firmware upgrade. The behavior tree
--   triggers recovery, then aborts. CPU spikes to 92%.
--
-- Span tree for MESSAGE FLOW FROM TOPIC '/scan':
--   span-a02-m3-scan  (ros.topic='/scan', seed for recursive CTE)
--   └── span-a02-m3-costmap  (child: processes /scan → produces /costmap)
--       └── span-a02-m3-ctrl (grandchild: /costmap → /cmd_vel)
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
-- Root: navigation action aborted
(NOW()::TIMESTAMP - INTERVAL '26 minutes',
 'trace-amr02-m3', 'span-a02-m3-root', '',
 '/navigate_to_pose', 'robot-amr-02', 18000000000, 'ERROR',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "aborted", "ros.action.goal_id": "goal-a02-m3"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- bt_navigator orchestrates the mission
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '50 milliseconds',
 'trace-amr02-m3', 'span-a02-m3-bt', 'span-a02-m3-root',
 '/bt_navigator/navigate', 'robot-amr-02', 17900000000, 'ERROR',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- global_planner produces the /plan (fast, OK)
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '100 milliseconds',
 'trace-amr02-m3', 'span-a02-m3-plan', 'span-a02-m3-bt',
 '/global_planner/make_plan', 'robot-amr-02', 450000000, 'OK',
 '{"ros.node": "/global_planner", "ros.topic": "/plan", "ros.publisher_node": "/global_planner", "ros.subscriber_node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- /scan ingestion: seed for MESSAGE FLOW FROM TOPIC '/scan'
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '150 milliseconds',
 'trace-amr02-m3', 'span-a02-m3-scan', 'span-a02-m3-bt',
 '/local_costmap_node/receive_scan', 'robot-amr-02', 80000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan", "ros.publisher_node": "/lidar_node", "ros.subscriber_node": "/local_costmap_node"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- *** THE SMOKING GUN: costmap update hangs for 8 seconds ***
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '250 milliseconds',
 'trace-amr02-m3', 'span-a02-m3-costmap', 'span-a02-m3-scan',
 '/local_costmap_node/update', 'robot-amr-02', 8000000000, 'ERROR',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/costmap", "ros.publisher_node": "/local_costmap_node", "ros.subscriber_node": "/controller_server"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- controller_server fails to compute a path (downstream of blocked costmap)
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '8300 milliseconds',
 'trace-amr02-m3', 'span-a02-m3-ctrl', 'span-a02-m3-costmap',
 '/controller_server/compute_velocity', 'robot-amr-02', 9500000000, 'ERROR',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel", "ros.publisher_node": "/controller_server", "ros.subscriber_node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}');

-- ============================================================================
-- robot-amr-03 — Mission 1 (SUCCESS, 10s) at NOW()::TIMESTAMP-55min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '55 minutes',
 'trace-amr03-m1', 'span-a03-m1-root', '',
 '/navigate_to_pose', 'robot-amr-03', 10000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a03-m1"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '55 minutes' + INTERVAL '100 milliseconds',
 'trace-amr03-m1', 'span-a03-m1-bt', 'span-a03-m1-root',
 '/bt_navigator/navigate', 'robot-amr-03', 9900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '55 minutes' + INTERVAL '200 milliseconds',
 'trace-amr03-m1', 'span-a03-m1-ctrl', 'span-a03-m1-bt',
 '/controller_server/follow_path', 'robot-amr-03', 9800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '55 minutes' + INTERVAL '300 milliseconds',
 'trace-amr03-m1', 'span-a03-m1-costmap', 'span-a03-m1-ctrl',
 '/local_costmap_node/update', 'robot-amr-03', 600000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}');

-- ============================================================================
-- robot-amr-03 — Mission 2 (SUCCESS, 11s) at NOW()::TIMESTAMP-41min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '41 minutes',
 'trace-amr03-m2', 'span-a03-m2-root', '',
 '/navigate_to_pose', 'robot-amr-03', 11000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a03-m2"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '41 minutes' + INTERVAL '100 milliseconds',
 'trace-amr03-m2', 'span-a03-m2-bt', 'span-a03-m2-root',
 '/bt_navigator/navigate', 'robot-amr-03', 10900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '41 minutes' + INTERVAL '200 milliseconds',
 'trace-amr03-m2', 'span-a03-m2-ctrl', 'span-a03-m2-bt',
 '/controller_server/follow_path', 'robot-amr-03', 10800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '41 minutes' + INTERVAL '300 milliseconds',
 'trace-amr03-m2', 'span-a03-m2-costmap', 'span-a03-m2-ctrl',
 '/local_costmap_node/update', 'robot-amr-03', 650000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}');

-- ============================================================================
-- robot-amr-03 — Mission 3 (SUCCESS, 13s, slow) at NOW()::TIMESTAMP-25min
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
(NOW()::TIMESTAMP - INTERVAL '25 minutes',
 'trace-amr03-m3', 'span-a03-m3-root', '',
 '/navigate_to_pose', 'robot-amr-03', 13000000000, 'OK',
 '{"ros.node": "/bt_navigator", "ros.action.name": "/navigate_to_pose", "ros.action.status": "succeeded", "ros.action.goal_id": "goal-a03-m3"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '25 minutes' + INTERVAL '100 milliseconds',
 'trace-amr03-m3', 'span-a03-m3-bt', 'span-a03-m3-root',
 '/bt_navigator/navigate', 'robot-amr-03', 12900000000, 'OK',
 '{"ros.node": "/bt_navigator"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '25 minutes' + INTERVAL '200 milliseconds',
 'trace-amr03-m3', 'span-a03-m3-ctrl', 'span-a03-m3-bt',
 '/controller_server/follow_path', 'robot-amr-03', 12800000000, 'OK',
 '{"ros.node": "/controller_server", "ros.topic": "/cmd_vel"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}'),
(NOW()::TIMESTAMP - INTERVAL '25 minutes' + INTERVAL '300 milliseconds',
 'trace-amr03-m3', 'span-a03-m3-costmap', 'span-a03-m3-ctrl',
 '/local_costmap_node/update', 'robot-amr-03', 1800000000, 'OK',
 '{"ros.node": "/local_costmap_node", "ros.topic": "/scan"}',
 '{"robot.id": "robot-amr-03", "service.version": "v2.3.0"}');

-- ============================================================================
-- Nav2 topology spans for SHOW NODE GRAPH FOR ROBOT 'robot-amr-02'
--
-- These spans record the ROS2 pub/sub graph for robot-amr-02.
-- Each row represents one topic edge: publisher_node → topic → subscriber_node.
-- SHOW NODE GRAPH queries: SELECT DISTINCT publisher_node, subscriber_node, topic
-- ============================================================================

INSERT INTO otel_traces (timestamp, trace_id, span_id, parent_span_id, span_name, service_name, duration, status_code, span_attributes, resource_attributes) VALUES
-- /lidar_node → /scan → /local_costmap_node
(NOW()::TIMESTAMP - INTERVAL '15 minutes',
 'trace-amr02-topo', 'span-topo-01', '',
 'ros2.graph.edge', 'robot-amr-02', 1000000, 'OK',
 '{"ros.publisher_node": "/lidar_node", "ros.subscriber_node": "/local_costmap_node", "ros.topic": "/scan", "ros.node": "/lidar_node"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- /lidar_node → /scan → /global_costmap_node
(NOW()::TIMESTAMP - INTERVAL '15 minutes',
 'trace-amr02-topo', 'span-topo-02', '',
 'ros2.graph.edge', 'robot-amr-02', 1000000, 'OK',
 '{"ros.publisher_node": "/lidar_node", "ros.subscriber_node": "/global_costmap_node", "ros.topic": "/scan", "ros.node": "/lidar_node"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- /local_costmap_node → /costmap → /controller_server
(NOW()::TIMESTAMP - INTERVAL '15 minutes',
 'trace-amr02-topo', 'span-topo-03', '',
 'ros2.graph.edge', 'robot-amr-02', 1000000, 'OK',
 '{"ros.publisher_node": "/local_costmap_node", "ros.subscriber_node": "/controller_server", "ros.topic": "/costmap", "ros.node": "/local_costmap_node"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- /global_planner → /plan → /bt_navigator
(NOW()::TIMESTAMP - INTERVAL '15 minutes',
 'trace-amr02-topo', 'span-topo-04', '',
 'ros2.graph.edge', 'robot-amr-02', 1000000, 'OK',
 '{"ros.publisher_node": "/global_planner", "ros.subscriber_node": "/bt_navigator", "ros.topic": "/plan", "ros.node": "/global_planner"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- /global_costmap_node → /global_costmap → /global_planner
(NOW()::TIMESTAMP - INTERVAL '15 minutes',
 'trace-amr02-topo', 'span-topo-05', '',
 'ros2.graph.edge', 'robot-amr-02', 1000000, 'OK',
 '{"ros.publisher_node": "/global_costmap_node", "ros.subscriber_node": "/global_planner", "ros.topic": "/global_costmap", "ros.node": "/global_costmap_node"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- /odom_node → /odom → /controller_server
(NOW()::TIMESTAMP - INTERVAL '15 minutes',
 'trace-amr02-topo', 'span-topo-06', '',
 'ros2.graph.edge', 'robot-amr-02', 1000000, 'OK',
 '{"ros.publisher_node": "/odom_node", "ros.subscriber_node": "/controller_server", "ros.topic": "/odom", "ros.node": "/odom_node"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}'),
-- /controller_server → /cmd_vel → /cmd_vel_mux
(NOW()::TIMESTAMP - INTERVAL '15 minutes',
 'trace-amr02-topo', 'span-topo-07', '',
 'ros2.graph.edge', 'robot-amr-02', 1000000, 'OK',
 '{"ros.publisher_node": "/controller_server", "ros.subscriber_node": "/cmd_vel_mux", "ros.topic": "/cmd_vel", "ros.node": "/controller_server"}',
 '{"robot.id": "robot-amr-02", "service.version": "v2.4.0"}');
