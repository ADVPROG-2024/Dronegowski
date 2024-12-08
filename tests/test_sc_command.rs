mod common;
mod test_forwarding;

use common::default_drone;
use crossbeam_channel;
use dronegowski::{DroneDebugOption, DroneState, MyDrone};
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::SourceRoutingHeader;
use wg_2024::packet::{Ack, Fragment, Nack, NackType, Packet, PacketType};
#[test]
fn set_pdr_failure() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    let mut my_drone = MyDrone::new(
        1,
        def_drone_opts.clone().get_sim_controller_send(),
        def_drone_opts.clone().get_sim_controller_recv(),
        def_drone_opts.clone().get_packet_recv(),
        def_drone_opts.clone().get_packet_send(),
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
