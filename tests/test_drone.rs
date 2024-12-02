mod common;

use common::default_drone;
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Ack, Packet, PacketType};

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_too_big() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        def_drone_opts.packet_send,
        1.5, // PDR fuori dai limiti
    );
}

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_negative() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        def_drone_opts.packet_send,
        -0.1, // PDR fuori dai limiti
    );
}

#[test]
#[should_panic(expected = "neighbor with id 1 which is the same as drone")]
fn neighbor_is_self() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let (sender, _) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, sender); // Il neighbor ha lo stesso ID del drone

    MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        senders,
        0.5, // PDR valido
    );
}

// Per utilizzare questo test bisogna rendere forward_packet public
#[test]
fn forward_packet_success() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let (sender, receiver) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(2, sender); // Drone 2 è il neighbor

    let mut my_drone = MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        senders,
        0.1, // PDR valido
    );

    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // Percorso: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    assert!(my_drone.forward_packet(packet).is_ok());

    // Controlla che il pacchetto sia stato inoltrato correttamente
    assert!(receiver.try_recv().is_ok());
}

// Per utilizzare questo test bisogna rendere forward_packet public
#[test]
fn forward_packet_no_neighbor() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let mut my_drone = MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        HashMap::new(), // Nessun neighbor
        0.1,            // PDR valido
    );

    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // Percorso: Drone 1 -> Drone 2 (Drone 2 non è neighbor)
        },
        session_id: 1,
    };

    assert!(my_drone.forward_packet(packet).is_err());
}

// Per utilizzare questo test bisogna rendere set_pdr public
#[test]
fn set_pdr_failure() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    let mut my_drone = MyDrone::new(
        1,
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        def_drone_opts.packet_send,
        0.1,
    );

    assert!(my_drone.set_pdr(1.5).is_err());
    assert!(my_drone.set_pdr(-0.1).is_err());
}
