pub mod fs_watcher;
pub mod model;
pub mod networking;

use model::{Model, Note};
use networking::Connection;

use fs_watcher::watch_workspace;

use std::fs::{create_dir, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs::remove_dir_all, process::exit};

pub enum InstanceKind {
    ServerKind,
    ClientKind,
}

fn main() {
    let workspace_path_string = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");

    let instance_kind_string = std::env::args()
        .nth(2)
        .expect("Argument 2 needs to be either 'server' or 'client'");

    let mut workspace_path = PathBuf::new();
    workspace_path.push(workspace_path_string);

    let instance_kind = match instance_kind_string.as_str() {
        "server" => InstanceKind::ServerKind,
        "client" => InstanceKind::ClientKind,
        _ => {
            eprintln!("invalid instance kind '{}'", instance_kind_string);
            exit(1)
        }
    };

    // create connection
    let connection = match instance_kind {
        InstanceKind::ServerKind => {
            match Connection::new("127.0.0.1:55000", "ws://127.0.0.1:55001") {
                Ok(connection) => connection,
                Err(error) => {
                    eprintln!("cannot open connection -> {}", error);
                    exit(1)
                }
            }
        }
        InstanceKind::ClientKind => {
            match Connection::new("127.0.0.1:55001", "ws://127.0.0.1:55000") {
                Ok(connection) => connection,
                Err(error) => {
                    eprintln!("cannot open connection -> {}", error);
                    exit(1)
                }
            }
        }
    };

    // server sends model to client
    // TODO: eventually remove this code
    let mut model = match instance_kind {
        InstanceKind::ServerKind => {
            let model = Model::new();
            println!("send model to client");
            match connection.send(&model) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("cannot write model to stream -> {}", error);
                    exit(1)
                }
            };

            model
        }
        InstanceKind::ClientKind => {
            println!("receive model from server");
            // let mut de = serde_json::Deserializer::from_reader(client.as_reader());
            // match Model::deserialize(&mut de) {\
            match connection.as_receiver().recv() {
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
    let watch_receiver = match watch_workspace(workspace_path.clone(), model.clone()) {
        Ok(receiver) => receiver,
        Err(error) => {
            eprintln!("cannot watch workspace -> {}", error);
            exit(1)
        }
    };

    loop {
        match watch_receiver.try_recv() {
            Ok(updated_model) => {
                model = updated_model;
                println!("{}", model);
                match connection.send(&model) {
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("cannot write model to stream -> {}", error);
                        exit(1)
                    }
                };
            }
            Err(_) => {}
        }
        match connection.as_receiver().try_recv() {
            Ok(updated_model) => {
                model = updated_model;
                println!("{}", model);
                write_workspace(&workspace_path, &model);
            }
            Err(_) => {}
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
                Ok(()) => match write!(file, "{}\n\n{}", note.title, note.body) {
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
            Ok(mut file) => match write!(file, "{}\n\n{}", note.title, note.body) {
                Ok(_) => (),
                Err(error) => {
                    println!("could not write file '{:?}': {}", path, error)
                }
            },
            Err(error) => {
                println!("could not create file '{:?}': {}", path, error)
            }
        }
    }
}
