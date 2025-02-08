use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{debug, error, info, warn};
use rand::Rng;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodResponse, Nack, NackType, NodeType, Packet, PacketType};

mod components_drone; // This line is fine as is.

/// Represents the possible states of a drone.
#[derive(Clone, Debug, PartialEq)]
pub enum DroneState {
    /// The drone is operating normally.
    Active,
    /// The drone is in the process of crashing (still processing packets).
    Crashing,
    /// The drone has crashed and is no longer functioning.
    Crashed,
}

/// `Dronegowski` represents a drone within the simulation.
/// It handles packet forwarding, crash events, and communication with the simulator controller.
#[derive(Debug, Clone)]
pub struct Dronegowski {
    /// The unique identifier of the drone.
    id: NodeId,
    /// Channel for sending events to the simulation controller.
    sim_controller_send: Sender<DroneEvent>,
    /// Channel for receiving commands from the simulation controller.
    sim_controller_recv: Receiver<DroneCommand>,
    /// Channel for receiving packets from other nodes.
    packet_recv: Receiver<Packet>,
    /// Map of neighboring node IDs to channels for sending packets to them.
    packet_send: HashMap<NodeId, Sender<Packet>>,
    /// Packet Drop Rate (PDR) - the probability of a packet being dropped.
    pdr: f32,
    /// The current state of the drone.
    state: DroneState,
    /// Set of (flood_id, session_id) tuples to track already-seen FloodRequests and prevent loops.
    flood_id_vec: HashSet<(u64, u64)>,
}

impl Drone for Dronegowski {
    /// Creates a new `Dronegowski` instance.
    ///
    /// # Arguments
    ///
    /// * `id`: The unique identifier of the drone.
    /// * `controller_send`: Sender for communicating with the simulation controller.
    /// * `controller_recv`: Receiver for commands from the simulation controller.
    /// * `packet_recv`: Receiver for packets from the network.
    /// * `packet_send`: Map of neighbor NodeIds to packet senders.
    /// * `pdr`: The packet drop rate (0.0 to 1.0).
    ///
    /// # Panics
    ///
    /// Panics if the `packet_send` map contains the drone's own ID or if the PDR is out of bounds.
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
        info!("Created drone {id} with PDR: {pdr}"); // Log drone creation

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

    /// Runs the main loop of the drone.
    ///
    /// The drone listens for commands from the simulation controller and packets from other nodes.
    /// The behavior depends on the drone's state (`Active`, `Crashing`, `Crashed`).
    fn run(&mut self) {
        info!(
            "Drone {} entering the run loop in state {:?}",
            self.id, self.state
        );
        loop {
            match self.state {
                DroneState::Active => {
                    // Use `select_biased` to prioritize handling commands over packets.
                    select_biased! {
                        recv(self.sim_controller_recv) -> command_res => {
                            match command_res {
                                Ok(command) => {
                                    self.handle_command(command);
                                },
                                Err(e) => {
                                    error!("Drone {}: Error receiving command: {:?}", self.id, e);
                                }
                            }
                        },
                        recv(self.packet_recv) -> packet_res => {
                            match packet_res {
                                Ok(packet) => {
                                    self.handle_packet(packet);
                                },
                                Err(e) => {
                                    error!("Drone {}: Error receiving packet: {:?}", self.id, e);
                                }
                            }
                        }
                    }
                }
                DroneState::Crashing => {
                    // In the Crashing state, continue processing packets until the packet_recv channel is empty.
                    match self.packet_recv.recv() {
                        Ok(packet) => {
                            info!("Drone {} processing packet in Crashing state", self.id);
                            self.handle_packet(packet);
                        }
                        Err(_) => {
                            info!(
                                "Drone {} has completed crashing. Transitioning to Crashed state.",
                                self.id
                            );
                            // Transition to Crashed state when the channel is empty.
                            self.set_drone_state(DroneState::Crashed); // Use the setter
                        }
                    }
                }
                DroneState::Crashed => {
                    // In the Crashed state, exit the loop.
                    info!("Drone {} is in Crashed state. Exiting loop", self.id);
                    break;
                }
            }
        }
        info!("Drone {} has terminated execution", self.id); // Log drone termination
    }
}

