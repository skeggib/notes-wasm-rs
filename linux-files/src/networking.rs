use std::net::{TcpListener, TcpStream};
use std::str;

pub enum InstanceKind {
    ServerKind(SingleConnectionServer),
    ClientKind(Client),
}

pub struct SingleConnectionServer {
    listener: TcpListener,
    stream: TcpStream,
}

impl SingleConnectionServer {
    /// Binds the provided address and awaits exactly one connection
    pub fn new(address: &str) -> Result<SingleConnectionServer, String> {
        println!("binding {}", address);
        let listener = match TcpListener::bind(address) {
            Ok(listener) => listener,
            Err(error) => return Err(format!("cannot bind {} -> {}", address, error)),
        };
        println!("waiting for client...");
        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) => return Err(format!("cannot accept incoming connection -> {}", error)),
        };
        println!("connected");
        Ok(SingleConnectionServer { listener, stream })
    }

    pub fn as_writer(self: &Self) -> &TcpStream {
        &self.stream
    }
}

pub struct Client {
    pub stream: TcpStream,
}

impl Client {
    pub fn new(address: &str) -> Result<Client, String> {
        println!("connecting to 127.0.0.1:55000...");
        let stream = match TcpStream::connect(address) {
            Ok(stream) => stream,
            Err(error) => return Err(format!("Could not connect to {} -> {}", address, error)),
        };
        println!("connected");
        Ok(Client { stream })
    }
}
