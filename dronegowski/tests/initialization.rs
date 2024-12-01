use crossbeam_channel::{unbounded, Receiver, Sender};
use dronegowski::MyDrone;
use rand::Rng;
use std::collections::HashMap;
use std::fs;
use std::thread;
use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::drone::{Drone, DroneOptions};
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

    let (node_event_send, _node_event_recv) = unbounded();
    let mut controller_drones: HashMap<NodeId, Sender<DroneCommand>> = HashMap::new();
    let mut packet_channels: HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)> = HashMap::new();

    for drone in config.drone.iter() {
        packet_channels.insert(drone.id, unbounded());
    }
    for client in config.client.iter() {
        packet_channels.insert(client.id, unbounded());
    }
    for server in config.server.iter() {
        packet_channels.insert(server.id, unbounded());
    }
    let mut handles = Vec::new();

    for drone in config.drone.into_iter() {
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
            let mut drone = MyDrone::new(DroneOptions {
                id: drone.id,
                controller_recv: controller_drone_recv,
                controller_send: node_event_send,
                packet_recv,
                packet_send,
                pdr: drone.pdr,
            });

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
