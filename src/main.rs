mod utils;
mod structs;
mod packet_out;
mod packets;

use packet_out::*;
use packets::*;
use tokio::sync::mpsc::{Sender, channel};
use utils::*;
use structs::*;

#[macro_use] extern crate serde_derive;

use std::collections::HashMap;
use std::io::Read;
use std::task::Poll;
use std::time::Duration;
use std::{fs, sync::Arc};
use std::path::Path;

use tokio::{io::AsyncReadExt, sync::Mutex};

use tokio::net::{TcpListener, TcpStream};
use utils::read_var_int;

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new("players").is_dir() {
        fs::create_dir("players").unwrap();
    }
    

    let entity_id = 0;
    let players: HashMap<String, (Player, Sender<Packet>)> = HashMap::new();

    let players_accessor = Arc::new(Mutex::new(players));
    let entity_accessor = Arc::new(Mutex::new(entity_id));

    let listener = TcpListener::bind("0.0.0.0:51413").await?;
    let (broadcast_send, mut broadcast_recv) = channel::<Packet>(1024);

    let accessor = players_accessor.clone();
    tokio::spawn(async move {
        loop {
            while let Some(msg) = broadcast_recv.recv().await {
                let players = accessor.lock().await;
                for (player, stream) in players.values().into_iter() {
                    if player.entity_id == msg.entity_id { continue };
                    stream.send(msg.clone()).await.unwrap();
                }
            }
        }
    });

    loop {
        let (mut stream, _) = listener.accept().await?;
        let broadcast_send = broadcast_send.clone();
        let (client_send, mut client_recv) = channel(1024);

        let players_accessor = players_accessor.clone();
        let entity_accessor = entity_accessor.clone();

        tokio::spawn(async move {
            let length = read_var_int(&mut stream).await;

            if length.is_none() { return }

            let packet_id = read_var_int(&mut stream).await.unwrap();

            match packet_id {
                0 => {
                    let mut buffer: Vec<u8> = vec![];
                    let _version = read_var_int(&mut stream).await.unwrap();
                    let _address = read_string(&mut stream).await.unwrap();
                    let _port = stream.read_u16().await.unwrap();
                    let nextstate = read_var_int(&mut stream).await.unwrap();

                    match nextstate {
                        1 => {
                            let players = players_accessor.lock().await;
                            //send status
                            let status = ServerStatus {
                                version: StatusVersion { name: "1.19.3".to_string(), protocol: 761 },
                                players: StatusPlayerCount { max: 20, online: players.len() as i32, sample: populate_players(players.values()) },
                                description: StatusDescription { text: "hello aa".to_string() },
                                enforces_secure_chat: false,
                            };
                            drop(players);

                            write_string(&mut buffer, serde_json::to_string(&status).unwrap());
                            flush(&mut stream, &mut buffer, CStatusPacketid::Status as i32).await;
                            
                            //check ping
                            let handle = stream.read_i64().await;
                            if handle.is_err() { return }
                            let handle = handle.unwrap();

                            buffer.extend(handle.to_be_bytes());
                            
                            flush(&mut stream, &mut buffer, CStatusPacketid::Ping as i32).await;
                            return;
                        },
                        2 => {
                            let _connectionid = read_var_int(&mut stream).await.unwrap();
                            let _identifier = read_var_int(&mut stream).await.unwrap();

                            let username = read_string(&mut stream).await.unwrap();

                            let mut uuid: u128 = 0;
                            let has_guid = read_next(&mut stream).await.unwrap();
                            if has_guid == 1 {
                                let array = read_bytes(&mut stream, 16).await.unwrap();
                                uuid = u128::from_be_bytes(array.as_slice().try_into().unwrap());
                            }

                            let players = players_accessor.lock().await;
                            //allow user if not playing
                            if players.contains_key(&username) {
                                let mut buffer: Vec<u8> = vec![];
                                write_string_chat(&mut buffer, &"You are already connected to this server!".to_string());
                                flush(&mut stream, &mut buffer, CLoginPacketid::Kick as i32).await;
                                return;
                            }
                            drop(players);

                            let mut entity_id = entity_accessor.lock().await;
                            *entity_id += 1;

                            //has player played before? otherwise create new player
                            let pathname: String = format!("./players/{}.bin", username);
                            let path: &Path = Path::new(&pathname);
                            
                            let mut player: Player = Player {
                                username: username.clone(), 
                                uuid: u128::from_be_bytes(uuid.to_be_bytes()),
                                entity_id: *entity_id,

                                x: 8.0, 
                                y: 90.0, 
                                z: 8.0,

                                yaw: 0.0,
                                pitch: 0.0,

                                gamemode: 0
                            };
                            drop(entity_id);
                            
                            if path.exists() {
                                let mut file = fs::File::open(path).unwrap();

                                file.read_to_end(&mut buffer).unwrap();
                                player.x = f64::from_be_bytes(buffer.drain(0..8).as_slice().try_into().unwrap());
                                player.y = f64::from_be_bytes(buffer.drain(0..8).as_slice().try_into().unwrap());
                                player.z = f64::from_be_bytes(buffer.drain(0..8).as_slice().try_into().unwrap());

                                player.yaw = f32::from_be_bytes(buffer.drain(0..4).as_slice().try_into().unwrap());
                                player.pitch = f32::from_be_bytes(buffer.drain(0..4).as_slice().try_into().unwrap());
                                
                                player.gamemode = u8::from_be_bytes(buffer.drain(0..1).as_slice().try_into().unwrap());
                                
                                buffer.clear()
                            } else {
                                save_player(&player);
                            }

                            let mut buffer: Vec<u8> = vec![];

                            //player isn't currently playing, let him in!
                            buffer.extend(&uuid.to_be_bytes());
                            write_string(&mut buffer, username.clone());
                            write_var_int(&mut buffer, 0);
                            flush(&mut stream, &mut buffer, CLoginPacketid::Success as i32).await;

                            //send play 
                            let loginplay = PacketOutLoginPlay::new(player.entity_id);
                            PacketOutLoginPlay::serialize(&loginplay, &mut buffer);
                            flush(&mut stream, &mut buffer, CPlayPacketid::LoginPlay as i32).await;
                            
                            //update tab
                            buffer.push(9);
                            write_var_int(&mut buffer, 1);
                            buffer.extend(uuid.to_be_bytes());
                            write_string(&mut buffer, username);
                            write_var_int(&mut buffer, 0);
                            buffer.push(1);
                            broadcast(&broadcast_send, buffer.clone(), CPlayPacketid::PlayerInfo as i32, player.entity_id).await;
                            flush(&mut stream, &mut buffer, CPlayPacketid::PlayerInfo as i32).await;

                            //send default spawn
                            let flytis: f32 = 0.0;
                            write_position(&mut buffer, 8, 64, 8);
                            buffer.extend(flytis.to_be_bytes());
                            flush(&mut stream, &mut buffer, CPlayPacketid::SetDefaultSpawn as i32).await;

                            //send one chunk
                            let mut chunk_data = vec![];
                            let mut handle = fs::File::open("chunk.bin").unwrap();
                            handle.read_to_end(&mut chunk_data).unwrap();
                            drop(handle);

                            for x in 0..7 {
                                for y in 0..7 {
                                    let chunk_x: i32 = x - 3;
                                    let chunk_z: i32 = y - 3;
                                    buffer.extend(chunk_x.to_be_bytes());
                                    buffer.extend(chunk_z.to_be_bytes());
                                    buffer.extend(chunk_data.clone());
                                    flush(&mut stream, &mut buffer, CPlayPacketid::LoadChunk as i32).await;
                                }
                            }

                            let sync_player = SynchronizePlayerPosition::new(player.x, player.y, player.z, player.yaw, player.pitch, 0, false);
                            SynchronizePlayerPosition::serialize(&sync_player, &mut buffer);
                            flush(&mut stream, &mut buffer, CPlayPacketid::PlayerPos as i32).await;

                            //push player to playerlist
                            let mut players = players_accessor.lock().await;
                            for (p, sender) in players.values().into_iter() {
                                buffer.push(9);
                                write_var_int(&mut buffer, 1);
                                buffer.extend(p.uuid.to_be_bytes());
                                write_string(&mut buffer, p.username.clone());
                                write_var_int(&mut buffer, 0);
                                buffer.push(1);
                                flush(&mut stream, &mut buffer, CPlayPacketid::PlayerInfo as i32).await;
                                
                                write_var_int(&mut buffer, p.entity_id);
                                buffer.extend(p.uuid.to_be_bytes());
                                buffer.extend(p.x.to_be_bytes());
                                buffer.extend(p.y.to_be_bytes());
                                buffer.extend(p.z.to_be_bytes());
                                buffer.extend([0, 0]);
                                flush(&mut stream, &mut buffer, CPlayPacketid::SpawnPlayer as i32).await;
                            }

                            write_var_int(&mut buffer, player.entity_id);
                            buffer.extend(player.uuid.to_be_bytes());
                            buffer.extend(player.x.to_be_bytes());
                            buffer.extend(player.y.to_be_bytes());
                            buffer.extend(player.z.to_be_bytes());
                            buffer.extend([0, 0]);
                            broadcast(&broadcast_send, buffer, CPlayPacketid::SpawnPlayer as i32, player.entity_id).await;

                            let mut buffer = vec![];
                            write_string_chat(&mut buffer, &format!("{} joined the game", player.username));
                            buffer.push(0);
                            broadcast(&broadcast_send, buffer, CPlayPacketid::Chat as i32, player.entity_id).await;

                            players.insert(player.username.clone(), (player.clone(), client_send.clone()));
                            drop(players);

                            let waker = futures::task::noop_waker();
                            let mut cx = std::task::Context::from_waker(&waker);

                            let mut tick_counter = 0;

                            let broadcast_send = broadcast_send.clone();
                            loop {
                                while let Poll::Ready(Some(mut packet)) = client_recv.poll_recv(&mut cx){
                                    flush(&mut stream, &mut packet.data, packet.packet_id).await;
                                }

                                match stream.poll_read_ready(&mut cx) {
                                    Poll::Ready(Ok(())) => {
                                        let abc = handle_stream(&mut stream, &mut player, &broadcast_send).await;
                                        if abc.is_none() {

                                            let mut players = players_accessor.lock().await;
                                            players.remove(&player.username);
                                            println!("player {} disconnected, total: {}", player.username, players.len());
                                            drop(players);

                                            let mut buffer = vec![];
                                            write_string_chat(&mut buffer, &format!("{} left the game", &player.username));
                                            buffer.push(0);
                                            broadcast(&broadcast_send, buffer, CPlayPacketid::Chat as i32, player.entity_id).await;

                                            let mut buffer = vec![];
                                            write_var_int(&mut buffer, 1);
                                            buffer.extend(player.uuid.to_be_bytes());
                                            broadcast(&broadcast_send, buffer, CPlayPacketid::PlayerLeft as i32, player.entity_id).await;

                                            let mut buffer = vec![];
                                            write_var_int(&mut buffer, 1);
                                            write_var_int(&mut buffer, player.entity_id);
                                            broadcast(&broadcast_send, buffer, CPlayPacketid::RemoveEntities as i32, player.entity_id).await;

                                            save_player(&player);
                                            drop(stream);
                                            break; 
                                        }
                                    },
                                    Poll::Ready(Err(e)) => {
                                        println!("error {}", e);
                                    }
                                    Poll::Pending => {},
                                }

                                tick_counter += 1;
                                if tick_counter > 5 {
                                    tick_counter = 0;

                                    let lastkeepalive: i64 = 69;

                                    let mut buffer: Vec<u8> = vec![];
                                    buffer.extend(lastkeepalive.to_be_bytes());
                                    flush(&mut stream, &mut buffer, CPlayPacketid::KeepAlive as i32).await;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        });
    }
}

async fn handle_stream(stream: &mut TcpStream, player: &mut Player, broadcast_send: &Sender<Packet>) -> Option<usize> {
    let size = read_var_int(stream).await;

    if size.is_none() { return None }

    let mut data = read_bytes(stream, size.unwrap() as usize).await.unwrap();
    let packet_id = read_var_int_buf(&mut data).unwrap();

    return handle_packet_id(packet_id as i32, &mut data, stream, player, broadcast_send).await;
}

async fn handle_packet_id(packet_id: i32, data: &mut Vec<u8>, stream: &mut TcpStream, player: &mut Player, broadcast_send: &Sender<Packet>) -> Option<usize> {
    match packet_id {
        4 => {
            let message = read_string_buf(data);

            let args = message.split(" ").collect::<Vec<&str>>();
            
            if args.len() < 1 { return None }

            match args[0] {
                "give" => {
                    if args.len() < 3 {
                        send_chat_message(stream, "incorrect usage".to_string()).await;
                        return None
                    }

                    let id: Result<u16, _> = args[1].parse();
                    let count: Result<u8, _> = args[2].parse();

                    if id.is_err() || count.is_err() {
                        send_chat_message(stream, "invalid numbers".to_string()).await;
                        return None
                    }

                    let realid = id.unwrap();

                    let mut buffer = vec![];
                    buffer.push(0);
                    write_var_int(&mut buffer, 0);
                    write_var_int(&mut buffer, 37);
                    for _num in 0..36 {
                        buffer.push(0);
                    }
                    buffer.push(1);
                    write_var_int(&mut buffer, realid as i32);
                    buffer.extend(&[count.unwrap(),0]);
                    buffer.push(0);
                    flush(stream, &mut buffer, CPlayPacketid::ContainerContent as i32).await;
                }
                "gm" => {
                    if args.len() < 2 {
                        send_chat_message(stream, "incorrect usage".to_string()).await;
                        return None
                    }

                    let gamemode_handle: Result<u8, _> = args[1].parse();
                    
                    if gamemode_handle.is_err() {
                        send_chat_message(stream, "invalid gamemode number".to_string()).await;
                        return None
                    }

                    let gamemode: f32 = gamemode_handle.unwrap() as f32;

                    if gamemode > 3.0 {
                        send_chat_message(stream, "invalid gamemode number".to_string()).await;
                        return None
                    }

                    let mut buffer = vec![];
                    buffer.push(3);
                    buffer.extend(gamemode.to_be_bytes());
                    flush(stream, &mut buffer, CPlayPacketid::GameEvent as i32).await;

                    send_actionbar(stream, format!("gamemode updated to {}", gamemode)).await;
                }
                "respawn" => {
                    player.x = 8.0;
                    player.y = 80.0;
                    player.z = 8.0;

                    player.yaw = player.yaw.rem_euclid(360.0);

                    let angle_x: u8 = (player.yaw * 0.71) as u8;
                    let angle_y: i8 = (player.pitch * 0.71) as i8;

                    let mut buffer = vec![];
                    let sync_player = SynchronizePlayerPosition::from_player(&player, 0, false);
                    SynchronizePlayerPosition::serialize(&sync_player, &mut buffer);
                    flush(stream, &mut buffer, CPlayPacketid::PlayerPos as i32).await;

                    write_var_int(&mut buffer, player.entity_id);
                    buffer.extend(player.x.to_be_bytes());
                    buffer.extend(player.y.to_be_bytes());
                    buffer.extend(player.z.to_be_bytes());
                    buffer.push(angle_x); //yaw
                    buffer.push(angle_y as u8); //pitch
                    buffer.push(1);
                    broadcast(&broadcast_send, buffer, CPlayPacketid::PlayerTeleport as i32, player.entity_id).await;
                }
                _ => { 
                    send_chat_message(stream, "command not found".to_string()).await;
                    return None
                 }
            }
        }
        5 => {
            let message = read_string_buf(data);
    
            println!("{}: {}", player.username.clone(), message);
            let chatmsg = format!("{}: {}", player.username.clone(), message);
    
            let mut buffer = vec![];
            write_string_chat(&mut buffer, &chatmsg);
            buffer.push(0);
    
            broadcast(&broadcast_send, buffer.clone(), CPlayPacketid::Chat as i32, player.entity_id).await;
            flush(stream, &mut buffer, CPlayPacketid::Chat as i32).await;
        }
        7 => {
            let _locale = read_string_buf(data);
            let _view_distance: i8 = data.remove(0) as i8;
            let _chat_mode = read_var_int_buf(data).unwrap();
            let _char_colors = data.remove(0);
            let skin_parts = data.remove(0);
            let _main_hand = read_var_int_buf(data).unwrap();
            let _text_filtering = data.remove(0);
            let _allow_server_listening = data.remove(0);

            let mut buffer: Vec<u8> = vec![];
            write_var_int(&mut buffer, player.entity_id as i32);

            buffer.push(17);
            write_var_int(&mut buffer, 0);
            buffer.push(skin_parts);

            buffer.push(0xff);
            
            broadcast(&broadcast_send, buffer.clone(), CPlayPacketid::EntityMetadata as i32, player.entity_id).await;
            flush(stream, &mut buffer, CPlayPacketid::EntityMetadata as i32).await;
        }
        12 => {
            //let channel = read_string_buf(&mut data);
            //println!("channel: {}, data: {:?}", channel, &data);
            return Some(0);
        }
        17 => {
            return Some(0);
        }
        19 => {
            let prev_player = player.clone();
            player.x = f64::from_be_bytes(data.drain(0..8).as_slice().try_into().unwrap());
            player.y = f64::from_be_bytes(data.drain(0..8).as_slice().try_into().unwrap());
            player.z = f64::from_be_bytes(data.drain(0..8).as_slice().try_into().unwrap());
            data.remove(0);

            let delta_x: i16 = (((player.x * 32.0) - (prev_player.x * 32.0)) * 128.0) as i16;
            let delta_y: i16 = (((player.y * 32.0) - (prev_player.y * 32.0)) * 128.0) as i16;
            let delta_z: i16 = (((player.z * 32.0) - (prev_player.z * 32.0)) * 128.0) as i16;

            let mut buffer = vec![];
            write_var_int(&mut buffer, player.entity_id);
            buffer.extend(delta_x.to_be_bytes());
            buffer.extend(delta_y.to_be_bytes());
            buffer.extend(delta_z.to_be_bytes());
            buffer.extend([1]);
            broadcast(&broadcast_send, buffer, CPlayPacketid::EntityUpdatePos as i32, player.entity_id).await;
        }  
        20 => {
            let prev_player = player.clone();
            player.x = f64::from_be_bytes(data.drain(0..8).as_slice().try_into().unwrap());
            player.y = f64::from_be_bytes(data.drain(0..8).as_slice().try_into().unwrap());
            player.z = f64::from_be_bytes(data.drain(0..8).as_slice().try_into().unwrap());
            
            player.yaw = f32::from_be_bytes(data.drain(0..4).as_slice().try_into().unwrap());
            player.pitch = f32::from_be_bytes(data.drain(0..4).as_slice().try_into().unwrap());

            data.remove(0);

            let delta_x: i16 = (((player.x * 32.0) - (prev_player.x * 32.0)) * 128.0) as i16;
            let delta_y: i16 = (((player.y * 32.0) - (prev_player.y * 32.0)) * 128.0) as i16;
            let delta_z: i16 = (((player.z * 32.0) - (prev_player.z * 32.0)) * 128.0) as i16;

            player.yaw = player.yaw.rem_euclid(360.0);

            let angle_x: u8 = (player.yaw * 0.71) as u8;
            let angle_y: i8 = (player.pitch * 0.71) as i8;

            let mut buffer = vec![];
            write_var_int(&mut buffer, player.entity_id);
            buffer.extend(delta_x.to_be_bytes());
            buffer.extend(delta_y.to_be_bytes());
            buffer.extend(delta_z.to_be_bytes());
            buffer.extend([angle_x, angle_y as u8]);
            buffer.push(1);
            broadcast(&broadcast_send, buffer, CPlayPacketid::EntityUpdatePosAndRot as i32, player.entity_id).await;

            let mut buffer = vec![];
            write_var_int(&mut buffer, player.entity_id);
            buffer.push(angle_x);
            broadcast(&broadcast_send, buffer, CPlayPacketid::HeadRot as i32, player.entity_id).await;
        }
        21 => {
            player.yaw = f32::from_be_bytes(data.drain(0..4).as_slice().try_into().unwrap());
            player.pitch = f32::from_be_bytes(data.drain(0..4).as_slice().try_into().unwrap());
            data.remove(0);

            player.yaw = player.yaw.rem_euclid(360.0);

            let angle_x: u8 = (player.yaw * 0.71) as u8;
            let angle_y: i8 = (player.pitch * 0.71) as i8;

            let mut buffer = vec![];
            write_var_int(&mut buffer, player.entity_id);
            buffer.push(angle_x);
            buffer.push(angle_y as u8);
            buffer.push(1);
            broadcast(&broadcast_send, buffer, CPlayPacketid::EntityUpdateRot as i32, player.entity_id).await;

            let mut buffer = vec![];
            write_var_int(&mut buffer, player.entity_id);
            buffer.push(angle_x);
            broadcast(&broadcast_send, buffer, CPlayPacketid::HeadRot as i32, player.entity_id).await;
            //player rotation
        }
        28 => {
            let status = read_var_int_buf(data).unwrap();
            let (x,y,z) = read_position(data);
            data.remove(0); // face
            let _seq = read_var_int_buf(data);

            
            match status {
                2 => {
                    //send_chat_message(&mut stream, format!("{:?}", position));
                    let mut buffer = vec![];
                    write_position(&mut buffer, x, y, z);
                    write_var_int(&mut buffer, 0);
                    broadcast(&broadcast_send, buffer.clone(), CPlayPacketid::BlockUpdate as i32, player.entity_id).await;
                    flush(stream, &mut buffer, CPlayPacketid::BlockUpdate as i32).await;
                }
                _ => {}
            }
        }
        29 => {
            let entity_id = read_var_int_buf(data).unwrap();
            let action = read_var_int_buf(data).unwrap();
            let _jump_boost = read_var_int_buf(data).unwrap();

            let mut buffer: Vec<u8> = vec![];
            write_var_int(&mut buffer, entity_id as i32);

            if (action & 1) == 0 {
                buffer.push(0);
                write_var_int(&mut buffer, 0);
                buffer.push(2);

                buffer.push(6);
                write_var_int(&mut buffer, 19);
                write_var_int(&mut buffer, 5);

                buffer.push(0xff);
                broadcast(&broadcast_send, buffer, CPlayPacketid::EntityMetadata as i32, player.entity_id).await;
            }else if (action & 0) == 0 {
                buffer.push(0);
                write_var_int(&mut buffer, 0);
                buffer.push(0);

                buffer.push(6);
                write_var_int(&mut buffer, 19);
                write_var_int(&mut buffer, 0);

                buffer.push(0xff);
                broadcast(&broadcast_send, buffer, CPlayPacketid::EntityMetadata as i32, player.entity_id).await;
            }
        }
        47 => {
            let mut buffer = vec![];
            write_var_int(&mut buffer, player.entity_id);
            buffer.extend([0]);
            broadcast(&broadcast_send, buffer, CPlayPacketid::SwingArm as i32, player.entity_id).await;
        }
        _ => {
            println!("packet {} [0x{:X}] {:?}", packet_id, packet_id, &data);
        }
    }

    return Some(data.len());
}