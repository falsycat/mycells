use crate::render::{self, Renderer, SiteGraphCtx};
use crate::site::Site;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use notify::{EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

struct SiteCache {
    site: Arc<Site>,
    graph: Arc<SiteGraphCtx>,
}

impl SiteCache {
    fn load(cells_dir: &std::path::Path) -> anyhow::Result<Self> {
        let site = Arc::new(Site::load(cells_dir)?);
        let graph = Arc::new(render::build_graph(&site));
        Ok(SiteCache { site, graph })
    }
}

struct AppState {
    cells_dir: PathBuf,
    template_dir: Option<PathBuf>,
    vars: HashMap<String, String>,
    cache: RwLock<SiteCache>,
}

impl AppState {
    fn get_cache(&self) -> (Arc<Site>, Arc<SiteGraphCtx>) {
        let c = self.cache.read().unwrap();
        (c.site.clone(), c.graph.clone())
    }

    fn reload(&self) {
        match SiteCache::load(&self.cells_dir) {
            Ok(fresh) => {
                *self.cache.write().unwrap() = fresh;
                eprintln!("site reloaded");
            }
            Err(e) => eprintln!("reload error: {e}"),
        }
    }
}

pub async fn serve(
    cells_dir: PathBuf,
    template_dir: Option<PathBuf>,
    vars: HashMap<String, String>,
    port: u16,
) -> anyhow::Result<()> {
    let cache = SiteCache::load(&cells_dir)?;

    let state = Arc::new(AppState {
        cells_dir: cells_dir.clone(),
        template_dir,
        vars,
        cache: RwLock::new(cache),
    });

    // Watch cells_dir for .md file changes and reload.
    let watch_state = Arc::clone(&state);
    let watch_dir = cells_dir.clone();
    tokio::task::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                let _ = tx.send(ev);
            }
        })
        .expect("watcher init failed");
        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .expect("watch failed");

        for event in rx {
            let is_md = event.paths.iter().any(|p| {
                p.extension().and_then(|e| e.to_str()) == Some("md")
            });
            let is_write = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            );
            if is_md && is_write {
                watch_state.reload();
            }
        }
    });

    let app = Router::new()
        .route("/", get(handle_index))
        .route("/search.json", get(handle_search_json))
        .route("/graph.json", get(handle_graph_json))
        .route("/:slug", get(handle_slug_no_slash))
        .route("/:slug/", get(handle_cell))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("serving on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_index(State(state): State<Arc<AppState>>) -> Response {
    render_slug(&state, "").await
}

async fn handle_slug_no_slash(Path(slug): Path<String>) -> Response {
    Redirect::permanent(&format!("/{slug}/")).into_response()
}

async fn handle_cell(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Response {
    render_slug(&state, &slug).await
}

async fn handle_search_json(State(state): State<Arc<AppState>>) -> Response {
    let (_, graph) = state.get_cache();
    match render::generate_search_json(&graph) {
        Ok(json) => (
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            json,
        )
            .into_response(),
        Err(e) => {
            eprintln!("error generating search.json: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn handle_graph_json(State(state): State<Arc<AppState>>) -> Response {
    let (_, graph) = state.get_cache();
    match render::generate_graph_json(&graph) {
        Ok(json) => (
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            json,
        )
            .into_response(),
        Err(e) => {
            eprintln!("error generating graph.json: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn render_slug(state: &AppState, slug: &str) -> Response {
    let renderer = match &state.template_dir {
        Some(dir) => Renderer::from_dir(dir),
        None => Renderer::default_template(),
    };
    let renderer = match renderer {
        Ok(r) => r,
        Err(e) => {
            eprintln!("template error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let (site, graph) = state.get_cache();

    let cell = match site.get_by_slug(slug) {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    match renderer.render(cell, &site, &graph, &state.vars) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("render error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
