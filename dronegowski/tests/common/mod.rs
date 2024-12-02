use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use dronegowski::MyDrone;
use wg_2024::{
    controller::{DroneCommand, NodeEvent},
    packet::Packet,
};

pub fn default_drone() -> (
    MyDrone,
    Receiver<NodeEvent>,
    Sender<DroneCommand>,
    Sender<Packet>,
) {
    let (sim_controller_send, sim_controller_recv) = crossbeam_channel::unbounded::<NodeEvent>();
    let (command_send, command_recv) = crossbeam_channel::unbounded::<DroneCommand>();
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();

    let drone = MyDrone {
        id: 0,
        sim_controller_send,
        sim_controller_recv: command_recv,
        packet_recv,
        packet_send: HashMap::new(),
        pdr: 0.1,
    };

    (drone, sim_controller_recv, command_send, packet_send)
}