impl Dronegowski {
    /// Returns the drone's PDR.
    pub fn get_pdr(self) -> f32 {
        self.pdr
    }

    /// Returns the receiver for commands from the simulation controller.
    #[must_use]
    pub fn get_sim_controller_recv(self) -> Receiver<DroneCommand> {
        self.sim_controller_recv
    }

    /// Returns the sender for events to the simulation controller.
    #[must_use]
    pub fn get_sim_controller_send(self) -> Sender<DroneEvent> {
        self.sim_controller_send
    }

    /// Returns the receiver for packets from other nodes.
    #[must_use]
    pub fn get_packet_recv(self) -> Receiver<Packet> {
        self.packet_recv
    }

    /// Returns the map of neighboring node IDs to packet senders.
    #[must_use]
    pub fn get_packet_send(self) -> HashMap<NodeId, Sender<Packet>> {
        self.packet_send
    }

    /// Sets the drone's PDR.
    ///
    /// # Arguments
    ///
    /// * `pdr`: The new packet drop rate (0.0 to 1.0).
    ///
    /// # Panics
    ///
    /// Panics if the PDR is out of bounds.
    pub fn set_pdr(&mut self, pdr: f32) {
        if pdr < 0.0 {
            panic!("pdr {} is out of bounds because is negative", pdr);
        } else if pdr > 1.0 {
            panic!("pdr {} is out of bounds because is too big", pdr);
        } else {
            self.pdr = pdr;
            info!("Drone {}: PDR updated to {}", self.id, pdr);
        }
    }

    /// Returns the drone's ID.
    pub fn get_id(self) -> NodeId {
        self.id
    }

    /// Returns the drone's current state.
    pub fn get_state(self) -> DroneState {
        self.state
    }

