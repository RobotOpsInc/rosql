-- Fixture: ros2_events — deployment events and node lifecycle events
--
-- Deployment events enable SHOW DEPLOYMENTS and version correlation.
-- Node lifecycle events record when Nav2 nodes started/stopped.
--
-- Firmware versions across the fleet:
--   v2.3.0 → initial release (robot-amr-03 still running this)
--   v2.3.1 → stability fix for costmap (robot-amr-01 upgraded)
--   v2.4.0 → new planner algorithm, rolled out to robot-amr-02 (introduced bug)

-- ============================================================================
-- Deployment events (version rollouts)
-- ============================================================================

INSERT INTO ros2_events (timestamp, robot_id, event_type, node_name, version, payload) VALUES
-- v2.3.0 initial deployment — all robots
(NOW()::TIMESTAMP - INTERVAL '21 days',
 'robot-amr-01', 'deployment', '', 'v2.3.0',
 '{"environment": "warehouse-prod", "deployed_by": "fleet-controller", "notes": "Initial release"}'),
(NOW()::TIMESTAMP - INTERVAL '21 days',
 'robot-amr-02', 'deployment', '', 'v2.3.0',
 '{"environment": "warehouse-prod", "deployed_by": "fleet-controller", "notes": "Initial release"}'),
(NOW()::TIMESTAMP - INTERVAL '21 days',
 'robot-amr-03', 'deployment', '', 'v2.3.0',
 '{"environment": "warehouse-prod", "deployed_by": "fleet-controller", "notes": "Initial release"}'),

-- v2.3.1 hotfix — deployed to amr-01 only (amr-03 not yet upgraded)
(NOW()::TIMESTAMP - INTERVAL '14 days',
 'robot-amr-01', 'deployment', '', 'v2.3.1',
 '{"environment": "warehouse-prod", "deployed_by": "fleet-controller", "notes": "Costmap stability fix: reduced update timeout from 2s to 1s"}'),

-- v2.4.0 new planner — deployed to amr-02 only (experimental)
(NOW()::TIMESTAMP - INTERVAL '2 days',
 'robot-amr-02', 'deployment', '', 'v2.4.0',
 '{"environment": "warehouse-prod", "deployed_by": "fleet-controller", "notes": "New global planner algorithm + faster nav2 bringup. Known issue: /scan subscription lag under investigation."}');

-- ============================================================================
-- Node lifecycle events for robot-amr-02 (for SHOW NODE GRAPH context)
-- ============================================================================

INSERT INTO ros2_events (timestamp, robot_id, event_type, node_name, version, payload) VALUES
-- Nav2 nodes come up at mission start
(NOW()::TIMESTAMP - INTERVAL '70 minutes',
 'robot-amr-02', 'node_started', '/bt_navigator', 'v2.4.0',
 '{"state": "active", "lifecycle_state": "active"}'),
(NOW()::TIMESTAMP - INTERVAL '70 minutes',
 'robot-amr-02', 'node_started', '/controller_server', 'v2.4.0',
 '{"state": "active", "lifecycle_state": "active"}'),
(NOW()::TIMESTAMP - INTERVAL '70 minutes',
 'robot-amr-02', 'node_started', '/local_costmap_node', 'v2.4.0',
 '{"state": "active", "lifecycle_state": "active"}'),
(NOW()::TIMESTAMP - INTERVAL '70 minutes',
 'robot-amr-02', 'node_started', '/global_planner', 'v2.4.0',
 '{"state": "active", "lifecycle_state": "active"}'),
(NOW()::TIMESTAMP - INTERVAL '70 minutes',
 'robot-amr-02', 'node_started', '/lidar_node', 'v2.4.0',
 '{"state": "active", "lifecycle_state": "active"}'),
-- /local_costmap_node briefly enters error state during mission 3 failure
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '8 seconds',
 'robot-amr-02', 'node_error', '/local_costmap_node', 'v2.4.0',
 '{"state": "error", "reason": "costmap_update_timeout", "elapsed_ms": "8000"}'),
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '18 seconds',
 'robot-amr-02', 'node_started', '/local_costmap_node', 'v2.4.0',
 '{"state": "active", "lifecycle_state": "active", "reason": "recovery_restart"}');
