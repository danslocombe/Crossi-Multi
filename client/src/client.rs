use crossy_multi_core::interop::*;
use crossy_multi_core::game;

use std::io::Result;
use std::net::{UdpSocket, SocketAddr, Ipv4Addr, SocketAddrV4};

pub struct Client
{
    server : SocketAddr, 
    socket : Option<UdpSocket>,
    //game : Option<game::Game>
}

fn connect(socket: &mut UdpSocket, addr : SocketAddr) -> Result<()>
{
    println!("Connecting..");
    socket.send_to(&INIT_MESSAGE, addr)?;
    let response = crossy_receive::<InitServerResponse>(socket)?;
    println!("Got response!");
    Ok(())
}

impl Client
{
    pub fn try_create() -> Result<Self>
    {
        let mut socket = UdpSocket::bind("127.0.0.1:8080")?;
        let server = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081));

        connect(&mut socket, server)?;

        Ok(Client {
            server : server,
            socket: Some(socket),
            //game : None,
        })
    }
}