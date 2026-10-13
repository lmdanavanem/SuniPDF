//! CLI SuniPDF : sortie JSON orientée automatisation (messages en anglais, codes d'erreur stables).
//! Même moteur que l'application de bureau : `sunipdf-core`.

use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use sunipdf_core::{Engine, EngineError};

#[derive(Parser)]
#[command(
    name = "sunipdf-cli",
    version,
    about = "SuniPDF command-line interface"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the application's command registry (stable ids) as JSON.
    Commands,
    /// Extract text (all pages, or one page) to stdout as plain text.
    Text {
        file: PathBuf,
        /// 1-based page number; omit for the whole document.
        #[arg(long)]
        page: Option<usize>,
        #[arg(long)]
        password: Option<String>,
    },
    /// Search text; prints matches (page, rectangles in PDF points) as JSON.
    Search {
        file: PathBuf,
        query: String,
        #[arg(long)]
        match_case: bool,
        #[arg(long)]
        password: Option<String>,
    },
    /// Print document information (page count, page sizes, title) as JSON.
    Info {
        file: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Render one page to a PNG file.
    Render {
        file: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Pixels per PDF point (1.0 = 72 dpi, 2.0 = 144 dpi).
        #[arg(long, default_value_t = 2.0)]
        scale: f32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
}

fn run(cli: Cli) -> Result<serde_json::Value, EngineError> {
    if let Command::Commands = cli.command {
        return Ok(serde_json::json!(sunipdf_core::REGISTRY
            .iter()
            .map(|c| serde_json::json!({
                "id": c.id,
                "menu": format!("{:?}", c.menu).to_lowercase(),
                "needs_document": c.needs_document,
            }))
            .collect::<Vec<_>>()));
    }
    let engine = Engine::new()?;
    match cli.command {
        Command::Commands => unreachable!(),
        Command::Text {
            file,
            page,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let pages: Vec<usize> = match page {
                Some(n) => vec![n.checked_sub(1).ok_or(EngineError::PageOutOfRange(0))?],
                None => (0..doc.page_count()).collect(),
            };
            let mut out = std::io::stdout().lock();
            for p in pages {
                let text = doc.page_text(p)?;
                writeln!(out, "{text}").map_err(EngineError::Io)?;
                if page.is_none() {
                    writeln!(out, "\u{c}").map_err(EngineError::Io)?;
                }
            }
            return Ok(serde_json::Value::Null);
        }
        Command::Search {
            file,
            query,
            match_case,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let matches = doc.search(&query, match_case)?;
            Ok(serde_json::json!({
                "query": query,
                "count": matches.len(),
                "matches": matches.iter().map(|m| serde_json::json!({
                    "page": m.page + 1,
                    "rects": m.rects.iter().map(|r| serde_json::json!([r.left, r.bottom, r.right, r.top])).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }))
        }
        Command::Info { file, password } => {
            let doc = engine.open(&file, password.as_deref())?;
            let info = doc.info();
            Ok(serde_json::json!({
                "path": info.path,
                "title": info.title,
                "page_count": info.page_count(),
                "pages": info.page_sizes.iter().enumerate().map(|(i, s)| serde_json::json!({
                    "index": i + 1, "width_pt": s.width_pt, "height_pt": s.height_pt
                })).collect::<Vec<_>>(),
            }))
        }
        Command::Render {
            file,
            page,
            scale,
            out,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let index = page.checked_sub(1).ok_or(EngineError::PageOutOfRange(0))?;
            let img = doc.render_page(index, scale)?;
            image::save_buffer(
                &out,
                &img.rgba,
                img.width,
                img.height,
                image::ColorType::Rgba8,
            )
            .map_err(|e| EngineError::Other(e.to_string()))?;
            Ok(
                serde_json::json!({ "out": out, "width": img.width, "height": img.height, "page": page }),
            )
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(serde_json::Value::Null) => ExitCode::SUCCESS,
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).expect("json"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!(
                "{}",
                serde_json::json!({ "error": e.code(), "message": e.to_string() })
            );
            ExitCode::from(match e {
                EngineError::PasswordRequired => 3,
                EngineError::InvalidFormat => 4,
                EngineError::Io(_) => 5,
                _ => 1,
            })
        }
    }
}
