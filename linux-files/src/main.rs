pub mod model;
pub mod networking;
pub mod fs_watcher;

use model::{Model, Note};
use networking::{InstanceKind, SingleConnectionServer, WsClient};

use fs_watcher::{watch_workspace};

use serde::Deserialize;
use std::fs::{create_dir, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::{fs::remove_dir_all, process::exit};
use std::{thread};

fn main() {
    let workspace_path_string = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");

    let instance_kind_string = std::env::args()
        .nth(2)
        .expect("Argument 2 needs to be either 'server' or 'client'");

    let mut workspace_path = PathBuf::new();
    workspace_path.push(workspace_path_string);

    let bind_address = "127.0.0.1:55000";
    let connect_address = "ws://127.0.0.1:55000";
    let mut instance_kind = match instance_kind_string.as_str() {
        "server" => match SingleConnectionServer::new(bind_address) {
            Ok(server) => InstanceKind::ServerKind(server),
            Err(error) => {
                eprintln!("cannot create server -> {}", error);
                exit(1)
            }
        },
        "client" => match WsClient::new(connect_address) {
            Ok(client) => InstanceKind::ClientKind(client),
            Err(error) => {
                eprintln!("cannot create client -> {}", error);
                exit(1)
            }
        },
        _ => {
            eprintln!("invalid instance kind '{}'", instance_kind_string);
            exit(1)
        }
    };

    // at this point, client and server are connected

    let mut model = match instance_kind {
        InstanceKind::ServerKind(ref mut server) => {
            let model = Model::new();

            println!("send model to client");
            match serde_json::to_writer(server.as_writer(), &model) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("cannot write model to stream -> {}", error);
                    exit(1)
                }
            };

            model
        }
        InstanceKind::ClientKind(ref mut client) => {
            println!("receive model from server");
            let mut de = serde_json::Deserializer::from_reader(client.as_reader());
            match Model::deserialize(&mut de) {
                Ok(model) => model,
                Err(error) => {
                    eprintln!("cannot read model from stream -> {}", error);
                    exit(1)
                }
            }
        }
    };

    println!("initialize workspace");
    match destroy_workspace(&workspace_path) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("cannot destroy workspace -> {}", error);
            exit(1)
        }
    };
    match init_workspace(&workspace_path, &model) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("cannot init workspace -> {}", error);
            exit(1)
        }
    };

    println!("watch workspace...");
    match instance_kind {
        InstanceKind::ServerKind(ref mut server) => {
            match watch_workspace(workspace_path, model) {
                Ok(watch_receiver) => loop {
                    match watch_receiver.recv() {
                        Ok(updated_model) => {
                            model = updated_model;
                            println!("{}", model);
                            match serde_json::to_writer(server.as_writer(), &model) {
                                Ok(_) => {}
                                Err(error) => {
                                    eprintln!("cannot write model to stream -> {}", error);
                                    exit(1)
                                }
                            };
                        }
                        Err(_) => {}
                    };
                },
                Err(error) => {
                    eprintln!("cannot watch workspace -> {}", error);
                    exit(1)
                }
            };
        }
        InstanceKind::ClientKind(mut client) => {
            let (stream_sender, stream_receiver) = channel();
            thread::spawn(move || loop {
                let mut de = serde_json::Deserializer::from_reader(client.as_reader());
                match Model::deserialize(&mut de) {
                    Ok(model) => match stream_sender.send(model) {
                        Ok(_) => {}
                        Err(error) => {
                            eprintln!("cannot send model -> {}", error);
                        }
                    },
                    Err(error) => {
                        eprintln!("cannot read model from stream -> {}", error);
                    }
                }
            });
            match watch_workspace(workspace_path.clone(), model.clone()) {
                Ok(watch_receiver) => loop {
                    match watch_receiver.try_recv() {
                        Ok(updated_model) => {
                            model = updated_model;
                            println!("{}", model);
                        }
                        Err(_) => {}
                    }
                    match stream_receiver.try_recv() {
                        Ok(updated_model) => {
                            model = updated_model;
                            println!("{}", model);
                            write_workspace(&workspace_path, &model);
                        }
                        Err(_) => {}
                    }
                },
                Err(error) => {
                    eprintln!("cannot watch workspace -> {}", error);
                    exit(1)
                }
            }
        }
    }
}

fn destroy_workspace(workspace_path: &Path) -> Result<(), String> {
    if !workspace_path.exists() {
        Ok(())
    } else {
        match remove_dir_all(workspace_path) {
            Ok(_) => Ok(()),
            Err(error) => Err(format!(
                "cannot delete workspace '{:?}': {}",
                workspace_path, error
            )),
        }
    }
}

fn init_workspace(workspace_path: &Path, model: &Model) -> Result<(), String> {
    let path = Path::new(workspace_path);
    match create_dir(path) {
        Ok(_) => {}
        Err(error) => {
            return Err(format!(
                "could not create directory '{:?}': {}",
                workspace_path, error
            ))
        }
    };
    write_workspace(workspace_path, model);
    Ok(())
}

fn write_workspace(workspace_path: &Path, model: &Model) -> () {
    model
        .notes
        .iter()
        .for_each(|(path, note)| write_note(workspace_path.join(path).as_path(), note));
}

fn write_note(path: &Path, note: &Note) -> () {
    if path.exists() {
        match OpenOptions::new().write(true).open(path) {
            Ok(mut file) => match file.set_len(0) {
                Ok(()) => match writeln!(file, "{}\n\n{}", note.title, note.body) {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!("could not write file '{:?}': {}", path, error);
                    }
                },
                Err(error) => {
                    eprintln!("could not clear file '{:?}': {}", path, error);
                }
            },
            Err(error) => {
                eprintln!("cannot update existing note -> {}", error)
            }
        }
    } else {
        match File::create(path.clone()) {
            Ok(mut file) => match writeln!(file, "{}\n\n{}", note.title, note.body) {
                Ok(_) => (),
                Err(error) => {
                    println!("could not write file '{:?}': {}", path, error)
                }
            },
            Err(error) => {
                println!(
                    "could not create file '{:?}': {}",
                    path, error
                )
            }
        }
    }
}
