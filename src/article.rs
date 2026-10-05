use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Article {
    pub title: String,
    pub byline: String,
    pub source: String,
    pub content: Vec<Block>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub kind: String,
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub ordered: bool,
    #[serde(default)]
    pub runs: Vec<Inline>,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Inline {
    Text { text: String },
    Br,
    Strong { children: Vec<Inline> },
    Em { children: Vec<Inline> },
    S { children: Vec<Inline> },
    Code { children: Vec<Inline> },
    Sup { children: Vec<Inline> },
    Sub { children: Vec<Inline> },
    Link { href: String, children: Vec<Inline> },
}

pub fn article_url(input: &str) -> Result<String, String> {
    let u = Url::parse(input.trim()).map_err(|_| "Enter a complete https://x.com article URL.")?;
    if input.len() > 2048
        || u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
        || !matches!(
            u.host_str(),
            Some("x.com" | "www.x.com" | "twitter.com" | "www.twitter.com")
        )
    {
        return Err("Only HTTPS X/Twitter article URLs are supported.".into());
    }
    let parts: Vec<_> = u.path().trim_end_matches('/').split('/').skip(1).collect();
    let numeric = |s: &str| !s.is_empty() && s.len() <= 24 && s.bytes().all(|b| b.is_ascii_digit());
    let supported = match parts.as_slice() {
        [name, "status", id] => {
            !name.is_empty()
                && name.len() <= 15
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && numeric(id)
        }
        ["i", "article", id] => numeric(id),
        _ => false,
    };
    if !supported {
        return Err("Use the article’s /name/status/123 or /i/article/123 URL.".into());
    }
    Ok(format!("https://x.com{}", u.path().trim_end_matches('/')))
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_inline(items: &[Inline], depth: usize) -> Result<String, String> {
    if depth > 24 {
        return Err("Article formatting is too deeply nested.".into());
    }
    let mut out = String::new();
    for item in items {
        let (tag, children) = match item {
            Inline::Text { text } => {
                out.push_str(&escape(text));
                continue;
            }
            Inline::Br => {
                out.push_str("<br>");
                continue;
            }
            Inline::Strong { children } => ("strong", children),
            Inline::Em { children } => ("em", children),
            Inline::S { children } => ("s", children),
            Inline::Code { children } => ("code", children),
            Inline::Sup { children } => ("sup", children),
            Inline::Sub { children } => ("sub", children),
            Inline::Link { href, children } => {
                let inner = render_inline(children, depth + 1)?;
                let valid = Url::parse(href).ok().filter(|u| {
                    matches!(u.scheme(), "https" | "http")
                        && u.username().is_empty()
                        && u.password().is_none()
                });
                if let Some(u) = valid {
                    out.push_str(&format!(
                        "<a href=\"{}\" rel=\"noreferrer\">{inner}</a>",
                        escape(u.as_str())
                    ));
                } else {
                    out.push_str(&inner);
                }
                continue;
            }
        };
        out.push_str(&format!(
            "<{tag}>{}</{tag}>",
            render_inline(children, depth + 1)?
        ));
    }
    Ok(out)
}

impl Article {
    pub fn html(&self, paper: &str) -> Result<String, String> {
        let source = article_url(&self.source)?;
        if self.content.is_empty()
            || self.content.len() > 10000
            || self.title.len() > 4096
            || self.byline.len() > 4096
        {
            return Err("Article is empty or exceeds the supported size.".into());
        }
        if !matches!(paper, "Letter" | "A4") {
            return Err("Choose Letter or A4 paper.".into());
        }
        let mut content = String::new();
        let mut list = "";
        for b in &self.content {
            let next = if b.kind == "list-item" {
                if b.ordered {
                    "ol"
                } else {
                    "ul"
                }
            } else {
                ""
            };
            if list != next {
                if !list.is_empty() {
                    content.push_str(&format!("</{list}>"));
                }
                if !next.is_empty() {
                    content.push_str(&format!("<{next}>"));
                }
                list = next;
            }
            let tag = match b.kind.as_str() {
                "paragraph" => "p".into(),
                "list-item" => "li".into(),
                "quote" => "blockquote".into(),
                "code" => "pre".into(),
                "heading" => format!("h{}", b.level.clamp(2, 6)),
                _ => return Err("Unsupported article block.".into()),
            };
            let text = if b.kind == "code" {
                escape(&b.text)
            } else {
                render_inline(&b.runs, 0)?
            };
            content.push_str(&format!("<{tag}>{text}</{tag}>\n"));
        }
        if !list.is_empty() {
            content.push_str(&format!("</{list}>"));
        }
        // No raw article HTML enters this document. CSP is a second boundary.
        Ok(format!("<!doctype html><html><head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\"><title>{}</title><style>{}\n@page {{ size: {paper}; }}\n</style></head><body><main id=\"document\"><header><h1>{}</h1><p id=\"byline\">{}</p></header><article id=\"content\">{content}</article><footer><a href=\"{}\">{}</a></footer></main></body></html>", escape(&self.title), include_str!("../assets/print.css"), escape(&self.title), escape(&self.byline), escape(&source), escape(&source)))
    }

    pub fn preview(&self) -> String {
        fn plain(items: &[Inline], out: &mut String) {
            for i in items {
                match i {
                    Inline::Text { text } => out.push_str(text),
                    Inline::Br => out.push('\n'),
                    Inline::Strong { children }
                    | Inline::Em { children }
                    | Inline::S { children }
                    | Inline::Code { children }
                    | Inline::Sup { children }
                    | Inline::Sub { children }
                    | Inline::Link { children, .. } => plain(children, out),
                }
            }
        }
        let mut s = String::new();
        for b in &self.content {
            plain(&b.runs, &mut s);
            s.push_str(&b.text);
            s.push_str("\n\n");
        }
        s.chars().take(16000).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn url_boundaries() {
        assert_eq!(
            article_url("https://twitter.com/tom/status/123?s=20").unwrap(),
            "https://x.com/tom/status/123"
        );
        assert!(article_url("https://x.com/i/article/123").is_ok());
        for s in [
            "file:///etc/passwd",
            "https://x.com.evil.test/a/status/1",
            "https://me@x.com/a/status/1",
            "https://x.com:444/a/status/1",
            "http://x.com/a/status/1",
            "https://x.com/home",
            "--disable-web-security",
            "https://x.com/a/status/%31",
        ] {
            assert!(article_url(s).is_err(), "{s}");
        }
    }
    #[test]
    fn html_is_data_and_lists_keep_order() {
        let a: Article =
            serde_json::from_str(include_str!("../demo/fixtures/article.json")).unwrap();
        let h = a.html("A4").unwrap();
        assert!(h.contains("<ol><li>"));
        assert!(h.contains("&lt;script&gt;"));
        assert!(!h.contains("href=\"javascript:"));
        assert!(!h.contains("<script>"));
        assert!(h.find("First paragraph").unwrap() < h.find("let answer").unwrap());
        assert!(h.find("let answer").unwrap() < h.find("After the code").unwrap());
        assert_eq!(h.matches("Repeated on purpose").count(), 2);
        assert!(a.html("A3; background:url(https://evil.test)").is_err());
    }
}
