use std::{io::{Read, Write}, net::TcpStream};

use native_tls::{TlsConnector, TlsStream};
use url::{form_urlencoded, Url};

use crate::tofu;

/// This file implements the Gemini protocol
/// It takes in a url/uri and returns either (data, status code) or an error string.

const CRLF: &str = "\r\n";

#[derive(Debug, PartialEq)] // allow debug and comparisons
pub enum StatusCode {
    InputExpected,
    InputSensitive,
    Success,
    RedirectTemp,
    RedirectPerm,
    FailureServerTemp,
    FailureServerUnavailable,
    FailureServerCgiError,
    FailureServerProxyError,
    FailureServerSlowdown,
    FailureServer,
    FailureServerNotfound,
    FailureServerGone,
    FailureServerProxyrefused,
    FailureServerBadReq,
    FailureCertNeeded,
    FailureCertUnauthorized,
    FailureCertInvalid,
    StatusUnknown,
    FailureClient,
    ResponseError,
}

impl StatusCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusCode::InputExpected => "Input Expected",
            StatusCode::InputSensitive => "Input Sensitive",
            StatusCode::Success => "Success",
            StatusCode::RedirectTemp => "Temporary Redirect",
            StatusCode::RedirectPerm => "Permanent Redirect",
            StatusCode::FailureServerTemp => "Temporary Server Failure",
            StatusCode::FailureServerUnavailable => "Server Unavailable",
            StatusCode::FailureServerCgiError => "CGI Error",
            StatusCode::FailureServerProxyError => "Proxy Error",
            StatusCode::FailureServerSlowdown => "Server Slowdown",
            StatusCode::FailureServer => "Permanent Server Failure",
            StatusCode::FailureServerNotfound => "Not Found",
            StatusCode::FailureServerGone => "Gone",
            StatusCode::FailureServerProxyrefused => "Proxy Refused",
            StatusCode::FailureServerBadReq => "Bad Request",
            StatusCode::FailureCertNeeded => "Client Certificate Needed",
            StatusCode::FailureCertUnauthorized => "Client Certificate Unauthorized",
            StatusCode::FailureCertInvalid => "Client Certificate Invalid",
            StatusCode::StatusUnknown => "Unknown Status",
            StatusCode::FailureClient => "Client Failure",
            StatusCode::ResponseError => "Response Error",
        }
    }
}

impl From<i32> for StatusCode {
    fn from(code: i32) -> Self {
        match code {
            10 => StatusCode::InputExpected,
            11 => StatusCode::InputSensitive,
            12..=19 => StatusCode::InputExpected,
            20..=29 => StatusCode::Success,
            30 => StatusCode::RedirectTemp,
            31 => StatusCode::RedirectPerm,
            32..=39 => StatusCode::RedirectPerm,
            40 => StatusCode::FailureServerTemp,
            41 => StatusCode::FailureServerUnavailable,
            42 => StatusCode::FailureServerCgiError,
            43 => StatusCode::FailureServerProxyError,
            44 => StatusCode::FailureServerSlowdown,
            45..=49 => StatusCode::FailureServerTemp,
            50 => StatusCode::FailureServer,
            51 => StatusCode::FailureServerNotfound,
            52 => StatusCode::FailureServerGone,
            53 => StatusCode::FailureServerProxyrefused,
            54..=58 => StatusCode::FailureServer,
            59 => StatusCode::FailureServerBadReq,
            60 => StatusCode::FailureCertNeeded,
            61 => StatusCode::FailureCertUnauthorized,
            62 => StatusCode::FailureCertInvalid,
            63..=69 => StatusCode::FailureCertNeeded,
            _ => StatusCode::StatusUnknown
        }
    }
}


/// Given a url of format(s):
/// 1. {protocol}://address/*
/// 2. address/*
/// 
/// return only the 'address/*' part
fn _strip_protocol_from_url(url: &String) -> String {
    let mut temp_url = url.clone();
    // 1. Check if the URL starts with a protocol (e.g., "http://", "gemini://").
    // If it does, remove the protocol and the "://" part.
    if let Some(protocol_end_idx) = temp_url.find("://") {
        // We add 3 to the index to skip "://"
        temp_url = temp_url[(protocol_end_idx + 3)..].to_string();
    }
    temp_url
}
/// Given a url of format(s):
/// 1. {protocol}://address/*  
/// 2. address/* 
///  
/// Extract 'address' and return it as a String
fn _extract_address_from_url(url: &String) -> String {
    let temp_url = _strip_protocol_from_url(url);

    // Find the index of the first slash '/' in the remaining string.
    // The address part will be the substring before this slash.
    if let Some(slash_idx) = temp_url.find('/') {
        // Return the substring from the beginning of temp_url up to the first slash.
        temp_url[..slash_idx].to_string()
    } else {
        temp_url
    }
}

