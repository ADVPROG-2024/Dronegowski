use crossbeam_channel::{select_biased, Receiver, Sender};
use rand::Rng;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodResponse, Nack, NackType, NodeType, Packet, PacketType};

mod components_drone;
#[derive(Clone, Debug, PartialEq)]
pub enum DroneState {
    Active,
    Crashing,
    Crashed,
}

#[derive(Debug, Clone)]
pub struct Dronegowski {
    id: NodeId,
    sim_controller_send: Sender<DroneEvent>, //Channel used to send commands to the SC
    sim_controller_recv: Receiver<DroneCommand>, //Channel used to receive commands from the SC
    packet_recv: Receiver<Packet>,           //Channel used to receive packets from nodes
    packet_send: HashMap<NodeId, Sender<Packet>>, //Map containing the sending channels of neighbour nodes
    pdr: f32,                                     //PDR
    state: DroneState,                            //Drone state
    flood_id_vec: HashSet<(u64, NodeId)>,         //HashSet storing ids of already received flood_id
}

impl Drone for Dronegowski {
    fn new(
        id: NodeId,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        pdr: f32,
    ) -> Self {
        assert!(
            !packet_send.contains_key(&id),
            "neighbor with id {} which is the same as drone",
            id
        );
        assert!(!(pdr > 1.0 || pdr < 0.0), "pdr out of bounds");
        format!("Created drone {id} with PDR: {pdr}");

        Self {
            id,
            sim_controller_send: controller_send,
            sim_controller_recv: controller_recv,
            packet_recv,
            packet_send,
            pdr,
            state: DroneState::Active,
            flood_id_vec: HashSet::new(),
        }
    }

    fn run(&mut self) {
        log::info!(
            "Drone {} entering the run loop in state {:?}",
            self.id,
            self.state
        );
        loop {
            match self.state {
                DroneState::Active => {
                    select_biased! {
                        recv(self.sim_controller_recv) -> command_res => {
                            if let Ok(command) = command_res {
                                self.handle_command(command);
                            }
                        },
                        recv(self.packet_recv) -> packet_res => {
                            if let Ok(packet) = packet_res {
                                self.handle_packet(packet);
                            }
                        }

                    }
                }
                DroneState::Crashing => match self.packet_recv.recv() {
                    Ok(packet) => {
                        log::info!("Drone {} processing packet in Crashing state", self.id);
                        self.handle_packet(packet);
                    }
                    Err(_) => {
                        log::info!(
                            "Drone {} has completed crashing. Transitioning to Crashed state.",
                            self.id
                        );
                        self.state = DroneState::Crashed;
                        break;
                    }
                },
                DroneState::Crashed => {
                    log::info!("Drone {} is in Crashed state. Exiting loop", self.id);
                    break;
                }
            }
        }
        log::info!("Drone {} has terminated execution", self.id);
    }
}

impl Dronegowski {
    pub fn get_pdr(self) -> f32 {
        self.pdr
    }

    #[must_use]
    pub fn get_sim_controller_recv(self) -> Receiver<DroneCommand> {
        self.sim_controller_recv
    }

    #[must_use]
    pub fn get_sim_controller_send(self) -> Sender<DroneEvent> {
        self.sim_controller_send
    }

    #[must_use]
    pub fn get_packet_recv(self) -> Receiver<Packet> {
        self.packet_recv
    }

    #[must_use]
    pub fn get_packet_send(self) -> HashMap<NodeId, Sender<Packet>> {
        self.packet_send
    }

    pub fn set_pdr(&mut self, pdr: f32) {
        if pdr < 0.0 {
            panic!("pdr {} is out of bounds because is negative", pdr);
        } else if pdr > 1.0 {
            panic!("pdr {} is out of bounds because is too big", pdr);
        } else {
            self.pdr = pdr;
            log::info!("Drone {}: PDR updated to {}", self.id, pdr);
        }
    }

    pub fn get_id(self) -> NodeId {
        self.id
    }

    pub fn get_state(self) -> DroneState {
        self.state
    }

