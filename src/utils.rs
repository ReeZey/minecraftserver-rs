use std::{ net::TcpStream, io::{Read, Write} };

use crate::structs::{Player, ServerPlayer};

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
    let mut buffer: u8 = read_next(stream);

   while (buffer & CONTINUE_BIT) == CONTINUE_BIT
   {
       value |= ((buffer & SEGMENT_BITS) as u32) << (size * 7);
       size += 1;
       if size > 5 { return Err("something badding") };
       buffer = read_next(stream);
   }

   return Ok((value | (((buffer & SEGMENT_BITS) as u32) << (size * 7))).into());
}

pub fn read_string(stream: &mut TcpStream, length: usize) -> String {
    let readed = read_bytes(stream, length);
    return String::from_utf8(readed).unwrap();
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

        if done { return };
    }
}

pub fn write_string(buffer: &mut Vec<u8>, string: String){
    write_var_int(buffer, string.len() as i32);
    buffer.extend_from_slice(String::into_bytes(string).as_slice())
}

pub fn flush(stream: &mut TcpStream, data: &Vec<u8>, id: i32){
    let mut data_buffer: Vec<u8> = vec![];
    write_var_int(&mut data_buffer, id);
    data_buffer.extend(data);

    let mut buffer: Vec<u8> = vec![];
    write_var_int(&mut buffer, data_buffer.len() as i32);
    buffer.extend(data_buffer);

    stream.write(&buffer).expect("couldn't flush data");
}

//currently borken
pub fn _write_position(buffer: &mut Vec<u8>, x: i32, y: i32, z: i32) {
    let pos = ((x as u64 & 0x3FFFFFF) << 38) | ((z as u64 & 0x3FFFFFF) << 12) | (y as u64  & 0xFFF);
    buffer.push(pos.try_into().unwrap());
}

//currently borken
pub fn _disconnect_player(players: &Vec<Player>, username: String){
    for plr in players.iter() {
        if plr.username == username { drop(plr) }
    }
}

pub fn has_player(players: &Vec<Player>, username: String) -> bool {
    for plr in players.into_iter() {
        if plr.username == username { return true };
    }
    return false;
}

pub fn populate_players(players: &Vec<Player>) -> Vec<ServerPlayer> {
    let mut plrs: Vec<ServerPlayer> = vec![];

    if players.len() == 0 { return plrs };

    for plr in players {
        let player = ServerPlayer{ name: plr.username.clone(), id: plr.uuid.clone() };
        plrs.push(player);
    }

    return plrs;
}