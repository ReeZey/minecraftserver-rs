extern crate serde;
extern crate serde_json;
use std::net::TcpStream;

// Import this crate to derive the Serialize and Deserialize traits.

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
    pub x: f64,
    pub y: f64,
    pub z: f64,

    pub username: String,
    pub uuid: String,
}

pub struct OnlinePlayer {
    pub player: Player,
    pub stream: TcpStream,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerPlayer {
    pub name: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerPlayerCount {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<ServerPlayer>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerDescription {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerStatus {
    pub version:  ServerVersion,
    pub players: ServerPlayerCount,
    pub description: ServerDescription,
    pub enforces_secure_chat: bool,
}