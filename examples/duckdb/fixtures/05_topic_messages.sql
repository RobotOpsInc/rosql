-- Fixture: topic_messages — /plan, /odom, /battery_state for all 3 robots
--
-- trace_id is populated for PATH DEVIATION FOR TRACE 'trace-amr02-m3' to work.
-- Battery data for robot-amr-02 drops below 11.5V within the last 2 hours.
-- Path deviation for trace-amr02-m3: /odom drifts from /plan significantly (>0.5m).

-- ============================================================================
-- /plan — goal poses for each robot's mission 3
-- (PATH DEVIATION FOR TRACE needs /plan and /odom with trace_id = 'trace-amr02-m3')
-- ============================================================================

INSERT INTO topic_messages (robot_id, topic_name, timestamp, trace_id, fields, message_type) VALUES
-- amr-01 mission 3 plan
('robot-amr-01', '/plan', NOW()::TIMESTAMP - INTERVAL '27 minutes' - INTERVAL '5 seconds',
 'trace-amr01-m3',
 '{"pose.pose.position.x": "5.0", "pose.pose.position.y": "9.0", "waypoints": "16"}',
 'nav_msgs/msg/Path'),
-- amr-02 mission 3 plan — the mission that fails
('robot-amr-02', '/plan', NOW()::TIMESTAMP - INTERVAL '26 minutes' - INTERVAL '5 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "18.0", "pose.pose.position.y": "8.0", "waypoints": "22"}',
 'nav_msgs/msg/Path'),
-- amr-03 mission 3 plan
('robot-amr-03', '/plan', NOW()::TIMESTAMP - INTERVAL '25 minutes' - INTERVAL '5 seconds',
 'trace-amr03-m3',
 '{"pose.pose.position.x": "2.0", "pose.pose.position.y": "1.0", "waypoints": "8"}',
 'nav_msgs/msg/Path');

-- ============================================================================
-- /odom — robot-amr-02 mission 3: deviates from planned path (>0.5m deviation)
--
-- Plan: (0,0) → (18,8). Robot drifts laterally as costmap stalls.
-- Lateral deviation grows from 0 to ~2.1m before abort.
-- trace_id = 'trace-amr02-m3' so PATH DEVIATION FOR TRACE finds these rows.
-- ============================================================================

INSERT INTO topic_messages (robot_id, topic_name, timestamp, trace_id, fields, message_type) VALUES
(NOW()::TIMESTAMP - INTERVAL '26 minutes',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "0.0", "pose.pose.position.y": "0.0", "orientation.z": "0.4"}',
 'nav_msgs/msg/Odometry'),
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '2 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '2 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "0.9", "pose.pose.position.y": "0.5", "orientation.z": "0.42"}',
 'nav_msgs/msg/Odometry'),
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '4 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '4 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "1.8", "pose.pose.position.y": "0.8", "orientation.z": "0.3"}',
 'nav_msgs/msg/Odometry'),
-- costmap starts to stall — robot slows and begins to drift
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '6 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '6 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "2.3", "pose.pose.position.y": "0.8", "orientation.z": "0.1"}',
 'nav_msgs/msg/Odometry'),
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '8 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '8 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "2.6", "pose.pose.position.y": "0.5", "orientation.z": "-0.1"}',
 'nav_msgs/msg/Odometry'),
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '10 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '10 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "2.9", "pose.pose.position.y": "0.2", "orientation.z": "-0.3"}',
 'nav_msgs/msg/Odometry'),
-- Significant lateral drift (~1.5m below planned y)
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '12 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '12 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "3.1", "pose.pose.position.y": "-0.3", "orientation.z": "-0.4"}',
 'nav_msgs/msg/Odometry'),
-- Max deviation ~2.1m before abort
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '14 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '14 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "3.2", "pose.pose.position.y": "-0.5", "orientation.z": "-0.35"}',
 'nav_msgs/msg/Odometry'),
-- Navigation aborted — robot stops
(NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '17 seconds',
 '/odom', NOW()::TIMESTAMP - INTERVAL '26 minutes' + INTERVAL '17 seconds',
 'trace-amr02-m3',
 '{"pose.pose.position.x": "3.2", "pose.pose.position.y": "-0.5", "orientation.z": "0.0"}',
 'nav_msgs/msg/Odometry');

-- ============================================================================
-- /battery_state — robot-amr-02 (gradual drain, drops below 11.5V within 2h)
--
-- Query: FROM topics WHERE topic_name = '/battery_state'
--          AND fields['voltage'] < 11.5 V
--          FOR ROBOT 'robot-amr-02' SINCE 2 h ago
-- Rows below 11.5V: all readings from NOW()::TIMESTAMP-50min onward for robot-amr-02
-- ============================================================================

