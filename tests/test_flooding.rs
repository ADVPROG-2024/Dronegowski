mod common;

use common::default_drone;
use dronegowski::MyDrone;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use crossbeam_channel;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{SourceRoutingHeader, NodeId};
use wg_2024::packet::{Ack, FloodRequest, Packet, PacketType, NodeType};

#[test]
fn test_flood_request_handling() {
    // Creazione dei canali
    let (sender1, _) = crossbeam_channel::unbounded::<Packet>();  // Drone 2 neighbour
    let (sender2, _) = crossbeam_channel::unbounded::<Packet>();  // Drone 2 neighbour

    let (sim_controller_send, sim_controller_receive) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    // Mapping dei neighbour ai canali
    let mut senders = HashMap::new();
    senders.insert(2, sender1);
    senders.insert(3, sender2);



    // Creazione del drone con configurazione dei canali
    let mut my_drone = MyDrone::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders,
        0.1,
    );

    // Creazione del pacchetto FloodRequest da inviare al drone
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

    println!("Invio pacchetto FloodRequest...");

    //Invio il pacchetto al mio drone
    packet_send.send(packet.clone()).expect("Errore nell'invio del pacchetto!");

    //Verifica che il pacchetto venga ricevuto correttamente dal drone
    match packet_receive.recv() {
        Ok(received_packet) => {
            println!("Pacchetto ricevuto: {:?}", received_packet.pack_type);
        }
        Err(_) => println!("Timeout: Nessun pacchetto ricevuto."),
    }

}
