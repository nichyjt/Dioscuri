use std::{collections::HashMap, fs::{self, File}, hash::Hash, io::{BufReader, Read, Write}, path::PathBuf};

use axum::{extract::{path, Path, Query}, http::StatusCode, response::{Html, IntoResponse}, routing::get, Router};
use tokio::io::AsyncReadExt;

mod gemini;
mod utils;
mod browser;

#[tokio::main]
async fn main() {

    // Load the HTTP frontend
    let app = Router::new()
        .route("/", get(browser::get_homepage))
        .route("/:protocol/*address", get(browser::get_webpage))
        .route("/static/*resource", get(browser::get_html_resource));
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1965").await.unwrap();
    println!("Client running on http://localhost:1965. Kill me with CTRL-C.");
    axum::serve(listener, app).await.unwrap(); 
    // blocks forever

}
