//! CLI SuniPDF : sortie JSON orientée automatisation (messages en anglais, codes d'erreur stables).
//! Même moteur que l'application de bureau : `sunipdf-core`.

mod annot_cmd;

use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use sunipdf_core::{
    form_data, Engine, EngineError, FillMode, ImportPlan, Stamp, StampPosition, TextStyle,
};

#[derive(Parser)]
#[command(
    name = "sunipdf-cli",
    version,
    about = "SuniPDF command-line interface"
)]
struct Cli {
    /// Allow replacing existing output files. Without it, every command refuses to overwrite
    /// anything (exit code 6, error "already_exists").
    #[arg(long, global = true)]
    overwrite: bool,
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
        #[arg(long, allow_negative_numbers = true)]
        degrees: i32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Stamp text on pages (watermark): {n} = page number, {total} = page count.
    Stamp {
        file: PathBuf,
        #[arg(long)]
        text: String,
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long, default_value_t = 48.0)]
        size: f32,
        /// Hex colour, e.g. 808080 or ff0000.
        #[arg(long, default_value = "808080")]
        color: String,
        #[arg(long, default_value_t = 0.3)]
        opacity: f32,
        #[arg(long, default_value_t = 45.0, allow_negative_numbers = true)]
        angle: f32,
        /// center, top-left, top-center, top-right, bottom-left, bottom-center, bottom-right
        #[arg(long, default_value = "center")]
        position: String,
        #[arg(long)]
        bold: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Add page numbers: --template uses {n} and {total} (default "{n} / {total}").
    PageNumbers {
        file: PathBuf,
        #[arg(long, default_value = "{n} / {total}")]
        template: String,
        #[arg(long, default_value = "bottom-center")]
        position: String,
        #[arg(long, default_value_t = 10.0)]
        size: f32,
        #[arg(long, default_value = "all")]
        pages: String,
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
    /// Irreversibly redact rectangles and/or every occurrence of a text, then save to --out.
    /// Content under the areas is removed from the file, not covered. Nothing is written if the
    /// result cannot be verified (error "redaction_incomplete").
    Redact {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Area "PAGE:LEFT,BOTTOM,RIGHT,TOP" (1-based page, points, origin bottom-left of the page
        /// as displayed, the coordinates printed by `search`). Repeatable.
        #[arg(long = "rect")]
        rects: Vec<String>,
        /// Redact every occurrence of this text. Repeatable.
        #[arg(long = "text")]
        texts: Vec<String>,
        #[arg(long)]
        match_case: bool,
        /// Replace every redacted page by an image of itself (no text left on these pages).
        #[arg(long)]
        rasterize: bool,
        /// Keep metadata and the structure tree (removed by default).
        #[arg(long)]
        keep_hidden_data: bool,
        /// Resolution of the image patches and rasterized pages (72 to 600).
        #[arg(long, default_value_t = 200.0)]
        dpi: f32,
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
    /// Search text; prints matches (page, rectangles in points, origin bottom-left of the page as
    /// displayed, i.e. after /Rotate and CropBox) as JSON.
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
    /// List the content objects of a page (0-based indices, display-space coordinates in points).
    Objects {
        file: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        password: Option<String>,
    },
    /// Add text (one object per line, "\n" separated) with its first baseline at (x, y) points.
    AddText {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long, allow_negative_numbers = true)]
        x: f32,
        #[arg(long, allow_negative_numbers = true)]
        y: f32,
        #[arg(long)]
        text: String,
        #[arg(long, default_value_t = 12.0)]
        size: f32,
        #[arg(long, default_value = "000000")]
        color: String,
        #[arg(long, default_value_t = 1.0)]
        opacity: f32,
        /// TrueType font file to embed (whole file).
        #[arg(long, conflicts_with = "font_name")]
        font_file: Option<PathBuf>,
        /// Name of an installed system font, as listed by `fonts` (e.g. "Arial Bold").
        #[arg(long)]
        font_name: Option<String>,
        #[arg(long)]
        bold: bool,
        #[arg(long)]
        italic: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Set the opacity (0 to 1) of objects (0-based indices, e.g. "0,3").
    SetOpacity {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        objects: String,
        #[arg(long)]
        opacity: f32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Rotate one object by any angle (degrees, clockwise) around its centre.
    RotateObject {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        object: usize,
        #[arg(long, allow_negative_numbers = true)]
        degrees: f32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Copy objects (pasted as one group) to another page, optionally of another file (--into).
    CopyObjects {
        file: PathBuf,
        #[arg(long)]
        from_page: usize,
        /// 0-based object indices, e.g. "0,2".
        #[arg(long)]
        objects: String,
        #[arg(long)]
        to_page: usize,
        /// Destination document (default: the source document itself).
        #[arg(long)]
        into: Option<PathBuf>,
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        dx: f32,
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        dy: f32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Replace the content of an image object with another image file (same frame).
    ReplaceImage {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        object: usize,
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Place an image on a page, e.g. a scanned handwritten signature. Visual mark only: this is
    /// NOT a digital signature (no certificate, no integrity protection, removable by anyone).
    AddImage {
        file: PathBuf,
        /// Image file; a PNG with transparency is used as is.
        #[arg(long)]
        image: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Bottom-left corner, in points from the bottom-left of the page as displayed.
        #[arg(long, allow_negative_numbers = true)]
        x: f32,
        #[arg(long, allow_negative_numbers = true)]
        y: f32,
        /// Width in points; the height follows the image proportions.
        #[arg(long, default_value_t = 150.0)]
        width: f32,
        /// Make the light background transparent: luminance threshold 1-255 (200 suits a photo
        /// of a signature on white paper); the result is trimmed to the ink.
        #[arg(long, value_name = "THRESHOLD")]
        remove_background: Option<u8>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Crop pages (hides the margins; content stays in the file) or restore them with --reset.
    Crop {
        file: PathBuf,
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long, default_value_t = 0.0)]
        left: f32,
        #[arg(long, default_value_t = 0.0)]
        top: f32,
        #[arg(long, default_value_t = 0.0)]
        right: f32,
        #[arg(long, default_value_t = 0.0)]
        bottom: f32,
        /// Unit of the margins: pt or mm.
        #[arg(long, default_value = "pt")]
        unit: String,
        #[arg(long)]
        reset: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Resize pages: keep the content (anchored top-left) or scale it to fit (centred).
    ResizePages {
        file: PathBuf,
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long)]
        width: f32,
        #[arg(long)]
        height: f32,
        #[arg(long, default_value = "pt")]
        unit: String,
        /// keep or scale
        #[arg(long, default_value = "keep")]
        mode: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Insert blank pages before page --at (1-based; 0 = append), sized explicitly or like a page.
    InsertBlank {
        file: PathBuf,
        #[arg(long, default_value_t = 0)]
        at: usize,
        #[arg(long, default_value_t = 1)]
        count: usize,
        #[arg(long, requires = "height")]
        width: Option<f32>,
        #[arg(long, requires = "width")]
        height: Option<f32>,
        #[arg(long, default_value = "pt")]
        unit: String,
        /// Use the size of this 1-based page (default: page 1).
        #[arg(long, conflicts_with = "width")]
        like: Option<usize>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// List the installed TrueType (.ttf) system fonts that can be embedded by add-text.
    Fonts {
        /// Case-insensitive substring of the font name.
        #[arg(long)]
        filter: Option<String>,
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
        /// Only render this region "x,y,w,h", in pixels of the page rendered at --scale (origin at
        /// the top left); allows scales whose full page would exceed 16384 px.
        #[arg(long, value_parser = parse_region)]
        region: Option<sunipdf_core::PixelRect>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// List the AcroForm fields (name, type, value, options, widgets with 1-based pages) as JSON.
    FormFields {
        file: PathBuf,
        /// Print only the {name: value} object (the format accepted by `fill-form --values`;
        /// read-only fields are included with their current value, password fields never).
        #[arg(long)]
        values_only: bool,
        #[arg(long)]
        password: Option<String>,
    },
    /// Fill fields from a JSON object (--values FILE, "-" = stdin) and/or --set NAME=VALUE
    /// (repeatable; checkbox: true/false, list box: choices separated by ";"). Strict by default:
    /// an unknown field or a rejected value fails before anything is written.
    FillForm {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        values: Option<PathBuf>,
        #[arg(long = "set", value_name = "NAME=VALUE")]
        set: Vec<String>,
        /// Skip unknown fields and rejected values instead of failing (they are reported).
        #[arg(long)]
        ignore_unknown: bool,
        /// Flatten the whole document after filling (fields become page content).
        #[arg(long)]
        flatten: bool,
        #[arg(long)]
        password: Option<String>,
    },
    /// Flatten form fields and annotations into the page content ("all" or "1,3-5"); flattening
    /// every page also removes the form from the document.
    Flatten {
        file: PathBuf,
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// List the annotations of one page (--page) or of every page as JSON: index (0-based, the
    /// one the other annotation commands take), subtype, rect (left,bottom,right,top in displayed
    /// page points), text, author, style, link target, reply thread and review state.
    Annotations {
        file: PathBuf,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long)]
        password: Option<String>,
    },
    /// Add a real PDF annotation: ink, square, circle, line, arrow, freetext or link. Coordinates
    /// are displayed-page points (rotation and crop applied). An encrypted document stays encrypted.
    Annotate {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// ink, square, circle, line, arrow, freetext or link.
        #[arg(long)]
        kind: String,
        /// left,bottom,right,top (a line or arrow runs from left,bottom to right,top).
        #[arg(long, allow_negative_numbers = true)]
        rect: Option<String>,
        /// Ink stroke as "x,y x,y ..." (repeatable, one stroke each).
        #[arg(long = "stroke", value_name = "POINTS", allow_negative_numbers = true)]
        strokes: Vec<String>,
        /// Text of a free text box.
        #[arg(long)]
        text: Option<String>,
        /// Font size of a free text box.
        #[arg(long, default_value_t = 12.0)]
        size: f32,
        /// Stroke colour (free text: text colour), RRGGBB.
        #[arg(long)]
        color: Option<String>,
        /// Fill colour (free text: background), RRGGBB.
        #[arg(long)]
        fill: Option<String>,
        /// Line width in points (free text: border; default 2, free text 0).
        #[arg(long)]
        width: Option<f32>,
        #[arg(long, default_value_t = 1.0)]
        opacity: f32,
        /// Link to a page (1-based).
        #[arg(long, conflicts_with = "url")]
        goto_page: Option<usize>,
        /// Link to http, https or mailto address; anything else is refused.
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Delete an annotation by index; --thread also deletes its replies and review states.
    AnnotationDelete {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        thread: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Reply to an annotation (comment thread, no icon on the page).
    Reply {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        text: String,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Set the review state of an annotation thread: none, accepted, rejected, cancelled, completed.
    ReviewState {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        state: String,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
}

/// `NAME=VALUE` coupé au premier `=` (la valeur peut en contenir).
fn split_set(spec: &str) -> Result<(&str, &str), EngineError> {
    match spec.split_once('=') {
        Some((name, value)) if !name.trim().is_empty() => Ok((name.trim(), value)),
        _ => Err(EngineError::Other(format!(
            "expected NAME=VALUE, got {spec:?}"
        ))),
    }
}

/// "x,y,w,h" → région en pixels.
fn parse_region(spec: &str) -> Result<sunipdf_core::PixelRect, String> {
    let v: Vec<u32> = spec
        .split(',')
        .map(|p| p.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|e| format!("expected x,y,w,h (non-negative integers): {e}"))?;
    match v[..] {
        [x, y, w, h] => Ok(sunipdf_core::PixelRect::new(x, y, w, h)),
        _ => Err(format!("expected 4 values x,y,w,h, got {}", v.len())),
    }
}

fn run(cli: Cli) -> Result<serde_json::Value, EngineError> {
    if let Command::Fonts { filter } = &cli.command {
        let filter = filter.as_deref().unwrap_or("").to_lowercase();
        return Ok(serde_json::json!(sunipdf_core::fonts::system_fonts()
            .into_iter()
            .filter(|f| f.name.to_lowercase().contains(&filter))
            .map(|f| serde_json::json!({
                "name": f.name, "path": f.path, "size_bytes": f.size_bytes,
            }))
            .collect::<Vec<_>>()));
    }
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
    let overwrite = cli.overwrite;
    if !overwrite {
        if let Some(out) = output_path(&cli.command) {
            if out.exists() {
                return Err(EngineError::AlreadyExists(out.to_path_buf()));
            }
        }
    }
    if let Command::Rotate { degrees, .. } = &cli.command {
        quarter_turn(*degrees)?;
    }
    let engine = Engine::new()?;
    match cli.command {
        Command::Commands | Command::Fonts { .. } => unreachable!(),
        Command::Text {
            file,
            page,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let pages: Vec<usize> = match page {
                Some(n) => vec![n.checked_sub(1).ok_or_else(page_zero)?],
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
            // D046 : les pages copiées dans un document neuf ne sont pas chiffrées ; dit ici.
            Ok(serde_json::json!({
                "out": out, "page_count": n, "encryption_removed": doc.is_encrypted()
            }))
        }
        Command::Split {
            file,
            every,
            out_dir,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let stem = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "document".into());
            let files = doc.split_to(every, &out_dir, &stem, overwrite)?;
            Ok(serde_json::json!({
                "files": files, "count": files.len(), "encryption_removed": doc.is_encrypted()
            }))
        }
        Command::Rotate {
            file,
            pages,
            degrees,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let delta = quarter_turn(degrees)?;
            for p in &selection {
                let current = doc.page_rotation(*p)?;
                doc.set_page_rotation(*p, current.turned(delta))?;
            }
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "rotated": selection.len() }))
        }
        Command::Stamp {
            file,
            text,
            pages,
            size,
            color,
            opacity,
            angle,
            position,
            bold,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let position = StampPosition::from_id(&position)
                .ok_or_else(|| EngineError::Other(format!("unknown position: {position}")))?;
            let stamp = Stamp {
                text,
                style: TextStyle {
                    size_pt: size,
                    bold,
                    rgb: parse_hex_color(&color)?,
                    opacity,
                    ..Default::default()
                },
                angle,
                position,
                margin_pt: 28.0,
            };
            let count = doc.stamp_pages(Some(&selection), &stamp)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "stamped": count }))
        }
        Command::PageNumbers {
            file,
            template,
            position,
            size,
            pages,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let position = StampPosition::from_id(&position)
                .ok_or_else(|| EngineError::Other(format!("unknown position: {position}")))?;
            let stamp = Stamp {
                style: TextStyle {
                    size_pt: size,
                    ..Default::default()
                },
                ..Stamp::page_numbers(&template, position)
            };
            let count = doc.stamp_pages(Some(&selection), &stamp)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "numbered": count }))
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
                from.checked_sub(1).ok_or_else(page_zero)?,
                to.checked_sub(1).ok_or_else(page_zero)?,
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
        Command::Redact {
            file,
            out,
            rects,
            texts,
            match_case,
            rasterize,
            keep_hidden_data,
            dpi,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let mut marks = rects
                .iter()
                .map(|s| parse_rect_spec(s))
                .collect::<Result<Vec<_>, _>>()?;
            for t in &texts {
                marks.extend(doc.redact_marks_for_text(t, match_case)?);
            }
            if marks.is_empty() {
                return Err(EngineError::Other(
                    "nothing to redact: no rectangle given and no occurrence found".into(),
                ));
            }
            let options = sunipdf_core::RedactOptions {
                rasterize_pages: rasterize,
                clean_hidden_data: !keep_hidden_data,
                raster_dpi: dpi,
            };
            let r = doc.redact(&marks, &options)?;
            let mut remaining = 0;
            for t in &texts {
                remaining += doc.search(t, match_case)?.len();
            }
            if remaining > 0 {
                return Err(EngineError::RedactionIncomplete);
            }
            doc.save_as(&out)?;
            Ok(serde_json::json!({
                "out": out, "marks": r.marks, "pages": r.pages, "objects_removed": r.objects_removed,
                "annotations_removed": r.annotations_removed, "fields_removed": r.fields_removed,
                "pages_rasterized": r.pages_rasterized, "hidden_data_cleaned": r.hidden_data_cleaned,
                "remaining_matches": remaining,
            }))
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
            let files = doc.export_images(&out_dir, dpi, overwrite)?;
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
                    "index": i + 1, "width_pt": s.width_pt, "height_pt": s.height_pt,
                    "rotation": doc.page_rotation(i).map(|r| r.degrees()).unwrap_or(0),
                })).collect::<Vec<_>>(),
            }))
        }
        Command::Objects {
            file,
            page,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let size = doc.page_size(p)?;
            let hex =
                |c: Option<(u8, u8, u8)>| c.map(|(r, g, b)| format!("#{r:02x}{g:02x}{b:02x}"));
            Ok(serde_json::json!({
                "page": page,
                "rotation": doc.page_rotation(p)?.degrees(),
                "size": { "width_pt": size.width_pt, "height_pt": size.height_pt },
                "objects": doc.page_objects(p)?.iter().map(|o| serde_json::json!({
                    "index": o.index,
                    "kind": format!("{:?}", o.kind).to_lowercase(),
                    "bounds": [o.bounds.left, o.bounds.bottom, o.bounds.right, o.bounds.top],
                    "angle": o.angle_deg,
                    "opacity": o.opacity,
                    "text": o.text,
                    "font": o.font_name,
                    "font_size": (o.kind == sunipdf_core::ObjectKind::Text).then_some(o.font_size),
                    "fill": hex(o.fill_rgb),
                    "stroke": hex(o.stroke_rgb),
                })).collect::<Vec<_>>(),
            }))
        }
        Command::AddText {
            file,
            page,
            x,
            y,
            text,
            size,
            color,
            opacity,
            font_file,
            font_name,
            bold,
            italic,
            out,
            password,
        } => {
            let font_file = match (font_file, font_name) {
                (Some(path), _) => Some(path),
                (None, Some(name)) => Some(
                    sunipdf_core::fonts::system_fonts()
                        .into_iter()
                        .find(|f| f.name.eq_ignore_ascii_case(&name))
                        .ok_or_else(|| EngineError::Other(format!("unknown system font: {name}")))?
                        .path,
                ),
                (None, None) => None,
            };
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let style = TextStyle {
                size_pt: size,
                bold,
                italic,
                rgb: parse_hex_color(&color)?,
                opacity,
                font_file: font_file.clone(),
                ..Default::default()
            };
            let added = doc.add_text(p, x, y, &text.replace("\\n", "\n"), &style)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "added": added, "embedded_font": font_file }))
        }
        Command::SetOpacity {
            file,
            page,
            objects,
            opacity,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let indices = parse_indices(&objects)?;
            doc.set_objects_style(
                p,
                &indices,
                &sunipdf_core::ObjectStyle {
                    opacity: Some(opacity),
                    ..Default::default()
                },
            )?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "objects": indices.len(),
                    "opacity": (opacity.clamp(0.0, 1.0) as f64 * 1000.0).round() / 1000.0 }))
        }
        Command::RotateObject {
            file,
            page,
            object,
            degrees,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            doc.rotate_object(p, object, degrees)?;
            let angle = doc.object_info(p, object)?.angle_deg;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "angle": angle }))
        }
        Command::CopyObjects {
            file,
            from_page,
            objects,
            to_page,
            into,
            dx,
            dy,
            out,
            password,
        } => {
            let src = engine.open(&file, password.as_deref())?;
            let from = page_index(from_page, src.page_count())?;
            let indices = parse_indices(&objects)?;
            let clip = src.copy_objects(from, &indices)?;
            let mut dest = match &into {
                Some(path) => engine.open(path, None)?,
                None => src,
            };
            let to = page_index(to_page, dest.page_count())?;
            let pasted = dest.paste_objects(to, &clip, dx, dy)?;
            dest.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "copied": clip.count, "pasted_index": pasted }))
        }
        Command::ReplaceImage {
            file,
            page,
            object,
            image,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            doc.replace_image(p, object, &image)?;
            let b = doc.object_info(p, object)?.bounds;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "bounds": [b.left, b.bottom, b.right, b.top] }))
        }
        Command::AddImage {
            file,
            image,
            page,
            x,
            y,
            width,
            remove_background,
            out,
            password,
        } => {
            use sunipdf_core::signature_image as sig;
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let mut img = sig::load(&image, 4096)?;
            if let Some(t) = remove_background {
                img = sig::trim(&sig::remove_background(&img, t), 4).ok_or_else(|| {
                    EngineError::Other("image is empty after background removal".into())
                })?;
            } else if sig::is_blank(&img) {
                // A fully transparent image would place an invisible object.
                return Err(EngineError::Other("image is fully transparent".into()));
            }
            let (w, h) = img.dimensions();
            doc.add_image_data(p, &image::DynamicImage::ImageRgba8(img), x, y, width)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({
                "out": out,
                "page": page,
                "x": x,
                "y": y,
                "width_pt": width,
                "height_pt": width * h as f32 / w as f32,
                "background_removed": remove_background.is_some(),
                // Constant: a visual mark never makes the document digitally signed.
                "digital_signature": false,
            }))
        }
        Command::Crop {
            file,
            pages,
            left,
            top,
            right,
            bottom,
            unit,
            reset,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let k = unit_factor(&unit)?;
            let margins = sunipdf_core::PageMargins {
                left: left * k,
                top: top * k,
                right: right * k,
                bottom: bottom * k,
            };
            for &p in &selection {
                if reset {
                    doc.reset_crop(p)?;
                } else {
                    doc.crop_page(p, margins)?;
                }
            }
            doc.save_as(&out)?;
            Ok(serde_json::json!({
                "out": out,
                "cropped": selection.len(),
                "sizes": selection.iter().map(|&p| {
                    let s = doc.page_size(p).unwrap_or(sunipdf_core::PageSize { width_pt: 0.0, height_pt: 0.0 });
                    serde_json::json!({ "page": p + 1, "width_pt": s.width_pt, "height_pt": s.height_pt })
                }).collect::<Vec<_>>(),
            }))
        }
        Command::ResizePages {
            file,
            pages,
            width,
            height,
            unit,
            mode,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let k = unit_factor(&unit)?;
            let mode = match mode.as_str() {
                "keep" => sunipdf_core::ResizeMode::KeepContent,
                "scale" => sunipdf_core::ResizeMode::ScaleContent,
                other => return Err(EngineError::Other(format!("unknown mode: {other}"))),
            };
            for &p in &selection {
                doc.resize_page(p, width * k, height * k, mode)?;
            }
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "resized": selection.len() }))
        }
        Command::InsertBlank {
            file,
            at,
            count,
            width,
            height,
            unit,
            like,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let (w, h) = match (width, height) {
                (Some(w), Some(h)) => {
                    let k = unit_factor(&unit)?;
                    (w * k, h * k)
                }
                _ => {
                    let s = doc.page_size(page_index(like.unwrap_or(1), doc.page_count())?)?;
                    (s.width_pt, s.height_pt)
                }
            };
            let at = if at == 0 { doc.page_count() } else { at - 1 };
            doc.insert_blank_pages(at, count, w, h)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "inserted": count, "page_count": doc.page_count() }))
        }
        Command::Render {
            file,
            page,
            scale,
            region,
            out,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let index = page.checked_sub(1).ok_or_else(page_zero)?;
            let img = match region {
                Some(r) => doc.render_region(index, scale, r)?,
                None => doc.render_page(index, scale)?,
            };
            let page_pixels = sunipdf_core::tiles::page_pixels(doc.page_size(index)?, scale);
            // Toujours du PNG (quelle que soit l'extension), écrit de façon atomique.
            let mut png = Vec::new();
            image::write_buffer_with_format(
                &mut std::io::Cursor::new(&mut png),
                &img.rgba,
                img.width,
                img.height,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            )
            .map_err(|e| EngineError::Other(e.to_string()))?;
            sunipdf_core::write_atomic(&out, &png)?;
            Ok(serde_json::json!({
                "out": out,
                "width": img.width,
                "height": img.height,
                "page": page,
                "region": region.map(|r| [r.x, r.y, r.width, r.height]),
                "page_pixels": page_pixels.map(|(w, h)| [w, h]),
            }))
        }
        Command::FormFields {
            file,
            values_only,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let info = doc.form_info()?;
            Ok(if values_only {
                form_data::values_to_json(&info)
            } else {
                form_data::info_to_json(&info)
            })
        }
        Command::Annotations {
            file,
            page,
            password,
        } => {
            let doc = engine.open(&file, password.as_deref())?;
            let pages: Vec<usize> = match page {
                Some(n) => vec![page_index(n, doc.page_count())?],
                None => (0..doc.page_count()).collect(),
            };
            let mut out = Vec::new();
            for p in pages {
                let list: Vec<_> = doc
                    .annotation_details(p)?
                    .iter()
                    .map(annot_cmd::detail_json)
                    .collect();
                out.push(serde_json::json!({ "page": p + 1, "annotations": list }));
            }
            Ok(serde_json::json!({ "pages": out }))
        }
        Command::Annotate {
            file,
            page,
            kind,
            rect,
            strokes,
            text,
            size,
            color,
            fill,
            width,
            opacity,
            goto_page,
            url,
            author,
            out,
            password,
        } => {
            use sunipdf_core::{AnnotShape, AnnotStyle, FreeTextStyle, LinkTarget};
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let need_rect = || -> Result<sunipdf_core::PointRect, EngineError> {
                annot_cmd::parse_rect(
                    rect.as_deref()
                        .ok_or_else(|| EngineError::Other("--rect is required".into()))?,
                )
            };
            let stroke_rgb = color.as_deref().map(parse_hex_color).transpose()?;
            let fill_rgb = fill.as_deref().map(parse_hex_color).transpose()?;
            let style = AnnotStyle {
                rgb: stroke_rgb.unwrap_or(AnnotStyle::default().rgb),
                fill_rgb,
                width_pt: width.unwrap_or(2.0),
                opacity,
            };
            let author = author.as_deref();
            let index = match kind.as_str() {
                "ink" => {
                    let strokes = strokes
                        .iter()
                        .map(|s| annot_cmd::parse_stroke(s))
                        .collect::<Result<Vec<_>, _>>()?;
                    doc.add_ink_annotation(p, &strokes, &style, author)?
                }
                "square" | "circle" | "line" | "arrow" => {
                    let shape = match kind.as_str() {
                        "square" => AnnotShape::Square,
                        "circle" => AnnotShape::Circle,
                        "line" => AnnotShape::Line,
                        _ => AnnotShape::Arrow,
                    };
                    doc.add_shape_annotation(p, shape, need_rect()?, &style, author)?
                }
                "freetext" => {
                    let text = text
                        .as_deref()
                        .ok_or_else(|| EngineError::Other("--text is required".into()))?;
                    let fts = FreeTextStyle {
                        size_pt: size,
                        rgb: stroke_rgb.unwrap_or((0, 0, 0)),
                        fill_rgb,
                        border_pt: width.unwrap_or(0.0),
                    };
                    doc.add_free_text_annotation(p, need_rect()?, text, &fts, author)?
                }
                "link" => {
                    let target = match (goto_page, url) {
                        (Some(n), None) => {
                            LinkTarget::Page(n.checked_sub(1).ok_or_else(page_zero)?)
                        }
                        (None, Some(u)) => LinkTarget::Uri(u),
                        _ => {
                            return Err(EngineError::Other(
                                "a link needs --goto-page or --url".into(),
                            ))
                        }
                    };
                    doc.add_link_annotation(p, need_rect()?, &target)?
                }
                other => {
                    return Err(EngineError::Other(format!(
                        "unknown annotation kind: {other}"
                    )))
                }
            };
            doc.save_as(&out)?;
            Ok(serde_json::json!({
                "out": out,
                "page": page,
                "kind": kind,
                "index": index,
                "annotation_count": doc.annotations(p)?.len(),
            }))
        }
        Command::AnnotationDelete {
            file,
            page,
            index,
            thread,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let deleted = if thread {
                doc.delete_annotation_thread(p, index)?
            } else {
                doc.delete_annotation(p, index)?;
                1
            };
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "deleted": deleted }))
        }
        Command::Reply {
            file,
            page,
            index,
            text,
            author,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let reply = doc.add_reply(p, index, &text, author.as_deref())?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "index": reply }))
        }
        Command::ReviewState {
            file,
            page,
            index,
            state,
            author,
            out,
            password,
        } => {
            let status = sunipdf_core::ReviewStatus::from_id(&state)
                .ok_or_else(|| EngineError::Other(format!("unknown review state: {state}")))?;
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            doc.set_review_status(p, index, status, author.as_deref())?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "state": status.id() }))
        }
        Command::FillForm {
            file,
            out,
            values,
            set,
            ignore_unknown,
            flatten,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let info = doc.form_info()?;
            let mut plan = ImportPlan::default();
            if let Some(path) = values {
                let text = if path.as_os_str() == "-" {
                    let mut s = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
                    s
                } else {
                    std::fs::read_to_string(&path)?
                };
                let json: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| EngineError::FormValueInvalid(format!("invalid JSON: {e}")))?;
                plan = form_data::plan_from_json(&info, &json)?;
            }
            for spec in &set {
                let (name, value) = split_set(spec)?;
                match info.field(name) {
                    // Lecture seule avec sa valeur courante : accepté, rien à écrire.
                    Some(f) => match f.coerce_text(value) {
                        Ok(_) if f.read_only => {}
                        Ok(v) => plan.values.push((name.to_string(), v)),
                        Err(reason) => plan.skipped_invalid.push((name.to_string(), reason)),
                    },
                    None => plan.skipped_unknown.push(name.to_string()),
                }
            }
            if !ignore_unknown {
                if let Some(n) = plan.skipped_unknown.first() {
                    return Err(EngineError::FormFieldNotFound(n.clone()));
                }
                if let Some((n, reason)) = plan.skipped_invalid.first() {
                    return Err(EngineError::FormValueInvalid(format!("{n}: {reason}")));
                }
            }
            let mode = if ignore_unknown {
                FillMode::Lenient
            } else {
                FillMode::Strict
            };
            let report = doc.fill_form(&plan.values, mode)?;
            if flatten {
                doc.flatten(None)?;
            }
            doc.save_as(&out)?;
            let mut unknown = plan.skipped_unknown;
            unknown.extend(report.skipped_unknown);
            let invalid: Vec<serde_json::Value> = plan
                .skipped_invalid
                .into_iter()
                .chain(report.skipped_invalid)
                .map(|(field, reason)| serde_json::json!({ "field": field, "reason": reason }))
                .collect();
            Ok(serde_json::json!({
                "out": out,
                "applied": report.applied.len(),
                "skipped": { "unknown": unknown, "invalid": invalid },
                "flattened": flatten,
                "page_count": doc.page_count(),
            }))
        }
        Command::Flatten {
            file,
            pages,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let all = pages.trim().eq_ignore_ascii_case("all");
            let n = doc.flatten(if all { None } else { Some(&selection) })?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "annotations": n, "pages": selection.len() }))
        }
    }
}

