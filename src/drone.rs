use crossbeam_channel::{select, Receiver, Sender};
use rand::Rng;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodResponse, Nack, NackType, NodeType, Packet, PacketType};

#[derive(Clone, Debug)]
enum DroneState {
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
    pub state: DroneState,                            // Stato del drone
    pub flood_id_vec: HashSet<u64>,                   // HashSet degli id delle FloodRequest ricevute
}

impl PartialEq for DroneState {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
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

        println!("Drone {} creato con PDR: {}", id, pdr);

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
            select! {
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        if let Some(node_id) = packet.routing_header.hops.get(packet.routing_header.hop_index) {
                            if *node_id == self.id {
                                match packet.pack_type {
                                    PacketType::Ack(_) | PacketType::Nack(_) => {
                                        match self.forward_packet(packet.clone()) {
                                            Ok(()) => {
                                                // Ack || Nack inoltrato correttamente
                                                println!("Drone {} forwarded packet", self.id);
                                            },
                                            Err(_) => {
                                                // Nack: ErrorInRouting || DestinationIsDrone
                                                // Segnalato al SC che un pachetto ACK/NACK è stato droppato
                                                println!("Drone {} send ack | nack", self.id);
                                                self.sim_controller_send.send(DroneEvent::PacketDropped(packet.clone()));
                                            }
                                        }
                                    },
                                    PacketType::MsgFragment(ref fragment) => {
                                        // Se il Drone è in Crashing Behaviour allora non gestisce il MsgFragment
                                        if self.state == DroneState::Crashed {
                                            let nack_packet = self.packet_nack(packet.clone(), Nack {fragment_index: fragment.fragment_index, nack_type: NackType::ErrorInRouting(*packet.routing_header.hops.get(packet.routing_header.hop_index+1).unwrap())});
                                            if let Err(err) = self.forward_packet(nack_packet) {
                                                panic!("{err:?}");
                                            }
                                        } else if self.drop_packet() {
                                            let nack_packet = self.packet_nack(packet.clone(), Nack {
                                                fragment_index: fragment.fragment_index,
                                                nack_type: NackType::Dropped
                                            });
                                            if let Err(err) = self.forward_packet(nack_packet) {
                                                panic!("{err:?}");
                                            }
                                        } else if let Err(nack) = self.forward_packet(packet.clone()) {
                                            let nack_packet = self.packet_nack(packet.clone(), nack);
                                            if let Err(err) = self.forward_packet(nack_packet) {
                                                panic!("{err:?}");
                                            }
                                        }
                                    },
                                    PacketType::FloodRequest(mut flood_request) => {
                                        if self.flood_id_vec.insert(flood_request.flood_id){
                                            //Il flood_id non era presente, il che significa che la floodRequest passa per la prima volta in questo drone
                                            let Some((previous_id, _)) = flood_request.path_trace.get(flood_request.path_trace.len()-1);
                                            flood_request.path_trace.push((self.id, NodeType::Drone));
                                            if self.packet_send.capacity() <= 1{
                                                //Il drone non ha altri vicini oltre al mandante, procedo a inviare indietro una floodResponse
                                                let flood_response = Packet::new_flood_response(SourceRoutingHeader{hop_index: 1, hops: flood_request.path_trace.clone().rev()}, packet.session_id, FloodResponse {flood_id: flood_request.flood_id, path_trace: flood_request.path_trace});
                                                match self.forward_packet(flood_response.clone()) {
                                                    Ok(()) => {
                                                        // FloodResponse inoltrata correttamente
                                                    },
                                                    Err(_) => {
                                                        // Nack: ErrorInRouting || DestinationIsDrone
                                                        // Segnalato al SC che un pacchetto ACK/NACK è stato droppato
                                                        self.sim_controller_send.send(DroneEvent::PacketDropped(flood_response.clone()));
                                                    }
                                                }
                                            }
                                            for neighbour in self.packet_send{
                                                if &neighbour.0 != previous_id{
                                                    match self.forward_packet(packet.clone()) {
                                                        Ok(()) => {
                                                            // FloodRequest inoltrata correttamente
                                                        },
                                                        Err(_) => {

                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        else{
                                            //Il flood_id era gia presente, significa che era gia passato per di qua, procedo a inviare indietro una floodResponse
                                            flood_request.path_trace.push((self.id, NodeType::Drone));

                                            let flood_response = Packet::new_flood_response(SourceRoutingHeader{hop_index: 1, hops: flood_request.path_trace.clone().rev()}, packet.session_id, FloodResponse {flood_id: flood_request.flood_id, path_trace: flood_request.path_trace});
                                            match self.forward_packet(flood_response.clone()) {
                                                Ok(()) => {
                                                    // FloodResponse inoltrata correttamente
                                                },
                                                Err(_) => {
                                                    // Nack: ErrorInRouting || DestinationIsDrone
                                                    // Segnalato al SC che un pacchetto ACK/NACK è stato droppato
                                                    self.sim_controller_send.send(DroneEvent::PacketDropped(flood_response.clone()));
                                                }
                                            }
                                        }
                                    },
                                    PacketType::FloodResponse(_) => {
                                        //Ricevuta una FloodResponse, mandiamola indietro al nodo precedente
                                        match self.forward_packet(packet.clone()) {
                                            Ok(()) => {
                                                // FloodResponse inoltrata correttamente
                                            },
                                            Err(_) => {
                                                // Nack: ErrorInRouting || DestinationIsDrone
                                                // Segnalato al SC che un pacchetto ACK/NACK è stato droppato
                                                self.sim_controller_send.send(DroneEvent::PacketDropped(packet.clone()));
                                            }
                                        }
                                    },
                                }
                            }
                            else {
                            // Nack: UnexpectedRecipient
                                match self.forward_packet(self.packet_nack(packet.clone(), Nack {fragment_index: 0, nack_type: NackType::UnexpectedRecipient(self.id)})) {
                                    Ok(()) => {
                                        // Nack packet inviato correttamente al prossimo nodo
                                    },
                                    Err(err) => {
                                        panic!("{err:?}");
                                    }
                                }
                            }
                        }
                        else {
                            if(self.state == DroneState::Crashing) {
                                self.state = DroneState::Crashed;
                                break; // Forse bisogna farlo in un altro modo controllare!
                            }
                        }
                    }
                },
                recv(self.sim_controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        match command {
                            DroneCommand::SetPacketDropRate(pdr) => {
                                self.set_pdr(pdr).expect("Error in PDR setting");
                            },
                            DroneCommand::Crash => {
                                self.set_drone_state(DroneState::Crashing);
                                // Setta lo stato in crashing limitando le funzionalità del drone
                            },
                            DroneCommand::AddSender(node_id, sender) => {
                                self.add_sender(node_id, sender).expect("Sender already present!");
                            },
                            DroneCommand::RemoveSender(node_id) => {
                                self.remove_sender(&node_id).expect("Sender is not in self.sender");
                            }
                        }
                    }
                }
            }
        }
        println!("Drone {} in esecuzione...", self.id);
        println!("Drone {}: Uscito dal loop", self.id);
    }
}

