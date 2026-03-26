-- Fixture: mcap_metadata — MCAP recording covering the failure window
--
-- One recording session that covers all three actions, including the
-- battery-related abort. This is what SHOW RECORDING queries find.

INSERT INTO mcap_metadata (robot_id, session_id, start_time, end_time, s3_key, topics) VALUES
('robot_sim_001', 'session-2026-03-24-001', '2026-03-24T09:55:00Z', '2026-03-24T11:00:00Z',
 's3://robotops-recordings/robot_sim_001/2026-03-24/session-001.mcap',
 ['/odom', '/cmd_vel', '/scan', '/battery_state', '/joint_states', '/tf', '/rosout']);
