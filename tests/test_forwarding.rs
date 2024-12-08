use crossbeam_channel::{unbounded, Sender};
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Fragment, NackType, Packet, PacketType};

#[test]
fn packet_to_neighbours() {
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
        routing_header: SourceRoutingHeader { hop_index: 0, hops: vec![1, 2] },
        session_id: 0,
    };

    // Invia il frammento
    assert!(my_drone.forward_packet(fragment_packet.clone()).is_ok());

    // Controlla che il frammento sia stato ricevuto dal neighbor
    let received_packet = neighbor_recv.try_recv();
    assert!(received_packet.is_ok());
    if let PacketType::MsgFragment(received_fragment) = received_packet.unwrap().pack_type {
        assert_eq!(received_fragment.fragment_index, 1);
        assert_eq!(received_fragment.total_n_fragments, 3);
        assert_eq!(received_fragment.length, 128);
    } else {
        panic!("Il pacchetto ricevuto non è un frammento!");
    }
}

#[test]
fn test_next_hop_not_neighbor() {
    use crossbeam_channel::unbounded;

    // Setup canali per la simulazione
    let (event_send, _event_recv) = unbounded::<DroneEvent>();
    let (command_send, command_recv) = unbounded::<DroneCommand>();
    let (packet_send, packet_recv) = unbounded::<Packet>();

    // Nessun vicino per il drone
    let neighbors: HashMap<NodeId, Sender<Packet>> = HashMap::new();

    // Crea un drone senza vicini
    let mut my_drone = MyDrone::new(
        1,                // ID del drone
        event_send,       // Canale eventi
        command_recv,     // Canale comandi
        packet_recv,      // Canale ricezione pacchetti
        neighbors,        // Nessun neighbor
        0.0,              // PDR
    );

    // Crea un pacchetto con un next_hop non valido
    let packet = Packet {
        pack_type: PacketType::MsgFragment(Fragment {
            fragment_index: 1,
            total_n_fragments: 3,
            length: 128,
            data: [0; 128],
        }), // Usando un frammento come esempio
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![1, 2], // 2 è il next_hop non valido
        },
        session_id: 1,
    };
    

    // Simula l'invio del pacchetto al drone
    packet_send.send(packet.clone()).expect("Errore nell'invio del pacchetto");

    // Esegui una iterazione del loop del drone usando run_once
    my_drone.run();

    // Non abbiamo accesso diretto ai pacchetti Nack, quindi usiamo eventi per verificare
    if let DroneEvent::PacketDropped(dropped_packet) = _event_recv.try_recv().expect("Nessun evento ricevuto") {
        if let PacketType::Nack(nack) = dropped_packet.pack_type {
            assert_eq!(nack.nack_type, NackType::ErrorInRouting(2));
            assert_eq!(dropped_packet.routing_header.hops, vec![1]); // Percorso inverso al mittente
        } else {
            panic!("Il pacchetto ricevuto non è un Nack");
        }
    } else {
        panic!("Non è stato generato un evento PacketDropped");
    }
}
