use std::{net::TcpStream, io::{Read, Write}, fs};

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

pub fn write_bytes(stream: &mut TcpStream, buffer: &Vec<u8>){
    stream.write(buffer).unwrap();
}

pub fn write_var_int(buffer: &mut Vec<u8>, value: i32){
    let mut internal = value;

    while (internal & CONTINUE_BIT as i32) != 0 {
        buffer.push((internal as u8 & SEGMENT_BITS) | CONTINUE_BIT);
        internal = internal >> 7;
    }

    buffer.push(internal.try_into().unwrap());
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

    //fs::write("file.bin", &buffer).expect("Failed to write file");

    write_bytes(stream,  &buffer);
}