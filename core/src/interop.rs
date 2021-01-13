use std::net::{UdpSocket, SocketAddr};
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

pub const INIT_MESSAGE : [u8; 4] = ['h' as u8, 'e' as u8, 'l' as u8, 'o' as u8];

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct InitServerResponse
{
    pub server_version : u8,
    pub player_count : u8,
    pub player_id : super::game::PlayerId,
}

pub fn crossy_send<T: Serialize>(x : &T, socket: &mut UdpSocket, addr : SocketAddr) -> std::io::Result<()>
{
    let serialized = bincode::serialize(x).unwrap();
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

    Ok(())
}

pub fn crossy_receive<T : DeserializeOwned>(socket : &mut UdpSocket) -> std::io::Result<T>
{
    let mut small_buffer : [u8;4] = [0;4];
    socket.peek(&mut small_buffer);
    let len = unsafe {
        std::mem::transmute::<[u8;4], u32>(small_buffer)
    };

    let mut buffer = vec![0; len as usize];
    socket.recv(&mut buffer)?;

    Ok(bincode::deserialize(&buffer).unwrap())
}