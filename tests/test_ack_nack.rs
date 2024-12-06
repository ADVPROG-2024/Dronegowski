mod common;

use crossbeam_channel::unbounded;
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Ack, Nack, NackType, Packet, PacketType};


#[test]
fn send_ack_to_neighbor() {

    // Creazione dei canali
    let (controller_send, _recv_event) = unbounded();
    let (_send_command, controller_recv) = unbounded();
    let (_send_packet, packet_recv) = unbounded();

    // Creazione del canale per il neighbor
    let (neighbor_send, neighbor_recv) = unbounded();

    // Crea una mappa per i neighbors
    let mut packet_send = HashMap::new();
    packet_send.insert(2, neighbor_send); // Drone 2 come neighbor

    // Inizializza il drone
    let mut my_drone = MyDrone::new(
        1, // ID del drone
        controller_send,
        controller_recv,
        packet_recv,
        packet_send,
        0.1, // PDR
    );

    // Creazione del pacchetto Ack
    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // Percorso: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    // Invia il pacchetto
    assert!(my_drone.forward_packet(packet.clone()).is_ok());

    // Controlla che il pacchetto sia stato ricevuto dal neighbor
    let received_packet = neighbor_recv.try_recv();
    assert!(received_packet.is_ok());
    // assert_eq!(received_packet.unwrap(), packet);
}

#[test]
fn send_nack_to_neighbor() {

    // Creazione dei canali
    let (controller_send, _recv_event) = unbounded();
    let (_send_command, controller_recv) = unbounded();
    let (_send_packet, packet_recv) = unbounded();

    // Creazione del canale per il neighbor
    let (neighbor_send, neighbor_recv) = unbounded();

    // Crea una mappa per i neighbors
    let mut packet_send = HashMap::new();
    packet_send.insert(2, neighbor_send); // Drone 2 come neighbor

    // Inizializza il drone
    let mut my_drone = MyDrone::new(
        1, // ID del drone
        controller_send,
        controller_recv,
        packet_recv,
        packet_send,
        0.1, // PDR
    );

    // Creazione del pacchetto Nack
    let nack_packet = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 0, // Indice del frammento che ha causato il Nack
            nack_type: NackType::ErrorInRouting(2), // Motivo del Nack
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // Percorso: Drone 1 -> Drone 2
        },
        session_id: 1,
    };

    // Invia il pacchetto Nack
    assert!(my_drone.forward_packet(nack_packet.clone()).is_ok());

    // Controlla che il pacchetto sia stato ricevuto dal neighbor
    let received_packet = neighbor_recv.try_recv();
    assert!(received_packet.is_ok());
    // assert_eq!(received_packet.unwrap(), nack_packet);
}

