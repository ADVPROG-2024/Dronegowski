use crossbeam_channel::unbounded;
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Fragment, Packet, PacketType};

#[test]
fn fragment_to_neightbours() {
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

    let fragment_packet = Packet {
        pack_type: PacketType::MsgFragment(Fragment {
            fragment_index: 1,
            total_n_fragments: 3,
            length: 128,
            data: [0; 128],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2],
        },
        session_id: 0,
    };

    // Marco
    //assert!(my_drone.forward_packet(fragment_packet.clone()).is_ok());

    // Controlla che il frammento sia stato ricevuto dal neighbor
    let received_packet = neighbor_recv.try_recv();
    assert!(received_packet.is_ok());
}
