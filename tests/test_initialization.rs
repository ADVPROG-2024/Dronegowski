use crossbeam_channel::{unbounded, Receiver, Sender};
use dronegowski::Dronegowski;
use std::collections::HashMap;
use std::fs;
use std::thread;
use wg_2024::config::Config;
use wg_2024::controller::DroneCommand;
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Fragment, Packet, PacketType};

/// Parsing config.toml file.
pub fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).expect("error reading config.toml file");
    println!("Parsing config.toml file...");
    toml::from_str(&file_str).expect("Error occurred while parsing config.toml file")
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

    //Running all the drones
    for drone in config.drone.clone().into_iter() {
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
            let mut drone = Dronegowski::new(
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

    //Emulate a short execution time
    thread::sleep(std::time::Duration::from_secs(2));

    //Simulating packet sending between the drones (it fails because DroneIsDestination)
    test_send_packet_between_nodes(&config, &packet_channels);

    //Updating PDR of all drones
    test_command_set_pdr(&controller_drones);

    //Sending crash command to all drones
    test_crash_all(&controller_drones);
}

fn test_send_packet_between_nodes(
    config: &Config,
    packet_channels: &HashMap<NodeId, (Sender<Packet>, Receiver<Packet>)>,
)
{
    let packet = Packet {
        pack_type: (PacketType::MsgFragment(Fragment {
            fragment_index: 0,
            total_n_fragments: 1,
            length: 12,
            data: [3; 128],
        })),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![config.drone[0].id, config.drone[1].id, config.drone[2].id],
        },
        session_id: 0,
    };

    if let Some(sender) = packet_channels.get(&config.drone[1].id).map(|(tx, _)| tx) {
        sender
            .send(packet)
            .expect("Error while sending the packet!");
    } else {
        panic!("Channel not found for drone 2!");
    }
}

fn test_command_set_pdr(controller_drones: &HashMap<NodeId, Sender<DroneCommand>>) {
    for sender in controller_drones.values() {
        sender
            .send(DroneCommand::SetPacketDropRate(0.3))
            .expect("Error occurred while updating PDR!");
    }
}

fn test_crash_all(controller_drones: &HashMap<NodeId, Sender<DroneCommand>>) {
    for (node, sender) in controller_drones {
        crash_node(controller_drones, node);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn crash_node(controller_drones: &HashMap<NodeId, Sender<DroneCommand>>, node_id: &NodeId) {
    let drone_crash = controller_drones.get(&node_id).unwrap();
    drone_crash
        .send(DroneCommand::Crash)
        .expect("Error occurred while terminating the drone");
}
