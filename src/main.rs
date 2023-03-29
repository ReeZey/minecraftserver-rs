mod utils;
mod structs;
mod packet_out;
mod packets;

use structs::*;
use utils::*;
use packet_out::*;
use packets::*;

use uuid::Uuid;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs, thread };
use std::io::{ Read, Write, Cursor };
use std::net::TcpListener;
use byteorder::{BigEndian, ReadBytesExt};

#[macro_use] extern crate serde_derive;

fn main() -> std::io::Result<()> {
    if !Path::new("players").is_dir() {
        fs::create_dir("players").unwrap();
    }

    let listener = TcpListener::bind("0.0.0.0:51413")?;
    let players: Vec<Player> = vec![];
    let players_accessor = Arc::new(Mutex::new(players));

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

                        let _address = read_string(&mut stream);
                        let _port = u16::from_be_bytes(read_bytes(&mut stream, 2).try_into().unwrap());

                        //println!("{address}:{port}");

                        let nextstate = read_var_int(&mut stream).unwrap();

                        match nextstate {
                            1 => {
                                let players = players_accessor.lock().unwrap();
                                //send status
                                let status = ServerStatus {
                                    version: StatusVersion { name: "1.19.3".to_string(), protocol: 761 },
                                    players: StatusPlayerCount { max: 20, online: players.len() as i32, sample: populate_players(&players) },
                                    description: StatusDescription { text: "hello aa".to_string() },
                                    enforces_secure_chat: false,
                                };
                                drop(players);

                                write_string(&mut buffer, serde_json::to_string(&status).unwrap());
                                flush(&mut stream, &mut buffer, CStatusPacketid::Status as i32);
                                buffer.clear();
                                
                                //check ping
                                let handle = read_bytes(&mut stream, 8);
                                buffer.extend(&handle);
                                
                                flush(&mut stream, &mut buffer, CStatusPacketid::Ping as i32);
                                buffer.clear();
                            },
                            2 => {
                                //user connecting
                                let _connectionid = read_var_int(&mut stream).unwrap();
                                let _identifier = read_var_int(&mut stream).unwrap();

                                let username = read_string(&mut stream);

                                let mut uuid: u128 = 0;
                                let has_guid = read_next(&mut stream);
                                if has_guid == 1 {
                                    let array = read_bytes(&mut stream, 16);
                                    uuid = u128::from_be_bytes(array.as_slice().try_into().unwrap());
                                }

                                let players = players_accessor.lock().unwrap();
                                //allow user if not playing
                                if has_player(&players, username.clone()) {
                                    let mut buffer: Vec<u8> = vec![];
                                    write_string_chat(&mut buffer, "You are already connected to this server!".to_string());
                                    flush(&mut stream, &mut buffer, CLoginPacketid::Kick as i32);
                                    continue;
                                }
                                drop(players);

                                //println!("{} {} {}", username, uuid, has_guid);

                                //player isn't currently playing, let him in!
                                let mut buffer: Vec<u8> = vec![];
                                buffer.extend(&uuid.to_be_bytes());
                                write_string(&mut buffer, username.clone());
                                write_var_int(&mut buffer, 0);
                                flush(&mut stream, &mut buffer, CLoginPacketid::Success as i32);
                                buffer.clear();

                                //has player played before? otherwise create new player
                                let pathname: String = format!("./players/{}.bin", username.clone());
                                let path: &Path = Path::new(&pathname);

                                let mut player: Player = Player{
                                    username: username.clone(), 
                                    uuid: Uuid::from_bytes(uuid.to_be_bytes()).to_string(),
                                    
                                    x: 8.0, 
                                    y: 90.0, 
                                    z: 8.0,

                                    yaw: 0.0,
                                    pitch: 0.0,

                                    gamemode: 0,
                                };

                                if path.exists() {
                                    println!("aoegbaeogba");
                                    let mut file: File = fs::File::open(path).unwrap();

                                    file.read_to_end(&mut buffer).unwrap();
                                    let mut crs = Cursor::new(&buffer);
                                    player.x = crs.read_f64::<BigEndian>().unwrap();
                                    player.y = crs.read_f64::<BigEndian>().unwrap();
                                    player.z = crs.read_f64::<BigEndian>().unwrap();

                                    player.yaw = crs.read_f32::<BigEndian>().unwrap();
                                    player.pitch = crs.read_f32::<BigEndian>().unwrap();
                                    player.gamemode = crs.read_u8().unwrap();
                                    
                                    buffer.clear()
                                } else {
                                    save_player(&player);
                                }

                                println!("{}", player.x.clone());

                                //send play 
                                let loginplay = PacketOutLoginPlay::new(0);
                                PacketOutLoginPlay::serialize(&loginplay, &mut buffer);
                                flush(&mut stream, &mut buffer, CPlayPacketid::LoginPlay as i32);
                                buffer.clear();
                                
                                //update tab
                                buffer.write(&[9]).unwrap();
                                write_var_int(&mut buffer, 1);
                                buffer.extend(uuid.to_be_bytes());
                                write_string(&mut buffer, username);
                                write_var_int(&mut buffer, 0);
                                buffer.write(&[1]).unwrap();
                                flush(&mut stream, &buffer, CPlayPacketid::PlayerInfo as i32);
                                buffer.clear();
                                
                                //send default spawn
                                let flytis: f32 = 0.0;
                                write_position(&mut buffer, 8, 64, 8);
                                buffer.write(&flytis.to_be_bytes()).unwrap();
                                flush(&mut stream, &mut buffer, CPlayPacketid::SetDefaultSpawn as i32);
                                buffer.clear();

                                //teleport player
                                let sync_player = SynchronizePlayerPosition::new(player.x, player.y, player.z, player.yaw, player.pitch, 0, false);
                                SynchronizePlayerPosition::serialize(&sync_player, &mut buffer);
                                flush(&mut stream, &mut buffer, CPlayPacketid::PlayerPos as i32);
                                buffer.clear();

                                //send one chunk
                                let mut chunkdata = vec![];
                                let mut handle = fs::File::open("chunk.bin").unwrap();
                                handle.read_to_end(&mut chunkdata).unwrap();
                                drop(handle);
                                stream.write_all(&chunkdata).unwrap();

                                //push player to playerlist
                                let mut players = players_accessor.lock().unwrap();
                                players.push(player.clone());
                                drop(players);

                                //create new thread, better solution required
                                let playersmutex = players_accessor.clone();
                                thread::spawn(move || {
                                    let players = playersmutex.lock().unwrap();
                                    println!("player {} connected, total: {}", player.username, players.len());
                                    drop(players);
                                    let mut tick = 0;
                                    let mut lastkeepalive: u64 = 0;

                                    player.gamemode = 0;

                                    loop {
                                        let size = read_var_int(&mut stream).unwrap();
                                        if size == 0 {
                                            let mut players = playersmutex.lock().unwrap();
                                            disconnect_player(&mut players, player.username.clone());
                                            println!("player {} disconnected, total: {}", player.username, players.len());
                                            save_player(&player);
                                            break; 
                                        }

                                        if tick > 20 {
                                            //println!("keep alive");
                                            tick = 0;

                                            lastkeepalive = 69;

                                            let mut buffer: Vec<u8> = vec![];
                                            buffer.extend(lastkeepalive.to_be_bytes());
                                            flush(&mut stream, &buffer, CPlayPacketid::KeepAlive as i32);
                                        }

                                        let mut reading: Vec<u8> = vec![0; size.try_into().unwrap()];
                                        stream.read(&mut reading).unwrap();

                                        let packetid = read_var_int_buf(&mut reading).unwrap();
                                        match packetid {
                                            4 => {
                                                let stringlen = read_var_int_buf(&mut reading).unwrap();
                                                let message = read_string_buf(&mut reading, stringlen as usize);

                                                let args = message.split(" ").collect::<Vec<&str>>();
                                                
                                                if args.len() < 1 {
                                                    continue;
                                                }

                                                match args[0] {
                                                    "give" => {
                                                        if args.len() < 3 {
                                                            send_chat_message(&mut stream, "incorrect usage".to_string());
                                                            continue;
                                                        }
                                                        let id: Result<u8, _> = args[1].parse();
                                                        let count: Result<u8, _> = args[2].parse();
                                                        if id.is_err() || count.is_err() {
                                                            send_chat_message(&mut stream, "invalid numbers".to_string());
                                                            continue;
                                                        }

                                                        let mut buffer = vec![];
                                                        buffer.write(&[0]).unwrap();
                                                        write_var_int(&mut buffer, 0);
                                                        write_var_int(&mut buffer, 37);
                                                        for _num in 0..36 {
                                                            buffer.write(&[0]).unwrap();
                                                        }
                                                        buffer.write(&[1,id.unwrap(),count.unwrap(),0]).unwrap();
                                                        buffer.write(&[0]).unwrap();
                                                        flush(&mut stream, &buffer, CPlayPacketid::ContainerContent as i32);
                                                    }
                                                    _ => { 
                                                        send_chat_message(&mut stream, "command not found".to_string());
                                                        continue;
                                                     }
                                                }
                                            }
                                            5 => {
                                                let stringlen = read_var_int_buf(&mut reading).unwrap();
                                                let message = read_string_buf(&mut reading, stringlen as usize);

                                                println!("{}: {}", player.username.clone(), message);

                                                send_chat_message(&mut stream, format!("{}: {}", player.username.clone(), message));
                                            }
                                            6 => {
                                                //0x1C 
                                                if player.gamemode == 0 {
                                                    player.gamemode = 3;
                                                } else {
                                                    player.gamemode = 0;
                                                }

                                                let gamemode = player.gamemode as f32;
                                                let mut buffer = vec![];
                                                buffer.write(&[3]).unwrap();
                                                buffer.extend(gamemode.to_be_bytes());
                                                flush(&mut stream, &buffer, CPlayPacketid::GameEvent as i32);
                                            }
                                            17 => {
                                                if lastkeepalive == u64::from_be_bytes(reading.drain(0..8).as_slice().try_into().unwrap()) {
                                                    continue;
                                                }
                                                
                                                let mut buffer: Vec<u8> = vec![];
                                                write_string_chat(&mut buffer, "invalid keep-alive response".to_string());
                                                flush(&mut stream, &buffer, CPlayPacketid::Kick as i32);

                                                let mut players = playersmutex.lock().unwrap();
                                                disconnect_player(&mut players, player.username.clone());
                                                drop(stream);
                                                break;
                                            }
                                            11 => {
                                                let gamemode: f32 = 0.0;
                                                let mut buffer = vec![];
                                                buffer.write(&[3]).unwrap();
                                                buffer.extend(gamemode.to_be_bytes());
                                                flush(&mut stream, &buffer, CPlayPacketid::GameEvent as i32);
                                            }
                                            12 => {
                                                //plugin message
                                                //println!("{}", String::from_utf8(reading).unwrap());
                                            }
                                            19 => { 
                                                //update move 
                                            }
                                            20 => { 
                                                //update move and rot
                                            }
                                            21 => { 
                                                //update rot
                                            }
                                            29 => { 
                                                //sprint / sneak
                                            }
                                            33 => {
                                                player.gamemode = 1;

                                                let gamemode = player.gamemode as f32;
                                                let mut buffer = vec![];
                                                buffer.write(&[3]).unwrap();
                                                buffer.extend(gamemode.to_be_bytes());
                                                flush(&mut stream, &buffer, CPlayPacketid::GameEvent as i32);
                                            }
                                            47 => {
                                                //break block
                                            }
                                            49 => {
                                                //block interact
                                                println!("packet {} [0x{:X}] {:?}", packetid, packetid, &reading);
                                            }
                                            _ => {
                                                println!("unhandled packet {} [0x{:X}] {:?}", packetid, packetid, &reading);
                                            }
                                        }

                                        tick += 1;
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