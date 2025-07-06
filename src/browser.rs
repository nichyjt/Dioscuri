use std::{fs, path::PathBuf};

/// The browser module provides a frontend accessible by http
/// The following functionality are exposed to other modules

// Constants
static HTML_HEAD_FILENAME: &str = "head.html";
static HTML_BODY_FILENAME: &str = "body.html";
static HTML_HOME_FILENAME: &str = "home.html";
static HTML_DEFAULT_HOMEPAGE: &str = "
<h1>Welcome to Project Dioscuri!</h1>
<h2>A hackable, accessible Gemini client.</h2>
<p>This is the default homepage.</p>
<p>Try browsing with some of these links:</p>
<ul>
    <li><a href=\"/geminiprotocol.net/\">geminiprotocol.net (Gemini Protocol)</a></li>
    <li><a href=\"/kennedy.gemi.dev\">kennedy.gemi.dev (Kennedy Search Engine)</a></li>
    <li><a href=\"/bbs.geminispace.org\">bbs.geminispace.org (Gemini BBS)</a></li>
</ul>
";

static HTML_DEFAULT_INPUT: &str = "
<form method=\"get\"><label><input type=\"text\" name=\"query\"></label><input type=\"submit\" value=\"Submit\"></form>
";

static COMPONENT_MAIN: &str = "<Dioscuri/>";
static COMPONENT_PROMPT: &str = "<DioscuriPrompt/>";
static COMPONENT_INPUT: &str = "<DioscuriInput/>";


use axum::{
    body::Body, extract::Path, http::{self, Uri}, response::{Html, IntoResponse, Response}, routing::get, Router
};

use crate::{gemini::{get_gemini, StatusCode}, gemtext::gemtext_to_html};

/// Starts a HTTP server that acts as a proxy between gemini servers and the user interacting via a browser
/// This is a blocking function.
pub fn start_browser()  {
    _browser_setup_directory();
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
        .route("/.src/{*path}", get(get_resource))
        ;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1965").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Sets up the browser resource directory
fn _browser_setup_directory() {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory does not exist?")
    }).unwrap();
    let dioscuri_dir = home_dir.join(".dioscuri/browser");
    if !dioscuri_dir.exists() {
        fs::create_dir_all(&dioscuri_dir).unwrap();
        println!("Creating directory: {:?}", dioscuri_dir);
    }
}

/// Returns ~/.dioscuri/browser
/// Assumes that the folder has been setup properly
fn get_resource_dir() -> PathBuf {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory does not exist?")
    }).unwrap();
    let dioscuri_dir = home_dir.join(".dioscuri/browser");
    if !dioscuri_dir.exists() {
        let _ = fs::create_dir_all(&dioscuri_dir);
        println!("Creating directory: {:?}", dioscuri_dir);
    }
    dioscuri_dir
}

/// Searches the resource directory for body.html 
/// if does not exist, return ""
fn load_body() -> String {
    let path = get_resource_dir().join(HTML_BODY_FILENAME);
    fs::read_to_string(path).unwrap_or_else(|_| "".to_string())
}

/// Searches the resource directory for head.html
/// if does not exist, return ""
fn load_header() -> String {
    let path = get_resource_dir().join(HTML_HEAD_FILENAME);
    fs::read_to_string(path).unwrap_or_else(|_| "".to_string())
}

/// Searches the resource directory for home.html
/// if does not exist, return HTML_DEFAULT_HOMEPAGE
fn load_home() -> String {
    let path = get_resource_dir().join(HTML_HOME_FILENAME);
    fs::read_to_string(path).unwrap_or_else(|_| HTML_DEFAULT_HOMEPAGE.to_string())
}

/// Loads head.html concat body.html.
/// Once they are concatenated, ensure that the skeleton html contains the injectable tags.
/// If any <Dioscuri/> or <DioscuriPrompt/> or <DioscuriInput/> are missing,
/// append them to the back of the skeleton.
/// this ensures that the skeleton html content can properly render all content regardless
/// of the existence of head.html and body.html
fn load_skeleton() -> String {
    let mut skeleton = format!("{}{}",load_header(), load_body());
    if !skeleton.contains(COMPONENT_MAIN){
        skeleton.push_str(COMPONENT_MAIN);
    }
    if !skeleton.contains(COMPONENT_PROMPT){
        skeleton.push_str(COMPONENT_PROMPT);
    }
    if !skeleton.contains(COMPONENT_INPUT){
        skeleton.push_str(COMPONENT_INPUT);
    }
    skeleton
}

/// returns .dioscuri/browser/home.html, else a default if not found
async fn get_home() -> Html<String>{
    return Html(format!("{}{}",load_header(),load_home()));
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
            let html = gemtext_to_html(body, url);
            let skeleton = load_skeleton();
            // inject the components
            let res = skeleton.replace(COMPONENT_MAIN, &html)
                                    .replace(COMPONENT_INPUT, "")
                                    .replace(COMPONENT_PROMPT, "");
            return Html(res)
        },
        StatusCode::InputExpected => {
            let skeleton = load_skeleton();
            // inject the components
            let res = skeleton.replace(COMPONENT_MAIN, "")
                                    .replace(COMPONENT_INPUT, HTML_DEFAULT_INPUT)
                                    .replace(COMPONENT_PROMPT, &header);
            return Html(res);
        },
        StatusCode::InputSensitive => {
            let skeleton = load_skeleton();
            // inject the components
            let res = skeleton.replace(COMPONENT_MAIN, "")
                                    .replace(COMPONENT_INPUT, HTML_DEFAULT_INPUT)
                                    .replace(COMPONENT_PROMPT, &header);
            return Html(res);
        },
        _ => {
            let skeleton = load_skeleton();
            // inject the components
            let res = skeleton.replace(COMPONENT_MAIN, &header)
                                    .replace(COMPONENT_INPUT, "")
                                    .replace(COMPONENT_PROMPT, "");
            return Html(res);
        }
    } 
}

/// Searches ~/.dioscuri/browser/{my_path_to_file} by extracting my_path_to_file
/// The filepath must only exist within the browser/ folder for security concerns
async fn get_resource(Path(filepath): Path<String>) -> impl IntoResponse {
    // get the resource directory first
    let parent_dir = get_resource_dir();
    let resource_path = parent_dir.join(filepath);

    // resolve symlinks and relative components
    let Ok(resource_path_canon) = resource_path.canonicalize() else {
        return (http::StatusCode::NOT_FOUND, "Invalid file path").into_response();
    };

    let Ok(parent_canon) = parent_dir.canonicalize() else {
        return (http::StatusCode::INTERNAL_SERVER_ERROR, "Dioscuri's browser resource folder is invalid!").into_response();
    };

    // resource should be a child of the browser folder
    if !resource_path_canon.starts_with(&parent_canon) {
        return (http::StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    // read and serve the resource
    match fs::read(&resource_path_canon) {
        Ok(contents) => {
            Response::builder()
                .status(http::StatusCode::OK)
                .body(Body::from(contents))
                .unwrap()
        }
        Err(_) => (http::StatusCode::NOT_FOUND, "File not found").into_response(),
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