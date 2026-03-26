-- Fixture: otel_metrics — topic rates, CPU, memory metrics

-- /cmd_vel topic publish rate (Hz) — normally 10 Hz, drops during failures
INSERT INTO otel_metrics (timestamp, metric_name, value, attributes, service_name) VALUES
('2026-03-24T10:00:00Z', 'ros2.topic.rx_rate_hz', 10.0, '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:02:00Z', 'ros2.topic.rx_rate_hz', 10.1, '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:04:00Z', 'ros2.topic.rx_rate_hz', 9.9,  '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:06:00Z', 'ros2.topic.rx_rate_hz', 10.0, '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:08:00Z', 'ros2.topic.rx_rate_hz', 10.0, '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:09:00Z', 'ros2.topic.rx_rate_hz', 10.0, '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:11:00Z', 'ros2.topic.rx_rate_hz', 8.5,  '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:13:00Z', 'ros2.topic.rx_rate_hz', 5.2,  '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:15:00Z', 'ros2.topic.rx_rate_hz', 0.0,  '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:25:00Z', 'ros2.topic.rx_rate_hz', 10.0, '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:35:00Z', 'ros2.topic.rx_rate_hz', 9.8,  '{"topic": "/cmd_vel"}', 'robot_sim_001'),
('2026-03-24T10:45:00Z', 'ros2.topic.rx_rate_hz', 9.5,  '{"topic": "/cmd_vel"}', 'robot_sim_001');

-- CPU usage (%) — baseline 30%, spikes to 92% during action 2
INSERT INTO otel_metrics (timestamp, metric_name, value, attributes, service_name) VALUES
('2026-03-24T10:00:00Z', 'system.cpu.total_usage_pct', 28.0, '{}', 'robot_sim_001'),
('2026-03-24T10:02:00Z', 'system.cpu.total_usage_pct', 31.0, '{}', 'robot_sim_001'),
('2026-03-24T10:04:00Z', 'system.cpu.total_usage_pct', 29.0, '{}', 'robot_sim_001'),
('2026-03-24T10:06:00Z', 'system.cpu.total_usage_pct', 30.0, '{}', 'robot_sim_001'),
('2026-03-24T10:08:00Z', 'system.cpu.total_usage_pct', 32.0, '{}', 'robot_sim_001'),
('2026-03-24T10:09:00Z', 'system.cpu.total_usage_pct', 45.0, '{}', 'robot_sim_001'),
('2026-03-24T10:11:00Z', 'system.cpu.total_usage_pct', 72.0, '{}', 'robot_sim_001'),
('2026-03-24T10:13:00Z', 'system.cpu.total_usage_pct', 92.0, '{}', 'robot_sim_001'),
('2026-03-24T10:15:00Z', 'system.cpu.total_usage_pct', 85.0, '{}', 'robot_sim_001'),
('2026-03-24T10:17:00Z', 'system.cpu.total_usage_pct', 40.0, '{}', 'robot_sim_001'),
('2026-03-24T10:25:00Z', 'system.cpu.total_usage_pct', 30.0, '{}', 'robot_sim_001'),
('2026-03-24T10:35:00Z', 'system.cpu.total_usage_pct', 31.0, '{}', 'robot_sim_001');

-- Memory usage (%) — stable at ~45%
INSERT INTO otel_metrics (timestamp, metric_name, value, attributes, service_name) VALUES
('2026-03-24T10:00:00Z', 'system.memory.usage_pct', 44.0, '{}', 'robot_sim_001'),
('2026-03-24T10:05:00Z', 'system.memory.usage_pct', 45.0, '{}', 'robot_sim_001'),
('2026-03-24T10:10:00Z', 'system.memory.usage_pct', 46.0, '{}', 'robot_sim_001'),
('2026-03-24T10:15:00Z', 'system.memory.usage_pct', 45.0, '{}', 'robot_sim_001'),
('2026-03-24T10:25:00Z', 'system.memory.usage_pct', 44.0, '{}', 'robot_sim_001'),
('2026-03-24T10:35:00Z', 'system.memory.usage_pct', 45.0, '{}', 'robot_sim_001');
