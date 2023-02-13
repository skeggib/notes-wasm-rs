use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use std::{str, thread};

use serde::Deserialize;
use websocket::sync::Server;
use websocket::ClientBuilder;

use crate::model::Model;

pub enum InstanceKind {
    ServerKind(Connection),
    ClientKind(Connection),
}

pub struct Connection {
    sender: Sender<Model>,
    receiver: Receiver<Model>,
}

impl Connection {
    // TODO: take ToSocketAddrs instead of str
    pub fn new(bind_address: &str, connect_address: &str) -> Result<Connection, String> {
        // bind address
        let (server_sender, server_receiver) = channel();
        let mut server = Server::bind(bind_address).map_err(|err| err.to_string())?;
        thread::spawn(move || {
            // wait for a client
            let mut client = server.accept().unwrap().accept().unwrap();
            // and listen for messages coming from the server channel
            for value in server_receiver {
                // send the messages to the client
                serde_json::to_writer(client.writer_mut(), &value).unwrap();
            }
        });

        let (client_sender, client_receiver) = channel();
        let connect_address_owned = connect_address.to_owned();
        thread::spawn(move || {
            // repeatedly try to connect the server
            let mut client = loop {
                match ClientBuilder::new(&connect_address_owned)
                    .unwrap()
                    .connect_insecure()
                {
                    Ok(client) => break client,
                    Err(_) => thread::sleep(Duration::from_secs(1)),
                }
            };
            // and listen for messages coming from the client

            loop {
                let mut de = serde_json::Deserializer::from_reader(client.reader_mut());
                match Model::deserialize(&mut de) {
                    // send the messages through the client channel
                    Ok(model) => {
                        client_sender.send(model).unwrap();
                    }
                    Err(error) => eprintln!("cannot read model from stream -> {}", error),
                }
            }
        });

        Ok(Connection {
            sender: server_sender,
            receiver: client_receiver,
        })
    }

    pub fn send(self: &Self, value: &Model) -> Result<(), String> {
        self.sender
            .send(value.clone())
            .map_err(|err| err.to_string())
    }

    pub fn as_receiver(self: &Self) -> &Receiver<Model> {
        &self.receiver
    }
}
