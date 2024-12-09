use dronegowski::Dronegowski;
use std::collections::HashMap;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{SourceRoutingHeader};
use wg_2024::packet::{Fragment, Nack, NackType, Packet, PacketType};
const TIMER: Duration = Duration::from_secs(5);

#[test]
fn forward_msg_fragment_to_neighbours() {
    //Create channel
    let (sim_controller_send, sim_controller_receive) = crossbeam_channel::unbounded::<DroneEvent>();
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
        0.0
    );

    let packet = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1, 2], //Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    println!("Sending fragment packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 2,
            hops: vec![0, 1, 2], //Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    match neighbor_receive.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }

    match sim_controller_receive.recv_timeout(TIMER) {
        Ok(DroneEvent::PacketSent(received_packet)) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the Simulation Controller", received_packet.pack_type);
        }
        _ => println!("Timeout: No packet received.")
    }
}

#[test]
fn forward_msg_fragment_destination_is_drone() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send_my_drone.clone());

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_my_drone.clone(),
        senders_my_drone,
        0.0
    );

    let _ = Dronegowski::new(
        0,
        sim_controller_send,
        controller_receive,
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0
    );

    let packet = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1],   //The destination is drone
        },
        session_id: 1,
    };

    println!("Sending fragment packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test = Packet {
        pack_type: PacketType::Nack(Nack{
            fragment_index: 10,
            nack_type: NackType::DestinationIsDrone,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 0],       //The destination is drone
        },
        session_id: 1,
    };

    match packet_receive_test_drone.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn forward_msg_fragment_no_neighbor() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_my_drone.clone(),
        senders_my_drone,
        0.0
    );

    let _ = Dronegowski::new(
        0,
        sim_controller_send,
        controller_receive,
        packet_receive_test_drone.clone(),
        HashMap::new(),
        0.0
    );

    let packet = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1, 2], //Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    println!("Sending fragment packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test = Packet {
        pack_type: PacketType::Nack(Nack{
            fragment_index: 10,
            nack_type: NackType::ErrorInRouting(2),
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 0],
        },
        session_id: 1,
    };

    match packet_receive_test_drone.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn forward_msg_fragment_wrong_id() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send_my_drone.clone());

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_my_drone.clone(),
        senders_my_drone,
        0.0
    );

    let _ = Dronegowski::new(
        0,
        sim_controller_send,
        controller_receive,
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0
    );

    let packet = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 2], // Path: ID Drone 1 != ID Drone 2
        },
        session_id: 1,
    };

    println!("Sending fragment packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test = Packet {
        pack_type: PacketType::Nack(Nack{
            fragment_index: 10,
            nack_type: NackType::UnexpectedRecipient(1),
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![2, 0],
        },
        session_id: 1,
    };

    match packet_receive_test_drone.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn forward_msg_fragment_dropped() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send_my_drone.clone());

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_my_drone.clone(),
        senders_my_drone,
        1.0         //Valid PDR but Packet must Dropped
    );

    let _ = Dronegowski::new(
        0,
        sim_controller_send,
        controller_receive,
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0         //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1, 2],
        },
        session_id: 1,
    };

    println!("Sending fragment packet...");

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test = Packet {
        pack_type: PacketType::Nack(Nack{
            fragment_index: 10,
            nack_type: NackType::Dropped,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 0],
        },
        session_id: 1,
    };

    match packet_receive_test_drone.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}