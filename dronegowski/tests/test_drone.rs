mod common;

use common::default_drone;
use crossbeam_channel::unbounded;
use dronegowski::MyDrone;
use std::collections::HashMap;
use std::{fs, thread};
use wg_2024::config::Config;
// use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::packet::Packet;

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_too_big() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    let _my_drone = MyDrone::new(DroneOptions {
        pdr: 345.3,
        ..def_drone_opts
    });
}

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_negative() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    let _my_drone = MyDrone::new(DroneOptions {
        pdr: -0.1,
        ..def_drone_opts
    });
}

#[test]
#[should_panic(expected = "neighbor with id 1 which is the same as drone")]
fn neighbor_is_self() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let (sender, _) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, sender);
    let _my_drone = MyDrone::new(DroneOptions {
        packet_send: senders,
        id: 1,

        ..def_drone_opts
    });
}