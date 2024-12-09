mod common;

use crossbeam_channel;
use dronegowski::{Dronegowski};
use std::collections::HashMap;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Ack, Fragment, Nack, NackType, Packet, PacketType};
const TIMER: Duration = Duration::from_secs(5);

#[test]
#[should_panic(expected = "pdr out of bounds because is negative")]
fn set_pdr_negative() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_send_packet, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, //Valid PDR
    );

    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::SetPacketDropRate(-0.7)).expect("Error sending the command...");

    //Wait until thread terminate and verify if the panic success
    handle.join().expect("pdr out of bounds because is negative");
}

#[test]
#[should_panic(expected = "pdr out of bounds because is too big")]
fn set_pdr_too_big() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_send_packet, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, //Valid PDR
    );

    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::SetPacketDropRate(1.7)).expect("Error sending the command...");

    //Wait until thread terminate and verify if the panic success
    handle.join().expect("pdr out of bounds because is too big");
}

#[test]
fn set_pdr(){
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_send_packet, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, //Valid PDR
    );

    std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::SetPacketDropRate(1.7)).expect("Error sending the command...");
}

#[test]
#[should_panic(expected = "The node it's not a neighbour")]
fn remove_sender_no_neighbour(){
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_send_packet, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, //Valid PDR
    );

    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::RemoveSender(0)).expect("Error sending the command...");

    //Wait until thread terminate and verify if the panic success
    handle.join().expect("The node it's not a neighbour");
}

#[test]
fn remove_sender(){
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();
    let(send_neighbour, _) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders = HashMap::new();
    senders.insert(0, packet_send_test_drone.clone());
    senders.insert(2, send_neighbour.clone());

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_my_drone.clone(),
        senders,
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
            hops: vec![0, 1, 2], //Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::RemoveSender(2)).expect("Error sending the command...");

    std::thread::sleep(Duration::from_millis(10));

    packet_send_my_drone.send(packet.clone()).expect("Error sending the fragment...");

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
#[should_panic(expected = "The neighbour is already present")]
fn add_sender_already_present(){
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_send_packet, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (neighbor_send, _) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders = HashMap::new();
    senders.insert(0, neighbor_send.clone());

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        senders,
        0.1, //Valid PDR
    );

    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::AddSender(0, neighbor_send)).expect("Error sending the command...");

    handle.join().expect("The neighbour is already present");
}

#[test]
fn add_sender(){
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (neighbor_send, neighbor_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        HashMap::new(), //No neighbor
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
            hops: vec![0, 1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::AddSender(2, neighbor_send)).expect("Error sending the command...");

    std::thread::sleep(Duration::from_millis(10));

    packet_send.send(packet.clone()).expect("Error sending the fragment...");

    let packet_test = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 2,
            hops: vec![0, 1, 2], // Path: Drone 1 -> Drone 2
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
fn crash_command_test() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create channel for the neighbor
    let (send_controller_test_drone, controller_receive_test_drone) = crossbeam_channel::unbounded::<DroneCommand>();
    let (neighbor_send, neighbor_receive) = crossbeam_channel::unbounded::<Packet>();

    //Create map for neighbors
    let mut senders_test_drone = HashMap::new();
    senders_test_drone.insert(1, packet_send.clone());
    let mut senders = HashMap::new();
    senders.insert(2, neighbor_send); //Drone 2 like neighbor

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive.clone(),
        senders,
        0.1,            //Valid PDR
    );

    let mut test_drone = Dronegowski::new(
        2,
        sim_controller_send,
        controller_receive_test_drone,
        neighbor_receive.clone(),
        senders_test_drone,
        0.1,            //Valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // Path: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    send_controller.send(DroneCommand::Crash).expect("Error sending the flood request...");

    packet_send.send(packet.clone()).expect("Error sending the fragment...");

    let packet_test = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 2],
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

    std::thread::spawn(move || {
        test_drone.run();
    });

    send_controller_test_drone.send(DroneCommand::RemoveSender(1)).expect("Error sending the command...");
    drop(packet_send);

    //Wait until thread terminate
    handle.join().expect("The drone is crashed");
}