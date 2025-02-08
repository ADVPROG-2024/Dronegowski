mod common;

use crossbeam_channel;
use dronegowski::{Dronegowski};
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::packet::{Packet};

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_too_big() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        HashMap::new(),
        1.7,   //PDR too big
    );
}

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_negative() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        HashMap::new(),
        -0.7,   //PDR negative
    );
}

#[test]
#[should_panic(expected = "neighbor with id 1 which is the same as drone")]
fn neighbor_is_self() {
    //Create channel
    let (sim_controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_controller_send, controller_receive) = crossbeam_channel::unbounded::<DroneCommand>();
    let (_, packet_receive) = crossbeam_channel::unbounded::<Packet>();

    let (neighbor_send, _) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, neighbor_send);  //Neighbor has same ID of drone

    Dronegowski::new(
        1,
        sim_controller_send,
        controller_receive,
        packet_receive.clone(),
        senders,
        0.7,   //Valid PDR
    );
}