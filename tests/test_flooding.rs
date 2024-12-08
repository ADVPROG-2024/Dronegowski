mod common;

use dronegowski::Dronegowski;
use std::collections::{HashMap};
use crossbeam_channel;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, Packet, PacketType, NodeType, FloodResponse};

#[test]
fn test_flood_request_handling() {
    // creation of channels
    let (sender1, receiver1) = crossbeam_channel::unbounded::<Packet>();

    let (sim_controller_send, sim_controller_recv) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    // Mapping of neighbours to channels
    let mut senders = HashMap::new();
    senders.insert(2, sender1); // Drone 2 neighbour

    // creation of drone with channels configuration
    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders.clone(),
        0.0
    );

    // creation of FloodRequest packet to forward do drone
    let packet = Packet {
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

    //Forward packet to my drone
    packet_send.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 123,
            initiator_id: 0,
            path_trace: vec![(0, NodeType::Client), (1, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![],
        },
        session_id: 1,
    };

    //Verify packet letting received correctly from neighbors
    match receiver1.recv() {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
    match sim_controller_recv.recv() {
        Ok(DroneEvent::PacketSent(received_packet)) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
        _ => {}
    }
}

#[test]
fn test_flood_request_no_neighbour(){
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    // Mapping of neighbour at channels
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send_my_drone.clone());
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());

    let _ = Dronegowski::new(
        0,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0
    );

    // creation of drone with configuration of channels
    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive_my_drone.clone(),
        senders_my_drone,
        0.0
    );

    // creation of FloodRequest packet to forward to the drone
    let packet = Packet {
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

    let packet_test = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse {
            flood_id: 123,
            path_trace: vec![(0, NodeType::Drone), (1, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 0],
        },
        session_id: 1,
    };

    //Forward packet to my drone
    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    match packet_receive_test_drone.recv() {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn test_flood_request_already_received(){
    let (sender1, receiver1) = crossbeam_channel::unbounded::<Packet>();

    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    // Mapping of neighbour to channels
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send_my_drone.clone());
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());
    senders_my_drone.insert(2, sender1);

    let _ = Dronegowski::new(
        0,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0
    );

    // creation of drone with configuration of channels
    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive_my_drone.clone(),
        senders_my_drone,
        0.0
    );

    // creation of FloodRequest packet to forward to the drone
    let packet = Packet {
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

    //Forward packet to my drone
    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test1 = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 123,
            initiator_id: 0,
            path_trace: vec![(0, NodeType::Drone), (1, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![],
        },
        session_id: 1,
    };

    let packet_test2 = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse {
            flood_id: 123,
            path_trace: vec![(0, NodeType::Drone), (1, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1,0],
        },
        session_id: 1,
    };

    match receiver1.recv() {
        Ok(received_packet) => {
            assert_eq!(packet_test1.clone(), received_packet.clone());
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }

    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");


    match packet_receive_test_drone.recv() {
        Ok(received_packet) => {
            assert_eq!(packet_test2.clone(), received_packet.clone());
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn send_flood_response_to_neighbor() {
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    // creation channel for neighbor
    let (neighbor_send, neighbor_recv) = crossbeam_channel::unbounded::<Packet>();

    // create map for neighbors
    let mut senders = HashMap::new();
    senders.insert(2, neighbor_send); // Drone 2 like neighbor

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders,
        0.1,            // valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse { flood_id: 0, path_trace: vec![(2, NodeType::Client), (1, NodeType::Drone), (0, NodeType::Drone)] }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };


    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send.send(packet.clone()).expect("Error sending the packet...");

    let packet_test = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse { flood_id: 0, path_trace: vec![(2, NodeType::Client), (1, NodeType::Drone), (0, NodeType::Drone)] }),
        routing_header: SourceRoutingHeader {
            hop_index: 2,
            hops: vec![0, 1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    match neighbor_recv.recv() {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn forward_flood_response_no_neighbor() {
    let (sim_controller_send, sim_controller_recv) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        HashMap::new(), // no neighbor
        0.1,            // valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse { flood_id: 0, path_trace: vec![(2, NodeType::Client), (1, NodeType::Drone), (0, NodeType::Drone)] }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send.send(packet.clone()).expect("Error sending the packet...");

    match sim_controller_recv.recv() {
        Ok(DroneEvent::ControllerShortcut(received_packet)) => {
            assert_eq!(packet.clone(), received_packet.clone());
            println!("Packet successfully sent to Simulation Controller {:?}", received_packet.pack_type);
        }
        _ => {println!("There is a problem");}
    }
}