/// "all", "1,3-5" (1-based) → indices 0-based validés.
/// Chemin de sortie d'une commande qui écrit un seul fichier.
fn output_path(command: &Command) -> Option<&std::path::Path> {
    match command {
        Command::Merge { out, .. }
        | Command::Extract { out, .. }
        | Command::Rotate { out, .. }
        | Command::Stamp { out, .. }
        | Command::PageNumbers { out, .. }
        | Command::Insert { out, .. }
        | Command::Move { out, .. }
        | Command::Protect { out, .. }
        | Command::Unprotect { out, .. }
        | Command::SetMetadata { out, .. }
        | Command::StripMetadata { out, .. }
        | Command::Redact { out, .. }
        | Command::Images2pdf { out, .. }
        | Command::Optimize { out, .. }
        | Command::AddText { out, .. }
        | Command::SetOpacity { out, .. }
        | Command::RotateObject { out, .. }
        | Command::CopyObjects { out, .. }
        | Command::ReplaceImage { out, .. }
        | Command::AddImage { out, .. }
        | Command::Crop { out, .. }
        | Command::ResizePages { out, .. }
        | Command::InsertBlank { out, .. }
        | Command::FillForm { out, .. }
        | Command::Flatten { out, .. }
        | Command::Annotate { out, .. }
        | Command::AnnotationDelete { out, .. }
        | Command::Reply { out, .. }
        | Command::ReviewState { out, .. }
        | Command::Render { out, .. } => Some(out.as_path()),
        _ => None,
    }
}

