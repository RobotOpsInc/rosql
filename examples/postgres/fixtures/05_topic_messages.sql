-- Fixture: topic_messages — /odom trajectory + /battery_state
--
-- /odom: position data for PATH DEVIATION analysis
--   Action 1: smooth path from (0,0) to (5,3)
--   Action 2: deviates from planned path — veers off course before abort
--
-- /battery_state: percentage field drops below 20% during action 2

-- ============================================================================
-- /odom — Action 1: smooth path (0,0) → (5,3)
-- ============================================================================

INSERT INTO topic_messages (robot_id, topic_name, "timestamp", fields, message_type) VALUES
('robot_sim_001', '/odom', '2026-03-24T10:00:00Z', '{"position.x": "0.0",  "position.y": "0.0", "orientation.z": "0.0"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:01:00Z', '{"position.x": "0.6",  "position.y": "0.4", "orientation.z": "0.6"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:02:00Z', '{"position.x": "1.3",  "position.y": "0.8", "orientation.z": "0.5"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:03:00Z', '{"position.x": "2.0",  "position.y": "1.2", "orientation.z": "0.5"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:04:00Z', '{"position.x": "2.8",  "position.y": "1.6", "orientation.z": "0.5"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:05:00Z', '{"position.x": "3.5",  "position.y": "2.0", "orientation.z": "0.5"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:06:00Z', '{"position.x": "4.2",  "position.y": "2.5", "orientation.z": "0.5"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:07:00Z', '{"position.x": "4.8",  "position.y": "2.9", "orientation.z": "0.4"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:08:00Z', '{"position.x": "5.0",  "position.y": "3.0", "orientation.z": "0.0"}', 'nav_msgs/msg/Odometry');

-- ============================================================================
-- /odom — Action 2: deviated path — robot veers right before abort
-- Planned path would go (5,3) → (10,7) but robot drifts to y=4.5
-- ============================================================================

INSERT INTO topic_messages (robot_id, topic_name, "timestamp", fields, message_type) VALUES
('robot_sim_001', '/odom', '2026-03-24T10:09:00Z', '{"position.x": "5.0",  "position.y": "3.0", "orientation.z": "0.7"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:10:00Z', '{"position.x": "5.8",  "position.y": "3.3", "orientation.z": "0.6"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:11:00Z', '{"position.x": "6.5",  "position.y": "3.5", "orientation.z": "0.3"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:12:00Z', '{"position.x": "7.0",  "position.y": "3.6", "orientation.z": "0.1"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:13:00Z', '{"position.x": "7.5",  "position.y": "3.7", "orientation.z": "0.0"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:14:00Z', '{"position.x": "7.8",  "position.y": "3.8", "orientation.z": "-0.1"}', 'nav_msgs/msg/Odometry'),
('robot_sim_001', '/odom', '2026-03-24T10:15:00Z', '{"position.x": "7.9",  "position.y": "3.8", "orientation.z": "0.0"}', 'nav_msgs/msg/Odometry');

-- ============================================================================
-- /plan — planned path for PATH DEVIATION analysis
-- ============================================================================

INSERT INTO topic_messages (robot_id, topic_name, "timestamp", fields, message_type) VALUES
('robot_sim_001', '/plan', '2026-03-24T09:59:00Z', '{"pose.pose.position.x": "0.0", "pose.pose.position.y": "0.0"}', 'nav_msgs/msg/Path'),
('robot_sim_001', '/plan', '2026-03-24T10:08:30Z', '{"pose.pose.position.x": "5.0", "pose.pose.position.y": "3.0"}', 'nav_msgs/msg/Path');

-- ============================================================================
-- /battery_state — percentage drops during action 2
-- ============================================================================

INSERT INTO topic_messages (robot_id, topic_name, "timestamp", fields, message_type) VALUES
('robot_sim_001', '/battery_state', '2026-03-24T10:00:00Z', '{"percentage": "35", "voltage": "12.4"}', 'sensor_msgs/msg/BatteryState'),
('robot_sim_001', '/battery_state', '2026-03-24T10:03:00Z', '{"percentage": "30", "voltage": "12.2"}', 'sensor_msgs/msg/BatteryState'),
('robot_sim_001', '/battery_state', '2026-03-24T10:06:00Z', '{"percentage": "25", "voltage": "12.0"}', 'sensor_msgs/msg/BatteryState'),
('robot_sim_001', '/battery_state', '2026-03-24T10:09:00Z', '{"percentage": "22", "voltage": "11.8"}', 'sensor_msgs/msg/BatteryState'),
('robot_sim_001', '/battery_state', '2026-03-24T10:11:00Z', '{"percentage": "18", "voltage": "11.5"}', 'sensor_msgs/msg/BatteryState'),
('robot_sim_001', '/battery_state', '2026-03-24T10:13:00Z', '{"percentage": "15", "voltage": "11.2"}', 'sensor_msgs/msg/BatteryState'),
('robot_sim_001', '/battery_state', '2026-03-24T10:15:00Z', '{"percentage": "12", "voltage": "10.9"}', 'sensor_msgs/msg/BatteryState');
