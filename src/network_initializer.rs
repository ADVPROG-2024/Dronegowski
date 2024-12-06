use crate::MyDrone;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::thread;
use thiserror::Error;
use wg_2024::config::Config;
use wg_2024::controller::DroneCommand;
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

/// Parsing del file di configurazione TOML.
pub fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).expect("Impossibile leggere il file di configurazione");
    println!("Parsing del file di configurazione...");
    toml::from_str(&file_str).expect("Errore durante il parsing del file TOML")
}

#[test]
fn test_initialization() {
    let config = parse_config("tests/common/config.toml");

    match validate_config(&config) {
        Ok(_) => println!("Config validation passed!"),
        Err(e) => {
            println!("Config validation failed: {e:?}");
            panic!("Validation failed.");
        }
    }

    let (node_event_send, _node_event_recv) = unbounded();
    let mut controller_drones: HashMap<NodeId, Sender<DroneCommand>> = HashMap::new();
    let mut packet_channels: HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)> = HashMap::new();

    for drone in &config.drone {
        packet_channels.insert(drone.id, unbounded());
    }
    for client in &config.client {
        packet_channels.insert(client.id, unbounded());
    }
    for server in &config.server {
        packet_channels.insert(server.id, unbounded());
    }
    let mut handles = Vec::new();

    for drone in config.drone {
        let (controller_drone_send, controller_drone_recv) = unbounded();
        controller_drones.insert(drone.id, controller_drone_send.clone());
        let node_event_send = node_event_send.clone();

        let packet_recv = packet_channels[&drone.id].1.clone();
        let packet_send: HashMap<NodeId, Sender<Packet>> = drone
            .connected_node_ids
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        handles.push(thread::spawn(move || {
            let mut drone = MyDrone::new(
                drone.id,
                node_event_send,
                controller_drone_recv,
                packet_recv,
                packet_send,
                drone.pdr,
            );

            drone.run();
        }));
    }

    // Simula un breve tempo di esecuzione
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Invia aggiornamento del PDR a tutti i droni
    for sender in controller_drones.values() {
        sender
            .send(DroneCommand::SetPacketDropRate(0.3))
            .expect("Errore nella modifica del PDR!");
    }

    // Invia il comando di terminazione a tutti i droni
    for sender in controller_drones.values() {
        sender
            .send(DroneCommand::Crash)
            .expect("Errore nell'invio del comando di terminazione");
    }

    // Aspetta che tutti i thread dei droni terminino
    while let Some(handle) = handles.pop() {
        handle
            .join()
            .expect("Errore durante la terminazione di un drone");
    }
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("The connection between node {0} and node {1} is not bidirectional.")]
    NotBidirectional(NodeId, NodeId),
    #[error("The graph is not connected.")]
    NotConnected,
    // Altri errori possono essere aggiunti qui.
}

pub fn validate_config(config: &Config) -> Result<(), ValidationError> {
    let mut graph: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();

    // Costruisce il grafo.
    for drone in &config.drone {
        for &connected_id in &drone.connected_node_ids {
            graph.entry(drone.id).or_default().insert(connected_id);
            graph.entry(connected_id).or_default(); // Assicura che il nodo connesso sia nel grafo.
        }
    }
    for client in &config.client {
        for &connected_id in &client.connected_drone_ids {
            graph.entry(client.id).or_default().insert(connected_id);
            graph.entry(connected_id).or_default();
        }
    }
    for server in &config.server {
        for &connected_id in &server.connected_drone_ids {
            graph.entry(server.id).or_default().insert(connected_id);
            graph.entry(connected_id).or_default();
        }
    }

    // Verifica la bidirezionalità.
    for (&node, connections) in &graph {
        for &connected_node in connections {
            //controllo della presenza del collegamento opposto
            if !graph
                .get(&connected_node)
                .map_or(false, |set| set.contains(&node))
            {
                return Err(ValidationError::NotBidirectional(node, connected_node));
            }
        }
    }

    // Verifica la connettività del grafo.
    let all_nodes: HashSet<_> = graph.keys().copied().collect();
    let mut visited = HashSet::new();
    // Prende un nodo qualsiasi come punto di partenza.
    let start_node = *all_nodes.iter().next().unwrap();

    dfs(start_node, &graph, &mut visited);

    if visited != all_nodes {
        return Err(ValidationError::NotConnected);
    }

    Ok(())
}

// Funzione DFS per verificare la connettività.
fn dfs(node: NodeId, graph: &HashMap<NodeId, HashSet<NodeId>>, visited: &mut HashSet<NodeId>) {
    if visited.contains(&node) {
        return;
    }
    visited.insert(node);
    if let Some(neighbors) = graph.get(&node) {
        for &neighbor in neighbors {
            dfs(neighbor, graph, visited);
        }
    }
}
