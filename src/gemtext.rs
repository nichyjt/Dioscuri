use url::Url;

/// Given a gemtext string, perform some manipulations and return the desired result

/// Takes in a gemtext string, converts it to md then converts it to html
/// baseurl is used to relativize all links the baseurl provided. leave as empty string if not needed
pub fn gemtext_to_html(gemtext: String, url: String) -> String {
    let md = gemtext_to_md(gemtext, url);
    return markdown::to_html(&md);
}

/// Converts gemtext to md.
/// See: https://portal.mozz.us/gemini/geminiprotocol.net/docs/gemtext-specification.gmi
/// Fortunately, gemtext is close enough to markdown to allow minimal changes.
/// All lines will be appended with a trailing \n
fn gemtext_to_md(gemtext: String, _baseurl: String) -> String {
    println!("{}", _baseurl);

    let mut result = String::new();

    for line in gemtext.lines() {
        let trimmed = line.trim_start();
        println!("{}",trimmed);
        // Convert links to md links
        if trimmed.starts_with("=>") {
            result.push_str(&format!("{}\n\n",resolve_links(trimmed.to_string(), _baseurl.clone())));
        } else {
            result.push_str(&format!("{}\n\n", trimmed)); // plain paragraph
        }
    }
    result
}


fn resolve_links(link: String, url: String) -> String {
    // Ensure URL is a proper Gemini URL for resolution
    let base_url = Url::parse(&format!("gemini://{}", url))
        .unwrap_or_else(|_| Url::parse("gemini://tmp/").unwrap());

    // Remove leading "=>"
    let trimmed = link.trim_start().strip_prefix("=>").unwrap_or(&link).trim();

    // Split into href and optional label
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let raw_href = parts.next().unwrap_or("");
    let label = parts.next().unwrap_or("").trim();

    // resolve urls
    let resolved_url = base_url.join(raw_href);
    match resolved_url {
        Ok(url) if url.scheme() == "gemini" => {
            let host = url.host_str().unwrap_or("invalid");
            let path = url.path();
            let mut proxy_path = format!("/{host}{}", path);
            if let Some(q) = url.query() {
                proxy_path.push('?');
                proxy_path.push_str(q);
            }

            let display = if label.is_empty() {
                proxy_path.clone()
            } else {
                label.to_string()
            };

            format!("[{}]({})", display, proxy_path)
        }
        _ => { // http(s) or other protocol link
            let display = if label.is_empty() { raw_href } else { label };
            format!("[{}]({})", display, raw_href)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_links() {
        fn check(link: &str, base: &str, expected: &str) {
            assert_eq!(resolve_links(link.to_string(), base.to_string()), expected);
        }

        check(
            "=> /index.gmi Welcome",
            "gemi.dev/cgi-bin/wp.cgi/a?b=c",
            "[Welcome](/gemi.dev/index.gmi)"
        );

        check(
            "=> foo.gmi Foo",
            "gemi.dev/cgi-bin/wp.cgi/a?b=c",
            "[Foo](/gemi.dev/cgi-bin/wp.cgi/foo.gmi)"
        );

        check(
            "=> ../bar.gmi Go up",
            "gemi.dev/cgi-bin/wp.cgi/a",
            "[Go up](/gemi.dev/cgi-bin/bar.gmi)"
        );

        check(
            "=> gemini://example.com/docs/ External",
            "gemi.dev/docs/",
            "[External](/example.com/docs/)"
        );

        check(
            "=> https://google.com Google",
            "gemi.dev/docs/",
            "[Google](https://google.com)"
        );

        check(
            "=> help",
            "gemi.dev/docs/",
            "[/gemi.dev/docs/help](/gemi.dev/docs/help)"
        );

        check(
            "=> /help",
            "gemi.dev/docs/",
            "[/gemi.dev/help](/gemi.dev/help)"
        );

        check(
            "=> /help/me/find/this.gmi foo bar",
            "gemi.dev/docs/",
            "[foo bar](/gemi.dev/help/me/find/this.gmi)"
        );

        check(
            "=> help",
            "gemi.dev/docs/tutorial.gmi",
            "[/gemi.dev/docs/help](/gemi.dev/docs/help)"
        );

        check(
            "=> /cgi-bin/wp.cgi/view?Siege+of+Breteuil Siege of Breteuil",
            "gemi.dev/cgi-bin/wp.cgi/featured",
            "[Siege of Breteuil](/gemi.dev/cgi-bin/wp.cgi/view?Siege+of+Breteuil)"
        );

    }

}
