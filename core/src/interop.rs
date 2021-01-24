use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{SocketAddr, UdpSocket};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum CrossyMessage {
    Hello(ClientHello),
    HelloResponse(InitServerResponse),
    ClientTick(ClientTick),
    ServerTick(ServerTick),
    OffsetPing(),
    OffsetPong(OffsetPong),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientHello {
    header: [u8; 4],
    version: u8,
    pub latency_us: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct OffsetPong {
    pub us_server: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct InitServerResponse {
    pub server_version: u8,
    pub player_count: u8,
    pub seed: u32,
    pub player_id: super::game::PlayerId,
}

pub const INIT_MESSAGE: [u8; 4] = ['h' as u8, 'e' as u8, 'l' as u8, 'o' as u8];
pub const CURRENT_VERSION: u8 = 1;

impl ClientHello {
    pub fn new(latency: u32) -> Self {
        ClientHello {
            header: INIT_MESSAGE,
            version: CURRENT_VERSION,
            latency_us: latency,
        }
    }

    pub fn check(&self, required_version: u8) -> bool {
        self.header == INIT_MESSAGE && self.version >= required_version
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientTick {
    pub time_us: u32,
    pub input: super::game::Input,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ServerTick {
    pub time_us: u32,
    pub states: Vec<super::game::PlayerState>,
}

pub fn crossy_send(
    x: &CrossyMessage,
    socket: &mut UdpSocket,
    addr: &SocketAddr,
) -> std::io::Result<()> {
    let serialized = bincode::serialize(x).unwrap();
    socket.send_to(&serialized[..], addr)?;
    Ok(())
}

pub fn crossy_receive(socket: &mut UdpSocket) -> std::io::Result<(CrossyMessage, SocketAddr)> {
    let mut buffer = [0; 2048];
    let (_, addr) = socket.recv_from(&mut buffer)?;
    let deserialized = bincode::deserialize(&buffer).unwrap();
    Ok((deserialized, addr))
}
