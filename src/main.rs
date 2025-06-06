use std::io::{self};

mod gemini;
mod tofu;

fn main() -> io::Result<()> {
    let url = "gemini://geminiprotocol.net/software/";
    let (code, header, body) = gemini::get_gemini(url.to_string());
    println!("CODE:{:?}\nHEADER:{}\n{}", code, header, body);
    Ok(())
}
