mod common;

use dronegowski::Dronegowski;
use std::collections::{HashMap};
use std::time::Duration;
use crossbeam_channel;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, Packet, PacketType, NodeType, FloodResponse};
const TIMER: Duration = Duration::from_secs(5);

#[test]
fn test_flood_request() {
    //Create channel
    let (sim_controller_send, sim_controller_receive) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (neighbor_send, neighbor_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders = HashMap::new();
    senders.insert(2, neighbor_send); //Drone 2 neighbour

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders.clone(),
        0.0         //Valid PDR
    );

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

    match neighbor_receive.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node 2", received_packet.pack_type);
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
fn test_flood_request_no_neighbour(){
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
        0.0     //Valid PDR
    );

    let _ = Dronegowski::new(
        0,
        sim_controller_send,
        controller_receive,
        packet_receive_test_drone.clone(),
        senders_test_drone,
        0.0     //Valid PDR
    );


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

    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    match packet_receive_test_drone.recv_timeout(TIMER) {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn test_flood_request_already_received(){
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();
    let (neighbor_send, neighbor_receiver) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders_my_drone = HashMap::new();
    senders_my_drone.insert(0, packet_send_test_drone.clone());
    senders_my_drone.insert(2, neighbor_send);
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

    match neighbor_receiver.recv_timeout(TIMER)  {
        Ok(received_packet) => {
            assert_eq!(packet_test1.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }

    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    match packet_receive_test_drone.recv_timeout(TIMER)  {
        Ok(received_packet) => {
            assert_eq!(packet_test2.clone(), received_packet.clone());
            println!("Packet {:?} successfully received by the node", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
fn send_flood_response_to_neighbor() {
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
        0.1,            //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse { flood_id: 0, path_trace: vec![(2, NodeType::Client), (1, NodeType::Drone), (0, NodeType::Drone)] }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1, 2], //Path: Drone 1 -> Drone 2
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
}

#[test]
fn forward_flood_response_no_neighbor() {
    //Create channel
    let (sim_controller_send, sim_controller_receive) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        HashMap::new(),     //No neighbor
        0.1,            //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse { flood_id: 0, path_trace: vec![(2, NodeType::Client), (1, NodeType::Drone), (0, NodeType::Drone)] }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], //Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    packet_send.send(packet.clone()).expect("Error sending the packet...");

    match sim_controller_receive.recv_timeout(TIMER) {
        Ok(DroneEvent::ControllerShortcut(received_packet)) => {
            assert_eq!(packet.clone(), received_packet.clone());
            println!("Packet {:?} successfully sent to Simulation Controller", received_packet.pack_type);
        }
        _ => println!("Timeout: No packet received.")
    }
}