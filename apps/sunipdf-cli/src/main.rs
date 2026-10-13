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
    /// Merge several PDFs (in order) into one output file.
    Merge {
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Extract pages (1-based, e.g. "1,3-5") into a new file, in the given order.
    Extract {
        file: PathBuf,
        #[arg(long)]
        pages: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Split into files of N pages each, written to a directory.
    Split {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        every: usize,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Rotate pages (1-based list, or "all") by 90, 180 or 270 degrees and save to --out.
    Rotate {
        file: PathBuf,
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long)]
        degrees: i32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Insert pages of another PDF (1-based list, default all) before page --at (1-based; 0 = append).
    Insert {
        file: PathBuf,
        #[arg(long)]
        from: PathBuf,
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long, default_value_t = 0)]
        at: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Move one page (1-based) to a new position (1-based).
    Move {
        file: PathBuf,
        #[arg(long)]
        from: usize,
        #[arg(long)]
        to: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Print document properties (metadata, version, encryption, permissions) as JSON.
    Properties {
        file: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Print the bookmark tree (title, page, depth) as JSON.
    Bookmarks {
        file: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Encrypt with AES-256 (or AES-128 with --aes128): passwords and permissions.
    Protect {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "")]
        user_password: String,
        #[arg(long, default_value = "")]
        owner_password: String,
        #[arg(long)]
        no_print: bool,
        #[arg(long)]
        no_copy: bool,
        #[arg(long)]
        no_modify: bool,
        #[arg(long)]
        no_annotate: bool,
        #[arg(long)]
        aes128: bool,
        /// Password of the input file if it is already encrypted.
        #[arg(long)]
        password: Option<String>,
    },
    /// Remove encryption (owner or user password required).
    Unprotect {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: String,
    },
    /// Set title, author, subject, keywords (omitted fields are left unchanged; "" clears).
    SetMetadata {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        subject: Option<String>,
        #[arg(long)]
        keywords: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    /// Remove all document metadata (Info dictionary and XMP stream).
    StripMetadata {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Build a PDF from image files (one page per image).
    Images2pdf {
        #[arg(required = true)]
        images: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        /// Pixels per inch used to size pages (150 = scanned page, 96 = screen).
        #[arg(long, default_value_t = 150.0)]
        dpi: f32,
    },
    /// Export every page as PNG into a directory.
    ExportImages {
        file: PathBuf,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long, default_value_t = 150.0)]
        dpi: f32,
        #[arg(long)]
        password: Option<String>,
    },
    /// Lossless size optimisation (stream recompression, unused objects removed).
    Optimize {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
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
            Ok(serde_json::Value::Null)
        }
        Command::Merge { inputs, out } => {
            let pages = engine.merge(&inputs, &out)?;
            Ok(serde_json::json!({ "out": out, "page_count": pages }))
        }
        Command::Extract {
            file,
            pages,
            out,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let selection = parse_pages(&pages, doc.page_count())?;
            let n = doc.extract_to(&selection, &out)?;
            Ok(serde_json::json!({ "out": out, "page_count": n }))
        }
        Command::Split {
            file,
            every,
            out_dir,
            password,
        } => {
            let files = engine.split(&file, password.as_deref(), every, &out_dir)?;
            Ok(serde_json::json!({ "files": files, "count": files.len() }))
        }
        Command::Rotate {
            file,
            pages,
            degrees,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = parse_pages(&pages, doc.page_count())?;
            for p in &selection {
                let current = doc.page_rotation(*p)?;
                doc.set_page_rotation(*p, current.turned(degrees))?;
            }
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "rotated": selection.len() }))
        }
        Command::Insert {
            file,
            from,
            pages,
            at,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let src_count = engine.open(&from, None)?.page_count();
            let selection = parse_pages(&pages, src_count)?;
            let at = if at == 0 { doc.page_count() } else { at - 1 };
            let n = doc.insert_pages_from(&from, None, &selection, at)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "inserted": n, "page_count": doc.page_count() }))
        }
        Command::Move {
            file,
            from,
            to,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let (from, to) = (
                from.checked_sub(1).ok_or(EngineError::PageOutOfRange(0))?,
                to.checked_sub(1).ok_or(EngineError::PageOutOfRange(0))?,
            );
            doc.move_page(from, to)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "moved": from + 1, "to": to + 1 }))
        }
        Command::Properties { file, password } => {
            let doc = engine.open(&file, password.as_deref())?;
            let p = doc.properties();
            Ok(serde_json::json!({
                "title": p.metadata.title, "author": p.metadata.author, "subject": p.metadata.subject,
                "keywords": p.metadata.keywords, "creator": p.creator, "producer": p.producer,
                "created": p.created, "modified": p.modified, "version": p.version,
                "page_count": p.page_count, "encrypted": p.encrypted, "file_size": p.file_size,
                "permissions": { "print": p.can_print, "copy": p.can_copy, "modify": p.can_modify, "annotate": p.can_annotate },
            }))
        }
        Command::Bookmarks { file, password } => {
            let doc = engine.open(&file, password.as_deref())?;
            Ok(serde_json::json!(doc
                .bookmarks()
                .iter()
                .map(|b| serde_json::json!({
                    "title": b.title, "page": b.page.map(|p| p + 1), "depth": b.depth,
                }))
                .collect::<Vec<_>>()))
        }
        Command::Protect {
            file,
            out,
            user_password,
            owner_password,
            no_print,
            no_copy,
            no_modify,
            no_annotate,
            aes128,
            password,
        } => {
            let input = std::fs::read(&file)?;
            let bytes = sunipdf_core::security::protect(
                &input,
                password.as_deref(),
                &sunipdf_core::Protection {
                    user_password,
                    owner_password,
                    allow_print: !no_print,
                    allow_copy: !no_copy,
                    allow_modify: !no_modify,
                    allow_annotate: !no_annotate,
                    aes256: !aes128,
                },
            )?;
            sunipdf_core::write_atomic(&out, &bytes)?;
            Ok(
                serde_json::json!({ "out": out, "cipher": if aes128 { "AES-128" } else { "AES-256" } }),
            )
        }
        Command::Unprotect {
            file,
            out,
            password,
        } => {
            let input = std::fs::read(&file)?;
            let bytes = sunipdf_core::security::unprotect(&input, &password)?;
            sunipdf_core::write_atomic(&out, &bytes)?;
            Ok(serde_json::json!({ "out": out }))
        }
        Command::SetMetadata {
            file,
            out,
            title,
            author,
            subject,
            keywords,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let current = doc.properties().metadata;
            drop(doc);
            let pick = |new: Option<String>, old: Option<String>| match new {
                Some(v) if v.is_empty() => None,
                Some(v) => Some(v),
                None => old,
            };
            let meta = sunipdf_core::Metadata {
                title: pick(title, current.title),
                author: pick(author, current.author),
                subject: pick(subject, current.subject),
                keywords: pick(keywords, current.keywords),
            };
            let input = std::fs::read(&file)?;
            let bytes = sunipdf_core::security::set_metadata(&input, password.as_deref(), &meta)?;
            sunipdf_core::write_atomic(&out, &bytes)?;
            Ok(
                serde_json::json!({ "out": out, "title": meta.title, "author": meta.author, "subject": meta.subject, "keywords": meta.keywords }),
            )
        }
        Command::StripMetadata {
            file,
            out,
            password,
        } => {
            let input = std::fs::read(&file)?;
            let bytes = sunipdf_core::security::strip_metadata(&input, password.as_deref())?;
            sunipdf_core::write_atomic(&out, &bytes)?;
            Ok(serde_json::json!({ "out": out }))
        }
        Command::Images2pdf { images, out, dpi } => {
            let pages = engine.images_to_pdf(&images, &out, dpi)?;
            Ok(serde_json::json!({ "out": out, "page_count": pages }))
        }
        Command::ExportImages {
            file,
            out_dir,
            dpi,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let files = doc.export_images(&out_dir, dpi)?;
            Ok(serde_json::json!({ "files": files, "count": files.len() }))
        }
        Command::Optimize {
            file,
            out,
            password,
        } => {
            let r = sunipdf_core::convert::optimize_file(&file, &out, password.as_deref())?;
            Ok(
                serde_json::json!({ "out": out, "before": r.before, "after": r.after,
                "saved_percent": if r.before > 0 { 100.0 * (1.0 - r.after as f64 / r.before as f64) } else { 0.0 } }),
            )
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

/// "all", "1,3-5" (1-based) → indices 0-based validés.
fn parse_pages(spec: &str, count: usize) -> Result<Vec<usize>, EngineError> {
    if spec.trim().eq_ignore_ascii_case("all") {
        return Ok((0..count).collect());
    }
    let mut out = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (part, part),
        };
        let bad = || EngineError::Other(format!("invalid page specification: {part}"));
        let (a, b): (usize, usize) = (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
        if a == 0 || b < a {
            return Err(bad());
        }
        if b > count {
            return Err(EngineError::PageOutOfRange(b - 1));
        }
        out.extend((a - 1)..b);
    }
    if out.is_empty() {
        return Err(EngineError::Other("no pages selected".into()));
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_page_specifications() {
        assert_eq!(parse_pages("all", 3).unwrap(), vec![0, 1, 2]);
        assert_eq!(parse_pages("1,3-4", 5).unwrap(), vec![0, 2, 3]);
        assert!(matches!(
            parse_pages("2-9", 5),
            Err(EngineError::PageOutOfRange(8))
        ));
        assert!(parse_pages("0", 5).is_err());
        assert!(parse_pages("3-1", 5).is_err());
        assert!(parse_pages("x", 5).is_err());
        assert!(parse_pages("", 5).is_err());
    }
}
