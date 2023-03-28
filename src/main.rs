mod utils;
mod structs;
mod packet_out;

use structs::*;
use utils::*;
use packet_out::*;

use uuid::Uuid;
use std::{fs, thread};
use std::io::{ Read, Write };
use std::net::TcpListener;

#[macro_use] extern crate serde_derive;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:51413")?;
    let mut players: Vec<Player> = vec![];

    // accept incoming connections and process them serially
    Ok(for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _length = read_var_int(&mut stream).unwrap();
                let packedid = read_var_int(&mut stream).unwrap();

                //println!("{} {}", length, packedid);
                
                match packedid {
                    0 => {
                        let mut buffer: Vec<u8> = vec![];
                        let _version = read_var_int(&mut stream).unwrap();

                        let address_len = read_var_int(&mut stream).unwrap() as usize;
                        let _address = read_string(&mut stream, address_len);
                        let _port = u16::from_be_bytes(read_bytes(&mut stream, 2).try_into().unwrap());

                        //println!("{address}:{port}");

                        let nextstate = read_var_int(&mut stream).unwrap();

                        match nextstate {
                            1 => {
                                //send status
                                let status = ServerStatus {
                                    version: ServerVersion { name: "1.19.3".to_string(), protocol: 761 },
                                    players: ServerPlayerCount { max: 20, online: players.len() as i32, sample: populate_players(&players) },
                                    description: ServerDescription { text: "hello aa".to_string() },
                                    enforces_secure_chat: false,
                                };

                                write_string(&mut buffer, serde_json::to_string(&status).unwrap());
                                flush(&mut stream, &mut buffer, 0);
                                buffer.clear();
                                
                                //check ping
                                let handle = read_bytes(&mut stream, 8);
                                buffer.extend(&handle);
                                
                                flush(&mut stream, &mut buffer, 1);
                                buffer.clear();
                            },
                            2 => {
                                //user connecting
                                let _connectionid = read_var_int(&mut stream).unwrap();
                                let _identifier = read_var_int(&mut stream).unwrap();
                                
                                let user_len = read_var_int(&mut stream).unwrap();
                                let username = read_string(&mut stream, user_len as usize);

                                let mut uuid: u128 = 0;
                                let has_guid = read_next(&mut stream);
                                if has_guid == 1 {
                                    let array = read_bytes(&mut stream, 16);
                                    uuid = u128::from_be_bytes(array.as_slice().try_into().unwrap());
                                }

                                //allow user if not playing
                                if has_player(&players, username.clone()) {
                                    let mut buffer: Vec<u8> = vec![];
                                    write_string(&mut buffer, "\"You are already connected to this server!\"".to_string());
                                    flush(&mut stream, &mut buffer, 0);
                                    continue;
                                }

                                println!("{} {} {}", username, uuid, has_guid);
                                
                                let mut buffer: Vec<u8> = vec![];
                                buffer.extend(&uuid.to_be_bytes());
                                write_string(&mut buffer, username.clone());
                                write_var_int(&mut buffer, 0);
                                flush(&mut stream, &mut buffer, 2);
                                buffer.clear();

                                //structure player
                                let player = Player{
                                    x: 8.0, 
                                    y: 90.0, 
                                    z: 8.0,

                                    yaw: 0.0,
                                    pitch: 0.0, 
                                    
                                    username, 
                                    uuid: Uuid::from_bytes(uuid.to_be_bytes()).to_string() 
                                };

                                //send play 
                                let abc = PacketOutLoginPlay::new(0);
                                PacketOutLoginPlay::serialize(&abc, &mut buffer);

                                flush(&mut stream, &mut buffer, 0x24);
                                buffer.clear();
                                
                                //send default spawn
                                let flytis: f32 = 0.0;
                                let numbaah: u64 = 0;
                                //write_position(&mut butter, 0, 0, 0);
                                buffer.write(&numbaah.to_be_bytes()).unwrap();
                                buffer.write(&flytis.to_be_bytes()).unwrap();
                                flush(&mut stream, &mut buffer, 0x4C);
                                buffer.clear();

                                //teleport player
                                let sync_player = SynchronizePlayerPosition::new(player.x, player.y, player.z, player.yaw, player.pitch, 0, false);
                                SynchronizePlayerPosition::serialize(&sync_player, &mut buffer);
                                flush(&mut stream, &mut buffer, 0x38);
                                buffer.clear();

                                //send one chunk
                                let mut chunkdata = vec![];
                                let mut handle = fs::File::open("chunk.bin").unwrap();
                                handle.read_to_end(&mut chunkdata).unwrap();
                                drop(handle);
                                stream.write_all(&chunkdata).unwrap();

                                //push player to playerlist
                                players.push(player);

                                //create new thread, better solution required
                                thread::spawn(move || {
                                    loop {
                                        let size = read_var_int(&mut stream).unwrap();
                                        let mut reading: Vec<u8> = vec![0; size.try_into().unwrap()];
                                        stream.read(&mut reading).unwrap();

                                        if size == 0 {
                                            //disconnect_player(&players, player.username);
                                            println!("player disconnected");
                                            break; 
                                        }

                                        println!("{:?}", reading);
                                    }
                                });
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    })
}