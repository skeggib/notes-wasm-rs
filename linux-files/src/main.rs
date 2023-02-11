pub mod model;

use model::{Client, InstanceKind, SingleConnectionServer};
use model::{Model, Note};

use notify::Watcher;
use serde::Deserialize;
use std::fs::{create_dir, read, File, OpenOptions};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::{fs::remove_dir_all, process::exit};
use std::{str, thread};
use string_join::display::Join;

fn main() {
    let workspace_path_string = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");

    let instance_kind_string = std::env::args()
        .nth(2)
        .expect("Argument 2 needs to be either 'server' or 'client'");

    let mut workspace_path = PathBuf::new();
    workspace_path.push(workspace_path_string);

    let address = "127.0.0.1:55000";
    let instance_kind = match instance_kind_string.as_str() {
        "server" => match SingleConnectionServer::new(address) {
            Ok(server) => InstanceKind::ServerKind(server),
            Err(error) => {
                eprintln!("cannot create server -> {}", error);
                exit(1)
            }
        },
        "client" => match Client::new(address) {
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
        InstanceKind::ServerKind(ref server) => {
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
        InstanceKind::ClientKind(ref client) => {
            println!("receive model from server");
            let mut de = serde_json::Deserializer::from_reader(&client.stream);
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
        InstanceKind::ServerKind(ref server) => {
            match watch_workspace(&workspace_path, &model) {
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
        InstanceKind::ClientKind(client) => {
            let (stream_sender, stream_receiver) = channel();
            thread::spawn(move || loop {
                let mut de = serde_json::Deserializer::from_reader(&client.stream);
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
            match watch_workspace(&workspace_path, &model) {
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
                            update_workspace(&workspace_path, &model);
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
    for (filename, note) in &model.notes {
        let file_path = path.join(filename);
        match File::create(file_path.clone()) {
            Ok(mut file) => match writeln!(file, "{}\n\n{}", note.title, note.body) {
                Ok(_) => (),
                Err(error) => {
                    return Err(format!("could not write file '{:?}': {}", file_path, error))
                }
            },
            Err(error) => {
                return Err(format!(
                    "could not create file '{:?}': {}",
                    file_path, error
                ))
            }
        }
    }
    Ok(())
}

fn watch_workspace(workspace_path: &Path, model: &Model) -> Result<(Receiver<Model>), String> {
    fn event_handler(
        res: Result<notify::Event, notify::Error>,
        model: &Model,
        workspace_path: &PathBuf,
    ) -> Result<Model, String> {
        match res {
            Ok(event) => match event.kind {
                notify::EventKind::Access(_) => Ok(model.clone()),
                notify::EventKind::Any => todo!(),
                notify::EventKind::Create(_) => {
                    let mut updated_model = model.clone();
                    for path in event.paths {
                        updated_model
                            .notes
                            .entry(
                                path.strip_prefix(workspace_path)
                                    .unwrap()
                                    .to_str()
                                    .unwrap()
                                    .to_string(),
                            )
                            .or_insert(Note::new());
                    }
                    Ok(updated_model)
                }
                notify::EventKind::Modify(kind) => match kind {
                    notify::event::ModifyKind::Any => Ok(model.clone()),
                    notify::event::ModifyKind::Data(_) => {
                        let mut updated_model = model.clone();
                        for path in event.paths {
                            match read_note(&path) {
                                Ok(note) => {
                                    *updated_model
                                        .notes
                                        .entry(
                                            path.strip_prefix(workspace_path)
                                                .unwrap()
                                                .to_str()
                                                .unwrap()
                                                .to_string(),
                                        )
                                        .or_insert(Note::new()) = note
                                }
                                Err(error) => {
                                    eprintln!("could not read note '{:?}': {}", path, error)
                                }
                            }
                        }
                        Ok(updated_model)
                    }
                    notify::event::ModifyKind::Metadata(_) => Ok(model.clone()),
                    notify::event::ModifyKind::Name(_) => todo!(),
                    notify::event::ModifyKind::Other => Ok(model.clone()),
                },
                notify::EventKind::Other => todo!(),
                notify::EventKind::Remove(_) => todo!(),
            },
            Err(error) => Err(format!("{}", error)),
        }
    }

    println!("{}", &model);

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(watcher) => watcher,
        Err(error) => return Err(format!("cannot create watcher -> {}", error)),
    };

    let (sender, receiver) = channel();

    let m = model.clone();
    let w = workspace_path.to_owned();
    thread::spawn(move || {
        match watcher.watch(&w, notify::RecursiveMode::NonRecursive) {
            Ok(_) => {
                let mut current_model = m;
                loop {
                    match rx.recv() {
                        Ok(res) => {
                            match event_handler(res, &current_model, &w) {
                                Ok(updated_model) => {
                                    current_model = updated_model;
                                    match sender.send(current_model.clone()) {
                                        Ok(_) => {}
                                        Err(error) => eprintln!("cannot send model: {}", error),
                                    };
                                }
                                Err(error) => eprintln!("cannot update model: {}", error),
                            };
                        }
                        Err(error) => {
                            eprintln!("rx stopped: {}", error);
                            break;
                        }
                    }
                }
            }
            Err(error) => eprintln!("cannot watch '{:?}' -> {}", &w, error),
        };
    });

    Ok(receiver)
}

fn read_note(path: &Path) -> Result<Note, String> {
    match read(&path) {
        Ok(buf) => match str::from_utf8(&buf) {
            Ok(text) => {
                let lines: Vec<&str> = text.split("\n").collect();
                if lines.len() > 1 {
                    Ok(Note {
                        title: lines[0].to_string(),
                        body: "\n".join(lines.iter().skip(1).skip_while(|line| line.is_empty())),
                    })
                } else {
                    Ok(Note {
                        title: "".to_string(),
                        body: text.to_string(),
                    })
                }
            }
            Err(error) => Err(format!("could not read '{:?}' -> {}", path, error)),
        },
        Err(error) => Err(format!("could not read '{:?}' -> {}", path, error)),
    }
}

fn update_workspace(workspace_path: &Path, model: &Model) -> () {
    model
        .notes
        .iter()
        .for_each(|(path, note)| update_node(workspace_path.join(path).as_path(), note));
}

fn update_node(path: &Path, note: &Note) -> () {
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
    }
}
