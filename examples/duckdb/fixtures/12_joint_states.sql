-- Fixture: joint_states — ROS2 /joint_states samples for the arm on robot-amr-02.
--
-- Enables `FROM joints` queries such as:
--   FROM joints WHERE effort > 10 FOR ROBOT 'robot-amr-02' SINCE 1 hour ago
--   FROM joints WHERE joint_name = 'shoulder_lift_joint' FOR ROBOT 'robot-amr-02'
--          SINCE 1 hour ago FACET joint_name
--
-- Schema mirrors the typed-column joint-states table: joint_name is a bare TEXT
-- column; position/velocity/effort are DOUBLE PRECISION (radians, rad/s, Nm).
--
-- Coverage:
--   * robot-amr-02 carries a 6-DoF arm (ur5e-style joints). We emit four time
--     samples per joint over a ~12 minute window so effort-over-time charts have
--     a real trend.
--   * The shoulder_lift_joint and elbow_joint carry the most load (effort > 10 Nm)
--     during the lift, so `WHERE effort > 10` returns rows. Wrist joints stay low.
--   * robot-amr-01 has a single light sample so cross-robot facets aren't empty.

-- ── robot-amr-02 arm: 6 joints x 4 time samples ─────────────────────────────
INSERT INTO joint_states (timestamp, org_id, robot_id, joint_name,
                          position, velocity, effort) VALUES
-- t-12m: arm starting to lift a payload
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-02', 'shoulder_pan_joint',   0.10,  0.05,  3.2),
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-02', 'shoulder_lift_joint', -0.90,  0.12, 14.5),
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-02', 'elbow_joint',          1.40, -0.08, 11.8),
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-02', 'wrist_1_joint',       -0.50,  0.02,  2.1),
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-02', 'wrist_2_joint',        1.57,  0.00,  0.9),
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-02', 'wrist_3_joint',        0.00,  0.01,  0.4),
-- t-9m: peak load mid-lift
(NOW()::TIMESTAMP - INTERVAL '9 minutes', 'acme', 'robot-amr-02', 'shoulder_pan_joint',   0.18,  0.07,  4.0),
(NOW()::TIMESTAMP - INTERVAL '9 minutes', 'acme', 'robot-amr-02', 'shoulder_lift_joint', -1.05,  0.20, 18.7),
(NOW()::TIMESTAMP - INTERVAL '9 minutes', 'acme', 'robot-amr-02', 'elbow_joint',          1.62, -0.15, 15.3),
(NOW()::TIMESTAMP - INTERVAL '9 minutes', 'acme', 'robot-amr-02', 'wrist_1_joint',       -0.55,  0.04,  2.4),
(NOW()::TIMESTAMP - INTERVAL '9 minutes', 'acme', 'robot-amr-02', 'wrist_2_joint',        1.57,  0.00,  1.0),
(NOW()::TIMESTAMP - INTERVAL '9 minutes', 'acme', 'robot-amr-02', 'wrist_3_joint',        0.02,  0.01,  0.5),
-- t-6m: holding payload
(NOW()::TIMESTAMP - INTERVAL '6 minutes', 'acme', 'robot-amr-02', 'shoulder_pan_joint',   0.20,  0.01,  3.6),
(NOW()::TIMESTAMP - INTERVAL '6 minutes', 'acme', 'robot-amr-02', 'shoulder_lift_joint', -1.10,  0.02, 16.2),
(NOW()::TIMESTAMP - INTERVAL '6 minutes', 'acme', 'robot-amr-02', 'elbow_joint',          1.68, -0.01, 13.0),
(NOW()::TIMESTAMP - INTERVAL '6 minutes', 'acme', 'robot-amr-02', 'wrist_1_joint',       -0.56,  0.00,  2.2),
(NOW()::TIMESTAMP - INTERVAL '6 minutes', 'acme', 'robot-amr-02', 'wrist_2_joint',        1.57,  0.00,  0.9),
(NOW()::TIMESTAMP - INTERVAL '6 minutes', 'acme', 'robot-amr-02', 'wrist_3_joint',        0.01,  0.00,  0.4),
-- t-3m: setting payload down, load easing off
(NOW()::TIMESTAMP - INTERVAL '3 minutes', 'acme', 'robot-amr-02', 'shoulder_pan_joint',   0.12, -0.04,  2.8),
(NOW()::TIMESTAMP - INTERVAL '3 minutes', 'acme', 'robot-amr-02', 'shoulder_lift_joint', -0.85, -0.10,  9.4),
(NOW()::TIMESTAMP - INTERVAL '3 minutes', 'acme', 'robot-amr-02', 'elbow_joint',          1.35, -0.12,  7.6),
(NOW()::TIMESTAMP - INTERVAL '3 minutes', 'acme', 'robot-amr-02', 'wrist_1_joint',       -0.48,  0.01,  1.8),
(NOW()::TIMESTAMP - INTERVAL '3 minutes', 'acme', 'robot-amr-02', 'wrist_2_joint',        1.57,  0.00,  0.8),
(NOW()::TIMESTAMP - INTERVAL '3 minutes', 'acme', 'robot-amr-02', 'wrist_3_joint',        0.00,  0.00,  0.3),

-- ── robot-amr-01: single light sample (idle arm) ────────────────────────────
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-01', 'shoulder_lift_joint', -0.20,  0.00,  1.1),
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-01', 'elbow_joint',          0.30,  0.00,  0.8);
