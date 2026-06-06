use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::sync::LazyLock;

static LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[(\d+)\]\]").unwrap());

static FILENAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+)-([a-z0-9][a-z0-9-]*)\.md$").unwrap());

static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)(?:^|[\s,;:.!?()\[\]{}'"])#([A-Za-z][A-Za-z0-9_-]*)"#).unwrap()
});

#[derive(Debug, Clone)]
pub struct Cell {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub raw: Box<str>,
    pub plain_text: String,
    pub outlinks: Vec<String>,
    pub tags: Vec<String>,
    pub history: Vec<crate::git::GitCommit>,
}

impl Cell {
    pub fn from_file(filename: &str, content: &str) -> Option<Self> {
        if filename == "index.md" {
            let title = extract_title(content)?;
            let plain_text = extract_plain_text(content);
            let outlinks = extract_outlinks(content);
            let tags = extract_tags(content);
            Some(Cell {
                id: "index".to_string(),
                slug: String::new(),
                title,
                raw: content.to_string().into_boxed_str(),
                plain_text,
                outlinks,
                tags,
                history: vec![],
            })
        } else {
            let caps = FILENAME_RE.captures(filename)?;
            let id = caps[1].to_string();
            let slug = caps[2].to_string();
            let title = extract_title(content)?;
            let plain_text = extract_plain_text(content);
            let outlinks = extract_outlinks(content);
            let tags = extract_tags(content);
            Some(Cell {
                id,
                slug,
                title,
                raw: content.to_string().into_boxed_str(),
                plain_text,
                outlinks,
                tags,
                history: vec![],
            })
        }
    }

    pub fn created_date(&self) -> &str {
        self.history
            .last()
            .and_then(|c| c.author_date.get(..10))
            .unwrap_or("")
    }

    pub fn url(&self) -> String {
        if self.slug.is_empty() {
            "/".to_string()
        } else {
            format!("/{}/", self.slug)
        }
    }
}

fn extract_title(content: &str) -> Option<String> {
    let parser = Parser::new_ext(content, Options::empty());
    let mut in_h1 = false;
    let mut title = String::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_h1 = true;
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => break,
            Event::Text(t) | Event::Code(t) if in_h1 => title.push_str(&t),
            _ => {}
        }
    }
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

pub fn extract_tags(content: &str) -> Vec<String> {
    let mut tags: Vec<String> = TAG_RE
        .captures_iter(content)
        .map(|c| c[1].to_lowercase())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

pub fn extract_outlinks(content: &str) -> Vec<String> {
    LINK_RE
        .captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}

pub fn extract_plain_text(content: &str) -> String {
    let parser = Parser::new_ext(content, Options::empty());
    let mut text = String::new();
    for event in parser {
        match event {
            Event::Text(t) | Event::Code(t) => {
                text.push_str(&t);
                text.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn link_re() -> &'static Regex {
    &LINK_RE
}