/// 90, 180, 270 (ou leurs équivalents négatifs) → quart de tour horaire ; le reste est refusé.
fn quarter_turn(degrees: i32) -> Result<i32, EngineError> {
    match degrees.rem_euclid(360) {
        d @ (90 | 180 | 270) => Ok(d),
        _ => Err(EngineError::Other(format!(
            "degrees must be 90, 180 or 270 (or -90), got {degrees}"
        ))),
    }
}

/// Pages triées sans doublon (opérations qui ne doivent toucher chaque page qu'une fois).
fn unique(mut pages: Vec<usize>) -> Vec<usize> {
    pages.sort_unstable();
    pages.dedup();
    pages
}

fn page_zero() -> EngineError {
    EngineError::Other("page numbers start at 1".into())
}

/// Numéro de page saisi (base 1) → index validé (base 0).
fn page_index(page: usize, count: usize) -> Result<usize, EngineError> {
    let p = page.checked_sub(1).ok_or_else(page_zero)?;
    if p >= count {
        return Err(EngineError::PageOutOfRange(p));
    }
    Ok(p)
}

/// "0,3,5" → index d'objets (base 0).
fn parse_indices(spec: &str) -> Result<Vec<usize>, EngineError> {
    let v: Result<Vec<usize>, _> = spec
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect();
    match v {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(EngineError::Other(format!("invalid object list: {spec}"))),
    }
}

