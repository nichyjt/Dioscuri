use std::io::Write;
use std::path::PathBuf;

use std::fs::{self, File};

/// Directory utils 

pub static DEFAULT_FILES: [&str; 3] = ["head.html", "nav.html", "body.html"];

/// Get the Path to ~/.dioscuri/ to load HTML elements.
/// Creates the dir and any missing default files and directories if they do not exist.
pub fn get_default_dir() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        "Error loading user home directory".to_owned()
    })?;

    // Ensure that the default dirs exist
    let default_dir = home_dir.join(".dioscuri");
    let static_dir = default_dir.join("static");
    let cert_dir = default_dir.join("cert");
    let cert_client_dir = cert_dir.join("client");
    let cert_server_dir = cert_dir.join("server");
    if !default_dir.exists() {
        let _ = fs::create_dir_all(&default_dir);
    }
    if !cert_dir.exists() {
        let _ = fs::create_dir_all(cert_dir);
        let _ = fs:: create_dir_all(cert_client_dir);
        let _ = fs:: create_dir_all(cert_server_dir);
    }
    if !static_dir.exists() {
        let _ = fs::create_dir_all(&static_dir);  
    }

    // Ensure that the mandatory default files exist
    for filename in DEFAULT_FILES {
        let filepath = default_dir.join(filename);
        if !filepath.exists(){
            let filedata = get_default_file(filename);
            let mut file = File::create(&filepath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            file.write_all(filedata.as_bytes())
                .map_err(|e| format!("Failed to write to file: {}", e))?;
        }
    }
    
    Ok(default_dir)
}

pub fn get_cert_dir() -> PathBuf {
    let dir = get_default_dir().unwrap();
    let cert_dir = dir.join("cert");
    return cert_dir;
}


/// Get the default file from the project's 
/// For now, a dirty const string implementation.
/// Consider refactoring to load dynamically.
/// This does not scale well.
fn get_default_file(filename: &str) -> &'static str {
    match filename {
        "head.html" => r#"
            <head>
            <meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
            <title>Dioscuri | Gemini Client</title>
            <!-- You can include as many stylesheets and scripts as you want! -->
            <link rel='stylesheet' href="/static/css/style.css"/>
            </head>
            "#,
        "nav.html" => r#"
            <!-- The nav html is just a way for you to store your bookmarks or type in an address directly -->
            <!-- You may leave this blank if you don't need one -->
            <div>
            <h1>Address Bar</h1>
            <form action="http://localhost:1965/" method="GET">
                <input type="text" name="gemini">
                <button type="submit">Enter</button>
            </form>
            <h3>Quick Links</h1>
            <a href="gemini/geminiprotocol.net/">geminiprotocol.net</a> <br>
            <a href="gemini/bbs.geminispace.org/">bbs.geminispace.org</a>
            </div>
            "#,
        "body.html" => r#"
            <!-- This is the default body html file that will be served -->
            <h1>Hello, world!</h1>
            "#,
        _ => "Error in getting default file."
    }
} 