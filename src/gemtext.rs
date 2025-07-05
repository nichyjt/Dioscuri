/// Given a gemtext string, perform some manipulations and return the desired result

/// Takes in a gemtext string, converts it to md then converts it to html
pub fn gemtext_to_html(gemtext: String) -> String {
    let md = gemtext_to_md(gemtext);
    return markdown::to_html(&md);
}

/// Converts gemtext to md.
/// See: https://portal.mozz.us/gemini/geminiprotocol.net/docs/gemtext-specification.gmi
/// Fortunately, gemtext is close enough to markdown to allow minimal changes.
/// All lines will be appended with a trailing \n
fn gemtext_to_md(gemtext: String) -> String {
    let mut result = String::new();

    for line in gemtext.lines() {
        let trimmed = line.trim_start();
        // println!("{}",relative_url);
        // Convert links to md links
        if trimmed.starts_with("=>") {
            // Remove the leading "=>"
            let rest = trimmed[2..].trim();

            // Split into URL and optional label
            let mut parts = rest.splitn(2, char::is_whitespace);
            let mut url = parts.next().unwrap_or("").trim().to_string();
            let label = parts.next().unwrap_or("").trim().to_string();

            // Relative link rule hacking
            // relative links start with '/'.
            // to transform into md friendly relative link, simply remove the '/'
            // redirects/new pages start with 'gemini://'
            // to transform into a new link, replace 'gemini://' with '/'
            // Replace gemini:// with /
            if url.starts_with('/'){
                if let Some(stripped_url) = url.strip_prefix('/') {
                    url = stripped_url.to_string();
                }
            }else if let Some(after_scheme) = url.strip_prefix("gemini://") {
                url = format!("/{}", after_scheme);
            }

            if label.is_empty() {
                result.push_str(&format!("[{}]({})\n\n", url, url));
            } else {
                result.push_str(&format!("[{}]({})\n\n", label, url));
            }
        } else {
            result.push_str(&format!("{}\n\n", trimmed)); // plain paragraph
        }
    }
    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_heading_and_paragraph() {
        let input = "# Title\nThis is a paragraph.";
        let expected = "# Title\n\nThis is a paragraph.\n\n";
        assert_eq!(gemtext_to_md(input.to_string()), expected);
    }

    #[test]
    fn test_links() {
        let in0 = "hello\n=>/relative/link.gmi";
        let out0 = "hello\n\n[relative/link.gmi](relative/link.gmi)\n\n";
        assert_eq!(gemtext_to_md(in0.to_string()), out0);
        let in1 = "hello\n=>/relative/link.gmi my custom / text!";
        let out1 = "hello\n\n[my custom / text!](relative/link.gmi)\n\n";
        println!("{}",gemtext_to_md(in1.to_string()));
        assert_eq!(gemtext_to_md(in1.to_string()), out1);
        let in2 = "hello\n=>gemini://new_address.net/foo.gmi go to new address!";
        let out2 = "hello\n\n[go to new address!](/new_address.net/foo.gmi)\n\n";
        assert_eq!(gemtext_to_md(in2.to_string()), out2);
    }

}
