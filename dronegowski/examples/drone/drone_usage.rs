#![allow(unused)]
use crossbeam_channel::{select_biased, unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::{fs, thread};
use wg_2024::config::Config;
use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::network::NodeId;
use wg_2024::packet::{Packet, PacketType};
use dronegowski::DroneOptions;
use dronegowski::Drone;

struct MyDrone {
    id: NodeId,
    controller_send: Sender<NodeEvent>, // Sending side of a channel
    controller_recv: Receiver<DroneCommand>, // The receiving side of a channel (for command form SC)
    packet_recv: Receiver<Packet>, // The receiving side of a channel (for packet from other nodes)
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>, // Sending side of the channel for send to other nodes
}

impl Drone for MyDrone {
    fn new(options: DroneOptions) -> Self {
        println!("Drone {} creato con PDR: {}", options.id, options.pdr);
        Self {
            id: options.id,
            controller_send: options.controller_send,
            controller_recv: options.controller_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr,
            packet_send: HashMap::new(),
        }
    }

    fn run(&mut self) {
        println!("Drone {} in esecuzione...", self.id);
        loop {
            select_biased! { // select biased ascolta entrambi i canali aspettando che uno dei due sia pronto
                recv(self.controller_recv) -> command => {
                    if let Ok(command) = command {
                        println!("Drone {} ha ricevuto comando: {:?}", self.id, command);
                        if let DroneCommand::Crash = command {
                            println!("Drone {} sta per crashare!", self.id);
                            break;
                        }
                        self.handle_command(command);
                    }
                }
                recv(self.packet_recv) -> packet => {
                    if let Ok(packet) = packet {
                        println!("Drone {} ha ricevuto il pacchetto: {:?}", self.id, packet);
                        self.handle_packet(packet);
                    }
                },
            }
        }
        println!("Drone {} terminato.", self.id);
    }
}

impl MyDrone {
    fn handle_packet(&mut self, packet: Packet) {
        println!("Drone {} sta gestendo pacchetto di tipo {:?}", self.id, packet.pack_type);
        match packet.pack_type {
            PacketType::Nack(_nack) => println!("Drone {}: Gestione NACK", self.id),
            PacketType::Ack(_ack) => println!("Drone {}: Gestione ACK", self.id),
            PacketType::MsgFragment(_fragment) => println!("Drone {}: Gestione MsgFragment", self.id),
            PacketType::FloodRequest(_flood_request) => println!("Drone {}: Gestione FloodRequest", self.id),
            PacketType::FloodResponse(_flood_response) => println!("Drone {}: Gestione FloodResponse", self.id),
        }
    }

    fn handle_command(&mut self, command: DroneCommand) {
        println!("Drone {} sta gestendo comando {:?}", self.id, command);
        match command {
            DroneCommand::AddSender(node_id, _sender) => {
                println!("Drone {}: Aggiunto sender per nodo {}", self.id, node_id);
            }
            DroneCommand::SetPacketDropRate(pdr) => {
                println!("Drone {}: Impostato nuovo PDR: {}", self.id, pdr);
                self.pdr = pdr;
            }
            DroneCommand::Crash => unreachable!(),
        }
    }
}

struct SimulationController {
    drones: HashMap<NodeId, Sender<DroneCommand>>,
    node_event_recv: Receiver<NodeEvent>,
}

impl SimulationController {
    fn crash_all(&mut self) {
        println!("Controller: Crash di tutti i droni!");
        for (_, sender) in self.drones.iter() {
            sender.send(DroneCommand::Crash).unwrap();
        }
    }

    fn send_command(&self, drone_id: NodeId, command: DroneCommand) {
        if let Some(sender) = self.drones.get(&drone_id) {
            println!("Controller: Inviato comando {:?} al drone {}", command, drone_id);
            sender.send(command).unwrap();
        } else {
            println!("Controller: Drone {} non trovato!", drone_id);
        }
    }
}

fn parse_config(file: &str) -> Config {
    let file_str = fs::read_to_string(file).unwrap();
    println!("Parsing del file di configurazione...");
    toml::from_str(&file_str).unwrap()
}

fn main() {
    let config = parse_config("examples/drone/config.toml");

    let mut controller_drones = HashMap::new();
    let (node_event_send, node_event_recv) = unbounded();

    let mut packet_channels = HashMap::new();
    for drone in config.drone.iter() {
        packet_channels.insert(drone.id, unbounded());
    }
    for client in config.client.iter() {
        packet_channels.insert(client.id, unbounded());
    }
    for server in config.server.iter() {
        packet_channels.insert(server.id, unbounded());
    }

    let mut handles = Vec::new();
    for drone in config.drone.into_iter() {
        let (controller_drone_send, controller_drone_recv) = unbounded();
        controller_drones.insert(drone.id, controller_drone_send);
        let node_event_send = node_event_send.clone();

        let packet_recv = packet_channels[&drone.id].1.clone();
        let packet_send = drone
            .connected_node_ids
            .into_iter()
            .map(|id| (id, packet_channels[&id].0.clone()))
            .collect();

        handles.push(thread::spawn(move || {
            let mut drone = MyDrone::new(DroneOptions {
                id: drone.id,
                controller_recv: controller_drone_recv,
                controller_send: node_event_send,
                packet_recv,
                packet_send,
                pdr: drone.pdr,
            });

            drone.run();
        }));
    }

    let mut controller = SimulationController {
        drones: controller_drones,
        node_event_recv,
    };

    println!("Simulazione avviata!");
    let mut input = String::new();
    loop {
        println!("Inserisci comando (crash_all, set_pdr <id> <pdr>, crash <id>, exit):");
        input.clear();
        std::io::stdin().read_line(&mut input).unwrap();
        let args: Vec<_> = input.trim().split_whitespace().collect();

        match args.as_slice() {
            ["crash_all"] => controller.crash_all(),
            ["set_pdr", id, pdr] => {
                let id = id.parse::<NodeId>().unwrap();
                let pdr = pdr.parse::<f32>().unwrap();
                controller.send_command(id, DroneCommand::SetPacketDropRate(pdr));
            }
            ["crash", id] => {
                let id = id.parse::<NodeId>().unwrap();
                controller.send_command(id, DroneCommand::Crash);
            }
            ["exit"] => break,
            _ => println!("Comando non riconosciuto!"),
        }
    }

    println!("In attesa della terminazione dei droni...");
    while let Some(handle) = handles.pop() {
        handle.join().unwrap();
    }
    println!("Simulazione terminata.");
}
