use crossbeam_channel::{select, Receiver, Sender};
use rand::Rng;
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Nack, NackType, Packet, PacketType};

#[derive(Debug, Clone)]
pub struct MyDrone {
    pub id: NodeId,
    pub sim_controller_send: Sender<DroneEvent>, // Canale per inviare eventi dal SC
    pub sim_controller_recv: Receiver<DroneCommand>, // Canale per ricevere comandi dal SC
    pub packet_recv: Receiver<Packet>,          // Canale per ricevere pacchetti
    pub packet_send: HashMap<NodeId, Sender<Packet>>, // Mappa dei canali per inviare pacchetti ai neighbours nodes
    pub pdr: f32,                                     // PDR
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
        }
    }

    fn run(&mut self) {
        println!("Drone {} in esecuzione...", self.id);
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
                                            },
                                            Err(nack) => {
                                                // Nack: ErrorInRouting || DestinationIsDrone
                                                match self.forward_packet(self.packet_nack(packet.clone(), nack)) {
                                                    Ok(()) => {
                                                        // Nack packet inviato correttamente al prossimo nodo
                                                    },
                                                    Err(err) => {
                                                        panic!("{err:?}");
                                                    },
                                                }
                                            }
                                        }
                                    }, // bisogna inoltare il pacchetto
                                    PacketType::MsgFragment(ref fragment) => {
                                        // Verifica se il pacchetto deve essere droppato per il DPR
                                        if self.drop_packet() {
                                            // Nack: Dropped
                                            match self.forward_packet(self.packet_nack(packet.clone(), Nack {fragment_index: fragment.fragment_index, nack_type: NackType::Dropped})) {
                                                Ok(()) => {
                                                    // Nack packet inviato correttamente al prossimo nodo
                                                    println!("Nack packet inviato correttamente al prossimo nodo");
                                                },
                                                Err(err) => {
                                                    panic!("{err:?}");
                                                },
                                            }
                                        } else {
                                            match self.forward_packet(packet.clone()) {
                                                Ok(()) => {
                                                    // frammento inoltrato correttamente
                                                    println!("frammento inoltrato correttamente");
                                                },
                                                Err(nack) => {
                                                    // Nack: ErrorInRouting || DestinationIsDrone
                                                    match self.forward_packet(self.packet_nack(packet.clone(), nack)) {
                                                        Ok(()) => {
                                                            // Nack packet inviato correttamente al prossimo nodo
                                                        },
                                                        Err(err) => {
                                                            panic!("{err:?}");
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    PacketType::FloodRequest(floodRequest) => unimplemented!(),
                                    PacketType::FloodResponse(floodResponse) => unimplemented!(),
                                }
                            } else {
                                // Nack: UnexpectedRecipient
                                match self.forward_packet(self.packet_nack(packet.clone(), Nack {fragment_index: 0, nack_type: NackType::UnexpectedRecipient(self.id)})) {
                                    Ok(()) => {
                                        // Nack packet inviato correttamente al prossimo nodo
                                    },
                                    Err(err) => {
                                        panic!("{err:?}");
                                    },
                                }
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
                                // Da terminare
                                println!("Drone {} terminato", self.id);
                                break;
                            },
                            DroneCommand::AddSender(node_id, sender) => {
                                self.add_sender(node_id, sender).expect("Sender already present!");
                            },
                            _ => {

                            }
                        }
                    }
                }
            }
        }
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

    fn add_sender(&mut self, node_id: NodeId, sender: Sender<Packet>) -> Result<(), String> {
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
}
