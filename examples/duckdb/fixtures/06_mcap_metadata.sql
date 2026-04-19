-- Fixture: mcap_metadata — one MCAP recording session per robot
--
-- Each session covers the robot's active mission window.
-- S3 keys follow: s3://robotops-recordings/{robot_id}/{date}/session-{n}.mcap

INSERT INTO mcap_metadata (robot_id, session_id, start_time, end_time, file_uri, topics) VALUES
(
  'robot-amr-01',
  'session-amr01-2026-04-12',
  NOW()::TIMESTAMP - INTERVAL '70 minutes',
  NOW()::TIMESTAMP - INTERVAL '20 minutes',
  's3://robotops-recordings/robot-amr-01/2026-04-12/session-001.mcap',
  ['/odom', '/cmd_vel', '/scan', '/battery_state', '/plan', '/tf', '/rosout', '/joint_states']
),
(
  'robot-amr-02',
  'session-amr02-2026-04-12',
  NOW()::TIMESTAMP - INTERVAL '70 minutes',
  NOW()::TIMESTAMP - INTERVAL '5 minutes',
  's3://robotops-recordings/robot-amr-02/2026-04-12/session-001.mcap',
  ['/odom', '/cmd_vel', '/scan', '/battery_state', '/plan', '/costmap', '/tf', '/rosout']
),
(
  'robot-amr-03',
  'session-amr03-2026-04-12',
  NOW()::TIMESTAMP - INTERVAL '70 minutes',
  NOW()::TIMESTAMP - INTERVAL '20 minutes',
  's3://robotops-recordings/robot-amr-03/2026-04-12/session-001.mcap',
  ['/odom', '/cmd_vel', '/scan', '/battery_state', '/plan', '/tf', '/rosout']
);