/// Given a string of format:
/// {foo}/{bar}/{baz}
/// urlencode baz and return 
/// {foo}/{bar}/{baz'}, where baz' is the urlencoded slug
/// if baz contains a query (i.e. ?), preserve the first ? and urlencode the rest of the input
fn encode_url_suffix(path_str: String) -> String {
    let (prefix_slice, baz_slice) = if let Some(last_slash_idx) = path_str.rfind('/') {
        // If a slash is found, split into the part before it and the part after it.
        // `last_slash_idx + 1` ensures we start after the slash for `baz_slice`.
        (&path_str[..last_slash_idx], &path_str[last_slash_idx + 1..])
    } else {
        // If no slash, the entire string is considered the 'baz' part, with an empty prefix.
        ("", &path_str[..])
    };

    // If baz_slice contains '?', split and encode only the RHS
    let encoded_baz = if let Some((before_q, after_q)) = baz_slice.split_once('?') {
        let encoded_right: String = form_urlencoded::byte_serialize(after_q.as_bytes()).collect();
        format!("{}?{}", before_q, encoded_right)
    } else {
        form_urlencoded::byte_serialize(baz_slice.as_bytes()).collect()
    };

    if prefix_slice.is_empty() {
        encoded_baz
    } else {
        format!("{}/{}", prefix_slice, encoded_baz)
    }
}

/// Given a response payload, extract the response code, header message and body
fn extract_response_header(response: String) -> (StatusCode, String, String) {
    let (header_line, body) = match response.split_once(CRLF) {
        Some((h, b)) => (h, b.to_string()), // Body can be multi-line
        None => {
            // If no CRLF, the response format is invalid.
            return (StatusCode::ResponseError, "Response does not have CRLF!".to_string(), "".to_string());
        }
    };

    // Process the header line
    let (code_str, header_message_str) = match header_line.split_once(" ") {
        Some((code_part, message_part)) => (code_part, message_part),
        None => {
            // If no space is found, the entire header_line is considered the code string,
            // and the header message is empty. This handles cases like "41\r\n".
            (header_line, "")
        }
    };

    // Ensure that the status code is precisely 2 digits
    if code_str.len() != 2 {
        return (StatusCode::StatusUnknown, "Server returned invalid status code!".to_string(), "".to_string());
    }
    let code_parsed = code_str.parse::<i32>();
    if code_parsed.is_err() {
        // invalid parse (non numeric)
        return (StatusCode::StatusUnknown, "Server returned invalid status code!".to_string(), "".to_string());
    }   
    let code_num = code_parsed.unwrap();
    if code_num < 10 || code_num > 69 {
        // status code not within 10 and 69 (undefined based on specs)
        return (StatusCode::StatusUnknown, "Server returned invalid status code!".to_string(), "".to_string());
    }
    let status = StatusCode::from(code_num);

    return (status, header_message_str.to_string(), body);
}

/// Return the uri but with \r\n appended
fn client_build_request_str(uri: String) -> String {
    format!("{}\r\n", uri)
}

