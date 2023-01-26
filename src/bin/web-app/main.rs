pub mod model;

use seed::{prelude::*, *};

// ------ ------
//     Init
// ------ ------

fn init(_: Url, _: &mut impl Orders<Msg>) -> Model {
    Model {
        notes: model::Model::new(),
        selected_note_filename: "note_1.txt".to_string(),
    }
}

// ------ ------
//     Model
// ------ ------

struct Model {
    notes: model::Model,
    selected_note_filename: String,
}

// ------ ------
//    Update
// ------ ------

enum Msg {
    TitleChanged(String),
    ContentsChanged(String),
    NoteSelected(String)
}

fn update(msg: Msg, model: &mut Model, _: &mut impl Orders<Msg>) {
    match msg {
        Msg::TitleChanged(new_title) => {
            let current_note = &model
                .notes
                .notes[&model.selected_note_filename];
            let mut updated_note = current_note.clone();
            updated_note.title = new_title;
            *model
                .notes
                .notes
                .entry(model.selected_note_filename.to_string())
                .or_insert(model::Note::new()) = updated_note;
        }
        Msg::ContentsChanged(new_contents) => {
            let current_note = &model
                .notes
                .notes[&model.selected_note_filename];
            let mut updated_note = current_note.clone();
            updated_note.body = new_contents;
            *model
                .notes
                .notes
                .entry(model.selected_note_filename.to_string())
                .or_insert(model::Note::new()) = updated_note;
        }
        Msg::NoteSelected(filename) => {
            model.selected_note_filename = filename
        }
    }
}

// ------ ------
//     View
// ------ ------

fn view(model: &Model) -> Node<Msg> {
    div![
        note_editor(
            &model
                .notes
                .notes[&model.selected_note_filename],
        ),
        div![ul![model.notes.notes.keys().map(|filename| {
            let selected_note_filename = filename.to_string();
            li![filename, ev(Ev::Click, move |_| Msg::NoteSelected(selected_note_filename))]
        })]]
    ]
}

fn note_editor(note: &model::Note) -> Node<Msg> {
    div![
        input![
            C!["edit"],
            attrs! {At::Value => note.title.clone()},
            input_ev(Ev::Input, |value| { Msg::TitleChanged(value) })
        ],
        textarea![
            C!["edit"],
            attrs! {At::Value => note.body.clone()},
            input_ev(Ev::Input, |value| { Msg::ContentsChanged(value) })
        ],
    ]
}

fn main() {
    App::start("app", init, update, view);
}
