mod utils;
mod structs;
mod packet_out;
mod packets;

use packet_out::*;
use packets::*;
use tokio::io::AsyncWriteExt;
use utils::*;
use structs::*;

#[macro_use] extern crate serde_derive;

use std::io::Read;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{fs, sync::Arc};
use std::path::Path;

use tokio::{io::AsyncReadExt, sync::Mutex};

use tokio::net::{TcpListener, TcpStream};
use utils::read_var_int;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new("players").is_dir() {
        fs::create_dir("players").unwrap();
    }

    let entity_id = 0;

    let players: Vec<Player> = vec![];
    let streams: Vec<TcpStream> = vec![];

    let players_accessor = Arc::new(Mutex::new(players));
    let streams_accessor = Arc::new(Mutex::new(streams));
    let entity_accessor = Arc::new(Mutex::new(entity_id));

    let listener = TcpListener::bind("0.0.0.0:51413").await?;

    let (tx, rx) = channel();

    let abc = streams_accessor.clone();
    tokio::spawn(async move {
        loop {
            let broadcast: Broadcast = rx.recv().unwrap();

            let mut streams = abc.lock().await;
            let mut_iter = streams.iter_mut();

            for stream in mut_iter {
                if stream.peer_addr().unwrap().to_string() == broadcast.stream_name { continue; }
                
                flush(stream, &mut broadcast.data.clone(), broadcast.packet_id).await;
            }
        }
    });

    loop {
        let (mut stream, _) = listener.accept().await?;
        let tx = tx.clone();

        let players_accessor = players_accessor.clone();
        let entity_accessor = entity_accessor.clone();
        let streams_accessor = streams_accessor.clone();

        tokio::spawn(async move {
            let _length = read_var_int(&mut stream).await;
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
                                players: StatusPlayerCount { max: 20, online: players.len() as i32, sample: populate_players(&players) },
                                description: StatusDescription { text: "hello aa".to_string() },
                                enforces_secure_chat: false,
                            };
                            drop(players);

                            write_string(&mut buffer, serde_json::to_string(&status).unwrap());
                            flush(&mut stream, &mut buffer, CStatusPacketid::Status as i32).await;
                            
                            //check ping
                            let handle = stream.read_i64().await.unwrap();
                            buffer.extend(handle.to_be_bytes());
                            
                            flush(&mut stream, &mut buffer, CStatusPacketid::Ping as i32).await;
                            drop(stream);
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
                            if has_player(&players, &username) {
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
                            broadcast(&tx, buffer.clone(), CPlayPacketid::PlayerInfo as i32, &stream);
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
                            for p in players.iter() {
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
                            players.push(player.clone());
                            drop(players);
                            
                            write_var_int(&mut buffer, player.entity_id);
                            buffer.extend(player.uuid.to_be_bytes());
                            buffer.extend(player.x.to_be_bytes());
                            buffer.extend(player.y.to_be_bytes());
                            buffer.extend(player.z.to_be_bytes());
                            buffer.extend([0, 0]);
                            broadcast(&tx, buffer, CPlayPacketid::SpawnPlayer as i32, &stream);

                            let mut buffer = vec![];
                            write_string_chat(&mut buffer, &format!("{} joined the game", player.username));
                            buffer.push(0);
                            broadcast(&tx, buffer, CPlayPacketid::Chat as i32, &stream);

                            let mut streams = streams_accessor.lock().await;
                            streams.push(stream);
                            drop(streams);

                            loop {
                                tokio::time::sleep(Duration::from_millis(10)).await;
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
    /*

    let listener = TcpListener::bind("0.0.0.0:51413")?;
    let players: Vec<Player> = vec![];
    let streams: Vec<TcpStream> = vec![];
    let players_accessor = Arc::new(Mutex::new(players));
    let streams_accessor = Arc::new(Mutex::new(streams));

    let mut entity_id = 0;

    let (tx, rx) = channel();

    let abc = streams_accessor.clone();
    thread::spawn(move || {
        loop {
            let broadcast: Broadcast = rx.recv().unwrap();

            for stream in abc.lock().unwrap().iter_mut() {
                if stream.peer_addr().unwrap().to_string() == broadcast.stream_name { continue; }
                
                flush(stream, &mut broadcast.data.clone(), broadcast.packet_id);
            }
        }
    });

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
                                
                                //check ping
                                let handle = read_bytes(&mut stream, 8);
                                buffer.extend(&handle);
                                
                                flush(&mut stream, &mut buffer, CStatusPacketid::Ping as i32);
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
                                if has_player(&players, &username) {
                                    let mut buffer: Vec<u8> = vec![];
                                    write_string_chat(&mut buffer, &"You are already connected to this server!".to_string());
                                    flush(&mut stream, &mut buffer, CLoginPacketid::Kick as i32);
                                    continue;
                                }
                                drop(players);

                                entity_id += 1;

                                //has player played before? otherwise create new player
                                let pathname: String = format!("./players/{}.bin", username);
                                let path: &Path = Path::new(&pathname);
                                
                                let mut player: Player = Player {
                                    username: username.clone(), 
                                    uuid: u128::from_be_bytes(uuid.to_be_bytes()),
                                    entity_id,

                                    x: 8.0, 
                                    y: 90.0, 
                                    z: 8.0,

                                    yaw: 0.0,
                                    pitch: 0.0,

                                    gamemode: 0
                                };

                                if path.exists() {
                                    let mut file: File = fs::File::open(path).unwrap();

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

                                let mut streams = streams_accessor.lock().unwrap();
                                streams.push(stream.try_clone().unwrap());
                                drop(streams);

                                //player isn't currently playing, let him in!
                                buffer.extend(&uuid.to_be_bytes());
                                write_string(&mut buffer, username.clone());
                                write_var_int(&mut buffer, 0);
                                flush(&mut stream, &mut buffer, CLoginPacketid::Success as i32);

                                //send play 
                                let loginplay = PacketOutLoginPlay::new(entity_id);
                                PacketOutLoginPlay::serialize(&loginplay, &mut buffer);
                                flush(&mut stream, &mut buffer, CPlayPacketid::LoginPlay as i32);
                                
                                //update tab
                                buffer.write(&[9]).unwrap();
                                write_var_int(&mut buffer, 1);
                                buffer.extend(uuid.to_be_bytes());
                                write_string(&mut buffer, username);
                                write_var_int(&mut buffer, 0);
                                buffer.write(&[1]).unwrap();
                                broadcast(&tx, buffer.clone(), CPlayPacketid::PlayerInfo as i32, &stream);
                                flush(&mut stream, &mut buffer, CPlayPacketid::PlayerInfo as i32);

                                //send default spawn
                                let flytis: f32 = 0.0;
                                write_position(&mut buffer, 8, 64, 8);
                                buffer.write(&flytis.to_be_bytes()).unwrap();
                                flush(&mut stream, &mut buffer, CPlayPacketid::SetDefaultSpawn as i32);

                                //teleport player

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
                                        flush(&mut stream, &mut buffer, CPlayPacketid::LoadChunk as i32);
                                    }
                                }

                                let sync_player = SynchronizePlayerPosition::new(player.x, player.y, player.z, player.yaw, player.pitch, 0, false);
                                SynchronizePlayerPosition::serialize(&sync_player, &mut buffer);
                                flush(&mut stream, &mut buffer, CPlayPacketid::PlayerPos as i32);

                                //push player to playerlist
                                let mut players = players_accessor.lock().unwrap();
                                for p in players.iter() {
                                    buffer.write(&[9]).unwrap();
                                    write_var_int(&mut buffer, 1);
                                    buffer.extend(p.uuid.to_be_bytes());
                                    write_string(&mut buffer, p.username.clone());
                                    write_var_int(&mut buffer, 0);
                                    buffer.write(&[1]).unwrap();
                                    flush(&mut stream, &mut buffer, CPlayPacketid::PlayerInfo as i32);
                                    
                                    write_var_int(&mut buffer, p.entity_id);
                                    buffer.extend(p.uuid.to_be_bytes());
                                    buffer.extend(p.x.to_be_bytes());
                                    buffer.extend(p.y.to_be_bytes());
                                    buffer.extend(p.z.to_be_bytes());
                                    buffer.extend([0, 0]);
                                    flush(&mut stream, &mut buffer, CPlayPacketid::SpawnPlayer as i32);
                                }
                                players.push(player.clone());
                                drop(players);
                                
                                write_var_int(&mut buffer, entity_id);
                                buffer.extend(player.uuid.to_be_bytes());
                                buffer.extend(player.x.to_be_bytes());
                                buffer.extend(player.y.to_be_bytes());
                                buffer.extend(player.z.to_be_bytes());
                                buffer.extend([0, 0]);
                                broadcast(&tx, buffer, CPlayPacketid::SpawnPlayer as i32, &stream);

                                let mut buffer = vec![];
                                write_string_chat(&mut buffer, &format!("{} joined the game", player.username));
                                buffer.push(0);
                                broadcast(&tx, buffer, CPlayPacketid::Chat as i32, &stream);

                                //create new thread, better solution required
                                let playersmutex = players_accessor.clone();
                                let tx = tx.clone();
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
                                            disconnect_player(&mut players, &player.username);
                                            println!("player {} disconnected, total: {}", player.username, players.len());

                                            let mut buffer = vec![];
                                            write_string_chat(&mut buffer, &format!("{} left the game", &player.username));
                                            buffer.write(&[0]).unwrap();
                                            broadcast(&tx, buffer, CPlayPacketid::Chat as i32, &stream);

                                            let mut buffer = vec![];
                                            write_var_int(&mut buffer, 1);
                                            buffer.extend(player.uuid.to_be_bytes());
                                            broadcast(&tx, buffer, CPlayPacketid::PlayerLeft as i32, &stream);

                                            let mut buffer = vec![];
                                            write_var_int(&mut buffer, 1);
                                            write_var_int(&mut buffer, player.entity_id);
                                            broadcast(&tx, buffer, CPlayPacketid::RemoveEntities as i32, &stream);

                                            save_player(&player);
                                            break; 
                                        }

                                        if tick > 20 {
                                            //println!("keep alive");
                                            tick = 0;

                                            lastkeepalive = 69;

                                            let mut buffer: Vec<u8> = vec![];
                                            buffer.extend(lastkeepalive.to_be_bytes());
                                            flush(&mut stream, &mut buffer, CPlayPacketid::KeepAlive as i32);

                                            player.yaw = player.yaw.rem_euclid(360.0);

                                            let angle_x: u8 = (player.yaw * 0.71) as u8;
                                            let angle_y: i8 = (player.pitch * 0.71) as i8;
                                            
                                            write_var_int(&mut buffer, player.entity_id);
                                            buffer.extend(player.x.to_be_bytes());
                                            buffer.extend(player.y.to_be_bytes());
                                            buffer.extend(player.z.to_be_bytes());
                                            buffer.push(angle_x); //yaw
                                            buffer.push(angle_y as u8); //pitch
                                            buffer.push(1);
                                            broadcast(&tx, buffer, CPlayPacketid::PlayerTeleport as i32, &stream);
                                        }

                                        let mut data: Vec<u8> = vec![0; size.try_into().unwrap()];
                                        stream.read(&mut data).unwrap();

                                        let packetid = read_var_int_buf(&mut data).unwrap();
                                        let mut handled = false;
                                        match packetid {
                                            4 => {
                                                let message = read_string_buf(&mut data);

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

                                                        let id: Result<u16, _> = args[1].parse();
                                                        let count: Result<u8, _> = args[2].parse();

                                                        if id.is_err() || count.is_err() {
                                                            send_chat_message(&mut stream, "invalid numbers".to_string());
                                                            continue;
                                                        }

                                                        let realid = id.unwrap();

                                                        let mut buffer = vec![];
                                                        buffer.write(&[0]).unwrap();
                                                        write_var_int(&mut buffer, 0);
                                                        write_var_int(&mut buffer, 37);
                                                        for _num in 0..36 {
                                                            buffer.write(&[0]).unwrap();
                                                        }
                                                        buffer.push(1);
                                                        write_var_int(&mut buffer, realid as i32);
                                                        buffer.write(&[count.unwrap(),0]).unwrap();
                                                        buffer.write(&[0]).unwrap();
                                                        flush(&mut stream, &mut buffer, CPlayPacketid::ContainerContent as i32);
                                                    }
                                                    "gm" => {
                                                        if args.len() < 2 {
                                                            send_chat_message(&mut stream, "incorrect usage".to_string());
                                                            continue;
                                                        }

                                                        let gamemode_handle: Result<u8, _> = args[1].parse();
                                                        
                                                        if gamemode_handle.is_err() {
                                                            send_chat_message(&mut stream, "invalid gamemode number".to_string());
                                                            continue;
                                                        }

                                                        let gamemode: f32 = gamemode_handle.unwrap() as f32;

                                                        if gamemode > 3.0 {
                                                            send_chat_message(&mut stream, "invalid gamemode number".to_string());
                                                            continue;
                                                        }

                                                        let mut buffer = vec![];
                                                        buffer.write(&[3]).unwrap();
                                                        buffer.extend(gamemode.to_be_bytes());
                                                        flush(&mut stream, &mut buffer, CPlayPacketid::GameEvent as i32);

                                                        send_actionbar(&mut stream, format!("gamemode updated to {}", gamemode));
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
                                                        flush(&mut stream, &mut buffer, CPlayPacketid::PlayerPos as i32);

                                                        write_var_int(&mut buffer, player.entity_id);
                                                        buffer.extend(player.x.to_be_bytes());
                                                        buffer.extend(player.y.to_be_bytes());
                                                        buffer.extend(player.z.to_be_bytes());
                                                        buffer.push(angle_x); //yaw
                                                        buffer.push(angle_y as u8); //pitch
                                                        buffer.push(1);
                                                        broadcast(&tx, buffer, CPlayPacketid::PlayerTeleport as i32, &stream);
                                                    }
                                                    /*
                                                    currently borken, need to find way to get stream
                                                    "kick" => {
                                                        if args.len() < 2 {
                                                            send_chat_message(&mut stream, &"no user".to_string());
                                                            continue;
                                                        }

                                                        let mut players = playersmutex.lock().unwrap();
                                                        let username = args[1].to_string();

                                                        let getted_player = get_player(&mut players, &username);


                                                        if getted_player.is_none() {
                                                            send_chat_message(&mut stream, &"player not found".to_string());
                                                            continue;
                                                        }

                                                        let playerdata = getted_player.unwrap();

                                                        disconnect_player(&mut players, &playerdata.username);
                                                        println!("player {} disconnected, total: {}", playerdata.username, players.len());
                                                        
                                                        write_string_chat(&mut buffer, &"kick by operator :c".to_string());
                                                        flush(&mut stream, &mut buffer, CPlayPacketid::Kick as i32);

                                                        write_string_chat(&mut buffer, &format!("{} left the game", playerdata.username));
                                                        buffer.push(0);
                                                        broadcast(&tx, &mut buffer, CPlayPacketid::Chat as i32, &stream);
            
                                                        write_var_int(&mut buffer, 1);
                                                        buffer.extend(playerdata.uuid.to_be_bytes());
                                                        broadcast(&tx, &mut buffer, CPlayPacketid::PlayerLeft as i32, &stream);
            
                                                        write_var_int(&mut buffer, 1);
                                                        write_var_int(&mut buffer, playerdata.entity_id);
                                                        broadcast(&tx, &mut buffer, CPlayPacketid::RemoveEntities as i32, &stream);
            
                                                        save_player(&playerdata);
                                                        drop(stream);
                                                        break;
                                                    }
                                                    */
                                                    _ => { 
                                                        send_chat_message(&mut stream, "command not found".to_string());
                                                        continue;
                                                     }
                                                }

                                                handled = true;
                                            }
                                            5 => {
                                                let message = read_string_buf(&mut data);

                                                println!("{}: {}", player.username.clone(), message);
                                                let chatmsg = format!("{}: {}", player.username.clone(), message);

                                                let mut buffer = vec![];
                                                write_string_chat(&mut buffer, &chatmsg);
                                                buffer.push(0);

                                                broadcast(&tx, buffer.clone(), CPlayPacketid::Chat as i32, &stream);
                                                flush(&mut stream, &mut buffer, CPlayPacketid::Chat as i32);
                                                handled = true;
                                            }
                                            7 => {
                                                let _locale = read_string_buf(&mut data);
                                                let _view_distance: i8 = data.remove(0) as i8;
                                                let _chat_mode = read_var_int_buf(&mut data).unwrap();
                                                let _char_colors = data.remove(0);
                                                let skin_parts = data.remove(0);
                                                let _main_hand = read_var_int_buf(&mut data).unwrap();
                                                let _text_filtering = data.remove(0);
                                                let _allow_server_listening = data.remove(0);

                                                let mut buffer: Vec<u8> = vec![];
                                                write_var_int(&mut buffer, entity_id as i32);

                                                buffer.push(17);
                                                write_var_int(&mut buffer, 0);
                                                buffer.push(skin_parts);

                                                buffer.push(0xff);
                                                
                                                broadcast(&tx, buffer.clone(), CPlayPacketid::EntityMetadata as i32, &stream);
                                                flush(&mut stream, &mut buffer, CPlayPacketid::EntityMetadata as i32);
                                            }
                                            12 => {
                                                //let channel = read_string_buf(&mut data);
                                                //println!("channel: {}, data: {:?}", channel, &data);
                                                handled = true;
                                            }
                                            17 => {
                                                if lastkeepalive == u64::from_be_bytes(data.drain(0..8).as_slice().try_into().unwrap()) {
                                                    continue;
                                                }
                                                
                                                let mut buffer: Vec<u8> = vec![];
                                                write_string_chat(&mut buffer, &"invalid keep-alive response".to_string());
                                                flush(&mut stream, &mut buffer, CPlayPacketid::Kick as i32);

                                                let mut players = playersmutex.lock().unwrap();
                                                disconnect_player(&mut players, &player.username);
                                                drop(stream);
                                                break;
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
                                                broadcast(&tx, buffer, CPlayPacketid::EntityUpdatePos as i32, &stream);
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
                                                broadcast(&tx, buffer, CPlayPacketid::EntityUpdatePosAndRot as i32, &stream);

                                                let mut buffer = vec![];
                                                write_var_int(&mut buffer, player.entity_id);
                                                buffer.push(angle_x);
                                                broadcast(&tx, buffer, CPlayPacketid::HeadRot as i32, &stream);
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
                                                broadcast(&tx, buffer, CPlayPacketid::EntityUpdateRot as i32, &stream);

                                                let mut buffer = vec![];
                                                write_var_int(&mut buffer, player.entity_id);
                                                buffer.push(angle_x);
                                                broadcast(&tx, buffer, CPlayPacketid::HeadRot as i32, &stream);
                                                //player rotation
                                            }
                                            28 => {
                                                let status = read_var_int_buf(&mut data).unwrap();
                                                let (x,y,z) = read_position(&mut data);
                                                data.remove(0); // face
                                                let _seq = read_var_int_buf(&mut data);

                                                
                                                match status {
                                                    2 => {
                                                        //send_chat_message(&mut stream, format!("{:?}", position));
                                                        let mut buffer = vec![];
                                                        write_position(&mut buffer, x, y, z);
                                                        write_var_int(&mut buffer, 0);
                                                        broadcast(&tx, buffer.clone(), CPlayPacketid::BlockUpdate as i32, &stream);
                                                        flush(&mut stream, &mut buffer, CPlayPacketid::BlockUpdate as i32);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            29 => {
                                                let entity_id = read_var_int_buf(&mut data).unwrap();
                                                let action = read_var_int_buf(&mut data).unwrap();
                                                let _jump_boost = read_var_int_buf(&mut data).unwrap();

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
                                                    broadcast(&tx, buffer, CPlayPacketid::EntityMetadata as i32, &stream);
                                                }else if (action & 0) == 0 {
                                                    buffer.push(0);
                                                    write_var_int(&mut buffer, 0);
                                                    buffer.push(0);

                                                    buffer.push(6);
                                                    write_var_int(&mut buffer, 19);
                                                    write_var_int(&mut buffer, 0);

                                                    buffer.push(0xff);
                                                    broadcast(&tx, buffer, CPlayPacketid::EntityMetadata as i32, &stream);
                                                }
                                            }
                                            47 => {
                                                let mut buffer = vec![];
                                                write_var_int(&mut buffer, player.entity_id);
                                                buffer.extend([0]);
                                                broadcast(&tx, buffer, CPlayPacketid::SwingArm as i32, &stream);
                                            }
                                            49 => {
                                                //block interact
                                                println!("packet {} [0x{:X}] {:?}", packetid, packetid, &data);
                                            }
                                            _ => {
                                                println!("unhandled packet {} [0x{:X}] {:?}", packetid, packetid, &data);
                                                handled = true;
                                            }
                                        }

                                        if !handled && data.len() > 0 {
                                            println!("data left in packet {} [0x{:X}] {:?}", packetid, packetid, &data);
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
    */