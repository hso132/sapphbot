#[macro_use]
extern crate serde_derive;

#[macro_use]
pub mod log {
    use std::io::Write;

    macro_rules! log {
        ($prefix: expr, $($x: expr),*) => {
            {
                let mut vec = Vec::new();
                $(
                    vec.push(format!("{}{:?}", $prefix, $x));
                 )*
                vec.join("\n")
            }
        }
    }

    macro_rules! e {
        ($($x: expr),*) => {{
            ::log::log(&log!("[ Error ]", $($x),*))}
        }}
    macro_rules! w {
        ($($x: expr),*) => {{
            ::log::log(&log!("[Warning]", $($x),*))}
        }}
    macro_rules! i {
        ($($x: expr),*) => {{
            ::log::log(&log!("[ Info  ]", $($x),*))}
        }}

    pub fn log(message: &str) -> () {
        println!("{}", message);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("log.txt")
        {
            let _ = file.write_all(message.as_bytes());
        }
    }
}
pub mod bot;