/// Points par unité : "pt" → 1, "mm" → 72 / 25,4.
fn unit_factor(unit: &str) -> Result<f32, EngineError> {
    match unit {
        "pt" => Ok(1.0),
        "mm" => Ok(72.0 / 25.4),
        other => Err(EngineError::Other(format!(
            "unknown unit: {other} (pt or mm)"
        ))),
    }
}

fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8), EngineError> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EngineError::Other(format!("invalid colour: {hex}")));
    }
    let byte = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16)
            .map_err(|_| EngineError::Other(format!("invalid colour: {hex}")))
    };
    Ok((byte(0)?, byte(2)?, byte(4)?))
}

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

/// "PAGE:LEFT,BOTTOM,RIGHT,TOP" (page en base 1, points d'affichage) → zone de caviardage.
fn parse_rect_spec(spec: &str) -> Result<sunipdf_core::RedactMark, EngineError> {
    let bad = || {
        EngineError::Other(format!(
            "invalid rectangle: {spec:?} (expected PAGE:L,B,R,T)"
        ))
    };
    let (page, coords) = spec.split_once(':').ok_or_else(bad)?;
    let page: usize = page.trim().parse().map_err(|_| bad())?;
    let v = coords
        .split(',')
        .map(|c| c.trim().parse::<f32>().map_err(|_| bad()))
        .collect::<Result<Vec<_>, _>>()?;
    if page == 0 || v.len() != 4 || !v.iter().all(|x| x.is_finite()) {
        return Err(bad());
    }
    Ok(sunipdf_core::RedactMark {
        page: page - 1,
        rect: sunipdf_core::PointRect {
            left: v[0],
            bottom: v[1],
            right: v[2],
            top: v[3],
        },
    })
}

