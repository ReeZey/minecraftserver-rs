pub enum CPlayPacketid {
    Kick = 0x17,
    ContainerContent = 0x10,
    GameEvent = 0x1C,
    KeepAlive = 0x1F,
    LoginPlay = 0x24,
    PlayerInfo = 0x36,
    PlayerPos = 0x38,
    SetDefaultSpawn = 0x4C,
    Chat = 0x60,
}

pub enum CLoginPacketid {
    Kick = 0,
    Success = 2,
}

pub enum CStatusPacketid {
    Status = 0,
    Ping = 1,
}