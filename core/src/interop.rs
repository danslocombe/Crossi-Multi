use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{SocketAddr, UdpSocket};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum CrossyMessage {
    Hello(ClientHello),
    HelloResponse(InitServerResponse),
    ServerDecription(ServerDescription),
    ClientTick(ClientTick),
    ServerTick(ServerTick),
    OffsetPing(),
    OffsetPong(OffsetPong),
    EmptyMessage(),
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
    pub player_id: crate::game::PlayerId,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ServerDescription {
    pub server_version: u8,
    pub seed: u32,
}

pub const INIT_MESSAGE: [u8; 4] = ['h' as u8, 'e' as u8, 'l' as u8, 'o' as u8];
pub const CURRENT_VERSION: u8 = 1;

impl ClientHello {
    pub fn new(latency_us: u32) -> Self {
        ClientHello {
            header: INIT_MESSAGE,
            version: CURRENT_VERSION,
            latency_us,
        }
    }

    pub fn check(&self, required_version: u8) -> bool {
        self.header == INIT_MESSAGE && self.version >= required_version
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientTick {
    pub time_us: u32,
    pub input: crate::game::Input,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ServerTick {
    pub latest : crate::timeline::RemoteTickState,
    pub last_client_sent : LastClientSentTicks
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct LastClientSentTicks
{
    // Used by individual clients for interpolation / prediction
    // Instead of sending each client their specific value we blanket send to all.
    last_client_sent : Vec<Option<crate::timeline::RemoteTickState>>,
}

impl LastClientSentTicks {
    pub fn new() -> Self {
        LastClientSentTicks {
            last_client_sent : Vec::with_capacity(8),
        }
    }

    pub fn set(&mut self, id: crate::PlayerId, state: crate::timeline::RemoteTickState) {
        let index = id.0 as usize;
        if (index >= self.last_client_sent.len())
        {
            self.last_client_sent.resize(index + 1, None);
        }

        self.last_client_sent[index] = Some(state);
    }

    pub fn get(&self, id: crate::PlayerId) -> Option<&crate::timeline::RemoteTickState> {
        let index = id.0 as usize;
        if index < self.last_client_sent.len()
        {
            self.last_client_sent[index].as_ref()
        }
        else
        {
            None
        }
    }
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
