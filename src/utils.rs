use std::{
    io::{Read, Write},
    net::TcpStream, path::Path, fs,
};

use crate::{structs::{Player, StatusPlayers}, packets::*};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

pub fn read_next(stream: &mut TcpStream) -> u8 {
    let mut buf = [0; 1];
    let handle = stream.read_exact(&mut buf);
    if handle.is_err() { drop(stream) }
    return buf[0];
}

pub fn read_bytes(stream: &mut TcpStream, length: usize) -> Vec<u8> {
    let mut arr = vec![0; length];
    stream.read(&mut arr).unwrap();
    return arr;
}

pub fn read_var_int(stream: &mut TcpStream) -> Result<u32, &'static str> {
    let mut value: u32 = 0;
    let mut size: u32 = 0;
    let mut current_byte: u8 = read_next(stream);

    while (current_byte & CONTINUE_BIT) == CONTINUE_BIT {
        value |= ((current_byte & SEGMENT_BITS) as u32) << (size * 7);
        size += 1;
        if size > 5 {
            return Err("something badding");
        };
        current_byte = read_next(stream);
    }

    return Ok((value | (((current_byte & SEGMENT_BITS) as u32) << (size * 7))).into());
}

pub fn read_var_int_buf(buffer: &mut Vec<u8>) -> Result<u32, &'static str> {
    let mut value: u32 = 0;
    let mut size: u32 = 0;
    let mut current_byte: u8 = buffer[0];
    buffer.remove(0);

    while (current_byte & CONTINUE_BIT) == CONTINUE_BIT {
        value |= ((current_byte & SEGMENT_BITS) as u32) << (size * 7);
        size += 1;
        if size > 5 {
            return Err("something badding");
        };
        current_byte = buffer[0];
        buffer.remove(0);
    }

    return Ok((value | (((current_byte & SEGMENT_BITS) as u32) << (size * 7))).into());
}

pub fn read_string(stream: &mut TcpStream) -> String {
    let stringlen = read_var_int(stream).unwrap() as usize;
    let readed = read_bytes(stream, stringlen);
    return String::from_utf8(readed).unwrap();
}

pub fn read_string_buf(buffer: &mut Vec<u8>) -> String {
    let stringlen = read_var_int_buf(buffer).unwrap() as usize;
    return String::from_utf8(buffer.drain(0..stringlen).as_slice().to_vec()).unwrap();
}

pub fn write_var_int(buffer: &mut Vec<u8>, value2: i32) {
    let mut val: i32 = value2;
    loop {
        let mut byte = val as u8;

        val >>= 6;
        let done = val == 0 || val == -1;
        if done {
            byte &= !CONTINUE_BIT;
        } else {
            val >>= 1;
            byte |= CONTINUE_BIT;
        }

        buffer.push(byte);

        if done {
            return;
        };
    }
}

pub fn write_string(buffer: &mut Vec<u8>, string: String) {
    write_var_int(buffer, string.len() as i32);
    buffer.extend_from_slice(String::into_bytes(string).as_slice())
}

pub fn write_string_chat(buffer: &mut Vec<u8>, string: String) {
    let string2 = format!("{{\"text\":\"{string}\"}}");
    write_var_int(buffer, string2.len() as i32);
    buffer.extend_from_slice(String::into_bytes(string2).as_slice())
}

pub fn flush(stream: &mut TcpStream, buffer: &mut Vec<u8>, id: i32) {
    let mut data_buffer: Vec<u8> = vec![];
    write_var_int(&mut data_buffer, id);
    data_buffer.extend(buffer.clone());

    let mut packet: Vec<u8> = vec![];
    write_var_int(&mut packet, data_buffer.len() as i32);
    packet.extend(data_buffer);

    stream.write(&packet).unwrap();
    buffer.clear();
}

pub fn send_chat_message(mut stream: &mut TcpStream, message: String){
    let mut buffer = vec![];
    write_string_chat(&mut buffer, message);
    buffer.write(&[0]).unwrap();
    flush(&mut stream, &mut buffer, CPlayPacketid::Chat as i32);
}

pub fn send_actionbar(mut stream: &mut TcpStream, message: String){
    let mut buffer = vec![];
    write_string_chat(&mut buffer, message);
    buffer.write(&[1]).unwrap();
    flush(&mut stream, &mut buffer, CPlayPacketid::Chat as i32);
}

pub fn write_position(buffer: &mut Vec<u8>, x: i32, y: i32, z: i32) {
    let pos = ((x as u64 & 0x3FFFFFF) << 38) | ((z as u64 & 0x3FFFFFF) << 12) | (y as u64 & 0xFFF);
    buffer.extend(pos.to_be_bytes());
}

pub fn disconnect_player(players: &mut Vec<Player>, username: String) {
    let mut index = 0;
    for plr in players.iter() {
        if plr.username == username {
            players.remove(index);
            break;
        }
        index += 1;
    }
}

pub fn has_player(players: &Vec<Player>, username: String) -> bool {
    for plr in players.into_iter() {
        if plr.username == username {
            return true;
        };
    }
    return false;
}

pub fn populate_players(players: &Vec<Player>) -> Vec<StatusPlayers> {
    let mut plrs: Vec<StatusPlayers> = vec![];

    if players.len() == 0 {
        return plrs;
    };

    for plr in players {
        let player = StatusPlayers {
            name: plr.username.clone(),
            id: plr.uuid.clone(),
        };
        plrs.push(player);
    }

    return plrs;
}

pub fn save_player(player: &Player){
    let pathname: String = format!("./players/{}.bin", player.username.clone());
    let path: &Path = Path::new(&pathname);

    let mut file = fs::File::create(path).unwrap();
    let mut buffer: Vec<u8> = vec![];

    buffer.extend(player.x.to_be_bytes());
    buffer.extend(player.y.to_be_bytes());
    buffer.extend(player.z.to_be_bytes());
    buffer.extend(player.yaw.to_be_bytes());
    buffer.extend(player.pitch.to_be_bytes());
    buffer.extend(player.gamemode.to_be_bytes());
    file.write_all(&buffer).unwrap();
}
