extern crate sapphbot;

use sapphbot::bot;

fn main() {
    let mut bot = bot::Bot::new().unwrap();
    bot.run().unwrap();
}
