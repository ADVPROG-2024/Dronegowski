//
// use crate::Dronegowski;
// use crossbeam_channel::{unbounded, Receiver, Sender};
// use std::collections::{HashMap, HashSet};
// use std::fs;
// use std::thread;
// use thiserror::Error;
// use wg_2024::config::Config;
// use wg_2024::controller::DroneCommand;
// use wg_2024::drone::Drone;
// use wg_2024::network::NodeId;
// use wg_2024::packet::Packet;
//
// /// Parsing file  config.toml
// pub fn parse_config(file: &str) -> Config {
//     let file_str = fs::read_to_string(file).expect("error reading config file");
//     println!("Parsing configuration file...");
//     toml::from_str(&file_str).expect("Error occurred during config file parsing")
// }
//
// #[test]
// fn test_initialization() {
//     let config = parse_config("tests/common/config.toml");
//
//     match validate_config(&config) {
//         Ok(_) => println!("Config validation passed!"),
//         Err(e) => {
//             println!("Config validation failed: {e:?}");
//             panic!("Validation failed.");
//         }
//     }
//
//     let (node_event_send, _node_event_recv) = unbounded();
//     let mut controller_drones: HashMap<NodeId, Sender<DroneCommand>> = HashMap::new();
//     let mut packet_channels: HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)> = HashMap::new();
//
//     for drone in &config.drone {
//         packet_channels.insert(drone.id, unbounded());
//     }
//     for client in &config.client {
//         packet_channels.insert(client.id, unbounded());
//     }
//     for server in &config.server {
//         packet_channels.insert(server.id, unbounded());
//     }
//     let mut handles = Vec::new();
//
//     for drone in config.drone {
//         let (controller_drone_send, controller_drone_recv) = unbounded();
//         controller_drones.insert(drone.id, controller_drone_send.clone());
//         let node_event_send = node_event_send.clone();
//
//         let packet_recv = packet_channels[&drone.id].1.clone();
//         let packet_send: HashMap<NodeId, Sender<Packet>> = drone
//             .connected_node_ids
//             .into_iter()
//             .map(|id| (id, packet_channels[&id].0.clone()))
//             .collect();
//
//         handles.push(thread::spawn(move || {
//             let mut drone = Dronegowski::new(
//                 drone.id,
//                 node_event_send,
//                 controller_drone_recv,
//                 packet_recv,
//                 packet_send,
//                 drone.pdr,
//             );
//
//             drone.run();
//         }));
//     }
//
//     // pretend a short execution time
//     std::thread::sleep(std::time::Duration::from_secs(2));
//
//     // sending pdr update to all drones
//     for sender in controller_drones.values() {
//         sender
//             .send(DroneCommand::SetPacketDropRate(0.3))
//             .expect("Error occurred during PDR setting!");
//     }
//
//     // sending crash command to all drones
//     for sender in controller_drones.values() {
//         sender
//             .send(DroneCommand::Crash)
//             .expect("Error occurred sending crash command");
//     }
//
//     // wait all the drone threads
//     while let Some(handle) = handles.pop() {
//         handle
//             .join()
//             .expect("Error occured while exiting a drone");
//     }
// }
//
// #[derive(Debug, Error)]
// pub enum ValidationError {
//     #[error("The connection between node {0} and node {1} is not bidirectional.")]
//     NotBidirectional(NodeId, NodeId),
//     #[error("The graph is not connected.")]
//     NotConnected,
//     // Other errors needed can be added here.
// }
//
// pub fn validate_config(config: &Config) -> Result<(), ValidationError> {
//     let mut graph: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
//
//     // building the graph
//     for drone in &config.drone {
//         for &connected_id in &drone.connected_node_ids {
//             graph.entry(drone.id).or_default().insert(connected_id);
//             graph.entry(connected_id).or_default(); // insert the neighbour node in the graph if not there
//         }
//     }
//     for client in &config.client {
//         for &connected_id in &client.connected_drone_ids {
//             graph.entry(client.id).or_default().insert(connected_id);
//             graph.entry(connected_id).or_default();
//         }
//     }
//     for server in &config.server {
//         for &connected_id in &server.connected_drone_ids {
//             graph.entry(server.id).or_default().insert(connected_id);
//             graph.entry(connected_id).or_default();
//         }
//     }
//
//     // bidirectional links checking
//     for (&node, connections) in &graph {
//         for &connected_node in connections {
//             //checking of the opposite link
//             if !graph
//                 .get(&connected_node)
//                 .map_or(false, |set| set.contains(&node))
//             {
//                 return Err(ValidationError::NotBidirectional(node, connected_node));
//             }
//         }
//     }
//
//     // connected graph checking
//     let all_nodes: HashSet<_> = graph.keys().copied().collect();
//     let mut visited = HashSet::new();
//     // takes any node as starting point
//     let start_node = *all_nodes.iter().next().unwrap();
//
//     dfs(start_node, &graph, &mut visited);
//
//     if visited != all_nodes {
//         return Err(ValidationError::NotConnected);
//     }
//
//     Ok(())
// }
//
// // DFS function used in the connected graph checking
// fn dfs(node: NodeId, graph: &HashMap<NodeId, HashSet<NodeId>>, visited: &mut HashSet<NodeId>) {
//     if visited.contains(&node) {
//         return;
//     }
//     visited.insert(node);
//     if let Some(neighbors) = graph.get(&node) {
//         for &neighbor in neighbors {
//             dfs(neighbor, graph, visited);
//         }
//     }
// }
