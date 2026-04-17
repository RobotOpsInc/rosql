-- robot_joint_map fixture — URDF-derived joint metadata for example robots.
-- Used by SHOW JOINTS and JOINT DEVIATION queries.

INSERT INTO robot_joint_map (robot_model, valid_from, valid_to, version, robot_ids, joint_map) VALUES
(
  'ur5e',
  '2026-01-01T00:00:00Z',
  NULL,
  'v1.0',
  ['robot_sim_001', 'robot_sim_002'],
  '[
    {"joint_name": "shoulder_pan_joint",  "joint_index": 0, "joint_type": "revolute", "lower_limit": -6.283185, "upper_limit": 6.283185},
    {"joint_name": "shoulder_lift_joint", "joint_index": 1, "joint_type": "revolute", "lower_limit": -6.283185, "upper_limit": 6.283185},
    {"joint_name": "elbow_joint",         "joint_index": 2, "joint_type": "revolute", "lower_limit": -3.141593, "upper_limit": 3.141593},
    {"joint_name": "wrist_1_joint",       "joint_index": 3, "joint_type": "revolute", "lower_limit": -6.283185, "upper_limit": 6.283185},
    {"joint_name": "wrist_2_joint",       "joint_index": 4, "joint_type": "revolute", "lower_limit": -6.283185, "upper_limit": 6.283185},
    {"joint_name": "wrist_3_joint",       "joint_index": 5, "joint_type": "revolute", "lower_limit": -6.283185, "upper_limit": 6.283185}
  ]'
),
(
  'panda',
  '2026-01-01T00:00:00Z',
  NULL,
  'v1.0',
  ['arm_01'],
  '[
    {"joint_name": "panda_joint1", "joint_index": 0, "joint_type": "revolute", "lower_limit": -2.897247, "upper_limit": 2.897247},
    {"joint_name": "panda_joint2", "joint_index": 1, "joint_type": "revolute", "lower_limit": -1.762782, "upper_limit": 1.762782},
    {"joint_name": "panda_joint3", "joint_index": 2, "joint_type": "revolute", "lower_limit": -2.897247, "upper_limit": 2.897247},
    {"joint_name": "panda_joint4", "joint_index": 3, "joint_type": "revolute", "lower_limit": -3.071779, "upper_limit": -0.069582},
    {"joint_name": "panda_joint5", "joint_index": 4, "joint_type": "revolute", "lower_limit": -2.897247, "upper_limit": 2.897247},
    {"joint_name": "panda_joint6", "joint_index": 5, "joint_type": "revolute", "lower_limit": -0.017453, "upper_limit": 3.752458},
    {"joint_name": "panda_joint7", "joint_index": 6, "joint_type": "revolute", "lower_limit": -2.897247, "upper_limit": 2.897247}
  ]'
);
