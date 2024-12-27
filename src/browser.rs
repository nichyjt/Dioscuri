use std::collections::HashMap;

use axum::{extract::{Path, Query}, http::StatusCode, response::{Html, IntoResponse}};
use tokio::{fs::File, io::{AsyncReadExt, BufReader}};

use crate::{gemini, utils::{self, get_default_dir}};

/// Browser bindings for the client to hook into via axum

static HTML_PREAMBLE: &str = "<!DOCTYPE HTML><html>";
static HTML_CLOSE_TAG: &str = "</html>";

/// Takes in the body string and wraps it with proper html tags
fn wrap_html_body(body: String) -> String {
    format!("{}{}{}", HTML_PREAMBLE, body, HTML_CLOSE_TAG)
}

/// Returns the homepage of the, unless there is a query.
/// Queries will be extracted as protocol-address pairs and 
pub async fn get_homepage(
    Query(params): Query<HashMap<String, String>>
) -> impl IntoResponse {
    // If params exist, we only process the first one
    for (protocol, address) in params {
        // launch the gemini subroutine
        println!("{}/{}", protocol, address);
        break;
    }
    
    // Load the homepage
    let default_dir = match utils::get_default_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error getting homepage: {}", e);
            return Html("<h1>Error loading homepage</h1>").into_response();
        }
    };

    let mut html_content = String::new();

    for filename in crate::utils::DEFAULT_FILES {
        let dir = default_dir.join(filename);
        match tokio::fs::File::open(&dir).await {
            Ok(file) => {
                let mut buf_reader = tokio::io::BufReader::new(file);
                let mut content = String::new();
                if let Err(e) = buf_reader.read_to_string(&mut content).await {
                    eprintln!("Error reading file {}: {}", filename, e);
                    continue;
                }
                html_content.push_str(&content);
            }
            Err(e) => {
                eprintln!("Error opening file {}: {}", filename, e);
                continue;
            }
        }
    }
    html_content = wrap_html_body(html_content);
    Html(html_content).into_response()
}

pub async fn get_webpage(
    Path((protocol, address)): Path<(String, String)>
) -> impl IntoResponse {
    let parts = address.split_once("/").unwrap();
    let address = parts.0.to_string();
    let uri_path = parts.1.to_string();
    println!("protocol: {}, address: {}, path: {}", protocol, address, uri_path);
    // TODO: add more protocol support, such as Spartan
    return Html(gemini::request(address, uri_path).await).into_response();
}

pub async fn get_html_resource(
    Path((resource_mime, filename)) : Path<(String, String)>
) -> impl IntoResponse {
    // check if dir/resource_mime/filename exists. 
    // if exists, serve it, else, return a 404
    let default_dir = get_default_dir().map_err(
        |e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    )?;
    
    let mut resource_dir = default_dir.clone();
    resource_dir.push(&resource_mime);
    resource_dir.push(&filename);

    if !resource_dir.exists() {
        return Err((StatusCode::NOT_FOUND, "Resource not found".to_string()));
    }

    let file = File::open(resource_dir).await.map_err(|e|
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    let mut buf_reader = BufReader::new(file);
    let mut content = String::new();
    buf_reader
        .read_to_string(&mut content)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, content))
}