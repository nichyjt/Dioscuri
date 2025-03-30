use std::{io::{self, Read, Write}, net::TcpStream};

use native_tls::TlsConnector;

fn client_build_request_str(uri: &str) -> String {
    format!("{}\r\n", uri)
}

fn main() -> io::Result<()> {
    let addr = "geminiprotocol.net:1965";
    let uri = "gemini://geminiprotocol.net/";

    let stream = TcpStream::connect(addr)?;
    
    // All gemini communication uses TLS.
    // TODO: Implement TOFU and store certs in ~/.dioscuri
    //       TOFU module should handle all cert handlers
    let connector = TlsConnector::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .unwrap();

    let mut stream = connector.connect("geminiprotocol.net", stream).unwrap();

    let request = client_build_request_str(uri);
    let _ = stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    println!("Response:\n{}", response);
    Ok(())
}