INSERT INTO topic_messages (robot_id, topic_name, timestamp, trace_id, fields, message_type) VALUES
-- robot-amr-01 battery — healthy, stays well above 11.5V
('robot-amr-01', '/battery_state', NOW()::TIMESTAMP - INTERVAL '115 minutes', '',
 '{"percentage": "78", "voltage": "12.8", "current": "-4.2", "temperature": "22.1"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-01', '/battery_state', NOW()::TIMESTAMP - INTERVAL '95 minutes', '',
 '{"percentage": "74", "voltage": "12.7", "current": "-4.3", "temperature": "22.3"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-01', '/battery_state', NOW()::TIMESTAMP - INTERVAL '75 minutes', '',
 '{"percentage": "70", "voltage": "12.6", "current": "-4.1", "temperature": "22.5"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-01', '/battery_state', NOW()::TIMESTAMP - INTERVAL '55 minutes', '',
 '{"percentage": "66", "voltage": "12.5", "current": "-4.4", "temperature": "22.8"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-01', '/battery_state', NOW()::TIMESTAMP - INTERVAL '35 minutes', '',
 '{"percentage": "62", "voltage": "12.4", "current": "-4.2", "temperature": "23.0"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-01', '/battery_state', NOW()::TIMESTAMP - INTERVAL '15 minutes', '',
 '{"percentage": "58", "voltage": "12.3", "current": "-4.1", "temperature": "23.2"}',
 'sensor_msgs/msg/BatteryState'),

-- robot-amr-02 battery — draining fast, drops below 11.5V
('robot-amr-02', '/battery_state', NOW()::TIMESTAMP - INTERVAL '115 minutes', '',
 '{"percentage": "45", "voltage": "12.2", "current": "-5.8", "temperature": "28.5"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-02', '/battery_state', NOW()::TIMESTAMP - INTERVAL '95 minutes', '',
 '{"percentage": "38", "voltage": "12.0", "current": "-5.9", "temperature": "29.1"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-02', '/battery_state', NOW()::TIMESTAMP - INTERVAL '75 minutes', '',
 '{"percentage": "30", "voltage": "11.8", "current": "-6.1", "temperature": "29.8"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-02', '/battery_state', NOW()::TIMESTAMP - INTERVAL '55 minutes', '',
 '{"percentage": "23", "voltage": "11.6", "current": "-6.3", "temperature": "30.5"}',
 'sensor_msgs/msg/BatteryState'),
-- *** Below 11.5V — these rows match the showcase query filter
('robot-amr-02', '/battery_state', NOW()::TIMESTAMP - INTERVAL '35 minutes', '',
 '{"percentage": "16", "voltage": "11.3", "current": "-6.5", "temperature": "31.2"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-02', '/battery_state', NOW()::TIMESTAMP - INTERVAL '15 minutes', '',
 '{"percentage": "11", "voltage": "11.0", "current": "-6.2", "temperature": "31.8"}',
 'sensor_msgs/msg/BatteryState'),

-- robot-amr-03 battery — moderate drain, stays above 11.5V
('robot-amr-03', '/battery_state', NOW()::TIMESTAMP - INTERVAL '115 minutes', '',
 '{"percentage": "62", "voltage": "12.4", "current": "-4.5", "temperature": "24.0"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-03', '/battery_state', NOW()::TIMESTAMP - INTERVAL '95 minutes', '',
 '{"percentage": "57", "voltage": "12.3", "current": "-4.6", "temperature": "24.3"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-03', '/battery_state', NOW()::TIMESTAMP - INTERVAL '75 minutes', '',
 '{"percentage": "52", "voltage": "12.2", "current": "-4.7", "temperature": "24.8"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-03', '/battery_state', NOW()::TIMESTAMP - INTERVAL '55 minutes', '',
 '{"percentage": "47", "voltage": "12.0", "current": "-4.8", "temperature": "25.2"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-03', '/battery_state', NOW()::TIMESTAMP - INTERVAL '35 minutes', '',
 '{"percentage": "43", "voltage": "11.9", "current": "-4.6", "temperature": "25.6"}',
 'sensor_msgs/msg/BatteryState'),
('robot-amr-03', '/battery_state', NOW()::TIMESTAMP - INTERVAL '15 minutes', '',
 '{"percentage": "39", "voltage": "11.7", "current": "-4.7", "temperature": "26.0"}',
 'sensor_msgs/msg/BatteryState');
