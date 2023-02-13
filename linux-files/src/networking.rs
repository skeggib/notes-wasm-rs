use std::net::TcpStream;
use std::str;

use websocket::sync::{Client, Server};
use websocket::ClientBuilder;

pub enum InstanceKind {
    ServerKind(SingleConnectionServer),
    ClientKind(Client<TcpStream>),
}

pub struct SingleConnectionServer {
    client: Client<TcpStream>,
}

impl SingleConnectionServer {
    /// Binds the provided address and awaits exactly one connection
    pub fn new(address: &str) -> Result<SingleConnectionServer, String> {
        println!("binding {}", address);
        let mut server = match Server::bind(address) {
            Ok(server) => server,
            Err(error) => return Err(format!("cannot bind {} -> {}", address, error)),
        };
        println!("waiting for client...");
        let client = match server.accept() {
            Ok(upgrade) => match upgrade.accept() {
                Ok(client) => client,
                Err(error) => return Err(format!("cannot accept upgrade request -> {:?}", error)),
            },
            Err(error) => return Err(format!("cannot accept incoming connection -> {:?}", error)),
        };
        println!("connected");
        Ok(SingleConnectionServer { client })
    }

    pub fn as_writer(self: &Self) -> &TcpStream {
        self.client.stream_ref()
    }
}

pub fn connect(address: &str) -> Result<Client<TcpStream>, String> {
    println!("connecting to 127.0.0.1:55000...");
    match ClientBuilder::new(address).unwrap().connect_insecure() {
        Ok(client) => {
            println!("connected");
            Ok(client)
        }
        Err(error) => return Err(format!("Could not connect to {} -> {}", address, error)),
    }
}
