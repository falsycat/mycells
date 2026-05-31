mod cell;
mod git;
mod render;
mod server;
mod site;

use anyhow::Context;
use clap::{Parser, Subcommand};
use render::Renderer;
use site::Site;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mcs", about = "mycells static site generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a static site into an output directory
    Build {
        /// Path to the cells directory
        #[arg(long, default_value = ".")]
        cells: PathBuf,

        /// Output directory
        #[arg(long, short, default_value = "dist")]
        output: PathBuf,

        /// Custom Tera template directory (must contain page.html)
        #[arg(long)]
        template: Option<PathBuf>,

        /// User-defined variables passed to templates (KEY=VALUE)
        #[arg(long, value_parser = parse_key_val, number_of_values = 1)]
        var: Vec<(String, String)>,
    },

    /// Start a live preview HTTP server
    Serve {
        /// Path to the cells directory
        #[arg(long, default_value = ".")]
        cells: PathBuf,

        /// Port to listen on
        #[arg(long, short, default_value_t = 3000)]
        port: u16,

        /// Custom Tera template directory (must contain page.html)
        #[arg(long)]
        template: Option<PathBuf>,

        /// User-defined variables passed to templates (KEY=VALUE)
        #[arg(long, value_parser = parse_key_val, number_of_values = 1)]
        var: Vec<(String, String)>,
    },
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("expected KEY=VALUE, got {s:?}"))?;
    Ok((k.to_string(), v.to_string()))
}

fn make_renderer(template: Option<&PathBuf>) -> anyhow::Result<Renderer> {
    match template {
        Some(dir) => Renderer::from_dir(dir).context("loading custom template"),
        None => Renderer::default_template(),
    }
}

fn make_vars(pairs: Vec<(String, String)>) -> HashMap<String, String> {
    pairs.into_iter().collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            cells,
            output,
            template,
            var,
        } => {
            let renderer = make_renderer(template.as_ref())?;
            let vars = make_vars(var);
            let site = Site::load(&cells).context("loading cells")?;

            std::fs::create_dir_all(&output)?;

            for cell in site.all_cells() {
                let html = renderer.render(cell, &site, &vars)?;

                let out_path = if cell.slug.is_empty() {
                    output.join("index.html")
                } else {
                    let dir = output.join(&cell.slug);
                    std::fs::create_dir_all(&dir)?;
                    dir.join("index.html")
                };

                std::fs::write(&out_path, html)?;
                eprintln!("wrote {}", out_path.display());
            }

            let search_json = render::generate_search_json(&site)?;
            let search_path = output.join("search.json");
            std::fs::write(&search_path, search_json)?;
            eprintln!("wrote {}", search_path.display());

            let graph_json = render::generate_graph_json(&site)?;
            let graph_path = output.join("graph.json");
            std::fs::write(&graph_path, graph_json)?;
            eprintln!("wrote {}", graph_path.display());

            eprintln!("build complete → {}", output.display());
        }

        Commands::Serve {
            cells,
            port,
            template,
            var,
        } => {
            let vars = make_vars(var);
            server::serve(cells, template, vars, port).await?;
        }
    }

    Ok(())
}
