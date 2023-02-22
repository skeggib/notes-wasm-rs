use model::{Model, Note};

use notify::{Error, Event, Watcher};
use std::fs::{read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::{str, thread};
use string_join::display::Join;

use crate::model;

pub fn watch_workspace(workspace_path: PathBuf, model: Model) -> Result<Receiver<Model>, String> {
    // channel used to receive notifications from notify
    let (notify_sender, notify_receiver) = channel();

    // channel used to send model updates to client
    let (watcher_sender, watcher_receiver) = channel();

    let mut watcher = match notify::recommended_watcher(notify_sender) {
        Ok(watcher) => watcher,
        Err(error) => return Err(format!("cannot create watcher -> {}", error)),
    };

    // start watching workspace in separate thread
    thread::spawn(move || {
        match watcher.watch(&workspace_path, notify::RecursiveMode::NonRecursive) {
            Ok(_) => {
                let mut current_model = model;
                match process_events(notify_receiver, &mut |event| {
                    println!("notify event: {:?}", event);
                    match event_handler(event, &current_model, &workspace_path) {
                        Ok(Some(updated_model)) => {
                            current_model = updated_model;
                            match watcher_sender.send(current_model.clone()) {
                                Ok(_) => {}
                                Err(error) => eprintln!("cannot send model: {}", error),
                            };
                        },
                        Ok(None) => {}
                        Err(error) => eprintln!("cannot update model: {}", error),
                    };
                }) {
                    Ok(()) => {},
                    Err(error) => eprintln!("watcher stopped: {}", error)
                };
            }
            Err(error) => eprintln!("cannot watch '{:?}' -> {}", &workspace_path, error),
        };
    });

    Ok(watcher_receiver)
}

fn process_events(receiver: Receiver<Result<Event, Error>>, callback: &mut impl FnMut(Event) -> ()) -> Result<(), String> {
    loop {
        match receiver.recv() {
            Ok(event_or_error) => match event_or_error {
                Ok(event) => callback(event),
                Err(error) => eprintln!("{}", error),
            },
            Err(error) => {
                return Err(format!("rx stopped: {}", error));
            }
        }
    }
}

fn event_handler(event: Event, model: &Model, workspace_path: &PathBuf) -> Result<Option<Model>, String> {
    match event.kind {
        notify::EventKind::Access(_) => Ok(None),
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
            Ok(Some(updated_model))
        }
        notify::EventKind::Modify(kind) => match kind {
            notify::event::ModifyKind::Any => Ok(None),
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
                Ok(Some(updated_model))
            }
            notify::event::ModifyKind::Metadata(_) => Ok(None),
            notify::event::ModifyKind::Name(_) => todo!(),
            notify::event::ModifyKind::Other => Ok(None),
        },
        notify::EventKind::Other => todo!(),
        notify::EventKind::Remove(_) => todo!(),
    }
}

fn read_note(path: &Path) -> Result<Note, String> {
    match read(&path) {
        Ok(buf) => match str::from_utf8(&buf) {
            Ok(text) => {
                let lines: Vec<&str> = text.split("\n").collect();
                if lines.len() > 1 {
                    let iterator = lines.iter();
                    let without_title = iterator.skip(1);
                    let body_lines = without_title.skip_while(|line| line.is_empty());
                    let body = "\n".join(body_lines);
                    Ok(Note {
                        title: lines[0].to_string(),
                        body: body,
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
