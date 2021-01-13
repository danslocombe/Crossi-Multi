extern crate crossy_multi_core;

use crossy_multi_core::interop::*;
use crossy_multi_core::game;

use std::io::Result;
use std::net::{UdpSocket, SocketAddr};

const SERVER_VERSION : u8 = 1;

fn main() {
    let mut s = Server {
        clients: vec![],
    };

    s.run();
}

struct Client
{
    addr : SocketAddr,
    id : game::PlayerId,
}

struct Server
{
    clients : Vec<Client>,
}

// todo: use Result(!) 
impl Server
{
    fn run(&mut self) -> Result<()>
    {
        let socket_config = "127.0.0.1:8081";
        println!("Binding socket to {}", socket_config);
        let mut socket = UdpSocket::bind(socket_config)?;
        let mut buffer = vec![0;4];

        loop
        {
            let (amt, src) = socket.recv_from(&mut buffer)?;

            if &buffer[..] == crossy_multi_core::interop::INIT_MESSAGE
            {
                println!("Player joined! {}", src);

                let client_id = game::PlayerId(self.clients.len() as u8);
                let client = Client 
                {
                    addr : src,
                    id : client_id,
                };

                self.clients.push(client);

                let response = InitServerResponse
                {
                    server_version : SERVER_VERSION,
                    player_count : self.clients.len() as u8,
                    player_id : client_id,
                };

                crossy_send(&response, &mut socket, src);
            }
            else
            {
                println!("Got something unrecognised, {}, {}, {}, {}", buffer[0], buffer[1], buffer[2], buffer[3]);
            }
        }

        Ok(())
    }
}