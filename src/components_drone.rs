mod components_drone {
    use crossbeam_channel::{select, select_biased, Receiver, Sender};
    use rand::Rng;
    use std::cmp::PartialEq;
    use std::collections::{HashMap, HashSet};
    use wg_2024::controller::{DroneCommand, DroneEvent};
    use wg_2024::drone::Drone;
    use wg_2024::network::{NodeId, SourceRoutingHeader};
    use wg_2024::packet;
    use wg_2024::packet::{FloodRequest, FloodResponse, Nack, NackType, NodeType, Packet, PacketType};

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
        sim_controller_send: Sender<DroneEvent>,                // Channel used to send commands to the SC
        sim_controller_recv: Receiver<DroneCommand>,            // Channel used to receive commands from the SC
        packet_recv: Receiver<Packet>,                          // Channel used to receive packets from nodes
        packet_send: HashMap<NodeId, Sender<Packet>>,           // Map containing the sending channels of neighbour nodes
        pdr: f32,                                               // PDR
        state: DroneState,                                      // Drone state
        flood_id_vec: HashSet<(u64, u64)>,                      // HashSet storing ids of already received flood_id
        drone_debug_options: HashMap<DroneDebugOption, bool>,   // Map used to know which Debug options are active and which aren't
    }
}