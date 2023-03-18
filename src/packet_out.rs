use crate::utils::{write_var_int, write_string};

pub struct PacketOutLoginPlay {
    entity_id: i32,
    hardcore: bool,
    gamemode: u8,
    prev_gamemode: i8,
    dimensions: Vec<String>,
    registry_codec : Vec<String>,
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
    death_dimension: String,
    death_location: [f64; 3],
}

impl PacketOutLoginPlay {
    
    pub fn new(entity_id: i32) -> PacketOutLoginPlay {
        return PacketOutLoginPlay { 
            entity_id, 
            hardcore: false, 
            gamemode: 0, 
            prev_gamemode: -1 as i8, 
            dimensions: vec![], 
            registry_codec: vec![], 
            dimension_type: "minecraft:overworld".to_string(), 
            dimension_name: "bajs".to_string(), 
            hashed_seed: 5, 
            max_players: 20, 
            view_distance: 5, 
            simulation_distance: 5, 
            reduced_debug_info: false, 
            enable_respawn_screen: true, 
            debug: false, 
            flat: false, 
            has_death: false, 
            death_dimension: "minecraft:the_nether".to_string(), 
            death_location: [0.0, 0.0, 0.0]
        }
    }

    pub fn serialize(packet: &PacketOutLoginPlay, buffer: &mut Vec<u8>){
        buffer.extend(packet.entity_id.to_be_bytes());
        buffer.push(packet.hardcore as u8);
        buffer.push(packet.gamemode);
        buffer.push(packet.prev_gamemode as u8);
        //dimensions
        write_var_int(buffer, 0);
        //write_string(buffer, "minecraft:overworld".to_string());
        //write_string(buffer, "minecraft:the_nether".to_string());
        //write_string(buffer, "minecraft:the_end".to_string());
        //codec
        buffer.extend([0x10, 0x00, 0x00, 0x00]);
        write_string(buffer, packet.dimension_type.to_string());
        write_string(buffer, packet.dimension_name.to_string());
        buffer.extend(packet.hashed_seed.to_be_bytes());
        buffer.extend(packet.max_players.to_be_bytes());
        buffer.extend(packet.view_distance.to_be_bytes());
        buffer.extend(packet.simulation_distance.to_be_bytes());
        buffer.push(packet.reduced_debug_info as u8);
        buffer.push(packet.enable_respawn_screen as u8);
        buffer.push(packet.debug as u8);
        buffer.push(packet.flat as u8);
        buffer.push(packet.has_death as u8);
    }
}