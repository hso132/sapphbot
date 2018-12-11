use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::{thread, time};

mod communicator;
mod derpiquery;
pub struct Bot {
    chats: HashSet<Chat>,
    last_update: time::Instant,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct Chat {
    pub chat_name: String,
    pub filter: String,
    pub requester: i64,
}

impl Chat {
    fn new(name: &str, re: &str, requester: i64) -> Chat {
        Chat {
            chat_name: name.to_owned(),
            filter: re.to_owned(),
            requester,
        }
    }
}

const TOKEN_PATH: &str = "token.txt";
const OFFSET_PATH: &str = "update_offset.txt";
const CHATS_PATH: &str = "chats.json";
const IMAGES_PATH: &str = "images.json";
fn read_to_string(path: &str) -> Result<String, ErrorString> {
    match std::fs::read_to_string(&path) {
        Ok(string) => Ok(string),
        Err(err) => Err(ErrorString(format!(
            "Error reading file {}\n{:?}",
            path, err
        ))),
    }
}

#[derive(Debug)]
struct ErrorString(String);
impl Error for ErrorString {}

impl std::fmt::Display for ErrorString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let ErrorString(ref string) = &self;
        formatter.write_str(string)
    }
}

impl Bot {
    pub fn new() -> Result<Bot, Box<Error>> {
        let token = read_to_string(TOKEN_PATH)?;
        let offset: i64 = read_to_string(OFFSET_PATH)?.trim().parse()?;
        let chats: HashSet<Chat> = match read_to_string(CHATS_PATH) {
            Ok(chats_json) => serde_json::from_str(&chats_json)?,
            Err(_) => HashSet::new(),
        };

        let last_update = time::Instant::now();
        Ok(Bot {
            chats,
            last_update,
        })
    }
}

fn get_artist(tags: &str) -> String {
    for dirty_tag in tags.split(',') {
        let tag = dirty_tag.trim();
        if tag.contains("artist:") {
            return tag.to_string();
        }
    }
    "".to_string()
}

fn tags_fit(tags: &str, filter: &str) -> bool {
    if filter == "any" {
        true
    } else {
        use self::List::*;
        let mut list = Nil;
        for dirty_tag in tags.split(',') {
            let tag = dirty_tag.trim();
            list = Cons(tag, Box::new(list));
        }
        tags_fit_list(list, filter)
    }
}

fn tags_fit_list(tags: List<&str>, filter: &str) -> bool {
    use self::List::*;
    match tags {
        Cons(string, rest) => {
            if string == filter {
                true
            } else {
                tags_fit_list(*rest, filter)
            }
        }
        Nil => false,
    }
}

// Unnecessary, but fun
enum List<T> {
    Cons(T, Box<List<T>>),
    Nil,
}

fn json_array(args: &[&str]) -> String {
    let formatted_args = args
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]", formatted_args)
}
