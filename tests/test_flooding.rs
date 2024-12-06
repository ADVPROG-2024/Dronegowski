mod common;

use common::default_drone;
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, FloodRequest, NodeType, Packet, PacketType};

#[test]
fn test_flood_request_handling() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let (sender1, receiver) = crossbeam_channel::unbounded::<Packet>();
    let (sender2, receiver) = crossbeam_channel::unbounded::<Packet>();

    let mut senders = HashMap::new();
    senders.insert(2, sender1); // Drone 2 è un neighbour
    senders.insert(3, sender2); // Drone 3 è un neighbour


    let mut my_drone = MyDrone::new(
        1, // ID del drone
        def_drone_opts.sim_controller_send,
        def_drone_opts.sim_controller_recv,
        def_drone_opts.packet_recv,
        senders,
        0.1, // PDR valido
    );

    let packet = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest{
            flood_id: 0,
            initiator_id: 0,
            path_trace: vec![(0, NodeType::Client)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![],
        },
        session_id: 1,
    };

    assert!(my_drone.forward_packet(packet).is_ok());

    // Controlla che il pacchetto sia stato inoltrato correttamente
    assert!(receiver.try_recv().is_ok());

}
