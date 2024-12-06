mod common;

use common::default_drone;
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, NodeType, Packet, PacketType};

fn test_flood_request_handling() {

    let (mut def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        def_drone_opts.packet_send,
        1.0, // PDR fuori dai limiti
    );

    // Avvia il drone in un thread separato
    let drone_thread = std::thread::spawn(move || {
        def_drone_opts.run();
    });
    let node_id: NodeId = 0;
    // Crea una FloodRequest e inviala al drone
    let flood_request = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 123,            // ID unico della flood
            ,
            path_trace: vec![(0, NodeType::Drone)], // Percorso iniziale
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1], // Il drone è il destinatario
        },
        session_id: 456, // Session ID
    };

    packet_send_to_drone.send(flood_request.clone()).unwrap();

    // Verifica la FloodRequest inoltrata al neighbour
    let received_packet = drone_to_neighbour_recv.recv().unwrap();
    if let PacketType::FloodRequest(ref req) = received_packet.pack_type {
        assert_eq!(req.flood_id, 123);
        assert!(req.path_trace.contains(&(1, NodeType::Drone)));
    } else {
        panic!("Il drone ha inviato un pacchetto non valido: {:?}", received_packet);
    }

    // Invia un comando per fermare il drone
    controller_send.send(DroneCommand::Crash).unwrap();
    drone_thread.join().unwrap();
}
