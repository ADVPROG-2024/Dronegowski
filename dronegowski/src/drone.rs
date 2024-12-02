use crossbeam_channel::{select, Receiver, Sender};
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
            packet_send: options.packet_send, // Corretto: usa il packet_send passato
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
                        match command {
                            DroneCommand::SetPacketDropRate(pdr) => {
                                if let Err(err) = self.set_pdr(pdr) {
                                    println!("Errore nel drone {}: {}", self.id, err);
                                }
                            },
                            DroneCommand::Crash => {
                                // Da terminare
                                println!("Drone {} terminato", self.id);
                                break;
                            },
                            DroneCommand::AddSender(node_id, sender) => {
                                self.add_channel(node_id, sender);
                            },
                        }
                    } else {
                        println!("Drone {}: Canale comandi chiuso", self.id);
                        break;
                    }
                }
            }
        }
        println!("Drone {}: Uscito dal loop", self.id);
    }
}

impl MyDrone {
    fn set_pdr(&mut self, pdr: f32) -> Result<(), String> {
        if pdr > 0.0 && pdr < 1.0 {
            println!("Drone {}: modificato PDR, {} -> {}", self.id, self.pdr, pdr);
            self.pdr = pdr;
            return Ok(());
        }

        Err("Incorrect value of PDR".to_string())
    }

    fn add_channel(&mut self, node_id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(node_id, sender);
    }
}