fn main() -> ExitCode {
    // Pile de 1 Mio du fil principal sous Windows : l'analyse clap et `run` (un `match` de 30
    // sous-commandes riches en `json!`) la dépassent en compilation de débogage. Un fil dédié de
    // 16 Mio, seul utilisateur du moteur (PDFium reste sur un seul fil).
    std::thread::Builder::new()
        .name("sunipdf-cli".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(cli_main)
        .expect("fil de la CLI")
        .join()
        .unwrap_or(ExitCode::FAILURE)
}

fn cli_main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(serde_json::Value::Null) => ExitCode::SUCCESS,
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).expect("json"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            // Numéros de page affichés en base 1, comme à la saisie.
            let message = match &e {
                EngineError::PageOutOfRange(i) if *i != usize::MAX => {
                    format!("page {} out of range", i + 1)
                }
                _ => e.to_string(),
            };
            eprintln!(
                "{}",
                serde_json::json!({ "error": e.code(), "message": message })
            );
            ExitCode::from(match e {
                EngineError::PasswordRequired => 3,
                EngineError::InvalidFormat => 4,
                EngineError::Io(_) => 5,
                EngineError::AlreadyExists(_) => 6,
                _ => 1,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rect_specifications() {
        let m = parse_rect_spec("2: 72, 500.5,272,560").unwrap();
        assert_eq!(m.page, 1);
        assert_eq!(
            (m.rect.left, m.rect.bottom, m.rect.right, m.rect.top),
            (72.0, 500.5, 272.0, 560.0)
        );
        for bad in [
            "0:1,2,3,4",
            "1:1,2,3",
            "1,2,3,4",
            "x:1,2,3,4",
            "1:1,2,3,NaN",
            "1:a,2,3,4",
        ] {
            assert!(parse_rect_spec(bad).is_err(), "{bad}");
        }
    }

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

    #[test]
    fn validates_colours_angles_and_selections() {
        assert_eq!(parse_hex_color("#FF8000").unwrap(), (255, 128, 0));
        for bad in ["1€23", "aé123", "+f+f+f", "12345", "GGGGGG"] {
            assert!(parse_hex_color(bad).is_err(), "{bad}");
        }
        assert_eq!(quarter_turn(90).unwrap(), 90);
        assert_eq!(quarter_turn(-90).unwrap(), 270);
        assert_eq!(quarter_turn(540).unwrap(), 180);
        for bad in [0, 45, 360, -45] {
            assert!(quarter_turn(bad).is_err(), "{bad}");
        }
        assert_eq!(unique(vec![2, 0, 1, 2, 1]), vec![0, 1, 2]);
    }

    #[test]
    fn set_specs_split_at_the_first_equals_sign() {
        assert_eq!(split_set("nom=Jean").unwrap(), ("nom", "Jean"));
        assert_eq!(split_set("a=b=c").unwrap(), ("a", "b=c"));
        assert_eq!(split_set("vide=").unwrap(), ("vide", ""));
        assert!(split_set("sans").is_err());
        assert!(split_set("=x").is_err());
    }

    #[test]
    fn render_region_parses() {
        assert_eq!(
            parse_region("4000, 7000,256,256").unwrap(),
            sunipdf_core::PixelRect::new(4000, 7000, 256, 256)
        );
        for bad in ["1,2,3", "1,2,3,4,5", "-1,0,1,1", "a,b,c,d", ""] {
            assert!(parse_region(bad).is_err(), "{bad}");
        }
        let cli = Cli::try_parse_from([
            "sunipdf-cli",
            "render",
            "a.pdf",
            "--scale",
            "40",
            "--region",
            "4000,7000,256,256",
            "--out",
            "tile.png",
        ])
        .unwrap();
        assert_eq!(
            output_path(&cli.command),
            Some(std::path::Path::new("tile.png"))
        );
        assert!(Cli::try_parse_from([
            "sunipdf-cli",
            "render",
            "a.pdf",
            "--region",
            "1,2",
            "--out",
            "t.png"
        ])
        .is_err());
    }

    #[test]
    fn object_lists_units_and_pages() {
        assert_eq!(parse_indices("0, 3,5").unwrap(), vec![0, 3, 5]);
        assert!(parse_indices("").is_err());
        assert!(parse_indices("a,1").is_err());
        assert_eq!(unit_factor("pt").unwrap(), 1.0);
        assert!((unit_factor("mm").unwrap() * 25.4 - 72.0).abs() < 1e-4);
        assert!(unit_factor("cm").is_err());
        assert_eq!(page_index(2, 3).unwrap(), 1);
        assert!(page_index(0, 3).is_err());
        assert!(matches!(
            page_index(4, 3),
            Err(EngineError::PageOutOfRange(3))
        ));
        let cli = Cli::try_parse_from([
            "sunipdf-cli",
            "crop",
            "a.pdf",
            "--left",
            "10",
            "--unit",
            "mm",
            "--out",
            "o.pdf",
        ])
        .unwrap();
        assert_eq!(
            output_path(&cli.command),
            Some(std::path::Path::new("o.pdf"))
        );
        assert!(
            Cli::try_parse_from([
                "sunipdf-cli",
                "add-text",
                "a.pdf",
                "--x",
                "1",
                "--y",
                "2",
                "--text",
                "t",
                "--font-file",
                "f.ttf",
                "--font-name",
                "Arial",
                "--out",
                "o.pdf",
            ])
            .is_err(),
            "police : fichier ou nom, pas les deux"
        );
    }

    #[test]
    fn negative_angles_parse_and_overwrite_is_global() {
        let cli = Cli::try_parse_from([
            "sunipdf-cli",
            "stamp",
            "a.pdf",
            "--text",
            "X",
            "--angle",
            "-45",
            "--out",
            "o.pdf",
            "--overwrite",
        ])
        .unwrap();
        assert!(cli.overwrite);
        assert!(matches!(cli.command, Command::Stamp { angle, .. } if angle == -45.0));
        assert!(Cli::try_parse_from([
            "sunipdf-cli",
            "rotate",
            "a.pdf",
            "--degrees",
            "-90",
            "--out",
            "o.pdf",
        ])
        .is_ok());
    }

    #[test]
    fn add_image_parses_and_writes_one_file() {
        let cli = Cli::try_parse_from([
            "sunipdf-cli",
            "add-image",
            "a.pdf",
            "--image",
            "s.png",
            "--x",
            "100",
            "--y",
            "520",
            "--remove-background",
            "200",
            "--out",
            "o.pdf",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::AddImage { width, remove_background: Some(200), page: 1, .. } if width == 150.0
        ));
        assert_eq!(
            output_path(&cli.command).map(|p| p.to_path_buf()),
            Some(PathBuf::from("o.pdf"))
        );
        assert!(
            Cli::try_parse_from(["sunipdf-cli", "add-image", "a.pdf", "--out", "o.pdf"]).is_err()
        );
    }
}
