use std::fmt::Debug;
use std::net::{UdpSocket, SocketAddr};
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum CrossyMessage
{
    Hello(ClientHello),
    HelloResponse(InitServerResponse),
    ClientTick(ClientTick),
    ServerTick(ServerTick),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientHello
{
    header : [u8; 4],
    version : u8,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct InitServerResponse
{
    pub server_version : u8,
    pub player_count : u8,
    pub seed : u32,
    pub player_id : super::game::PlayerId,
}

pub const INIT_MESSAGE : [u8; 4] = ['h' as u8, 'e' as u8, 'l' as u8, 'o' as u8];
pub const CURRENT_VERSION : u8 = 1;

impl ClientHello
{
    pub fn new() -> Self
    {
        ClientHello
        {
            header : INIT_MESSAGE,
            version : CURRENT_VERSION,
        }
    }

    pub fn check(&self, required_version : u8) -> bool {
        self.header == INIT_MESSAGE && self.version >= required_version
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ClientTick
{
    pub time_us : u32,
    pub input : super::game::Input,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ServerTick
{
    pub time_us : u32,
    pub states : Vec<super::game::PlayerState>,
}

pub fn crossy_send(x : &CrossyMessage, socket: &mut UdpSocket, addr : &SocketAddr) -> std::io::Result<()>
{
    let serialized = bincode::serialize(x).unwrap();
    socket.send_to(&serialized[..], addr)?;
    /*
    let serialized_len = serialized.len();
    let len_bytes = unsafe {
        std::mem::transmute::<u32, [u8;4]>(serialized_len as u32)
    };

    let mut buffer : Vec<u8> = Vec::new();
    buffer.reserve(serialized_len + 4);
    for b in len_bytes.iter().chain(serialized.iter())
    {
        buffer.push(*b);
    }

    socket.send_to(&buffer[..], addr)?;
    println!("Ok");
    */

    Ok(())
}

pub fn crossy_receive(socket : &mut UdpSocket) -> std::io::Result<(CrossyMessage, SocketAddr)>
{
    //let mut buffer = [0;std::mem::size_of::<CrossyMessage>()];
    let mut buffer = [0;2048];
    let (size, addr) = socket.recv_from(&mut buffer)?;
    let deserialized = bincode::deserialize(&buffer).unwrap();

    //println!("Peekin");
    /*
    let mut small_buffer : [u8;4] = [0;4];
    socket.peek(&mut small_buffer)?;
    let len = unsafe {
        std::mem::transmute::<[u8;4], u32>(small_buffer)
    };

    println!("Received size {}", len);

    let mut buffer = vec![0; 4 + len as usize];
    socket.recv(&mut buffer)?;

    let x = bincode::deserialize(&buffer).unwrap();
    */

    Ok((deserialized, addr))
}