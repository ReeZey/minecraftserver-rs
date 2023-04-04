use std::{
    io::Write, path::Path, fs, sync::mpsc::Sender,
};

use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};
use uuid::Uuid;

use crate::{structs::{Player, StatusPlayers, Broadcast}, packets::*};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

pub async fn read_next(stream: &mut TcpStream) -> Option<u8> {
    let _n = match stream.read_u8().await {
        Ok(n) => return Some(n),
        Err(e) => {
            eprintln!("failed to read from socket; err = {:?}", e);
            return None;
        }
    };
}

pub async fn read_bytes(stream: &mut TcpStream, length: usize) -> Option<Vec<u8>> {
    let mut arr = vec![0; length];

    match stream.read(&mut arr).await {
        Ok(_) => return Some(arr),
        Err(e) => {
            eprintln!("failed to read from socket; err = {:?}", e);
            return None;
        }
    };
}

pub async fn read_var_int(stream: &mut TcpStream) -> Option<u32> {
    let mut value: u32 = 0;
    let mut size: u32 = 0;
    let mut current_byte = read_next(stream).await?;

    while (current_byte & CONTINUE_BIT) == CONTINUE_BIT {
        value |= ((current_byte & SEGMENT_BITS) as u32) << (size * 7);
        size += 1;
        if size > 5 {
            return None;
        };
        current_byte = read_next(stream).await?;
    }

    return Some((value | (((current_byte & SEGMENT_BITS) as u32) << (size * 7))).into());
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

pub async fn read_string(stream: &mut TcpStream) -> Option<String> {
    let stringlen = read_var_int(stream).await? as usize;
    let readed = read_bytes(stream, stringlen).await?;
    return Some(String::from_utf8(readed).unwrap());
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

pub fn write_string_chat(buffer: &mut Vec<u8>, string: &String) {
    let string2 = format!("{{\"text\":\"{string}\"}}");
    write_var_int(buffer, string2.len() as i32);
    buffer.extend_from_slice(String::into_bytes(string2).as_slice())
}

pub async fn flush(stream: &mut TcpStream, buffer: &mut Vec<u8>, id: i32) {
    let mut data_buffer: Vec<u8> = vec![];
    write_var_int(&mut data_buffer, id);
    data_buffer.extend(buffer.clone());

    let mut packet: Vec<u8> = vec![];
    write_var_int(&mut packet, data_buffer.len() as i32);
    packet.extend(data_buffer);

    stream.write_all(&packet).await.unwrap();
    buffer.clear();
}

pub async fn send_chat_message(mut stream: &mut TcpStream, message: String){
    let mut buffer = vec![];
    write_string_chat(&mut buffer, &message);
    buffer.push(0);
    flush(&mut stream, &mut buffer, CPlayPacketid::Chat as i32).await;
}

pub async fn send_actionbar(mut stream: &mut TcpStream, message: String){
    let mut buffer = vec![];
    write_string_chat(&mut buffer, &message);
    buffer.push(1);
    flush(&mut stream, &mut buffer, CPlayPacketid::Chat as i32).await;
}

pub fn write_position(buffer: &mut Vec<u8>, x: i32, y: i32, z: i32) {
    let pos = ((x as u64 & 0x3FFFFFF) << 38) | ((z as u64 & 0x3FFFFFF) << 12) | (y as u64 & 0xFFF);
    buffer.extend(pos.to_be_bytes());
}

pub fn broadcast(tx: &Sender<Broadcast>, buffer: Vec<u8>, packet_id: i32, stream: &TcpStream){
    let broacast = Broadcast {
        data: buffer,
        packet_id,
        stream_name: stream.peer_addr().unwrap().to_string()
    };
    
    tx.send(broacast).unwrap();
}

pub fn read_position(buffer: &mut Vec<u8>) -> (i32, i32, i32) {
    let readpos = u64::from_be_bytes(buffer.drain(0..8).as_slice().try_into().unwrap());
    let mut x = (readpos >> 38) as i32;
    let mut y = (readpos << 52 >> 52) as i32;
    let mut z = (readpos << 26 >> 38) as i32;

    if x >= 1 << 25 { x -= 1 << 26 }
    if y >= 1 << 11 { y -= 1 << 12 }
    if z >= 1 << 25 { z -= 1 << 26 }

    return (x,y,z);
}

pub fn disconnect_player(players: &mut Vec<Player>, username: &String) {
    let mut index = 0;
    for plr in players.iter() {
        if plr.username == *username {
            players.remove(index);
            break;
        }
        index += 1;
    }
}

pub fn has_player(players: &Vec<Player>, username: &String) -> bool {
    for plr in players.iter() {
        if plr.username == *username {
            return true;
        };
    }
    return false;
}

pub fn _get_player(players: &Vec<Player>, username: &String) -> Option<Player> {
    for plr in players.iter() {
        if plr.username == *username {
            return Some(plr.clone());
        };
    }

    None
}

pub fn populate_players(players: &Vec<Player>) -> Vec<StatusPlayers> {
    let mut plrs: Vec<StatusPlayers> = vec![];

    if players.len() == 0 {
        return plrs;
    };

    for plr in players {
        let player = StatusPlayers {
            name: plr.username.clone(),
            id: Uuid::from_bytes(plr.uuid.to_be_bytes()).to_string(),
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