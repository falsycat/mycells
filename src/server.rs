use crate::render::{self, Renderer};
use crate::site::Site;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

struct AppState {
    cells_dir: PathBuf,
    template_dir: Option<PathBuf>,
    vars: HashMap<String, String>,
}

pub async fn serve(
    cells_dir: PathBuf,
    template_dir: Option<PathBuf>,
    vars: HashMap<String, String>,
    port: u16,
) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        cells_dir,
        template_dir,
        vars,
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
    let site = match Site::load(&state.cells_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error loading site: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    match render::generate_search_json(&site) {
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
    let site = match Site::load(&state.cells_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error loading site: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    match render::generate_graph_json(&site) {
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

    let site = match Site::load(&state.cells_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error loading site: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let cell = match site.get_by_slug(slug) {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    match renderer.render(cell, &site, &state.vars) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("render error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
