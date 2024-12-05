mod common;

use common::default_drone;
use dronegowski::MyDrone;
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Ack, NackType, Packet, PacketType};
use crossbeam_channel::{select, unbounded, Receiver, Sender};


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

#[test]
fn drone_listen_on_closed_channels() {
    use crossbeam_channel::unbounded;

    // Creiamo i canali
    let (event_send, event_recv) = unbounded::<DroneEvent>();
    let (command_send, command_recv) = unbounded::<DroneCommand>();
    let (packet_send, packet_recv) = unbounded::<Packet>();

    // Chiudiamo i canali lasciandoli fuori dallo scope
    let _ = event_recv;
    let _ = command_recv;
    let _ = packet_recv;

    // Creiamo il drone
    let mut my_drone = MyDrone::new(
        1,              // ID del drone
        event_send,     // Sender degli eventi
        command_recv,   // Receiver dei comandi (chiuso)
        packet_recv,    // Receiver dei pacchetti (chiuso)
        HashMap::new(), // Nessun neighbor
        0.1,            // PDR valido
    );

    // Eseguiamo il drone in un thread separato
    let handle = std::thread::spawn(move || {
        my_drone.run(); // Deve terminare senza panico
    });

    // Attesa breve per assicurarsi che il thread termini
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verifica che il thread non sia in deadlock
    assert!(handle.join().is_ok());
}

#[test]
fn ack_failure() {
        // Step 1: Configura i droni
        let (def_drone_opts_sender, _recv_event_sender, _send_command_sender, sender_to_receiver) = default_drone();
        let (def_drone_opts_receiver, _recv_event_receiver, _send_command_receiver, receiver_channel) = default_drone();

        // Drone mittente
        let mut sender_drone = MyDrone::new(
            1, // ID del drone mittente
            def_drone_opts_sender.sim_controller_send,
            def_drone_opts_sender.sim_controller_recv,
            def_drone_opts_sender.packet_recv,
            HashMap::new(), // Mappa vuota, aggiungiamo dopo
            0.1,            // PDR valido
        );

        // Drone ricevente
        let mut receiver_drone = MyDrone::new(
            2, // ID del drone ricevente
            def_drone_opts_receiver.sim_controller_send,
            def_drone_opts_receiver.sim_controller_recv,
            def_drone_opts_receiver.packet_recv.clone(),
            HashMap::new(),
            0.1,
        );

        // Collegare i droni come neighbor
        sender_drone
            .add_sender(2, sender_to_receiver.clone())
            .expect("Impossibile aggiungere il sender");

        receiver_drone
            .add_sender(1, receiver_channel.clone())
            .expect("Impossibile aggiungere il sender");

        // Creare un pacchetto di tipo Ack
        let ack_packet = Packet {
            pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: vec![1, 2], // Il percorso include i droni
            },
            session_id: 42,
        };

        // Inviare il pacchetto
        assert!(sender_drone.forward_packet(ack_packet.clone()).is_ok());

        // Verificare il risultato
        if let Ok(received_packet) = def_drone_opts_receiver.packet_recv.recv() {
            //assert_eq!(received_packet.pack_type, PacketType::Ack(Ack { fragment_index: 0 }));
            assert_eq!(received_packet.routing_header.hops, vec![1, 2]);
            assert_eq!(received_packet.session_id, 42);
        } else {
            panic!("Il drone ricevente non ha ricevuto il pacchetto!");
        }
}
