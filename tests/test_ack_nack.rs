mod common;

use crossbeam_channel;
use dronegowski::Dronegowski;
use std::collections::HashMap;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Ack, Nack, NackType, Packet, PacketType};
use wg_2024::tests::generic_fragment_forward;
const TIMER: Duration = Duration::from_secs(5);

#[test]
fn send_ack_to_neighbor() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (neighbor_send, neighbor_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders = HashMap::new();
    senders.insert(2, neighbor_send); //Drone 2 neighbor

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders,
        0.1, //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], //Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send
        .send(packet.clone())
        .expect("Error sending the packet...");

    let packet_test = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 2], //Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    match neighbor_receive.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!(
                "Packet {:?} successfully received by the node 2",
                received_packet.pack_type
            );
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn send_nack_to_neighbor() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (neighbor_send, neighbor_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders = HashMap::new();
    senders.insert(2, neighbor_send); //Drone 2 neighbor

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders,
        0.1, //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 0,
            nack_type: NackType::DestinationIsDrone,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], //Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send
        .send(packet.clone())
        .expect("Error sending the packet...");

    let packet_test = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 0,
            nack_type: NackType::DestinationIsDrone,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 2], //Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    match neighbor_receive.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!(
                "Packet {:?} successfully received by the node 2",
                received_packet.pack_type
            );
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn forward_ack_no_neighbor() {
    //Create channel
    let (sim_controller_send, sim_controller_receive) =
        crossbeam_channel::unbounded::<DroneEvent>();
    let (_controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        HashMap::new(), //No neighbor
        0.1,            //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], //Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send
        .send(packet.clone())
        .expect("Error sending the packet...");

    match sim_controller_receive.recv_timeout(TIMER) {
        Ok(DroneEvent::ControllerShortcut(received_packet)) => {
            assert_eq!(packet.clone(), received_packet.clone());
            println!(
                "Packet {:?} successfully sent to Simulation Controller",
                received_packet.pack_type
            );
        }
        _ => {
            println!("Timeout: No packet received.");
        }
    }
}

#[test]
fn forward_nack_no_neighbor() {
    //Create channel
    let (sim_controller_send, sim_controller_receive) =
        crossbeam_channel::unbounded::<DroneEvent>();
    let (_controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        HashMap::new(), //No neighbor
        0.1,            //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 0,
            nack_type: NackType::DestinationIsDrone,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], //Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send
        .send(packet.clone())
        .expect("Error sending the packet...");

    match sim_controller_receive.recv_timeout(TIMER) {
        Ok(DroneEvent::ControllerShortcut(received_packet)) => {
            assert_eq!(packet.clone(), received_packet.clone());
            println!(
                "Packet {:?} successfully sent to Simulation Controller",
                received_packet.pack_type
            );
        }
        _ => {
            println!("Timeout: No packet received.");
        }
    }
}

#[test]
fn test_from_gh() {
    generic_fragment_forward::<Dronegowski>();
}
