extern crate sapphbot;

use sapphbot::bot;

fn main() {
    let mut bot = match bot::Bot::new() {
        Err(err) => {
            println!("Error initiating bot. \n{:?}", err);
            std::process::exit(1)
        }
        Ok(bot) => bot,
    };

    match bot.run() {
        Err(err) => println!("The bot has abruptly stopped. \n{:?}", err),
        _ => (),
    }
}
