use std::{net::TcpStream, io::{Read, Write}};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

pub fn read_bytes(stream: &mut TcpStream, length: usize) -> Vec<u8> {
    let mut arr = vec![0; length];
    stream.read(&mut arr).unwrap();
    return arr;
}

pub fn read_var_int(stream: &mut TcpStream) -> Result<u32, &'static str> {
    //read(stream, 1)[0] & CONTINUE_BIT;
    let mut value: u32 = 0;
    let mut size: u32 = 0;
    let mut buffer: u8 = read_bytes(stream, 1)[0];

   while (buffer & CONTINUE_BIT) == CONTINUE_BIT
   {
       value |= ((buffer & SEGMENT_BITS) as u32) << (size * 7);
       size += 1;
       if size > 5 { return Err("something badding") };
       buffer = read_bytes(stream, 1)[0];
   }

   return Ok((value | (((buffer & SEGMENT_BITS) as u32) << (size * 7))).into());
}

pub fn read_string(stream: &mut TcpStream, length: usize) -> String {
    let readed = read_bytes(stream, length);
    return String::from_utf8(readed).unwrap();
}

pub fn write_bytes(stream: &mut TcpStream, bytes: Vec<u8>){
    stream.write(&bytes).unwrap();
}

pub fn write_var_int(stream: &mut TcpStream, value: i32){
    let mut internal = value;

    while (internal & CONTINUE_BIT as i32) != 0 {
        stream.write(&[((internal as u8 & SEGMENT_BITS) | CONTINUE_BIT)]).unwrap();
        internal = internal >> 7;
    }

    write_bytes(stream, internal.to_be_bytes().try_into().unwrap());
}

pub fn write_string(stream: &mut TcpStream, string: String){
    write_bytes(stream, String::into_bytes(string))
}

/*
pub fn test() -> String {
    return "hej".to_string();
}
*/