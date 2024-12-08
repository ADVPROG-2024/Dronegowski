mod common;

use common::default_drone;
use crossbeam_channel;
use dronegowski::{Dronegowski};
use std::collections::HashMap;
use wg_2024::drone::Drone;
use wg_2024::packet::{Packet};

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_too_big() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    Dronegowski::new(
        1, //
        def_drone_opts.clone().get_sim_controller_send(),
        def_drone_opts.clone().get_sim_controller_recv(),
        def_drone_opts.clone().get_packet_recv(),
        def_drone_opts.clone().get_packet_send(),
        1.5, // PDR out of bounds
    );
}

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_negative() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    Dronegowski::new(
        1, // ID del drone
        def_drone_opts.clone().get_sim_controller_send(),
        def_drone_opts.clone().get_sim_controller_recv(),
        def_drone_opts.clone().get_packet_recv(),
        def_drone_opts.clone().get_packet_send(),
        -0.1, // PDR fuori dai limiti
    );
}

#[test]
#[should_panic(expected = "neighbor with id 1 which is the same as drone")]
fn neighbor_is_self() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let (sender, _) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, sender); // Il neighbor has same ID of drone

    Dronegowski::new(
        1, // ID of drone
        def_drone_opts.clone().get_sim_controller_send(),
        def_drone_opts.clone().get_sim_controller_recv(),
        def_drone_opts.clone().get_packet_recv(),
        senders,
        0.5, // valid PDR
    );
}