/// Given a url, get the corresponding (code, header_data, data) tuple
/// The url string can be of format: gemini://{url} or simply {url}
/// Any client-side internal errors will be returned with the appropriate status code.
pub fn get_gemini(url: String) -> (StatusCode, String, String){
    // Extract out the domain/address, port and uri
    let port = 1965;
    let addr = format!("{}:{}", _extract_address_from_url(&url), port);
    let conn_res =  TcpStream::connect(&addr);
    if conn_res.is_err() {
        return (StatusCode::FailureClient, "".to_string(), "TcpStream failed to connect".to_string())
    }
    
    // All gemini communication uses TLS
    let stream = conn_res.unwrap();    
    let connector = TlsConnector::builder()
    .danger_accept_invalid_certs(true)
    .build()
    .unwrap();
    let mut stream = connector.connect(&addr, stream).unwrap();
    let _ = tofu::tofu_handle_certificate(stream.peer_certificate().unwrap().unwrap());

    // Ensure all strings are stripped to maintain a standard format;
    // then urlencode the last section,
    // then re-insert the 'gemini://' prefix
    let url_stripped = _strip_protocol_from_url(&url);

    // WARNING: Suffix encoding is omitted since many actual gemini servers are lazy and don't decode urlencoded payloads.
    // TODO: implement a fallback
    // let encoded_url = encode_url_suffix(url_stripped);
    // let final_url = format!("gemini://{}", encoded_url);
    let final_url = format!("gemini://{}", url_stripped);

    let request = client_build_request_str(final_url.clone());
    if let Err(e) = stream.write_all(request.as_bytes()) {
        return (StatusCode::FailureClient, "".to_string(), format!("Error while writing to TLS stream!\n{}", e).to_string())
    }

    let mut response = String::new();
    if let Err(e) = stream.read_to_string(&mut response) {
        return (StatusCode::FailureClient, "".to_string(), format!("Error while reading from TLS stream!\n{}", e).to_string())
    }
    // No issue with the response.
    let (code, header, body) = extract_response_header(response);
    if code  == StatusCode::RedirectPerm || code == StatusCode::RedirectTemp {
        return handle_redirect(final_url, header, stream)
    }
    return (code, header, body);
}

