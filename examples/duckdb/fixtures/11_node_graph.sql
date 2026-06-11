-- Fixture: node_graph_edges — ROS2 node-graph pub/sub edges for the amr robots.
--
-- Enables `FROM node_graph` queries such as:
--   FROM node_graph WHERE compatible = false FOR ROBOT 'robot-amr-02'
--   FROM node_graph WHERE topic = '/scan' FOR ROBOT 'robot-amr-02' FACET source_node
--
-- Schema mirrors the typed-column node-graph table: source_node/target_node/
-- topic/message_type/publisher_qos/subscriber_qos are bare TEXT columns, rate_hz
-- is DOUBLE PRECISION, and compatible is a BOOLEAN (false = a QoS mismatch).
--
-- Coverage:
--   * A typical nav/perception graph for robot-amr-02 (mostly compatible edges).
--   * One QoS-incompatible edge: the lidar driver publishes /scan as best_effort
--     but the costmap subscribes as reliable -> compatible = false. This is the
--     classic "messages silently dropped" bug the demo highlights.
--   * A second incompatible edge on robot-amr-03 (/odom reliable<->best_effort)
--     so cross-robot facet queries have more than one mismatch.

INSERT INTO node_graph_edges (timestamp, org_id, robot_id, source_node, target_node,
                              topic, message_type, publisher_qos, subscriber_qos,
                              rate_hz, compatible) VALUES
-- robot-amr-02: perception + navigation graph
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-02',
 '/lidar_driver', '/scan_filter', '/scan', 'sensor_msgs/LaserScan',
 'best_effort', 'best_effort', 15.0, TRUE),
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-02',
 '/scan_filter', '/costmap_2d', '/scan_filtered', 'sensor_msgs/LaserScan',
 'reliable', 'reliable', 15.0, TRUE),
-- QoS MISMATCH: lidar publishes best_effort, costmap subscribes reliable.
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-02',
 '/lidar_driver', '/costmap_2d', '/scan', 'sensor_msgs/LaserScan',
 'best_effort', 'reliable', 15.0, FALSE),
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-02',
 '/ekf_node', '/controller_server', '/odom', 'nav_msgs/Odometry',
 'reliable', 'reliable', 50.0, TRUE),
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-02',
 '/controller_server', '/base_driver', '/cmd_vel', 'geometry_msgs/Twist',
 'reliable', 'reliable', 20.0, TRUE),
(NOW()::TIMESTAMP - INTERVAL '10 minutes', 'acme', 'robot-amr-02',
 '/camera_driver', '/object_detector', '/image_raw', 'sensor_msgs/Image',
 'best_effort', 'best_effort', 30.0, TRUE),

-- robot-amr-01: a clean graph (all compatible)
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-01',
 '/lidar_driver', '/costmap_2d', '/scan', 'sensor_msgs/LaserScan',
 'reliable', 'reliable', 15.0, TRUE),
(NOW()::TIMESTAMP - INTERVAL '12 minutes', 'acme', 'robot-amr-01',
 '/ekf_node', '/controller_server', '/odom', 'nav_msgs/Odometry',
 'reliable', 'reliable', 50.0, TRUE),

-- robot-amr-03: a second QoS mismatch on /odom
(NOW()::TIMESTAMP - INTERVAL '11 minutes', 'acme', 'robot-amr-03',
 '/ekf_node', '/controller_server', '/odom', 'nav_msgs/Odometry',
 'reliable', 'best_effort', 50.0, FALSE),
(NOW()::TIMESTAMP - INTERVAL '11 minutes', 'acme', 'robot-amr-03',
 '/lidar_driver', '/costmap_2d', '/scan', 'sensor_msgs/LaserScan',
 'reliable', 'reliable', 15.0, TRUE);
