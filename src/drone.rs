use crossbeam_channel::{select, Receiver, Sender};
use rand::Rng;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodResponse, Nack, NackType, NodeType, Packet, PacketType};

#[derive(Clone, Debug, PartialEq)]
pub enum DroneState {
    Active,
    Crashing,
    Crashed,
}

#[derive(Debug, Clone)]
pub struct MyDrone {
    pub id: NodeId,
    pub sim_controller_send: Sender<DroneEvent>, // Canale per inviare eventi dal SC
    pub sim_controller_recv: Receiver<DroneCommand>, // Canale per ricevere comandi dal SC
    pub packet_recv: Receiver<Packet>,           // Canale per ricevere pacchetti
    pub packet_send: HashMap<NodeId, Sender<Packet>>, // Mappa dei canali per inviare pacchetti ai neighbours nodes
    pub pdr: f32,                                     // PDR
    pub state: DroneState,                                // Stato del drone
    pub flood_id_vec: HashSet<(u64, u64)>, // HashSet delle flood request gia ricevute

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

        format!("Drone {id} creato con PDR: {pdr}");

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
        loop {
            match self.state {
                DroneState::Active => {
                    select! {
                        recv(self.packet_recv) -> packet_res => {
                            if let Ok(packet) = packet_res {
                                self.handle_packet(packet);
                            }
                        },
                        recv(self.sim_controller_recv) -> command_res => {
                            if let Ok(command) = command_res {
                                self.handle_command(command);
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


impl MyDrone {
    fn handle_packet(&mut self, mut packet: Packet) {
        match packet.pack_type {
            PacketType::FloodRequest(ref mut flood_request) => {
                if self.flood_id_vec.insert((flood_request.flood_id, packet.session_id)) {
                    let previous_id = flood_request.path_trace.last().unwrap().clone();
                    flood_request.path_trace.push((self.id, NodeType::Drone));
                    if !self.packet_send.iter().any(|(id, _)| *id != previous_id.0){
                        let flood_response = Packet::new_flood_response(
                            SourceRoutingHeader {
                                hop_index: 0,
                                hops: flood_request.path_trace.iter().map(|(id, _)| *id).rev().collect(),
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
                                self.forward_packet_flood_request(packet.clone(), neighbour.clone());
                                println!(
                                    "Drone {} correctly sent the Flood Request to neighbor with {} id",
                                    self.id, neighbour.0
                                );
                            }
                        }
                    }
                } else {
                    flood_request.path_trace.push((self.id, NodeType::Drone));
                    let flood_response = Packet::new_flood_response(
                        SourceRoutingHeader {
                            hop_index: 0,
                            hops: flood_request.path_trace.iter().map(|(id, _)| *id).rev().collect(),
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
            },
            _ => {
                if let Some(node_id) = packet.routing_header.hops.get(packet.routing_header.hop_index) {
                    if *node_id == self.id {
                        match packet.pack_type {
                            PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                                self.forward_packet_safe(&packet);
                            },
                            PacketType::MsgFragment(ref fragment) => {
                                if self.state == DroneState::Crashed {
                                    self.handle_forwarding_error(&packet, NackType::ErrorInRouting(*packet.routing_header.hops.get(packet.routing_header.hop_index+1).unwrap()));
                                } else if self.should_drop_packet() {
                                    self.handle_forwarding_error(&packet, NackType::Dropped);
                                } else if let Err(nack) = self.forward_packet(packet.clone()) {
                                    self.handle_forwarding_error(&packet, nack.nack_type);
                                }
                            },
                            _ => (),
                        }
                    }
                    else {
                        self.forward_packet_safe(&packet);
                    }
                }
                DroneState::Crashed => {
                    println!("Drone {} is in Crashed state. Exiting loop", self.id);
                    break;
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
  
    // Metodo per inviare pacchetti al prossimo nodo presente nell'hops
    pub fn forward_packet(&self, mut packet: Packet) -> Result<(), Nack> {
        packet.routing_header.hop_index += 1;

        let next_hop = packet
            .routing_header
            .hops
            .get(packet.routing_header.hop_index);

        println!("Sender di Drone {} -> {:?}", self.id, self.packet_send);

        let fragment_index = match packet.pack_type.clone() {
            PacketType::MsgFragment(fragment) => fragment.fragment_index,
            // 0 perchè solo i MsgFragment possono essere frammentati
            _ => 0,
        };

        match next_hop {
            Some(next_node) => {
                match self.packet_send.get(next_node) {
                    Some(next_node_channel) => {
                        next_node_channel.send(packet);
                        Ok(())
                    }
                    // None se il next node non è neighbour del drone
                    None => Err(Nack {
                        fragment_index,
                        nack_type: NackType::ErrorInRouting(*next_node),
                    }),
                }
            }
            // Se next_hop ritorna None significa che il drone è la destinazione finale
            None => Err(Nack {
                fragment_index,
                nack_type: NackType::DestinationIsDrone,
            }),
        }
    }

    pub fn forward_packet_flood_request(&self, packet: Packet, neighbour: (NodeId, Sender<Packet>) ){
        println!("Sender di Drone {} -> {:?}", self.id, neighbour.0);
        neighbour.1.send(packet).expect("C'è un problema");
    }

    pub fn set_pdr(&mut self, pdr: f32) -> Result<(), String> {
        if pdr > 0.0 && pdr < 1.0 {
            println!("Drone {}: modificato PDR, {} -> {}", self.id, self.pdr, pdr);
            self.pdr = pdr;
            return Ok(());
        }

        Err("Incorrect value of PDR".to_string())
    }

    pub fn add_neighbor(&mut self, node_id: NodeId, sender: Sender<Packet>) -> Result<(), String> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.packet_send.entry(node_id) {
            e.insert(sender);
            Ok(())
        } else {
            Err(format!(
                "Sender per il nodo {node_id} è già presente nella mappa!",
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

    fn packet_nack(&self, packet: &Packet, nack: Nack) -> Packet {
        // path fino al source node
        let rev_path = packet
            .routing_header
            .hops
            .split_at(packet.routing_header.hop_index)
            .0
            .iter()
            .rev()
            .copied()
            .collect();

        // Packet con Nack
        Packet {
            pack_type: PacketType::Nack(nack),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: rev_path,
            },
            session_id: packet.session_id,
        }
    }

    pub fn remove_neighbor(&mut self, node_id: &NodeId) -> Result<(), String> {
        if self.packet_send.contains_key(node_id) {
            self.packet_send.remove(node_id);
            Ok(())
        } else {
            Err(format!(
                "Sender per il nodo {node_id} non è presente nella mappa!"
            ))
        }
    }

    fn handle_forwarding_error(&self, packet: &Packet, nack_type: NackType) {
        // Se il pacchetto è un nack/send/floodResponse viene mandato al sim controller, altrimenti viene creato il nack_packet e viene mandato
        match packet.pack_type {
            PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                self.sim_controller_send
                    .send(DroneEvent::PacketDropped(packet.clone()))
                    .expect("TODO: panic message");
            }
            _ => {
                let nack_packet = self.packet_nack(
                    packet,
                    Nack {
                        fragment_index: 0, // oppure il valore appropriato
                        nack_type,
                    },
                );
                self.forward_packet_safe(&nack_packet);
            }
        }
    }

    pub fn forward_packet_safe(&self, packet: &Packet) {
        if let Err(nack) = self.forward_packet(packet.clone()) {
            self.handle_forwarding_error(packet, nack.nack_type);
        }
    }
}
