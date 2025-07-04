use std::io::{self};

use crate::browser::start_browser;

mod gemini;
mod tofu;
mod browser;
mod gemtext;

// fn main() -> io::Result<()> {
fn main() {
    println!("Welcome to Project Dioscuri!\nAccess gemini by opening http://localhost:1965/ on any web browser!");
    start_browser();
}
