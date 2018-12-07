use std::fs::File;
use std::io::Write;
use std::{thread, time};
use std::collections::HashSet;
use std::error::Error;
use super::data;

pub struct Bot {
    token: String,
    offset: i64,
    chats: HashSet<Chat>,
    images: HashSet<i64>,
    last_update: time::Instant
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct Chat {
    pub chat_name: String,
    pub filter: String,
    pub requester: i64
}

impl Chat {
    fn new(name: &str, re: &str, requester: i64) -> Chat {
        Chat {
            chat_name: name.to_owned(),
            filter: re.to_owned(),
            requester
        }
    }
}

const TOKEN_PATH: &str = "token.txt";
const OFFSET_PATH: &str = "update_offset.txt";
const CHATS_PATH: &str = "chats.json";
const IMAGES_PATH: &str = "images.json";
#[derive(Debug, Serialize, Deserialize)]
struct Reply {
    ok: bool,
    result: Vec<Update>, 
}

#[derive(Debug, Serialize, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    from: User,
    message_id: i64,
    chat: ServerChat,
    text: Option<String>
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerChat {
    id: i64,
}

fn read_to_string(path: &str) -> Result<String, ErrorString> {
    match std::fs::read_to_string(&path) {
        Ok(string) => Ok(string),
        Err(err) => Err(ErrorString(format!("Error reading file {}\n{:?}", path, err)))
    }
}

#[derive(Debug)]
struct ErrorString(String);
impl Error for ErrorString {
}

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
            Err(_) => HashSet::new() 
        };
        let images: HashSet<i64> = match read_to_string(IMAGES_PATH) {
            Ok(images_json) => serde_json::from_str(&images_json)?,
            Err(_) => HashSet::new()
        };
        
        let last_update = time::Instant::now();
        Ok(Bot{token,
            offset,
            chats,
            images,
            last_update,
        })
    }

    pub fn run(&mut self) -> Result<(),String> {
        'updates: loop {
            let update_url = &self.url("getUpdates", &[
                                       ("allowed_updates", &json_array(&["message"])),
                                       ("offset", &self.offset.to_string())
            ]);
            let resp = reqwest::get(update_url).unwrap().text().unwrap();
            let parsed = serde_json::from_str::<Reply>(&resp); 
            if let Ok(val) = parsed {
                let mut messages = self.get_text_messages(val);
                for message in &mut messages {
                    self.handle_command(&message);

                    i!("Received message", message);
                }
                if time::Instant::now() - self.last_update >= time::Duration::from_secs(10) {
                    self.update_chats();
                    self.last_update = time::Instant::now();
                }
            } else {
                w!("Got bad message from server", parsed, resp);
            }

            thread::sleep(time::Duration::from_millis(100))
        }
    }

    fn get_text_messages(&mut self, val: Reply) -> Vec<Message> {
        let mut messages: Vec<Message> = Vec::new();
        for upd in &val.result {
            if upd.update_id >= self.offset {
                self.offset = upd.update_id+1;
            }
            match &upd.message {
                Some(message) => if message.text.is_some(){
                    messages.push(message.clone())
                },
                None => ()
            }
        }
        messages
    }

    fn add_to_chats(&mut self, chat: Chat) {
        self.chats.insert(chat);
        self.save_chats();
        
    }

    fn save_chats(&self) {
        match File::create(CHATS_PATH) {
            Ok(mut file) => {
                let json = serde_json::to_string(&self.chats).unwrap();
                match file.write_all(json.as_bytes()) {
                    Ok(_) =>
                        i!(&format!("Written {} bytes to file {}", json.as_bytes().len(), CHATS_PATH)),
                    Err(err) => e!("Could not write to file: ", CHATS_PATH, err)
                };
            },
            Err(err) => {
                e!("Could not write to file", CHATS_PATH, err);
            }
        }
    }
    fn reply_to_message(&self, message: &Message, reply: &str) {
        i!("Sending message to", message.chat.id);
        let reply_url = self.url("sendMessage", &[
                                 ("text", reply),
                                 ("chat_id", &message.chat.id.to_string())
        ]);
        if let Err(err) = reqwest::get(&reply_url) {
            e!("Could not send message", err);
        }
    }

    fn handle_command(&mut self, message: &Message) {
        let raw_text = message.text.clone().unwrap().to_string();
        let mut text = raw_text.split(" ");
        let add_command = "/add";
        let remove_command = "/remove";
        // Get the first part of the command
        if let Some(command) = text.next() {
            if command.starts_with(add_command) {
                self.handle_add_command(text, message)
            } else if command.starts_with(remove_command) {
                self.handle_remove_command(text, &message)
            } else {
                self.reply_to_message(&message, "Invalid command")
            }
        }
    }

    fn handle_remove_command<'a, T: Iterator<Item=&'a str>>(&mut self,
                                                            mut text: T,
                                                            message: &Message) {
        if let Some(filter) = text.next() {
            let mut get_chat_name = || {
                if let Some(chat) = text.next() {
                    chat.to_string()
                } else {
                    message.chat.id.to_string()
                }
            };

           let chat_name = get_chat_name();
            if self.chats.remove(&Chat::new(&chat_name, filter, message.from.id)) {
                self.reply_to_message(
                    message,
                    &format!("Successfully removed chat {} with filter {} from posting list.",
                            chat_name, 
                            filter));
            } else {
                self.reply_to_message(
                    message, "No matching setting found");
            }

        // No argument; removes all for that chat
        } else {
            let mut new_chats = HashSet::new();
            for chat in &self.chats {
                if chat.requester != message.from.id || 
                    chat.chat_name != message.chat.id.to_string() {
                        new_chats.insert(chat.clone());
                }
            }
            self.chats=new_chats;
        }
        self.save_chats()
    }

    fn handle_add_command<'a, T: Iterator<Item=&'a str>>(&mut self,
                                                         mut  text: T,
                                                         message: &Message) {
        if let Some(filter) = text.next() {
            // Figure out the chat name
            let chat_name = match text.next() {
                // Third argument; take it as chat name
                Some(name) => {
                    self.reply_to_message(
                        &message,
                        &format!("Added {} to list of chats, with filter {}",
                                 name, filter));
                    name.to_string()
                }
                // No third argument; take the message where it was posted
                None => {
                    self.reply_to_message(
                        &message,
                        &format!("Added this chat to list of chats, with filter {}", filter));
                    message.chat.id.to_string()
                }
            };

            // Add it to the list
            self.add_to_chats(Chat::new(&chat_name, filter, message.from.id));
        } else {
            self.reply_to_message(&message, "Missing argument; need a filter");
        }
    }

    fn update_chats(&mut self) {
        let new_images = self.compute_new_images();
        if new_images.len() > 0 {
            i!(&format!("Got {} new images", new_images.len()));
        }
        for chat in self.chats.clone() {
            for image in &new_images {
                if tags_fit(&image.tags, &chat.filter) {
                    let image_origin = format!("http:{}", image.representations.large);
                    let image_source = format!("http://derpibooru.org/{}", image.id);
                    let caption = format!("{}%0A{}%0A{}",
                                          &image_source,
                                          get_artist(&image.tags),
                                          image.source_url);
                    let message_url = self.url("sendPhoto", &[
                                               ("chat_id", &chat.chat_name),
                                               ("photo", &image_origin),
                                               ("caption", &caption),
                                               //("parse_mode", "html")
                    ]);
                    i!(&format!("Sending picture to {}", chat.chat_name));
                    let resp = reqwest::get(&message_url);
                        match resp {
                            Err(err) => e!("Could not communicate with server", err),
                            Ok(ok) => i!("Received response from server", ok)
                        }
                }
            }
        }
    }

    fn compute_new_images(&mut self) -> HashSet<data::ImageResponse> {
        let images = data::get_images();
        let mut new_images = HashSet::new();
        let mut new_image_ids = HashSet::new();
        for image in images {
            if !self.images.contains(&image.id) {
                new_image_ids.insert(image.id);
                new_images.insert(image);
            }
        }

        self.images=self.images.union(&new_image_ids).cloned().collect();
        match File::create(IMAGES_PATH) {
            Ok(mut file) => {
                let json = serde_json::to_string(&self.images).unwrap();
                if let Err(err) = file.write_all(&json.as_bytes()) {
                    e!("Could not write to file", IMAGES_PATH, err)
                }
            },
            Err(err) => e!("Could not write to file", IMAGES_PATH, err)
        }
        new_images
    }

    fn url(&self, method_name: &str, args: &[(&str,&str)]) -> String {
        let arg_string = args.iter()
            .map(|(p1,p2)| format!("{}={}", p1,p2))
            .collect::<Vec<_>>().join("&");
        let formatted_arg_string = 
            if arg_string.is_empty() {
                "".to_string()
            } else {
                "?".to_string() + &arg_string
            };
        format!("https://api.telegram.org/bot{}/{}{}",
                self.token,
                method_name,
                formatted_arg_string)
    }
}

fn get_artist(tags: &str) -> String {
    for dirty_tag in tags.split(',') {
        let tag = dirty_tag.trim();
        if tag.contains("artist:") {
                return tag.to_string()
            }
    }
    "".to_string()
}

fn tags_fit(tags: &str, filter: &str) -> bool {
    if filter == "any" { 
        true
    }
    else {
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
        Nil => false
    }
}

// Unnecessary, but fun
enum List<T> {
    Cons(T, Box<List<T>>),
    Nil
}


fn json_array(args: &[&str]) -> String {
    let formatted_args = args.iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>().join(", ");
    format!("[{}]", formatted_args)
}