    /// Handles an incoming packet.
    ///
    /// This method determines the packet type and takes appropriate action:
    /// - Forwards FloodRequests, creating and sending FloodResponses if necessary.
    /// - Forwards other packets to the next hop in their routing header.
    /// - Sends NACKs for errors.
    /// - Drops packets based on the PDR.
    fn handle_packet(&mut self, mut packet: Packet) {
        info!("Drone {}: Handling packet: {:?}", self.id, packet); // Log packet handling

        match packet.pack_type {
            PacketType::FloodRequest(ref mut flood_request) => {
                // Check if this FloodRequest has already been seen.
                if self
                    .flood_id_vec
                    .insert((flood_request.flood_id, packet.session_id))
                {
                    // This is a new FloodRequest.
                    let previous_id = flood_request.path_trace.last().unwrap().0; // Extract NodeId
                    info!(
                        "Drone {}: Received new FloodRequest. Previous hop: {}",
                        self.id, previous_id
                    );
                    // Update the path trace with the current drone.
                    flood_request.path_trace.push((self.id, NodeType::Drone));

                    // Check if there are any neighbors other than the one that sent the request.
                    if !self.packet_send.iter().any(|(id, _)| *id != previous_id) {
                        // No other neighbors, send a FloodResponse back.
                        info!(
                            "Drone {}: No other neighbors. Sending FloodResponse.",
                            self.id
                        );
                        let flood_response = Packet::new_flood_response(
                            SourceRoutingHeader {
                                hop_index: 0,
                                // Reverse the path trace for the response.
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
                        info!("Drone {} correctly sent back a Flood Response because has no neighbour", self.id);
                    } else {
                        // Forward the FloodRequest to all neighbors except the one that sent it.
                        info!(
                            "Drone {}: Forwarding FloodRequest to other neighbors.",
                            self.id
                        );
                        for (neighbor_id, neighbor_sender) in &self.packet_send {
                            // Iterate over references
                            if *neighbor_id != previous_id {
                                let neighbor = (*neighbor_id, neighbor_sender.clone()); // Create a tuple
                                self.forward_packet_flood_request(packet.clone(), neighbor);
                                info!(
                                    "Drone {} correctly sent the Flood Request to neighbor with {} id",
                                    self.id, neighbor_id
                                );
                            }
                        }
                    }
                } else {
                    // This FloodRequest has already been seen.  Send a FloodResponse back.
                    info!(
                        "Drone {}: FloodRequest already seen. Sending FloodResponse.",
                        self.id
                    );
                    // The following lines were missing from the original, *incorrectly* commented out, and *critical* for correct behavior.
                    flood_request.path_trace.push((self.id, NodeType::Drone)); // Add current drone
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
                    self.forward_packet_safe(&flood_response); // Use safe forwarding

                    info!("Drone {} correctly sent back a Flood Response because has already received this flood request", self.id);
                }
            }
            _ => {
                // Handle other packet types.
                // Check if the current drone is the intended recipient.
                if let Some(node_id) = packet
                    .routing_header
                    .hops
                    .get(packet.routing_header.hop_index)
                {
                    if *node_id == self.id {
                        // The current drone is the intended recipient.
                        match packet.pack_type {
                            PacketType::Ack(_)
                            | PacketType::Nack(_)
                            | PacketType::FloodResponse(_) => {
                                // Forward control packets directly to the simulation controller.
                                info!(
                                    "Drone {}: Forwarding control packet to simulation controller.",
                                    self.id
                                );
                                self.forward_packet_safe(&packet);
                            }
                            PacketType::MsgFragment(ref _fragment) => {
                                // Handle MsgFragment packets.
                                info!("Drone {}: Received MsgFragment.", self.id);
                                if self.state == DroneState::Crashing {
                                    // If crashing, send an ErrorInRouting NACK.
                                    info!(
                                        "Drone {}: Crashing. Sending ErrorInRouting NACK.",
                                        self.id
                                    );
                                    self.handle_forwarding_error(
                                        &packet,
                                        NackType::ErrorInRouting(
                                            *packet
                                                .routing_header
                                                .hops
                                                .get(packet.routing_header.hop_index + 1)
                                                .expect("Next hop should exist for ErrorInRouting"),
                                        ),
                                    );
                                } else if self.should_drop_packet() {
                                    // Drop the packet based on PDR.
                                    info!("Drone {}: Dropping packet due to PDR.", self.id);
                                    self.handle_forwarding_error(&packet, NackType::Dropped);
                                    self.sim_controller_send
                                        .send(DroneEvent::PacketDropped(packet.clone()))
                                        .expect("Failed to send PacketDropped event");
                                } else {
                                    // Forward the packet.
                                    info!("Drone {}: Forwarding MsgFragment.", self.id);
                                    self.forward_packet_safe(&packet);
                                }
                            }
                            _ => {
                                // Handle other packet types (if any).  This is a good place for a `debug!` log.
                                debug!("Drone {}: Received unexpected packet type at destination: {:?}", self.id, packet.pack_type);
                            }
                        }
                    } else {
                        // The current drone is *not* the intended recipient.  Send an UnexpectedRecipient NACK.
                        info!("Drone {}: Not the intended recipient. Sending UnexpectedRecipient NACK.", self.id);
                        self.handle_forwarding_error(
                            &packet,
                            NackType::UnexpectedRecipient(self.id),
                        );
                    }
                } else {
                    // The packet has no next hop.  This likely indicates an error, but might have valid cases.
                    warn!("Drone {}: Packet has no next hop: {:?}", self.id, packet);
                }
            }
        }
    }

    /// Handles a command received from the simulation controller.
    fn handle_command(&mut self, command: DroneCommand) {
        info!("Drone {}: Handling command: {:?}", self.id, command); // Log command handling

        match command {
            DroneCommand::SetPacketDropRate(pdr) => {
                self.set_pdr(pdr);
            }
            DroneCommand::Crash => {
                info!("Drone {} entering Crashing state.", self.id);
                self.set_drone_state(DroneState::Crashing); // Use the setter
            }
            DroneCommand::AddSender(node_id, sender) => {
                info!("Drone {}: Adding sender for node {}.", self.id, node_id);
                self.add_neighbor(node_id, sender);
            }
            DroneCommand::RemoveSender(node_id) => {
                info!("Drone {} removing sender {}.", self.id, node_id);
                self.remove_neighbor(&node_id);
            }
        }
    }

    /// Attempts to forward a packet to the next hop in its routing header.
    ///
    /// # Arguments
    ///
    /// * `packet`: The packet to forward.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the packet was successfully sent.
    /// * `Err(Nack)` if the packet could not be forwarded, containing a `Nack` indicating the reason.
    fn forward_packet(&self, mut packet: Packet) -> Result<(), Nack> {
        packet.routing_header.hop_index += 1;
        info!(
            "Drone {}: Forwarding packet. New hop index: {}",
            self.id, packet.routing_header.hop_index
        );

        let next_hop = packet
            .routing_header
            .hops
            .get(packet.routing_header.hop_index);

        let fragment_index = match packet.pack_type.clone() {
            PacketType::MsgFragment(fragment) => fragment.fragment_index,
            _ => 0, // 0 because only MsgFragment can be serialized
        };
        debug!("Drone {}: Next hop: {:?}", self.id, next_hop);

        match next_hop {
            Some(next_node) => {
                match self.packet_send.get(next_node) {
                    Some(next_node_channel) => match next_node_channel.send(packet.clone()) {
                        Ok(()) => {
                            info!(
                                "Drone {}: Packet successfully forwarded to {}.",
                                self.id, next_node
                            );
                            // Notify the simulation controller that the packet was sent.
                            self.sim_controller_send
                                .send(DroneEvent::PacketSent(packet.clone()))
                                .expect("Failed to send packet to sim controller");
                            Ok(())
                        }
                        Err(e) => {
                            // Handle send errors.
                            error!(
                                "Drone {}: Error forwarding packet to {}: {:?}",
                                self.id, next_node, e
                            );
                            panic!("Error occurred while forwarding the packet");
                        }
                    },
                    // None if the next hop is not a drone's neighbour
                    None => {
                        warn!(
                            "Drone {}: Next hop {} is not a neighbor.",
                            self.id, next_node
                        );
                        Err(Nack {
                            fragment_index,
                            nack_type: NackType::ErrorInRouting(*next_node),
                        })
                    }
                }
            }
            // Next_hop returns None if the drone is the final destination
            None => {
                debug!("Drone {}: Destination reached (no next hop).", self.id);
                Err(Nack {
                    fragment_index,
                    nack_type: NackType::DestinationIsDrone,
                })
            }
        }
    }

    /// Handles a forwarding error by sending a NACK or forwarding the packet to the simulation controller.
    fn handle_forwarding_error(&self, packet: &Packet, nack_type: NackType) {
        info!(
            "Drone {}: Handling forwarding error: {:?}",
            self.id, nack_type
        );

        match packet.pack_type {
            PacketType::Ack(_) | PacketType::Nack(_) | PacketType::FloodResponse(_) => {
                // Forward control packets to the simulation controller.
                info!(
                    "Drone {}: Forwarding control packet to simulation controller due to error.",
                    self.id
                );
                self.sim_controller_send
                    .send(DroneEvent::ControllerShortcut(packet.clone()))
                    .expect("Failed to send to controller");
            }
            _ => {
                // Send a NACK packet back to the sender.
                info!("Drone {}: Sending NACK: {:?}", self.id, nack_type);
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

    /// Forwards a packet, handling any forwarding errors.
    fn forward_packet_safe(&self, packet: &Packet) {
        info!("Drone {}: Forwarding packet safely.", self.id);
        if let Err(nack) = self.forward_packet(packet.clone()) {
            self.handle_forwarding_error(packet, nack.nack_type);
        }
    }

    /// Forwards a FloodRequest packet, handling send errors.  This is separate from `forward_packet`
    /// because FloodRequests don't use the routing header in the same way.
    fn forward_packet_flood_request(&self, packet: Packet, neighbour: (NodeId, Sender<Packet>)) {
        info!(
            "Drone {}: Forwarding FloodRequest to neighbor {}.",
            self.id, neighbour.0
        );
        match neighbour.1.send(packet.clone()) {
            Ok(()) => {
                info!(
                    "Drone {}: FloodRequest successfully forwarded to {}.",
                    self.id, neighbour.0
                );
                self.sim_controller_send
                    .send(DroneEvent::PacketSent(packet.clone()))
                    .expect("Failed to send PacketSent event");
            }
            Err(e) => {
                error!(
                    "Drone {}: Error sending FloodRequest to {}: {:?}",
                    self.id, neighbour.0, e
                );
                panic!("Error sending flood request");
            }
        }
    }
    /// Adds a neighbor to the drone's `packet_send` map.
    fn add_neighbor(&mut self, node_id: NodeId, sender: Sender<Packet>) {
        info!("Drone {}: Adding neighbor {}.", self.id, node_id);
        if let std::collections::hash_map::Entry::Vacant(e) = self.packet_send.entry(node_id) {
            e.insert(sender);
            info!(
                "Drone {}: Neighbor {} added successfully.",
                self.id, node_id
            );
        } else {
            // This is a significant error, and the original code panics.  Keep the panic.
            panic!("Sender for node {node_id} already stored in the map!");
        }
    }

    /// Sets the drone's state.  This is a separate function for clarity and potential future expansion.
    fn set_drone_state(&mut self, state: DroneState) {
        info!("Drone {}: Setting state to {:?}", self.id, state);
        self.state = state;
    }

    /// Determines whether a packet should be dropped based on the drone's PDR.
    fn should_drop_packet(&self) -> bool {
        let mut rng = rand::thread_rng(); // Use thread_rng for better performance
        let n: f32 = rng.gen_range(0.0..=1.0); // Use gen_range for inclusive range
        if n < self.pdr {
            info!(
                "Drone {}: Packet will be dropped (random value: {} < PDR: {}).",
                self.id, n, self.pdr
            );
            true
        } else {
            debug!(
                "Drone {}: Packet will not be dropped (random value: {} >= PDR: {}).",
                self.id, n, self.pdr
            );
            false
        }
    }

    /// Creates a NACK packet.
    fn packet_nack(&self, packet: &Packet, nack: Nack) -> Packet {
        info!("Drone {}: Creating NACK packet: {:?}", self.id, nack);
        // Construct the reverse path for the NACK.
        let rev_path = packet
            .routing_header
            .hops
            .split_at(packet.routing_header.hop_index + 1)
            .0
            .iter()
            .rev()
            .copied()
            .collect();

        // Create the NACK packet.
        Packet {
            pack_type: PacketType::Nack(nack),
            routing_header: SourceRoutingHeader {
                hop_index: 0,
                hops: rev_path,
            },
            session_id: packet.session_id,
        }
    }

    /// Removes a neighbor from the drone's `packet_send` map.
    fn remove_neighbor(&mut self, node_id: &NodeId) {
        info!("Drone {}: Removing neighbor {}.", self.id, node_id);
        if self.packet_send.contains_key(node_id) {
            self.packet_send.remove(node_id);
            info!(
                "Drone {}: Neighbor {} removed successfully.",
                self.id, node_id
            );
        } else {
            // This is a significant error, as it indicates an inconsistency in the neighbor management.
            panic!("the {} is not neighbour of the drone {}", node_id, self.id);
        }
    }
}
