mod common;
mod test_forwarding;

use common::default_drone;
use crossbeam_channel::unbounded;
use dronegowski::{DroneState, MyDrone};
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Ack, Nack, NackType, Packet, PacketType};

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
fn forward_packet_no_neighbor() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let my_drone = MyDrone::new(
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
fn crash_command_test() {
    use crossbeam_channel::unbounded;
    use std::thread;
    use std::time::Duration;

    // Creazione dei canali
    let (send_event, recv_event) = unbounded(); // Simula eventi al simulatore
    let (send_command, recv_command) = unbounded(); // Comandi dal simulatore
    let (send_packet, recv_packet) = unbounded(); // Pacchetti ricevuti dal drone

    // Inizializzazione del drone
    let mut my_drone = MyDrone::new(
        1,
        send_event,
        recv_command,
        recv_packet,
        HashMap::new(),
        0.1, // PDR valido
    );

    // Avvio del drone in un thread separato
    let drone_thread = thread::spawn(move || {
        my_drone.run();
    });

    // Invia il comando Crash
    send_command
        .send(DroneCommand::Crash)
        .expect("Errore nell'invio del comando Crash");

    // Aspetta un momento per assicurarsi che il drone processi il comando
    thread::sleep(Duration::from_millis(100));

    // Invia alcuni pacchetti al drone
    for _ in 0..3 {
        send_packet
            .send(Packet {
                pack_type: PacketType::Ack(Ack { fragment_index: 0 }),
                routing_header: SourceRoutingHeader {
                    hop_index: 0,
                    hops: vec![1, 2],
                },
                session_id: 1,
            })
            .expect("Errore nell'invio di un pacchetto");
    }

    // Aspetta un momento per permettere al drone di processare i pacchetti
    thread::sleep(Duration::from_millis(100));

    // Chiude il canale dei pacchetti per simulare il completamento
    drop(send_packet);

    // Aspetta per assicurarsi che il drone passi a "Crashed"
    thread::sleep(Duration::from_millis(100));

    // Aspetta che il thread del drone termini
    drone_thread
        .join()
        .expect("Il thread del drone non è terminato");
}