impl MyDrone {
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

    pub fn set_pdr(&mut self, pdr: f32) -> Result<(), String> {
        if pdr > 0.0 && pdr < 1.0 {
            println!("Drone {}: modificato PDR, {} -> {}", self.id, self.pdr, pdr);
            self.pdr = pdr;
            return Ok(());
        }

        Err("Incorrect value of PDR".to_string())
    }

    pub fn add_sender(&mut self, node_id: NodeId, sender: Sender<Packet>) -> Result<(), String> {
        if self.packet_send.contains_key(&node_id) {
            Err(format!(
                "Sender per il nodo {} è già presente nella mappa!",
                node_id
            ))
        } else {
            self.packet_send.insert(node_id, sender);
            Ok(())
        }
    }

    fn set_drone_state(&mut self, state: DroneState) {
        self.state = state;
    }

    fn drop_packet(&self) -> bool {
        let mut rng = rand::rng();
        let n: f32 = rng.random_range(0.0..=1.0);
        if n < self.pdr {
            return true;
        }
        false
    }

    fn packet_nack(&self, packet: Packet, nack: Nack) -> Packet {
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

    pub fn remove_sender(&mut self, node_id: &NodeId) -> Result<(), String> {
        if self.packet_send.contains_key(&node_id) {
            Err(format!(
                "Sender per il nodo {} è già presente nella mappa!",
                node_id
            ))
        } else {
            self.packet_send.remove(&node_id);
            Ok(())
        }
    }

    // Inutile è da eliminare
    pub fn remove_all_sender(&mut self) {
        for (node, sender) in self.clone().packet_send.iter() {
            self.remove_sender(node);
        }
        println!("Rimossi tutti i sendere del drone {:?}", self.id);
    }
}
