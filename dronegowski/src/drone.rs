use crossbeam_channel::{select, select_biased, Receiver, Sender};
use std::collections::HashMap;
use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

#[derive(Debug, Clone)]
pub struct MyDrone {
    pub id: NodeId,
    pub sim_controller_send: Sender<NodeEvent>, // Canale per inviare eventi dal SC
    pub sim_controller_recv: Receiver<DroneCommand>, // Canale per ricevere comandi dal SC
    pub packet_recv: Receiver<Packet>,          // Canale per ricevere pacchetti
    pub packet_send: HashMap<NodeId, Sender<Packet>>, // Mappa dei canali per inviare pacchetti ai neighbours nodes
    pub pdr: f32,                                     // PDR
}

impl Drone for MyDrone {
    fn new(options: DroneOptions) -> Self {
        assert!(
            !options.packet_send.contains_key(&options.id),
            "neighbor with id {} which is the same as drone",
            options.id
        );
        assert!(
            !(options.pdr > 1.0 || options.pdr < 0.0),
            "pdr out of bounds"
        );

        println!("Drone {} creato con PDR: {}", options.id, options.pdr);

        Self {
            id: options.id,
            sim_controller_send: options.controller_send,
            sim_controller_recv: options.controller_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr,
            packet_send: HashMap::new(),
        }
    }

    fn run(&mut self) {
        println!("Drone {} in esecuzione...", self.id);
        loop {
            select! {
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        println!("Drone {}: Ricevuto un pacchetto {:?}", self.id, packet);
                    } else {
                        println!("Drone {}: Canale dei pacchetti chiuso", self.id);
                        break;
                    }
                },
                recv(self.sim_controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        println!("Drone {}: Ricevuto un comando {:?}", self.id, command);
                        if let DroneCommand::Crash = command {
                            println!("Drone {}: Ricevuto comando di terminazione. Esco...", self.id);
                            break;
                        }
                        if let DroneCommand::SetPacketDropRate(a) = command {
                            self.pdr = a;
                            println!("Drone {}: Aggiornato il PDR!", self.id);
                        }
                    } else {
                        println!("Drone {}: Canale dei comandi chiuso", self.id);
                        break;
                    }
                }
            }
        }
    }
}

impl MyDrone {}
