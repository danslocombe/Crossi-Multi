use crossy_multi_core::interop::*;
use crossy_multi_core::game;

use std::io::Result;
use std::net::{UdpSocket, SocketAddr, Ipv4Addr, SocketAddrV4};

pub struct Client
{
    server : SocketAddr, 
    socket : UdpSocket,
    //game : Option<game::Game>
}

fn connect(socket: &mut UdpSocket, addr : &SocketAddr) -> Result<()>
{
    println!("Connecting..");
    //socket.send_to(&INIT_MESSAGE, addr)?;
    let hello = CrossyMessage::Hello(ClientHello::new());
    crossy_send(&hello, socket, addr).unwrap();
    let response = crossy_receive(socket)?;
    println!("Got response!, {:?}", &response);
    Ok(())
}

impl Client
{
    pub fn try_create() -> Result<Self>
    {
        let mut socket = UdpSocket::bind("127.0.0.1:8080")?;
        let server = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081));

        connect(&mut socket, &server)?;

        Ok(Client {
            server : server,
            socket: socket,
            //game : None,
        })
    }

    pub fn send(&mut self, input : game::Input, time_us : u32)
    {
        let client_update = CrossyMessage::ClientTick(ClientTick{
            time_us : time_us,
            input : input,
        });

        crossy_send(&client_update, &mut self.socket, &self.server).unwrap();
    }
}