use std::{fs, path::PathBuf};

/// The browser module provides a frontend accessible by http
/// The following functionality are exposed to other modules
/// start_browser

use axum::{
    extract::{Path}, http::Uri, response::Html, routing::get, Router
};

use crate::{gemini::{get_gemini, StatusCode}, gemtext::gemtext_to_html};

/// Starts a HTTP server that acts as a proxy between gemini servers and the user interacting via a browser
/// This is a blocking function.
pub fn start_browser()  {
    let _ = _browser_setup_directory();
    match tokio::runtime::Runtime::new() {
        Ok(runtime) => {
            runtime.block_on(start_axum());
        },
        Err(e) => {
            println!("Error launching tokio runtime: {e}\nGoodbye!")
        }
    }
}

async fn start_axum(){
    let app = Router::new()
        .route("/", get(get_home))
        .route("/{*url}", get(get_normal))
        ;
        // .route("/gemini/error/{code}", get(get_error))

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1965").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn _browser_setup_directory() -> Result<PathBuf, std::io::Error> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory does not exist?")
    })?;
    let dioscuri_dir = home_dir.join(".dioscuri/browser");
    if !dioscuri_dir.exists() {
        fs::create_dir_all(&dioscuri_dir)?;
        println!("Creating directory: {:?}", dioscuri_dir);
    }
    Ok(dioscuri_dir)
}

/// returns .dioscuri/browser/home.html, else a default if not found
async fn get_home() -> Html<String>{
    // Try to get the user's home directory
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            return Html("<h1>Error!</h1><p>Could not determine home directory.</p>".to_string());
        }
    };

    // full path to ~/.dioscuri/browser/home.html
    let file_path: PathBuf = home_dir.join(".dioscuri/browser/home.html");

    match fs::read_to_string(&file_path) {
        Ok(contents) => Html(contents),
        Err(_) => Html("
        <h1>Welcome to Project Dioscuri!</h1>
            <h2>A hackable, accessible Gemini client.</h2>
            <p>This is the default homepage.</p>
            <p>Try browsing with some of these links:</p>
            <ul>
                <li><a href=\"/geminiprotocol.net/\">geminiprotocol.net (Gemini Protocol)</a></li>
                <li><a href=\"/kennedy.gemi.dev\">kennedy.gemi.dev (Kennedy Search Engine)</a></li>
                <li><a href=\"/bbs.geminispace.org\">bbs.geminispace.org (Gemini BBS)</a></li>
            </ul>
        "
        .to_string()),
    }
}

/// Given a query {foo}={bar}, where bar can include more queries,
/// return {bar}
/// if no '=' delimiter exists, then return query
fn strip_first_url_query_key(query: String) -> String {
    if let Some((_, right)) = query.split_once('=') {
        right.to_string()
    } else {
        query
    }
}

/// Receives a url and handles it.
/// If the url has no query, forward it to the gemini server and return the result.
/// If the url has a query, only process the first query if multiple exist as per protocol specification
async fn get_normal(
    Path(url): Path<String>,
    uri: Uri,
) -> Html<String> {
    // Check if there are any query parameters
    let mut gem_url = url.clone();
    if let Some(q) = uri.query() {
        let query = strip_first_url_query_key(q.to_string());
        gem_url = format!("{url}?{query}");
    } 
    let (status, header, body) = get_gemini(gem_url);
    match status {
        StatusCode::Success => {
            return Html(gemtext_to_html(body));
        },
        StatusCode::InputExpected => {
            let str = format!(
                "{header}<br><form method=\"get\"><label><input type=\"text\" name=\"query\"></label><input type=\"submit\" value=\"Submit\"></form>"
            );
            return Html(str);
        },
        StatusCode::InputSensitive => {
            let str = format!(
                "{header}<br><form method=\"get\"><label><input type=\"text\" name=\"query\"></label><input type=\"submit\" value=\"Submit\"></form>"
            );
            return Html(str);
        },
        _ => {
            // error
            println!("{},{}", header, status.as_str());
            return Html("oops!".to_string())
        }
    } 
}

#[cfg(test)]
mod tests {
    use crate::browser::strip_first_url_query_key;

    #[test]
    fn test_strip_first_url_query_key(){
        let in0 = "hello_world".to_string();
        let out0 = "hello_world"; // no '='
        assert_eq!(strip_first_url_query_key(in0), out0);

        let in1 = "foo=bar".to_string();
        let out1 = "bar";
        assert_eq!(strip_first_url_query_key(in1), out1);

        let in2 = "bar=baz&x=1&y=2".to_string();
        let out2 = "baz&x=1&y=2";
        assert_eq!(strip_first_url_query_key(in2), out2);

        let in3 = "q=my query&q2=my second query".to_string();
        let out3 = "my query&q2=my second query";
        assert_eq!(strip_first_url_query_key(in3), out3);

        let in4 = "onlykey=".to_string();
        let out4 = "";
        assert_eq!(strip_first_url_query_key(in4), out4);
    }
}