    fn handle_packet(&mut self, mut packet: Packet) {
        match packet.pack_type {
            PacketType::FloodRequest(ref mut flood_request) => {
                if self
                    .flood_id_vec
                    .insert((flood_request.flood_id, flood_request.initiator_id))
                {
                    let previous_id = flood_request.path_trace.last().unwrap().clone();
                    flood_request.path_trace.push((self.id, NodeType::Drone));
                    if !self.packet_send.iter().any(|(id, _)| *id != previous_id.0) {
                        //There is no other neighbour, proceed to send back a flood response
                        let flood_response = Packet::new_flood_response(
                            SourceRoutingHeader {
                                hop_index: 0,
                                hops: flood_request
                                    .path_trace
                                    .iter()
                                    .map(|(id, _)| *id)
                                    .rev()
                                    .collect(),
                            },
                            packet.session_id,
                            FloodResponse {
                                flood_id: flood_request.flood_id,
                                path_trace: flood_request.path_trace.clone(),
                            },
                        );
                        self.forward_packet_safe(&flood_response);
                        log::info!("Drone {} correctly sent back a Flood Response because has no neighbour",self.id);
                    } else {
                        for neighbour in self.packet_send.clone() {
                            if neighbour.0 != previous_id.0 {
                                self.forward_packet_flood_request(
                                    packet.clone(),
                                    neighbour.clone(),
                                );
                                log::info!(
                                    "Drone {} correctly sent the Flood Request to neighbor with {} id",
                                    self.id, neighbour.0
                                );
                            }
                        }
                    }
                } else {
                    //The packet has already seen by this drone, proceed to send back a flood response
                    flood_request.path_trace.push((self.id, NodeType::Drone));
                    let flood_response = Packet::new_flood_response(
                        SourceRoutingHeader {
                            hop_index: 0,
                            hops: flood_request
                                .path_trace
                                .iter()
                                .map(|(id, _)| *id)
                                .rev()
                                .collect(),
                        },
                        packet.session_id,
                        FloodResponse {
                            flood_id: flood_request.flood_id,
                            path_trace: flood_request.path_trace.clone(),
                        },
                    );
                    self.forward_packet_safe(&flood_response);
                    log::info!("Drone {} correctly sent back a Flood Response because has already received this flood request",self.id);
                }
            }
            _ => {
                log::info!("Drone {}: Received packet {:?}", self.id, packet);
                if let Some(node_id) = packet
                    .routing_header
                    .hops
                    .get(packet.routing_header.hop_index)
                {
                    if *node_id == self.id {
                        match packet.pack_type {
                            PacketType::Ack(_)
                            | PacketType::Nack(_)
                            | PacketType::FloodResponse(_) => {
                                self.forward_packet_safe(&packet);
                            }
                            PacketType::MsgFragment(ref _fragment) => {
                                log::info!("Drone {}: Received fragment {:?}", self.id, _fragment);
                                if self.state == DroneState::Crashing {
                                    log::warn!("Drone {}: Drone is crashed, sending Nack", self.id);
                                    self.handle_forwarding_error(
                                        &packet,
                                        NackType::ErrorInRouting(
                                            *packet
                                                .routing_header
                                                .hops
                                                .get(packet.routing_header.hop_index + 1)
                                                .unwrap(),
                                        ),
                                    );
                                } else if self.should_drop_packet() {
                                    log::warn!("Drone {}: packet dropped, sending Nack", self.id);
                                    self.handle_forwarding_error(&packet, NackType::Dropped);
                                    self.sim_controller_send
                                        .send(DroneEvent::PacketDropped(packet.clone()))
                                        .expect("Something wrong");
                                } else {
                                    self.forward_packet_safe(&packet);
                                }
                            }
                            _ => {
                                log::warn!("Drone {}: Received Unrecognized packet", self.id);
                            }
                        }
                    } else {
                        log::warn!("Drone {}: Received packet not directed to me", self.id);
                        self.handle_forwarding_error(
                            &packet,
                            NackType::UnexpectedRecipient(self.id),
                        );
                    }
                }
            }
        }
    }

