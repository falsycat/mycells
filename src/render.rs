use crate::cell::Cell;
use crate::site::{PageRef, Site};
use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use tera::Tera;

static HTML_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]*>").unwrap());

// URL + hashtag combined replacement (used in normal text)
static FEATURES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)(https?://[^\s<>"']*[^\s<>"'.,;:!?)\[\]{}])|((?:^|[\s,;:.!?()\[\]{}'"])#([A-Za-z][A-Za-z0-9_-]*))"#).unwrap()
});

// Tag-only replacement (used inside existing <a> to avoid double-linking URLs)
static TAG_IN_TEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)(^|[\s,;:.!?()\[\]{}'"])#([A-Za-z][A-Za-z0-9_-]*)"#).unwrap()
});

const DEFAULT_TEMPLATE: &str = include_str!("../templates/default/page.html");

// ── Tera context types ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PageRefCtx {
    id: String,
    slug: String,
    date: String,
    title: String,
    url: String,
    tags: Vec<String>,
}

impl From<PageRef> for PageRefCtx {
    fn from(p: PageRef) -> Self {
        PageRefCtx {
            id: p.id,
            slug: p.slug,
            date: p.date,
            title: p.title,
            url: p.url,
            tags: p.tags,
        }
    }
}

#[derive(Serialize)]
struct GitCommitCtx {
    hash: String,
    author: String,
    author_date: String,
    message: String,
    diff: String,
}

impl From<&crate::git::GitCommit> for GitCommitCtx {
    fn from(c: &crate::git::GitCommit) -> Self {
        GitCommitCtx {
            hash: c.hash.clone(),
            author: c.author.clone(),
            author_date: c.author_date.clone(),
            message: c.message.clone(),
            diff: c.diff.clone(),
        }
    }
}

#[derive(Serialize)]
struct PageCtx {
    id: String,
    slug: String,
    date: String,
    date_formatted: String,
    last_modified: String,
    title: String,
    url: String,
    body: String,
    body_without_title: String,
    backlinks: Vec<PageRefCtx>,
    history: Vec<GitCommitCtx>,
    tags: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct SiteNodeCtx {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub url: String,
    pub text: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct SiteEdgeCtx {
    pub source: String,
    pub target: String,
}

#[derive(Serialize, Clone)]
pub struct SiteGraphCtx {
    pub nodes: Vec<SiteNodeCtx>,
    pub edges: Vec<SiteEdgeCtx>,
}

#[derive(Serialize)]
struct TemplateCtx {
    page: PageCtx,
    recent_pages: Vec<PageRefCtx>,
    site_graph: SiteGraphCtx,
    vars: HashMap<String, String>,
}

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    tera: Tera,
}

impl Renderer {
    pub fn default_template() -> anyhow::Result<Self> {
        let mut tera = Tera::default();
        tera.add_raw_template("page.html", DEFAULT_TEMPLATE)?;
        Ok(Renderer { tera })
    }

    pub fn from_dir(dir: &Path) -> anyhow::Result<Self> {
        let pattern = format!("{}/**/*.html", dir.display());
        let tera = Tera::new(&pattern)?;
        Ok(Renderer { tera })
    }

    pub fn render(
        &self,
        cell: &Cell,
        site: &Site,
        graph: &SiteGraphCtx,
        vars: &HashMap<String, String>,
    ) -> anyhow::Result<String> {
        let ctx = build_context(cell, site, graph, vars);
        let tera_ctx = tera::Context::from_serialize(&ctx)?;
        Ok(self.tera.render("page.html", &tera_ctx)?)
    }
}

// ── JSON export ───────────────────────────────────────────────────────────────

pub fn generate_search_json(graph: &SiteGraphCtx) -> anyhow::Result<String> {
    Ok(serde_json::to_string(&graph.nodes)?)
}

#[derive(Serialize)]
struct GraphNodeCtx<'a> {
    id: &'a str,
    slug: &'a str,
    title: &'a str,
    url: &'a str,
    tags: &'a [String],
}

#[derive(Serialize)]
struct GraphCtx<'a> {
    nodes: Vec<GraphNodeCtx<'a>>,
    edges: &'a Vec<SiteEdgeCtx>,
}

pub fn generate_graph_json(graph: &SiteGraphCtx) -> anyhow::Result<String> {
    let slim = GraphCtx {
        nodes: graph.nodes.iter().map(|n| GraphNodeCtx {
            id: &n.id,
            slug: &n.slug,
            title: &n.title,
            url: &n.url,
            tags: &n.tags,
        }).collect(),
        edges: &graph.edges,
    };
    Ok(serde_json::to_string(&slim)?)
}

pub fn build_graph(site: &Site) -> SiteGraphCtx {
    build_site_graph(site)
}

// ── Context building ──────────────────────────────────────────────────────────


fn build_site_nodes(site: &Site) -> Vec<SiteNodeCtx> {
    site.all_cells()
        .filter(|c| c.id != "index")
        .map(|c| SiteNodeCtx {
            id: c.id.clone(),
            slug: c.slug.clone(),
            title: c.title.clone(),
            url: c.url(),
            text: c.plain_text.clone(),
            tags: c.tags.clone(),
        })
        .collect()
}

