mod utils;
mod structs;

use utils::*;
use structs::*;

use std::io::{ Read, Write };
use std::net::TcpListener;
use uuid::Uuid;

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
                        let _version = read_var_int(&mut stream).unwrap();

                        let address_len = read_var_int(&mut stream).unwrap() as usize;
                        let _address = read_string(&mut stream, address_len);
                        let _port = u16::from_be_bytes(read_bytes(&mut stream, 2).try_into().unwrap());

                        //println!("{address}:{port}");

                        let nextstate = read_var_int(&mut stream).unwrap();

                        match nextstate {
                            1 => {

                                let status = ServerStatus {
                                    version: ServerVersion { name: "1.19.3".to_string(), protocol: 761 },
                                    players: ServerPlayerCount { max: 20, online: players.len() as i32, sample: populate_players(&players) },
                                    description: ServerDescription { text: "hello".to_string() },
                                    enforcesSecureChat: false,
                                };

                                let motd = serde_json::to_string(&status).unwrap();
                                println!("{}", motd);

                                let mut buffer: Vec<u8> = vec![];
                                write_string(&mut buffer, motd);
                                flush(&mut stream, &mut buffer, 0);
                                buffer.clear();

                                let mut test: [u8; 8] = Default::default();
                                stream.read(&mut test).expect("somethign");
                                
                                //println!("thing: {}", u64::from_be_bytes(test));
                                buffer.write(&test).expect("somethign");
                                flush(&mut stream, &mut buffer, 1);
                            },
                            2 => {
                                let _connectionid = read_var_int(&mut stream).unwrap();
                                let _identifier = read_var_int(&mut stream).unwrap();
                                
                                let user_len = read_var_int(&mut stream).unwrap();
                                let username = read_string(&mut stream, user_len as usize);

                                let mut uuid: u128 = 0;
                                let has_guid = read_bytes(&mut stream, 1);
                                if has_guid[0] == 1 {
                                    let array = read_bytes(&mut stream, 16);
                                    uuid = u128::from_be_bytes(array.as_slice().try_into().unwrap());
                                }

                                if has_player(&players, username.clone()) {
                                    let mut buffer: Vec<u8> = vec![];
                                    write_string(&mut buffer, "\"You are already connected to this server!\"".to_string());
                                    flush(&mut stream, &mut buffer, 0);
                                    continue;
                                }

                                println!("{} {} {}", username, uuid, has_guid[0]);
                                
                                let mut buffer: Vec<u8> = vec![];
                                buffer.extend(&uuid.to_be_bytes());
                                write_string(&mut buffer, username.clone());
                                write_var_int(&mut buffer, 0);
                                flush(&mut stream, &mut buffer, 2);

                                let plr = Player { x: 0.0, y: 0.0, z: 0.0, username, uuid: Uuid::from_bytes(uuid.to_be_bytes()).to_string(), stream };

                                players.push(plr);
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

fn has_player(players: &Vec<Player>, username: String) -> bool {
    for plr in players.into_iter() {
        if plr.username == username { return true };
    }
    return false;
}

fn populate_players(players: &Vec<Player>) -> Vec<ServerPlayer> {
    let mut plrs: Vec<ServerPlayer> = vec![];

    if players.len() == 0 { return plrs };

    for plr in players {
        let player = ServerPlayer{ name: plr.username.clone(), id: plr.uuid.clone() };
        plrs.push(player);
    }

    return plrs;
}