use crate::cell::Cell;
use crate::site::{PageRef, Site};
use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use tera::Tera;

const DEFAULT_TEMPLATE: &str = include_str!("../templates/default/page.html");

// ── Tera context types ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PageRefCtx {
    id: String,
    slug: String,
    date: String,
    title: String,
    url: String,
}

impl From<PageRef> for PageRefCtx {
    fn from(p: PageRef) -> Self {
        PageRefCtx {
            id: p.id,
            slug: p.slug,
            date: p.date,
            title: p.title,
            url: p.url,
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
}

#[derive(Serialize, Clone)]
pub struct SiteNodeCtx {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub url: String,
    pub text: String,
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
        }).collect(),
        edges: &graph.edges,
    };
    Ok(serde_json::to_string(&slim)?)
}

pub fn build_graph(site: &Site) -> SiteGraphCtx {
    build_site_graph(site)
}

// ── Context building ──────────────────────────────────────────────────────────

fn format_date(date: &str) -> String {
    if date.len() == 8 {
        format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    }
}

fn build_site_nodes(site: &Site) -> Vec<SiteNodeCtx> {
    site.all_cells()
        .filter(|c| c.id != "index")
        .map(|c| SiteNodeCtx {
            id: c.id.clone(),
            slug: c.slug.clone(),
            title: c.title.clone(),
            url: c.url(),
            text: c.plain_text.clone(),
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
            date: cell.date.clone(),
            date_formatted: format_date(&cell.date),
            last_modified,
            title: cell.title.clone(),
            url: cell.url(),
            body,
            body_without_title,
            backlinks,
            history,
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
    output
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
    output
}
