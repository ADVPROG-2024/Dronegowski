mod common;
mod test_from_gh;

use dronegowski::MyDrone;
use std::collections::HashMap;
use crossbeam_channel;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, Packet, PacketType, NodeType};

#[test]
fn test_flood_request_handling() {
    // Creazione dei canali
    let (sender1, receiver1) = crossbeam_channel::unbounded::<Packet>();

    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    // Mapping dei neighbour ai canali
    let mut senders = HashMap::new();
    senders.insert(2, sender1); // Drone 2 neighbour

    // Creazione del drone con configurazione dei canali
    let mut my_drone = MyDrone::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders,
        0.0
    );

    // Creazione del pacchetto FloodRequest da inviare al drone
    let mut packet = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 123,
            initiator_id: 0,
            path_trace: vec![(0, NodeType::Client)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![],
        },
        session_id: 1,
    };

    println!("Sending flood request packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    //Invio il pacchetto al mio drone
    packet_send.send(packet.clone()).expect("Error sending the flood request...");

    //Verifica che il pacchetto venga ricevuto correttamente dai vicini
    match receiver1.recv() {
        Ok(received_packet) => {
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn test_flood_request_no_neighbour(){
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    // Mapping dei neighbour ai canali
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send_my_drone.clone());
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());

    let _ = MyDrone::new(
        0,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0
    );

    // Creazione del drone con configurazione dei canali
    let mut my_drone = MyDrone::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive_my_drone.clone(),
        senders_my_drone,
        0.0
    );

    // Creazione del pacchetto FloodRequest da inviare al drone
    let mut packet = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 123,
            initiator_id: 0,
            path_trace: vec![(0, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![],
        },
        session_id: 1,
    };

    println!("Sending flood request packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    //Invio il pacchetto al mio drone
    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    match packet_receive_test_drone.recv() {
        Ok(received_packet) => {
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn test_flood_request_already_received(){
    let (sender1, receiver1) = crossbeam_channel::unbounded::<Packet>();

    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    // Mapping dei neighbour ai canali
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send_my_drone.clone());
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());
    senders_my_drone.insert(2, sender1);

    let _ = MyDrone::new(
        0,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0
    );

    // Creazione del drone con configurazione dei canali
    let mut my_drone = MyDrone::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive_my_drone.clone(),
        senders_my_drone,
        0.0
    );

    // Creazione del pacchetto FloodRequest da inviare al drone
    let mut packet = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 123,
            initiator_id: 0,
            path_trace: vec![(1, NodeType::Drone), (0, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![],
        },
        session_id: 1,
    };

    println!("Sending flood request packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    //Invio il pacchetto al mio drone
    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    match receiver1.recv() {
        Ok(received_packet) => {
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
            packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");
        }
        Err(_) => println!("Timeout: No packet received."),
    }

    match packet_receive_test_drone.recv() {
        Ok(received_packet) => {
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

