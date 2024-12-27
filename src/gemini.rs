
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, net::TcpStream};

/// This file contains the application layer logic for the Gemini protocol
/// There are a few specific things that this file does
/// 1. Define Client -> Server interactiomns
/// 2. Define Server responses
/// 3. Handle TLS

const CRLF: &str = "\r\n";
const REQUEST_PREAMBLE: &str = "gemini://";
const GEMINI_TCP_PORT: &str = "1965";
fn build_request_payload(address: &String, uri_path: &String) -> String {
    "\"".to_string() + REQUEST_PREAMBLE + address + uri_path + "/\" " + CRLF
}

/// Get the uri content from the gemini server
/// returns: String that server responds with, else error if any occured
/// TODO: proper error handling
pub async fn request(address: String, uri_path: String) -> String {
    // Open TCP connection with the gemini server
    // send the GET request
    let socket_address = address.clone() + ":" + GEMINI_TCP_PORT;
    let mut stream = match TcpStream::connect(socket_address).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("TCP connection error: {}", err.to_string());
            return err.to_string()
        }
    };
    // Get the response  
    let payload = build_request_payload(&address, &uri_path);
    println!("Payload: {}", payload);
    if let Err(err) = stream.write_all(&payload.into_bytes()).await {
        println!("Error writing to server: {}", err);
        return err.to_string();
    };
    println!("Written to server. Waiting for response...");
    // TODO: get the response from the server and handle it
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    let mut line_buf = String::new();
    while reader.read_line(&mut line_buf).await.unwrap() != 0 {
        println!("{}", line_buf);
        response.push_str(&line_buf);
        line_buf.clear();
    }
    println!("Done!");
    return response;
}
