pub enum CPlayPacketid {
    SpawnPlayer = 0x02,
    SwingArm = 0x03,
    BlockUpdate = 0x09,
    Kick = 0x17,
    ContainerContent = 0x10,
    GameEvent = 0x1C,
    KeepAlive = 0x1F,
    LoadChunk = 0x20,
    LoginPlay = 0x24,
    EntityUpdatePos = 0x27,
    EntityUpdatePosAndRot = 0x28,
    EntityUpdateRot = 0x29,
    RemoveEntities = 0x3A,
    HeadRot = 0x3E,
    PlayerLeft = 0x35,
    PlayerInfo = 0x36,
    PlayerPos = 0x38,
    CenterChunk = 0x4A,
    SetDefaultSpawn = 0x4C,
    Chat = 0x60,
    PlayerTeleport = 0x64,
}

pub enum CLoginPacketid {
    Kick = 0,
    Success = 2,
}

pub enum CStatusPacketid {
    Status = 0,
    Ping = 1,
}