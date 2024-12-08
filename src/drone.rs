use crossbeam_channel::{select, select_biased, Receiver, Sender};
use rand::Rng;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet;
use wg_2024::packet::{FloodRequest, FloodResponse, Nack, NackType, NodeType, Packet, PacketType};
mod components_drone;
#[derive(Clone, Debug, PartialEq)]
pub enum DroneState {
    Active,
    Crashing,
    Crashed,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum DroneDebugOption {
    Ack,
    Nack,
    FloodResponse,
    FloodRequest,
    MsgFragment,
}

#[derive(Debug, Clone)]
pub struct MyDrone {
    id: NodeId,
    sim_controller_send: Sender<DroneEvent>, // Channel used to send commands to the SC
    sim_controller_recv: Receiver<DroneCommand>, // Channel used to receive commands from the SC
    packet_recv: Receiver<Packet>,           // Channel used to receive packets from nodes
    packet_send: HashMap<NodeId, Sender<Packet>>, // Map containing the sending channels of neighbour nodes
    pdr: f32,                                     // PDR
    state: DroneState,                            // Drone state
    flood_id_vec: HashSet<(u64, u64)>, // HashSet storing ids of already received flood_id
    drone_debug_options: HashMap<DroneDebugOption, bool>, // Map used to know which Debug options are active and which aren't
}

impl Drone for MyDrone {
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

