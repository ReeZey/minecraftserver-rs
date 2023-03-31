extern crate serde;
extern crate serde_json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub username: String,
    pub uuid: u128,
    pub entity_id: i32,

    pub x: f64,
    pub y: f64,
    pub z: f64,
    
    pub yaw: f32,
    pub pitch: f32,

    pub chunk_x: i32,
    pub chunk_z: i32,

    pub gamemode: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusPlayers {
    pub name: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusPlayerCount {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<StatusPlayers>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusDescription {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerStatus {
    pub version:  StatusVersion,
    pub players: StatusPlayerCount,
    pub description: StatusDescription,
    pub enforces_secure_chat: bool,
}