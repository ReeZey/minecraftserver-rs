extern crate serde;
extern crate serde_json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    
    pub yaw: f32,
    pub pitch: f32,

    pub username: String,
    pub uuid: String,
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