    fn handle_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::SetPacketDropRate(pdr) => {
                self.set_pdr(pdr);
            }
            DroneCommand::Crash => {
                log::info!("Drone {} entering Crashing state.", self.id);
                self.set_drone_state(DroneState::Crashing);
            }
            DroneCommand::AddSender(node_id, sender) => {
                self.add_neighbor(node_id, sender);
            }
            DroneCommand::RemoveSender(node_id) => {
                log::info!("Drone {} removing sender {}.", self.id, node_id);
                self.remove_neighbor(&node_id)
            }
        }
    }

    // Method used to send packet to the next hop
    fn forward_packet(&self, mut packet: Packet) -> Result<(), Nack> {
        packet.routing_header.hop_index += 1;

        let next_hop = packet
            .routing_header
            .hops
            .get(packet.routing_header.hop_index);

        // println!("Sender of Drone {} -> {:?}", self.id, self.packet_send);

        let fragment_index = match packet.pack_type.clone() {
            PacketType::MsgFragment(fragment) => fragment.fragment_index,
            _ => 0, // 0 because only MsgFragment can be serialized
        };

        match next_hop {
            Some(next_node) => {
                match self.packet_send.get(next_node) {
                    Some(next_node_channel) => match next_node_channel.send(packet.clone()) {
                        Ok(()) => {
                            log::info!(
                                "Drone {}: packet forwarded to next hop {}",
                                self.id,
                                next_node
                            );
                            self.sim_controller_send
                                .send(DroneEvent::PacketSent(packet.clone()));
                            Ok(())
                        }
                        Err(..) => {
                            panic!("Error occurred while forwarding the packet");
                        }
                    },
                    // None if the next hop is not a drone's neighbour
                    None => {
                        log::warn!("Drone {}: next hop is not a neighbour", self.id);
                        Err(Nack {
                            fragment_index,
                            nack_type: NackType::ErrorInRouting(*next_node),
                        })
                    }
                }
            }
            // Next_hop returns None if the drone is the final destination
            None => Err(Nack {
                fragment_index,
                nack_type: NackType::DestinationIsDrone,
            }),
        }
    }

    fn handle_forwarding_error(&self, packet: &Packet, nack_type: NackType) {
        //If the packet is ACK / NACK / FloodResponse it's sent to the Simulation Controller, otherwise a NACK is created and sent
        match packet.pack_type {
            PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                self.sim_controller_send
                    .send(DroneEvent::ControllerShortcut(packet.clone()))
                    .expect("Something wrong");
            }

            //Error in send a MsgFragment, send back a NACK
            _ => {
                let nack_packet = self.packet_nack(
                    packet,
                    Nack {
                        fragment_index: packet.get_fragment_index(),
                        nack_type,
                    },
                );
                self.forward_packet_safe(&nack_packet);
            }
        }
    }

    fn forward_packet_safe(&self, packet: &Packet) {
        if let Err(nack) = self.forward_packet(packet.clone()) {
            self.handle_forwarding_error(packet, nack.nack_type);
        }
    }

    fn forward_packet_flood_request(&self, packet: Packet, neighbour: (NodeId, Sender<Packet>)) {
        match neighbour.1.send(packet.clone()) {
            Ok(()) => {
                self.sim_controller_send
                    .send(DroneEvent::PacketSent(packet.clone()))
                    .expect("Something wrong");
            }
            Err(..) => panic!("Error sending flood request"),
        }
    }

    fn add_neighbor(&mut self, node_id: NodeId, sender: Sender<Packet>) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.packet_send.entry(node_id) {
            e.insert(sender);
        } else {
            panic!("Sender for node {node_id} already stored in the map!");
        }
    }

    fn set_drone_state(&mut self, state: DroneState) {
        self.state = state;
    }

    fn should_drop_packet(&self) -> bool {
        let mut rng = rand::rng();
        let n: f32 = rng.random_range(0.0..=1.0);
        if n < self.pdr {
            return true;
        }
        false
    }

    fn packet_nack(&self, packet: &Packet, nack: Nack) -> Packet {
        //Path to the source node
        let rev_path = packet
            .routing_header
            .hops
            .split_at(packet.routing_header.hop_index + 1)
            .0
            .iter()
            .rev()
            .copied()
            .collect();

        //Nack packet
        Packet {
            pack_type: PacketType::Nack(nack),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: rev_path,
            },
            session_id: packet.session_id,
        }
    }

    fn remove_neighbor(&mut self, node_id: &NodeId) {
        if self.packet_send.contains_key(node_id) {
            self.packet_send.remove(node_id);
        } else {
            panic!("the {} is not neighbour of the drone {}", node_id, self.id);
        }
    }
}
