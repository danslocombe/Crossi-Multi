use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::net::{SocketAddr, UdpSocket};

use crate::game::Input;
use crate::timeline::RemoteTickState;
use crate::player_id_map::PlayerIdMap;
use crate::crossy_ruleset::CrossyRulesetFST;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum CrossyMessage {
    Hello(ClientHello),
    HelloResponse(InitServerResponse),
    ServerDecription(ServerDescription),
    ClientTick(ClientTick),
    ClientDrop(),
    ServerTick(ServerTick),
    OffsetPing(),
    OffsetPong(OffsetPong),
    EmptyMessage(),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientHello {
    header: [u8; 4],
    version: u8,
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

pub const INIT_MESSAGE: &[u8; 4] = b"helo";
pub const CURRENT_VERSION: u8 = 1;

impl Default for ClientHello {
    fn default() -> Self {
        ClientHello {
            header: *INIT_MESSAGE,
            version: CURRENT_VERSION,
        }
    }
}

impl ClientHello {
    pub fn check(&self, required_version: u8) -> bool {
        self.header == *INIT_MESSAGE && self.version >= required_version
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientTick {
    pub time_us: u32,
    pub input: Input,
    // TODO probably shouldnt be here?
    pub lobby_ready : bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ServerTick {
    pub latest : RemoteTickState,
    pub last_client_sent : PlayerIdMap<RemoteTickState>,
    pub rule_state : CrossyRulesetFST,
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
