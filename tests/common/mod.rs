use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use dronegowski::MyDrone;
use wg_2024::drone::Drone;
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    packet::Packet,
};

pub fn default_drone() -> (
    MyDrone,
    Receiver<DroneEvent>,
    Sender<DroneCommand>,
    Sender<Packet>,
) {
    let (sim_controller_send, sim_controller_recv) = crossbeam_channel::unbounded::<DroneEvent>();
    let (command_send, command_recv) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();

    let mut drone = MyDrone::new(
        0,
        sim_controller_send,
        command_recv,
        packet_recv,
        HashMap::new(),
        0.1,
    );

    (drone, sim_controller_recv, command_send, packet_send)
}
