use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str;
use string_join::Join;

#[derive(Serialize, Deserialize, Clone)]
pub struct Model {
    pub notes: HashMap<String, Note>,
}

impl Model {
    pub fn new() -> Model {
        Model {
            notes: HashMap::from([
                (
                    "note_1.txt".to_string(),
                    Note {
                        title: "Example note 1".to_string(),
                        body: "Some text".to_string(),
                    },
                ),
                (
                    "note_2.txt".to_string(),
                    Note {
                        title: "Example note 2".to_string(),
                        body: "Some text\nwith multiple lines".to_string(),
                    },
                ),
            ]),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Note {
    pub title: String,
    pub body: String,
}

impl Note {
    pub fn new() -> Note {
        return Note {
            title: "".to_string(),
            body: "".to_string(),
        };
    }
}

impl fmt::Display for Model {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "=============== Model ===============\n\n{}\n\n=====================================\n",
            "\n\n---------------------\n\n".join(
                self.notes
                    .iter()
                    .map(|element| format!("{:?}\n\n{}", element.0, element.1)),
            )
        )
    }
}

impl fmt::Display for Note {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}\n\n{}", self.title, self.body)
    }
}