        // initialize drone_debug_options having all values at false
        let drone_debug_options = DroneDebugOption::iter()
            .map(|opt| (opt, false))
            .collect::<HashMap<DroneDebugOption, bool>>();

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
            drone_debug_options,
        }
    }

    fn run(&mut self) {
        loop {
            match self.state {
                DroneState::Active => {
                    select_biased! {
                        // Usa try_recv per evitare il blocco
                        default => {
                            if let Ok(command) = self.sim_controller_recv.try_recv() {
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
                DroneState::Crashing => {
                    match self.packet_recv.recv() {
                        Ok(packet) => {
                            println!("Drone {} processing packet in Crashing state", self.id);
                            self.handle_packet(packet);
                        }
                        Err(_) => {
                            if self.packet_send.is_empty() {
                                println!("Drone {} has completed crashing. Transitioning to Crashed state.", self.id);
                                self.state = DroneState::Crashed;
                                break;
                            }
                        }
                    }
                }
                DroneState::Crashed => {
                    println!("Drone {} is in Crashed state. Exiting loop", self.id);
                    break;
                }
            }
        }
        println!("Drone {} has terminated execution", self.id);
    }
}

impl DroneDebugOption {
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            DroneDebugOption::Ack,
            DroneDebugOption::Nack,
            DroneDebugOption::FloodResponse,
            DroneDebugOption::FloodRequest,
            DroneDebugOption::MsgFragment,
        ]
        .into_iter()
    }
}

impl MyDrone {
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

    pub fn set_pdr(&mut self, pdr: f32) -> Result<(), String> {
        if pdr > 0.0 && pdr < 1.0 {
            println!("Drone {}: updated PDR, {} -> {}", self.id, self.pdr, pdr);
            self.pdr = pdr;
            return Ok(());
        }

        Err("Incorrect value of PDR".to_string())
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
                    .insert((flood_request.flood_id, packet.session_id))
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
                        println!("Drone {} correctly sent back a Flood Response because has no neighbour",self.id);
                    } else {
                        for neighbour in self.packet_send.clone() {
                            if neighbour.0 != previous_id.0 {
                                self.forward_packet_flood_request(
                                    packet.clone(),
                                    neighbour.clone(),
                                );
                                println!(
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
                    println!("Drone {} correctly sent back a Flood Response because has already received this flood request",self.id);
                }
            }
            _ => {
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
                                match packet.pack_type {
                                    PacketType::Ack(_) => {
                                        if self
                                            .clone()
                                            .in_drone_debug_options(DroneDebugOption::Ack)
                                        {
                                            println!(
                                            "[Drone {} - Ack Debug] Ack received and now trying to forward it",
                                            self.id
                                        );
                                        }
                                    }
                                    _ => todo!(), // Debug feature to be finished
                                }
                                self.forward_packet_safe(&packet);
                            }
                            PacketType::MsgFragment(ref fragment) => {
                                if self.state == DroneState::Crashing {
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
                                    self.handle_forwarding_error(&packet, NackType::Dropped);
                                    self.sim_controller_send
                                        .send(DroneEvent::PacketDropped(packet.clone()));
                                } else {
                                    self.forward_packet_safe(&packet);
                                }
                            }
                            _ => (),
                        }
                    } else {
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
                self.set_pdr(pdr).expect("Error in PDR setting");
            }
            DroneCommand::Crash => {
                println!("Drone {} entering Crashing state.", self.id);
                self.set_drone_state(DroneState::Crashing);
            }
            DroneCommand::AddSender(node_id, sender) => {
                self.add_neighbor(node_id, sender)
                    .expect("Sender already present!");
            }
            DroneCommand::RemoveSender(node_id) => {
                println!("Drone {} removing sender {}.", self.id, node_id);
                self.remove_neighbor(&node_id)
                    .expect("Sender is not in self.sender");
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
            // 0 because only MsgFragment can be serialized
            _ => 0,
        };

        match next_hop {
            Some(next_node) => {
                match self.packet_send.get(next_node) {
                    Some(next_node_channel) => match next_node_channel.send(packet.clone()) {
                        Ok(()) => {
                            match packet.pack_type {
                                PacketType::Ack(_) => {
                                    if self.clone().in_drone_debug_options(DroneDebugOption::Ack) {
                                        println!(
                                            "[Drone {} - Ack Debug] Ack sended to {}",
                                            self.id, next_node
                                        );
                                    }
                                }
                                _ => {}
                            }
                            self.sim_controller_send
                                .send(DroneEvent::PacketSent(packet.clone()));
                            Ok(())
                        }
                        Err(..) => {
                            panic!("Errore nell'invio del pacchetto al nodo successivo")
                        }
                    },
                    // None if the next hop is not a drone's neighbour
                    None => Err(Nack {
                        fragment_index,
                        nack_type: NackType::ErrorInRouting(*next_node),
                    }),
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
        // if the packet is a nack/send/floodResponse it's sent to the sim controller, otherwise a nack_packet is created and sent
        match packet.pack_type {
            PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                match packet.pack_type {
                    PacketType::Ack(_) => {
                        if self.clone().in_drone_debug_options(DroneDebugOption::Ack) {
                            println!(
                                "[Drone {} - Ack Debug] Ack sended to Simulation Controller after error",
                                self.id
                            );
                        }
                    }
                    _ => todo!(), // Debug to be finished
                }
                self.sim_controller_send
                    .send(DroneEvent::ControllerShortcut(packet.clone()));
            }

            //Error in send a MsgFragment, send back a NACK
            _ => {
                let nack_packet = self.packet_nack(
                    packet,
                    Nack {
                        fragment_index: packet.get_fragment_index(), // oppure il valore appropriato
                        nack_type,
                    },
                );
                self.forward_packet_safe(&nack_packet);
            }
        }
    }

    fn forward_packet_safe(&self, packet: &Packet) {
        if let Err(nack) = self.forward_packet(packet.clone()) {
            if self.clone().in_drone_debug_options(DroneDebugOption::Ack) {
                println!(
                    "[Drone {} - Ack Debug] Error {nack} in sending Ack!",
                    self.id
                );
            }
            self.handle_forwarding_error(packet, nack.nack_type);
        }
    }

    fn forward_packet_flood_request(&self, packet: Packet, neighbour: (NodeId, Sender<Packet>)) {
        match neighbour.1.send(packet.clone()) {
            Ok(()) => {
                self.sim_controller_send
                    .send(DroneEvent::PacketSent(packet.clone()));
            }
            Err(..) => panic!("Error sending flood request"),
        }
    }

    fn add_neighbor(&mut self, node_id: NodeId, sender: Sender<Packet>) -> Result<(), String> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.packet_send.entry(node_id) {
            e.insert(sender);
            Ok(())
        } else {
            Err(format!(
                "Sender for node {node_id} already stored in the map!",
            ))
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

    pub fn in_drone_debug_options(self, drone_debug_option: DroneDebugOption) -> bool {
        *self.drone_debug_options.get(&drone_debug_option).unwrap()
    }

    pub fn set_debug_option_active(&mut self, debug_option: &DroneDebugOption) {
        if let Some(option) = self.drone_debug_options.get_mut(&debug_option) {
            *option = true;
            println!("[Debug: {:?} Enabled]", debug_option);
        } else {
            eprintln!("Error: Debug option {debug_option:?} doesn't exist.");
        }
    }

    pub fn set_debug_option_disable(&mut self, debug_option: &DroneDebugOption) {
        if let Some(option) = self.drone_debug_options.get_mut(&debug_option) {
            *option = false;
        } else {
            eprintln!("Error: Debug option {debug_option:?} doesn't exist.",);
        }
    }

    fn packet_nack(&self, packet: &Packet, nack: Nack) -> Packet {
        // path to the source node
        let rev_path = packet
            .routing_header
            .hops
            .split_at(packet.routing_header.hop_index + 1)
            .0
            .iter()
            .rev()
            .copied()
            .collect();

        // Nack packet
        Packet {
            pack_type: PacketType::Nack(nack),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: rev_path,
            },
            session_id: packet.session_id,
        }
    }

    fn remove_neighbor(&mut self, node_id: &NodeId) -> Result<(), String> {
        if self.packet_send.contains_key(node_id) {
            self.packet_send.remove(node_id);
            Ok(())
        } else {
            Err(format!("Sender for node {node_id} not stored in the map!"))
        }
    }
}
