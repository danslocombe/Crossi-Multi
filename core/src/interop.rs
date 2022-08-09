use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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

    TimeRequestPacket(TimeRequestPacket),
    TimeRequestIntermediate(TimeRequestIntermediate),
    TimeResponsePacket(TimeResponsePacket),

    GoodBye(),

    EmptyMessage(),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientHello {
    header: [u8; 4],
    version: u8,
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
    // Removing as we are setting up a proper route
    pub exact_send_server_time_us : u32,

    pub latest : RemoteTickState,
    pub last_client_sent : PlayerIdMap<RemoteTickState>,
    pub rule_state : CrossyRulesetFST,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ReceivedServerTick {
    pub client_receive_time_us : u32,
    pub server_tick : ServerTick,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeRequestPacket
{
    pub client_send_time_us : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeRequestIntermediate
{
    pub client_send_time_us : u32,
    pub server_receive_time_us : u32,
    // HACKY only server understands this type
    pub socket_id : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeResponsePacket
{
    pub client_send_time_us : u32,
    pub server_receive_time_us : u32,
    pub server_send_time_us : u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TimeRequestEnd
{
    pub client_send_time_us : u32,
    pub client_receive_time_us : u32,
    pub server_receive_time_us : u32,
    pub server_send_time_us : u32,
}