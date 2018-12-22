use super::*;
use std::time;
use std::sync::mpsc::Receiver;
use std::io::Write;
use super::derpiquery::data::*;

#[derive(Debug, Serialize, Deserialize)]
struct Reply {
    ok: bool,
    result: Vec<Update>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    from: User,
    message_id: i64,
    chat: ServerChat,
    text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerChat {
    id: i64,
}


pub struct Communicator {
    offset: i64,
    token: String,
    chats: HashSet<Chat>,
}

impl Communicator {
    pub fn new(offset: i64, token: String, chats: HashSet<Chat>) -> Communicator  {
        Communicator{offset, token, chats}
    }
    pub fn run(&mut self, receiver: Receiver<Vec<ImageResponse>>) {
        'updates: loop {
            let update_url = &self.url(
                "getUpdates",
                &[
                    ("allowed_updates", &json_array(&["message"])),
                    ("offset", &self.offset.to_string())
                ]);
            let try_resp: Result<_, _> = reqwest::get(update_url);
            if let Ok(mut raw_resp) = try_resp {
                let try_text: Result<_, _> = raw_resp.text();
                if let Ok(text) = try_text {
                    let parsed = serde_json::from_str::<Reply>(&text);
                    if let Ok(val) = parsed {
                        let mut messages = self.get_messages_from_reply(val);
                        for message in &mut messages {
                            self.handle_command(&message);

                            i!("Received message", message);
                        }
                    } else {
                        w!("Got bad message from server", parsed, text);
                    }
                } else {
                    w!("Error while getting the text", raw_resp.status())
                }
            } else {
                e!("Could not communicate with server", try_resp);
            }

            if let Ok(new_images) = receiver.try_recv() {
                self.update_chats(new_images);
            }

            thread::sleep(time::Duration::from_millis(100))
        }
    }

    fn url(&self, method_name: &str, args: &[(&str, &str)]) -> String {
        let arg_string = args
            .iter()
            .map(|(p1, p2)| format!("{}={}", p1, p2))
            .collect::<Vec<_>>()
            .join("&");
        let formatted_arg_string = if arg_string.is_empty() {
            "".to_string()
        } else {
            "?".to_string() + &arg_string
        };
        format!(
            "https://api.telegram.org/bot{}/{}{}",
            self.token, method_name, formatted_arg_string
            )
    }

    /// Parses messages in the reply, and returns them if they contain text
    fn get_messages_from_reply(&mut self, val: Reply) -> Vec<Message> {
        let mut messages: Vec<Message> = Vec::new();
        for upd in &val.result {
            if upd.update_id >= self.offset {
                self.offset = upd.update_id + 1;
            }
            match &upd.message {
                Some(message) => {
                    if message.text.is_some() {
                        messages.push(message.clone())
                    }
                }
                None => (),
            }
        }
        messages
    }

    fn update_chats(&mut self, new_images: Vec<ImageResponse>) {
        if new_images.len() > 0 {
            i!(&format!("Got {} new images", new_images.len()));
        }
        for chat in &self.chats {
            for image in &new_images {
                if tags_fit(&image.tags, &chat.filter) {
                    let image_origin = format!("http:{}", image.representations.large);
                    let image_source = format!("http://derpibooru.org/{}", image.id);
                    let caption = format!(
                        "{}%0A{}%0A{}",
                        &image_source,
                        get_artist(&image.tags),
                        image.source_url
                    );
                    let message_url = self.url(
                        "sendPhoto",
                        &[
                            ("chat_id", &chat.chat_name),
                            ("photo", &image_origin),
                            ("caption", &caption),
                            //("parse_mode", "html")
                        ],
                    );
                    i!(&format!("Sending picture with id {} to {}", image.id, chat.chat_name), image.sha512_hash);
                    let resp = reqwest::get(&message_url);
                    match resp {
                        Err(err) => e!("Could not communicate with server", err),
                        Ok(ok) => i!("Received response from server", ok),
                    }
                }
            }
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

    fn handle_remove_command<'a, T: Iterator<Item = &'a str>>(
        &mut self,
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

            let chat_name: String = get_chat_name();
            if self.chats.remove(&Chat::new(&chat_name, filter, message.from.id)) {
                self.reply_to_message(
                    message,
                    &format!(
                        "Successfully removed chat {} with filter {} from posting list.",
                        chat_name, filter
                        ),
                        );
            } else {
                self.reply_to_message(message, "No matching setting found");
            }

            // No argument; removes all for that chat
        } else {
            let cond = |c: &Chat| {
                c.requester != message.from.id || c.chat_name != message.chat.id.to_string()
            };
            self.chats.retain(cond)
        }
        // In either case, save the chats
        save_chats(&self.chats)
    }

    fn handle_add_command<'a, T: Iterator<Item = &'a str>>(
        &mut self,
        mut text: T,
        message: &Message,
        ) {
        if let Some(filter) = text.next() {
            // Figure out the chat name
            let chat_name = match text.next() {
                // Third argument; take it as chat name
                Some(name) => {
                    self.reply_to_message(
                        &message,
                        &format!("Added {} to list of chats, with filter {}", name, filter),
                        );
                    name.to_string()
                }
                // No third argument; take the message where it was posted
                None => {
                    self.reply_to_message(
                        &message,
                        &format!("Added this chat to list of chats, with filter {}", filter),
                        );
                    message.chat.id.to_string()
                }
            };

            // Add it to the list
            self.add_to_chats(Chat::new(&chat_name, filter, message.from.id));
        } else {
            self.reply_to_message(&message, "Missing argument; need a filter");
        }
    }

    fn reply_to_message(&self, message: &Message, reply: &str) {
        i!("Sending message to", message.chat.id);
        let reply_url = self.url(
            "sendMessage",
            &[("text", reply), ("chat_id", &message.chat.id.to_string())],
        );
        if let Err(err) = reqwest::get(&reply_url) {
            e!("Could not send message", err);
        }
    }



    fn add_to_chats(&mut self, chat: Chat) {
        self.chats.insert(chat);
        save_chats(&self.chats);
    }
}

fn save_chats(chats: &HashSet<Chat>) {
    match File::create(CHATS_PATH) {
        Ok(mut file) => {
            let json = serde_json::to_string(chats).unwrap();
            match file.write_all(json.as_bytes()) {
                Ok(_) => i!(&format!(
                        "Written {} bytes to file {}",
                        json.as_bytes().len(),
                        CHATS_PATH
                        )),
                Err(err) => e!("Could not write to file: ", CHATS_PATH, err),
            };
        }
        Err(err) => {
            e!("Could not write to file", CHATS_PATH, err);
        }
    }
}
