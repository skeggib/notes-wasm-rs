use model::{Model, Note};

use notify::Watcher;
use std::fs::{read, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::{str, thread};
use string_join::display::Join;

use crate::model;

pub fn watch_workspace(workspace_path: &Path, model: &Model) -> Result<Receiver<Model>, String> {
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

pub fn update_workspace(workspace_path: &Path, model: &Model) -> () {
    model
        .notes
        .iter()
        .for_each(|(path, note)| update_node(workspace_path.join(path).as_path(), note));
}

pub fn update_node(path: &Path, note: &Note) -> () {
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
