/*use rand::Rng;

mod packet_drone {
    use wg_2024::controller::DroneEvent;
    use wg_2024::network::SourceRoutingHeader;
    use wg_2024::packet::{FloodResponse, NackType, NodeType, Packet, PacketType};
    use crate::{DroneDebugOption, DroneState, MyDrone};

    impl MyDrone {
        fn handle_packet(&mut self, mut packet: Packet) {
            match packet.pack_type {
                PacketType::FloodRequest(ref mut flood_request) => {
                    if self
                        .flood_id_vec
                        .insert((flood_request.flood_id, packet.session_id))
                    {
                        let previous_id = flood_request.path_trace.last().unwrap().clone();
                        flood_request.path_trace.push((self.clone().get_id(), NodeType::Drone));
                        if !self.clone().get_packet_send().iter().any(|(id, _)| *id != previous_id.0) {
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
                            println!("Drone {} correctly sent back a Flood Response because has no neighbour",self.clone().get_id());
                        } else {
                            for neighbour in self.clone().get_packet_send().clone() {
                                if neighbour.0 != previous_id.0 {
                                    self.forward_packet_flood_request(
                                        packet.clone(),
                                        neighbour.clone(),
                                    );
                                    println!(
                                        "Drone {} correctly sent the Flood Request to neighbor with {} id",
                                        self.clone().get_id(), neighbour.0
                                    );
                                }
                            }
                        }
                    } else {
                        //The packet has already seen by this drone, proceed to send back a flood response
                        flood_request.path_trace.push((self.clone().get_id(), NodeType::Drone));
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
                        println!("Drone {} correctly sent back a Flood Response because has already received this flood request",self.clone().get_id());
                    }
                },
                _ => {
                    if let Some(node_id) = packet
                        .routing_header
                        .hops
                        .get(packet.routing_header.hop_index){
                        if *node_id == self.clone().get_id() {
                            match packet.pack_type {
                                PacketType::Ack(_)
                                | PacketType::Nack(_)
                                | PacketType::FloodResponse(_) => {
                                    if self.clone().in_drone_debug_options(DroneDebugOption::Ack) {
                                        println!("[Drone {}] attempts to send Ack", self.clone().get_id())
                                    }
                                    self.forward_packet_safe(&packet);
                                }
                                PacketType::MsgFragment(ref fragment) => {
                                    //NON SONO SICURO DEL CONTROLLO SULLO STATO DEL DRONE
                                    if self.clone().get_state() == DroneState::Crashing {
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
                                        self.clone().get_sim_controller_send()
                                            .send(DroneEvent::PacketDropped(packet.clone()));
                                    } else {
                                        self.forward_packet_safe(&packet);
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                    else {
                        self.handle_forwarding_error(&packet, NackType::UnexpectedRecipient(self.id));
                    }

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
                        Some(next_node_channel) => {
                            if self.clone().in_drone_debug_options(DroneDebugOption::Ack) {
                                println!(
                                    "[Drone {} - Ack Debug] Ack sent correctly to Node {}",
                                    self.id, next_node
                                );
                            }
                            match next_node_channel.send(packet.clone()) {
                                Ok(()) => {
                                    self.sim_controller_send
                                        .send(DroneEvent::PacketSent(packet.clone()));
                                    Ok(())
                                }
                                Err(..) => {
                                    panic!("Errore nell'invio del pacchetto al nodo successivo")
                                }
                            }
                        }
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
                    if self.clone().in_drone_debug_options(DroneDebugOption::Ack) {
                        println!(
                            "[Drone {} - Ack Debug] Ack Packet Dropped sent to Simulation Controller",
                            self.id
                        );
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

        fn should_drop_packet(&self) -> bool {
            let mut rng = rand::rng();
            let n: f32 = rng.random_range(0.0..=1.0);
            if n < self.pdr {
                return true;
            }
            false
        }

        fn packet_nack(&self, packet: &Packet, nack: Nack) -> Packet {
            // path to the source node
            let rev_path = packet
                .routing_header
                .hops
                .split_at(packet.routing_header.hop_index)
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
    }
}*/