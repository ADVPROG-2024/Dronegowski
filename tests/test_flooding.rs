mod common;

use common::default_drone;
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, NodeType, Packet, PacketType};

fn test_flood_request_handling() {

    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let (sender, receiver) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(2, sender); // Drone 2 è il neighbour

    let mut my_drone = MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        senders,
        0.1, // PDR valido
    );

    // Avvia il drone in un thread separato
    let drone_thread = std::thread::spawn(move || {
        my_drone.run();
    });
    // Crea una FloodRequest e inviala al drone
    let flood_request = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 123,            // ID unico della flood
            initiator_id: 0,
            path_trace: vec![(0, NodeType::Drone)], // Percorso iniziale
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1], // Il drone è il destinatario
        },
        session_id: 456, // Session ID
    };

    assert!(my_drone.forward_packet(flood_request).is_ok());

    // Verifica la FloodRequest inoltrata al neighbour
    let received_packet = receiver.recv().unwrap();
    if let PacketType::FloodRequest(ref req) = received_packet.pack_type {
        assert_eq!(req.flood_id, 123);
        assert!(req.path_trace.contains(&(1, NodeType::Drone)));
    } else {
        panic!("Il drone ha inviato un pacchetto non valido: {:?}", received_packet);
    }

    // Invia un comando per fermare il drone
    drone_thread.join().unwrap();
}
