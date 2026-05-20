-- Fixture: tf_states — TF2 transform broadcasts for the 3 amr robots
--
-- Enables `FROM tf` queries such as:
--   FROM tf WHERE parent_frame = 'base_link' AND child_frame = 'tool0'
--          AND translation_z > 1.0 SINCE 1 hour ago FACET robot_id
--
-- Schema mirrors the typed-column TF table: parent_frame/child_frame are bare
-- TEXT columns; translation_{x,y,z} and rotation_{x,y,z,w} are DOUBLE PRECISION.
--
-- Coverage:
--   * map -> odom -> base_link -> tool0 chain for each robot (localisation + arm)
--   * robot-amr-02 has elevated tool0 (translation_z > 1.0) during mission 3
--   * a small drift over time keeps deviations measurable

-- ============================================================================
-- map -> odom (localisation root, near identity)
-- ============================================================================

INSERT INTO tf_states (timestamp, org_id, robot_id, parent_frame, child_frame,
                       translation_x, translation_y, translation_z,
                       rotation_x, rotation_y, rotation_z, rotation_w) VALUES
(NOW()::TIMESTAMP - INTERVAL '30 minutes', 'acme', 'robot-amr-01',
 'map', 'odom',  0.05,  0.02, 0.00,  0.0, 0.0, 0.00, 1.00),
(NOW()::TIMESTAMP - INTERVAL '30 minutes', 'acme', 'robot-amr-02',
 'map', 'odom',  0.10, -0.04, 0.00,  0.0, 0.0, 0.01, 0.99),
(NOW()::TIMESTAMP - INTERVAL '30 minutes', 'acme', 'robot-amr-03',
 'map', 'odom', -0.02,  0.01, 0.00,  0.0, 0.0, 0.00, 1.00);

-- ============================================================================
-- odom -> base_link (robot pose in the world frame)
-- ============================================================================

INSERT INTO tf_states (timestamp, org_id, robot_id, parent_frame, child_frame,
                       translation_x, translation_y, translation_z,
                       rotation_x, rotation_y, rotation_z, rotation_w) VALUES
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'acme', 'robot-amr-01',
 'odom', 'base_link',  5.20,  9.05, 0.00,  0.0, 0.0,  0.10, 0.99),
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'acme', 'robot-amr-02',
 'odom', 'base_link',  3.15, -0.42, 0.00,  0.0, 0.0, -0.35, 0.94),
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'acme', 'robot-amr-03',
 'odom', 'base_link',  2.05,  1.00, 0.00,  0.0, 0.0,  0.20, 0.98);

-- ============================================================================
-- base_link -> tool0 (end-effector pose, where translation_z matters)
--
-- amr-02 has the arm raised (translation_z = 1.25) during mission 3.
-- amr-01 and amr-03 keep the arm stowed (translation_z < 1.0).
-- ============================================================================

INSERT INTO tf_states (timestamp, org_id, robot_id, parent_frame, child_frame,
                       translation_x, translation_y, translation_z,
                       rotation_x, rotation_y, rotation_z, rotation_w) VALUES
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'acme', 'robot-amr-01',
 'base_link', 'tool0', 0.35, 0.00, 0.40,  0.0, 0.0, 0.0, 1.0),
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'acme', 'robot-amr-02',
 'base_link', 'tool0', 0.30, 0.05, 1.25,  0.0, 0.0, 0.0, 1.0),
(NOW()::TIMESTAMP - INTERVAL '25 minutes', 'acme', 'robot-amr-03',
 'base_link', 'tool0', 0.35, 0.00, 0.40,  0.0, 0.0, 0.0, 1.0),

-- A few additional samples for robot-amr-02 showing arm motion during the
-- mission window (translation_z dips and recovers).
(NOW()::TIMESTAMP - INTERVAL '20 minutes', 'acme', 'robot-amr-02',
 'base_link', 'tool0', 0.32, 0.04, 1.30,  0.0, 0.0, 0.0, 1.0),
(NOW()::TIMESTAMP - INTERVAL '15 minutes', 'acme', 'robot-amr-02',
 'base_link', 'tool0', 0.34, 0.02, 1.10,  0.0, 0.0, 0.0, 1.0),
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-02',
 'base_link', 'tool0', 0.36, 0.01, 1.05,  0.0, 0.0, 0.0, 1.0),
(NOW()::TIMESTAMP - INTERVAL '5 minutes',  'acme', 'robot-amr-02',
 'base_link', 'tool0', 0.36, 0.00, 0.95,  0.0, 0.0, 0.0, 1.0);