fn build_site_graph(site: &Site) -> SiteGraphCtx {
    let nodes = build_site_nodes(site);
    let mut edges = Vec::new();
    for cell in site.all_cells() {
        if cell.id == "index" {
            continue;
        }
        for target_id in &cell.outlinks {
            if target_id == "index" {
                continue;
            }
            if site.get_by_id(target_id).is_some() {
                edges.push(SiteEdgeCtx {
                    source: cell.id.clone(),
                    target: target_id.clone(),
                });
            }
        }
    }
    SiteGraphCtx { nodes, edges }
}

fn build_context(
    cell: &Cell,
    site: &Site,
    graph: &SiteGraphCtx,
    vars: &HashMap<String, String>,
) -> TemplateCtx {
    let body = render_to_html(&cell.raw, site);
    let body_without_title = render_without_title(&cell.raw, site);
    let backlinks = site
        .backlink_pagerefs(&cell.id)
        .into_iter()
        .map(PageRefCtx::from)
        .collect();
    let recent_pages = site
        .recent_pagerefs()
        .into_iter()
        .map(PageRefCtx::from)
        .collect();
    let history: Vec<GitCommitCtx> = cell.history.iter().map(GitCommitCtx::from).collect();
    let last_modified = cell
        .history
        .first()
        .and_then(|c| c.author_date.split(' ').next())
        .unwrap_or("")
        .to_string();

    TemplateCtx {
        page: PageCtx {
            id: cell.id.clone(),
            slug: cell.slug.clone(),
            date: cell.created_date().to_string(),
            date_formatted: cell.created_date().to_string(),
            last_modified,
            title: cell.title.clone(),
            url: cell.url(),
            body,
            body_without_title,
            backlinks,
            history,
            tags: cell.tags.clone(),
        },
        recent_pages,
        site_graph: SiteGraphCtx {
            nodes: graph.nodes.clone(),
            edges: graph.edges.clone(),
        },
        vars: vars.clone(),
    }
}

// ── Markdown helpers ──────────────────────────────────────────────────────────

fn postprocess_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len() + 128);
    let mut code_depth: u32 = 0;
    let mut link_depth: u32 = 0;
    let mut pos = 0;

    let process_text = |seg: &str, in_link: bool| -> String {
        if in_link {
            TAG_IN_TEXT_RE.replace_all(seg, |caps: &regex::Captures| {
                format!("{}<span class=\"tag\">#{}</span>", &caps[1], &caps[2])
            }).into_owned()
        } else {
            FEATURES_RE.replace_all(seg, |caps: &regex::Captures| {
                if let Some(url) = caps.get(1) {
                    let u = url.as_str();
                    format!("<a href=\"{u}\">{u}</a>")
                } else {
                    // caps[2] is the full " #tag" match; caps[3] is the tag name
                    let full = caps.get(2).map_or("", |m| m.as_str());
                    let tag = caps.get(3).unwrap().as_str();
                    // prefix = everything before '#' + tag name
                    let prefix_len = full.len().saturating_sub(1 + tag.len());
                    let prefix = &full[..prefix_len];
                    format!("{}<span class=\"tag\">#{}</span>", prefix, tag)
                }
            }).into_owned()
        }
    };

    for m in HTML_TAG_RE.find_iter(html) {
        let text_seg = &html[pos..m.start()];
        if code_depth == 0 && !text_seg.is_empty() {
            result.push_str(&process_text(text_seg, link_depth > 0));
        } else {
            result.push_str(text_seg);
        }

        let tag_str = m.as_str();
        let inner = &tag_str[1..tag_str.len() - 1];
        let (closing, name) = if inner.starts_with('/') {
            (true, inner[1..].trim_start().split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("").to_ascii_lowercase())
        } else {
            (false, inner.trim_start().split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap_or("").to_ascii_lowercase())
        };
        match (name.as_str(), closing) {
            ("pre" | "code", false) => code_depth += 1,
            ("pre" | "code", true)  => code_depth = code_depth.saturating_sub(1),
            ("a", false) => link_depth += 1,
            ("a", true)  => link_depth = link_depth.saturating_sub(1),
            _ => {}
        }
        result.push_str(tag_str);
        pos = m.end();
    }

    let remaining = &html[pos..];
    if code_depth == 0 && !remaining.is_empty() {
        result.push_str(&process_text(remaining, link_depth > 0));
    } else {
        result.push_str(remaining);
    }
    result
}

fn resolve_links(content: &str, site: &Site) -> String {
    crate::cell::link_re()
        .replace_all(content, |caps: &regex::Captures| {
            let id = &caps[1];
            match site.get_by_id(id) {
                Some(cell) => format!("[{}]({})", cell.title, cell.url()),
                None => format!("[[{id}]]"),
            }
        })
        .into_owned()
}

fn render_to_html(content: &str, site: &Site) -> String {
    let resolved = resolve_links(content, site);
    let parser = Parser::new_ext(&resolved, Options::all());
    let mut output = String::new();
    html::push_html(&mut output, parser);
    postprocess_html(&output)
}

fn render_without_title(content: &str, site: &Site) -> String {
    let resolved = resolve_links(content, site);
    let parser = Parser::new_ext(&resolved, Options::all());

    let mut in_first_h1 = false;
    let mut first_h1_done = false;
    let mut events: Vec<Event<'_>> = Vec::new();

    for event in parser {
        if first_h1_done {
            events.push(event);
            continue;
        }
        match &event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_first_h1 = true;
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) if in_first_h1 => {
                in_first_h1 = false;
                first_h1_done = true;
            }
            _ if in_first_h1 => {}
            _ => events.push(event),
        }
    }

    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    postprocess_html(&output)
}
