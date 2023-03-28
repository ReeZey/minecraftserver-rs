use std::{fs, io::Read};

use crate::utils::{write_var_int, write_string};
use std::io::Write;

pub struct PacketOutLoginPlay {
    entity_id: i32,
    hardcore: bool,
    gamemode: u8,
    prev_gamemode: i8,
    dimensions: Vec<String>,
    registry_codec : Vec<u8>,
    dimension_type: String,
    dimension_name: String,
    hashed_seed: i64,
    max_players: i64,
    view_distance: i64,
    simulation_distance: i64,
    reduced_debug_info: bool,
    enable_respawn_screen: bool,
    debug: bool,
    flat: bool,
    has_death: bool,

    //not used: might wanna use in the future
    _death_dimension: String,
    _death_location: [f64; 3],
}

impl PacketOutLoginPlay {
    pub fn new(entity_id: i32) -> PacketOutLoginPlay {

        let mut dimensions = vec![];
        dimensions.push("minecraft:overworld".to_string());
        dimensions.push("minecraft:the_nether".to_string());
        dimensions.push("minecraft:the_end".to_string());

        let mut registry_codec = vec![];
        let mut handle = fs::File::open("registry-codec.nbt").unwrap();
        handle.read_to_end(&mut registry_codec).unwrap();
        drop(handle);

        return PacketOutLoginPlay { 
            entity_id, 
            hardcore: false, 
            gamemode: 0, 
            prev_gamemode: -1 as i8, 
            dimensions,
            registry_codec, 
            dimension_type: "minecraft:overworld".to_string(), 
            dimension_name: "minecraft:overworld".to_string(), 
            hashed_seed: 5, 
            max_players: 20, 
            view_distance: 5, 
            simulation_distance: 5, 
            reduced_debug_info: false, 
            enable_respawn_screen: true, 
            debug: false, 
            flat: false, 
            has_death: false, 
            _death_dimension: "minecraft:the_nether".to_string(), 
            _death_location: [0.0, 0.0, 0.0]
        }
    }

    pub fn serialize(packet: &PacketOutLoginPlay, buffer: &mut Vec<u8>){
        buffer.extend(packet.entity_id.to_be_bytes());
        buffer.push(packet.hardcore as u8);
        buffer.push(packet.gamemode);
        buffer.push(packet.prev_gamemode as u8);
        write_var_int(buffer, packet.dimensions.len() as i32);
        for dim in packet.dimensions.clone()  {
            write_string(buffer, dim);
        }
        buffer.extend(&packet.registry_codec);
        write_string(buffer, packet.dimension_type.to_string());
        write_string(buffer, packet.dimension_name.to_string());
        buffer.extend(packet.hashed_seed.to_be_bytes());
        write_var_int(buffer, packet.max_players as i32);
        write_var_int(buffer, packet.view_distance as i32);
        write_var_int(buffer, packet.simulation_distance as i32);
        buffer.push(packet.reduced_debug_info as u8);
        buffer.push(packet.enable_respawn_screen as u8);
        buffer.push(packet.debug as u8);
        buffer.push(packet.flat as u8);
        buffer.push(packet.has_death as u8);
    }
}

pub struct SynchronizePlayerPosition {
    x: f64,
    y: f64,
    z: f64,

    yaw: f32,
    pitch: f32,

    flags: u8,
    dismount: bool,
}

impl SynchronizePlayerPosition {
    pub fn new(x: f64, y: f64, z: f64, yaw: f32, pitch: f32, flags: u8, dismount: bool) -> SynchronizePlayerPosition {
        return SynchronizePlayerPosition { x, y, z, yaw, pitch, flags, dismount }
    }

    pub fn serialize(packet: &SynchronizePlayerPosition, buffer: &mut Vec<u8>){
        buffer.write(&packet.x.to_be_bytes()).unwrap();
        buffer.write(&packet.y.to_be_bytes()).unwrap();
        buffer.write(&packet.z.to_be_bytes()).unwrap();
        buffer.write(&packet.yaw.to_be_bytes()).unwrap();
        buffer.write(&packet.pitch.to_be_bytes()).unwrap();
        buffer.push(packet.flags);
        write_var_int(buffer, 0);
        buffer.push(packet.dismount as u8);
    }
}