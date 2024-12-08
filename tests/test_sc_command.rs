mod common;

use crossbeam_channel;
use dronegowski::{Dronegowski};
use std::collections::HashMap;
use std::time::Duration;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Ack, Fragment, Nack, NackType, Packet, PacketType};

#[test]
#[should_panic(expected = "pdr out of bounds because is negative")]
fn set_pdr_negative() {
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, // starting PDR
    );

    // simulate behavior of drone in separated thread
    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::SetPacketDropRate(-0.7));

    // wait until thread terminate and verify if the panic success
    handle.join().expect("pdr out of bounds because is negative");
}

#[test]
#[should_panic(expected = "pdr out of bounds because is too big")]
fn set_pdr_too_big() {
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, // starting PDR
    );

    // simulate behavior of drone in separated thread
    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::SetPacketDropRate(1.7));

    // wait until thread terminate and verify if the panic success
    handle.join().expect("pdr out of bounds because is too big");
}

#[test]
fn set_pdr(){
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, // PDR iniziale
    );

    // simulate behavior of drone in separated thread
    std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::SetPacketDropRate(0.7));
}

#[test]
#[should_panic(expected = "The node it's not a neighbour")]
fn remove_sender_no_neighbour(){
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        HashMap::new(),
        0.1, // PDR iniziale
    );

    // simulate behavior of drone in separated thread
    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::RemoveSender(0));

    // wait until thread terminate and verify if the panic success
    handle.join().expect("The node it's not a neighbour");
}

#[test]
fn remove_sender(){
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send_my_drone, packet_receive_my_drone) = crossbeam_channel::unbounded::<Packet>();
    let (packet_send_test_drone, packet_receive_test_drone) = crossbeam_channel::unbounded::<Packet>();

    // Mapping dei neighbour ai canali
    let(send_neighbour, _) = crossbeam_channel::unbounded::<Packet>();

    let mut senders = HashMap::new();
    senders.insert(0, packet_send_test_drone.clone());
    senders.insert(2, send_neighbour.clone());

    let _ = Dronegowski::new(
        0,
        sim_controller_send.clone(),
        controller_receive.clone(),
        packet_receive_test_drone.clone(),
        HashMap::new(),
        0.0
    );

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive_my_drone.clone(),
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
            hops: vec![0, 1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::RemoveSender(2));
    std::thread::sleep(Duration::from_millis(10));
    packet_send_my_drone.send(packet.clone()).expect("Error sending the flood request...");

    let packet_test = Packet {
        pack_type: PacketType::Nack(Nack{
            fragment_index: 10,
            nack_type: NackType::ErrorInRouting(2),
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 0], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };
    match packet_receive_test_drone.recv() {
        Ok(received_packet) => {
            assert_eq!(packet_test.clone(), received_packet.clone());
            println!("Packet successfully received by the node {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: No packet received."),
    }
}

#[test]
#[should_panic(expected = "The neighbour is already present")]
fn add_sender_already_present(){
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let (sender1, receiver) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, sender1.clone());

    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send.clone(),
        controller_receive,
        packet_receive,
        senders,
        0.1, // PDR iniziale
    );

    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::AddSender(0, sender1));

    handle.join().expect("The neighbour is already present");
}

#[test]
fn add_sender(){
    let (sim_controller_send, sim_controller_recv) = crossbeam_channel::unbounded::<DroneEvent>();
    let (controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    // creation of channel for neighbor
    let (neighbor_send, neighbor_recv) = crossbeam_channel::unbounded::<Packet>();

    // initialize drone
    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
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
            hops: vec![0, 1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    std::thread::spawn(move || {
        my_drone.run();
    });

    controller_send.send(DroneCommand::AddSender(2, neighbor_send));
    std::thread::sleep(Duration::from_millis(10));
    packet_send.send(packet.clone());

    let packet_test = Packet {
        pack_type: PacketType::MsgFragment(Fragment{
            fragment_index: 10,
            total_n_fragments: 15,
            length: 5,
            data:[5; 128],
        }),
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
fn crash_command_test() {
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (send_controller, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (send_controller1, controller_receive1) = crossbeam_channel::unbounded::<DroneCommand>();

    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    // creation of channel for neighbors
    let (neighbor_send, neighbor_recv) = crossbeam_channel::unbounded::<Packet>();

    // create map for neighbors
    let mut senders1 = HashMap::new();
    senders1.insert(1, packet_send.clone());
    let mut senders = HashMap::new();
    senders.insert(2, neighbor_send); // Drone 2 like neighbor

    let mut test_drone = Dronegowski::new(
        2,
        sim_controller_send.clone(),
        controller_receive1,
        neighbor_recv.clone(),
        senders1,
        0.1,
    );
    let mut my_drone = Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders,
        0.1,            // valid PDR
    );

    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
        },
        session_id: 1,
    };

    let handle = std::thread::spawn(move || {
        my_drone.run();
    });

    send_controller.send(DroneCommand::Crash);
    packet_send.send(packet.clone()).expect("Error sending the packet...");

    let packet_test = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 2], // Path: Drone 1 -> Drone 2 (Drone 2 not neighbor)
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

    std::thread::spawn(move || {
        test_drone.run();
    });

    send_controller1.send(DroneCommand::RemoveSender(1));
    drop(packet_send);

    // wait until thread terminate
    handle.join().expect("Il thread del drone non è terminato");
}