/// Automatically handle redirects with depth limit
fn handle_redirect(initial_url: String, mut redirect_uri: String, mut stream: TlsStream<TcpStream>) -> (StatusCode, String, String) {
    let mut redirect_depth = 5;
    let mut current_url = initial_url;

    while redirect_depth > 0 {
        // Resolve relative redirect_uri against the current_url
        let resolved = match Url::parse(&current_url)
            .and_then(|base| base.join(&redirect_uri)) {
            Ok(u) => u.to_string(),
            Err(e) => return (
                StatusCode::FailureClient,
                "".to_string(),
                format!("Failed to resolve redirect: {}", e),
            ),
        };

        println!("Redirecting to: {}", resolved);
        if let Err(e) = stream.write_all(resolved.as_bytes()) {
            return (StatusCode::FailureClient, "".to_string(), format!("Error while writing to TLS stream!\n{}", e).to_string())
        }
        let mut response = String::new();
        if let Err(e) = stream.read_to_string(&mut response) {
            return (StatusCode::FailureClient, "".to_string(), format!("Error while reading from TLS stream!\n{}", e).to_string())
        }
        let (code, header, body) = extract_response_header(response);
        // If it's another redirect, continue
        if code == StatusCode::RedirectPerm || code == StatusCode::RedirectTemp {
            current_url = resolved;
            redirect_uri = header.clone(); // follow new redirect location
            redirect_depth -= 1;
            continue;
        }

        // Otherwise, return the result
        return (code, header, body);
    }

    (
        StatusCode::FailureClient,
        "".to_string(),
        "Too many redirects".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_address_extraction() {
        let in0 = "https://foobar.com";
        let out0 = "foobar.com";
        let in1 = "gemini://my-website.com/nonsense?ok";
        let out1 = "my-website.com"; 
        let in2 = "google.com";
        let out2 = "google.com";
        assert_eq!(_extract_address_from_url(&in0.to_string()), out0);
        assert_eq!(_extract_address_from_url(&in1.to_string()), out1);
        assert_eq!(_extract_address_from_url(&in2.to_string()), out2);
    }

    #[test]
    fn test_get_gemini_invalid_domain() {
        let (code, _, _) = get_gemini("this_website_does_not_exist_999.au".to_string());
        assert_eq!(code, StatusCode::FailureClient)
    }

    #[test]
    /// Test the 1* series of responses
    fn test_extract_response_header_10s() {
        let in0 = "10 What is the answer? To life, the universe\n and everything? \r\n".to_string();
        let out0 = (StatusCode::from(10), 
            "What is the answer? To life, the universe\n and everything? ".to_string(), "".to_string());
        assert_eq!(out0, extract_response_header(in0));

        let in1 = "11 P4$$w0rd\npl0x~\r\n".to_string();
        let out1 = (StatusCode::from(11),
            "P4$$w0rd\npl0x~".to_string(), "".to_string());
        assert_eq!(out1, extract_response_header(in1));
        
        let in2 = "12 foo\r\n".to_string();
        let out2 = (StatusCode::from(12),
            "foo".to_string(), "".to_string());
        assert_eq!(out2, extract_response_header(in2));
    }

    #[test]
    /// Test the 2* series of reponses
    fn test_extract_response_header_20s() {
        let in0 = "20 text/html\r\n Hello-!\nWorld\r\n1234#!\";;".to_string();
        let out0 = (StatusCode::from(20),
            "text/html".to_string(), " Hello-!\nWorld\r\n1234#!\";;".to_string());
        assert_eq!(out0, extract_response_header(in0));

        let in1 = "29 some/meme\r\noiiaioiiiai\r".to_string();
        let out1 = (StatusCode::from(29),
            "some/meme".to_string(),"oiiaioiiiai\r".to_string());
        assert_eq!(out1, extract_response_header(in1));
    }

    #[test]
    /// Test the 3* series of responses
    fn test_extract_response_header_30s() {
        // Test case for 30
        let in0 = "30 gemini://example.com/new/path\r\n".to_string();
        let out0 = (StatusCode::from(30),
                    "gemini://example.com/new/path".to_string(),
                    "".to_string()); // Body should be empty for redirects
        assert_eq!(out0, extract_response_header(in0));

        // Test case for 31 (Permanent redirect)
        let in1 = "31 /local/resource\r\n".to_string();
        let out1 = (StatusCode::from(31),
                    "/local/resource".to_string(),
                    "".to_string());
        assert_eq!(out1, extract_response_header(in1));

        // Test case for 32 (Temporary redirect with a more complex URI-reference)
        let in2 = "32 gemini://mirror.gmi/path?query=1&frag#section\r\n".to_string();
        let out2 = (StatusCode::from(32),
                    "gemini://mirror.gmi/path?query=1&frag#section".to_string(),
                    "".to_string());
        assert_eq!(out2, extract_response_header(in2));

        // Test case for 39 (General 3x response with a simple path)
        let in3 = "39 /another/place\r\n".to_string();
        let out3 = (StatusCode::from(39),
                    "/another/place".to_string(),
                    "".to_string());
        assert_eq!(out3, extract_response_header(in3));

        // Edge case: URI-reference with special characters that are still valid
        let in4 = "30 /some/path with spaces/and!symbols.gmi\r\n".to_string();
        let out4 = (StatusCode::from(30),
                    "/some/path with spaces/and!symbols.gmi".to_string(),
                    "".to_string());
        assert_eq!(out4, extract_response_header(in4));
    }

    #[test]
    /// Test the 4* series of responses
    fn test_extract_response_header_40s() {
        // Test case for 40 with an error message
        let in0 = "40 Service Unavailable\r\n".to_string();
        let out0 = (StatusCode::from(40),
                    "Service Unavailable".to_string(),
                    "".to_string());
        assert_eq!(out0, extract_response_header(in0));

        // Test case for 41 without an error message
        let in1 = "41\r\n".to_string();
        let out1 = (StatusCode::from(41),
                    "".to_string(), // Error message is empty
                    "".to_string());
        assert_eq!(out1, extract_response_header(in1));

        // Test case for 42 
        let in2 = "42 CGI error~\r\n".to_string();
        let out2 = (StatusCode::from(42),
                    "CGI error~".to_string(),
                    "".to_string());
        assert_eq!(out2, extract_response_header(in2));

        // Test case for 43
        let in3 = "43 Prox failed(cf http 502,504)\r\n".to_string();
        let out3 = (StatusCode::from(43),
                    "Prox failed(cf http 502,504)".to_string(),
                    "".to_string());
        assert_eq!(out3, extract_response_header(in3));

        // Test case for 44
        let in4 = "44 SLOW DOWN!\r\n".to_string();
        let out4 = (StatusCode::from(44),
                    "SLOW DOWN!".to_string(),
                    "".to_string());
        assert_eq!(out4, extract_response_header(in4));

        // Test case for 49 with a more specific error message
        let in5 = "49 Rate Limit Exceeded: Try again in 60s\r\n".to_string();
        let out5 = (StatusCode::from(49),
                    "Rate Limit Exceeded: Try again in 60s".to_string(),
                    "".to_string());
        assert_eq!(out5, extract_response_header(in5));
    }

    #[test]
    /// Test the 5* series of responsese
    fn test_extract_response_header_50s() {
        // Test case for 50 with an error message
        let in0 = "50 Not Found\r\n".to_string();
        let out0 = (StatusCode::from(50),
                    "Not Found".to_string(),
                    "".to_string());
        assert_eq!(out0, extract_response_header(in0));

        // Test case for 51 with an error message
        let in1 = "51 Bad Request\r\n".to_string();
        let out1 = (StatusCode::from(51),
                    "Bad Request".to_string(),
                    "".to_string());
        assert_eq!(out1, extract_response_header(in1));

        // Test case for 59 without an error message
        let in2 = "59\r\n".to_string();
        let out2 = (StatusCode::from(59),
                    "".to_string(),
                    "".to_string());
        assert_eq!(out2, extract_response_header(in2));
    }

    #[test]
    /// Test the 6* series of responses
    fn test_extract_response_header_60s() {
        // Test case for 60 with an error message
        let in0 = "60 Authentication Required\r\n".to_string();
        let out0 = (StatusCode::from(60),
                    "Authentication Required".to_string(),
                    "".to_string());
        assert_eq!(out0, extract_response_header(in0));

        // Test case for 61 with an error message
        let in1 = "61 Invalid Client Certificate\r\n".to_string();
        let out1 = (StatusCode::from(61),
                    "Invalid Client Certificate".to_string(),
                    "".to_string());
        assert_eq!(out1, extract_response_header(in1));

        // Test case for 69 without an error message
        let in2 = "69\r\n".to_string();
        let out2 = (StatusCode::from(69),
                    "".to_string(),
                    "".to_string());
        assert_eq!(out2, extract_response_header(in2));
    }

    #[test]
    /// Test that status codes from non [1*, 6*] are handled correctly 
    fn test_extract_response_header_unknown_statuscode() {
        let in0 = "71\r\n".to_string();
        let out0 = (StatusCode::from(71),
                    "Server returned invalid status code!".to_string(),
                    "".to_string());
        assert_eq!(out0, extract_response_header(in0));

        let in1 = "84 Hello there!\r\n".to_string();
        let out1 = (StatusCode::from(84),
                    "Server returned invalid status code!".to_string(),
                    "".to_string());
        assert_eq!(out1, extract_response_header(in1));

        let in2 = "99 Brooklyn\r\nCool, cool, cool, cool, cool. No doubt, no doubt, no doubt.".to_string();
        let out2 = (StatusCode::from(84),
                    "Server returned invalid status code!".to_string(),
                    "".to_string());
        assert_eq!(out2, extract_response_header(in2));

        let in3 = "123 Richard\r\nStallman".to_string();
        let out3 = (StatusCode::from(84),
                    "Server returned invalid status code!".to_string(),
                    "".to_string());
        assert_eq!(out3, extract_response_header(in3));

        let in4 = "5 Richard\r\nStallman".to_string();
        let out4 = (StatusCode::StatusUnknown,
                    "Server returned invalid status code!".to_string(),
                    "".to_string());
        assert_eq!(out4, extract_response_header(in4));

        let in5 = "-6 Edgar\r\nDijkstra".to_string();
        let out5 = (StatusCode::StatusUnknown,
                    "Server returned invalid status code!".to_string(),
                    "".to_string());
        assert_eq!(out5, extract_response_header(in5));
    }

    #[test]
    /// Test that the slug encoder works as expected
    /// Test for basic functionality and utf8 encoding
    fn test_encode_url_suffix() {
        assert_eq!(encode_url_suffix("foo/bar/baz".to_string()), "foo/bar/baz");
        assert_eq!(encode_url_suffix("a/b/c".to_string()), "a/b/c");
        assert_eq!(encode_url_suffix("data/document/ドキュメント.pdf".to_string()), "data/document/%E3%83%89%E3%82%AD%E3%83%A5%E3%83%A1%E3%83%B3%E3%83%88.pdf");
        assert_eq!(encode_url_suffix("justthefile.txt".to_string()), "justthefile.txt"); // No slashes, entire string is 'baz'
        assert_eq!(encode_url_suffix("category/subcategory/item/specific file.gem".to_string()), "category/subcategory/item/specific+file.gem");
        assert_eq!(encode_url_suffix("test/query/foo?bar=baz".to_string()), "test/query/foo?bar%3Dbaz");
}    

    #[test]
    fn test_encode_url_suffix_edge_cases() {
        // Empty input string
        assert_eq!(encode_url_suffix("".to_string()), "");
        // Path with spaces
        assert_eq!(encode_url_suffix("/root/path/file with spaces".to_string()), "/root/path/file+with+spaces");
        // Path with spaced subdirectory
        assert_eq!(encode_url_suffix("dir/subdir/ ".to_string()), "dir/subdir/+");
    }
}