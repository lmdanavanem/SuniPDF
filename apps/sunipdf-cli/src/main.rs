//! CLI SuniPDF : sortie JSON orientée automatisation (messages en anglais, codes d'erreur stables).
//! Même moteur que l'application de bureau : `sunipdf-core`.

mod annot_cmd;
mod feature_cmd;
mod password_input;
mod safe_output;
mod v010_cmd;

use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use sunipdf_core::{
    form_data, Engine, EngineError, FillMode, ImportPlan, Stamp, StampPosition, TextStyle,
};

const STAMP_HELP_TEMPLATE: &str = "{before-help}{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}{after-help}\nTemplate variables: {n} = page number, {total} = page count.\n";

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
    #[command(flatten)]
    passwords: password_input::PasswordSources,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prepare a local visual-signature PNG; this does not create a cryptographic signature.
    PrepareSignatureImage {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "keep", value_parser = ["keep", "threshold", "adaptive"])]
        background: String,
        #[arg(long, default_value_t = 200, value_parser = clap::value_parser!(u8).range(60..=250))]
        threshold: u8,
    },
    /// Inventory stored PDF dictionaries without executing actions or inspecting payload contents.
    Inspect {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Search only explicitly selected PDF files, with per-file failures and bounded page results.
    SearchFiles {
        #[arg(required = true, num_args = 1..=64)]
        files: Vec<PathBuf>,
        #[arg(long)]
        query: String,
        #[arg(long)]
        match_case: bool,
        /// One credential for the selected files; prefer --password-env or --password-stdin.
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Compare extracted text and 768-pixel previews of aligned physical pages, without modifying inputs.
    Compare {
        left: PathBuf,
        right: PathBuf,
        #[arg(long, default_value_t = 1)]
        left_start: usize,
        #[arg(long, default_value_t = 1)]
        right_start: usize,
        #[arg(long, default_value_t = 200)]
        max_pages: usize,
        /// One credential for both files; prefer --password-env or --password-stdin.
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Add a fillable text, checkbox, radio, combo or list field and save to --out.
    AddFormField {
        file: PathBuf,
        #[arg(long, value_parser = ["text", "checkbox", "radio", "combo", "list"])]
        kind: String,
        #[arg(long)]
        name: String,
        /// Repeat for radio/combo/list choices: EXPORT_VALUE=DISPLAY_LABEL.
        #[arg(long = "option", value_name = "VALUE=LABEL")]
        options: Vec<String>,
        /// Allow several selected choices (list fields only).
        #[arg(long)]
        multi_select: bool,
        /// Allow custom text in addition to listed choices (combo fields only).
        #[arg(long)]
        editable: bool,
        /// Displayed page coordinates: left,bottom,right,top (PDF points).
        #[arg(long, allow_hyphen_values = true)]
        rect: String,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Add a searchable, invisible text layer using local English/French OCR; existing text is skipped.
    Ocr {
        file: PathBuf,
        #[arg(long, default_value = "eng+fra", value_parser = ["eng", "fra", "eng+fra"])]
        language: String,
        /// 1-based page numbers, e.g. "1,3-5", or "all".
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Convert a guarded macro-free Office document with an already installed local LibreOffice.
    OfficeToPdf {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "standard", value_parser = ["standard", "print"])]
        profile: String,
    },
    /// Export simple editable text/cells, or explicit page-image slides, to a new Office copy.
    ExportOffice {
        file: PathBuf,
        #[arg(long, value_parser = ["word", "excel", "powerpoint"])]
        format: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Verify digital signatures over the saved PDF bytes; certificate identity is not trusted automatically.
    Signatures {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
        /// Permit Windows certificate-chain, revocation and timestamp network checks for this call.
        #[arg(long)]
        allow_network: bool,
    },
    /// Sign saved PDF bytes with a PFX/P12 identity into a new copy (encrypted PDFs are unsupported).
    Sign {
        file: PathBuf,
        #[arg(long)]
        certificate: PathBuf,
        /// Environment variable containing the certificate password (the value is never an argument).
        #[arg(
            long,
            conflicts_with = "certificate_password_stdin",
            required_unless_present = "certificate_password_stdin"
        )]
        certificate_password_env: Option<String>,
        /// Read the certificate password from piped stdin; remove one trailing line ending.
        #[arg(long, conflicts_with = "certificate_password_env")]
        certificate_password_stdin: bool,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        location: Option<String>,
        /// New signed copy; the input file cannot be replaced, even with --overwrite.
        #[arg(long)]
        out: PathBuf,
    },
    /// List the application's command registry (stable ids) as JSON.
    Commands,
    /// Extract text (all pages, or one page) to stdout as plain text.
    Text {
        file: PathBuf,
        /// 1-based page number; omit for the whole document.
        #[arg(long)]
        page: Option<usize>,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
    /// Retain the input's encryption and permissions; the input cannot be replaced.
    Extract {
        file: PathBuf,
        #[arg(long)]
        pages: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Split into files of N pages each, written to a directory.
    /// Retain the input's encryption and permissions on every part.
    Split {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        every: usize,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Stamp a single line of text (watermark).
    #[command(help_template = STAMP_HELP_TEMPLATE)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Add an editable foreground image watermark to selected pages, as a new copy.
    ImageWatermark {
        file: PathBuf,
        #[arg(long)]
        image: PathBuf,
        #[arg(long, default_value = "all")]
        pages: String,
        #[arg(long, default_value_t = 30.0)]
        width_percent: f32,
        #[arg(long, default_value_t = 0.3)]
        opacity: f32,
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        angle: f32,
        #[arg(long, default_value = "center")]
        position: String,
        #[arg(long, default_value_t = 28.0)]
        margin: f32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Add single-line page numbers.
    #[command(help_template = STAMP_HELP_TEMPLATE)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Set navigation page labels for a physical page range; visible page content is unchanged.
    SetPageLabels {
        file: PathBuf,
        /// First physical page (1-based).
        #[arg(long, default_value_t = 1)]
        from: usize,
        /// Last physical page (1-based); defaults to the last page.
        #[arg(long)]
        to: Option<usize>,
        #[arg(long, default_value = "decimal", value_parser = ["decimal", "roman-upper", "roman-lower", "letters-upper", "letters-lower", "prefix-only"])]
        style: String,
        #[arg(long, default_value_t = 1)]
        start: u32,
        #[arg(long, default_value = "")]
        prefix: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Remove all navigation page labels; visible page content is unchanged.
    ResetPageLabels {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Print document properties (metadata, version, encryption, permissions) as JSON.
    Properties {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// List existing local named destinations, refusing ambiguous or malformed name trees.
    LinkDestinations {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Print the bookmark tree (title, page, depth) as JSON.
    Bookmarks {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Add a page bookmark to a new copy. Bookmark/page indices start at 1; omit --parent for root.
    BookmarkAdd {
        file: PathBuf,
        #[arg(long)]
        title: String,
        #[arg(long,value_parser=positive_bookmark_number)]
        page: usize,
        #[arg(long,value_parser=positive_bookmark_number)]
        parent: Option<usize>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Rename a bookmark in a new copy, preserving its original action and children.
    BookmarkRename {
        file: PathBuf,
        #[arg(long,value_parser=positive_bookmark_number)]
        index: usize,
        #[arg(long)]
        title: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Replace a bookmark action with a local page destination in a new copy.
    BookmarkDestination {
        file: PathBuf,
        #[arg(long,value_parser=positive_bookmark_number)]
        index: usize,
        #[arg(long,value_parser=positive_bookmark_number)]
        page: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Move a bookmark subtree. --position is the 1-based sibling slot after removing the item.
    BookmarkMove {
        file: PathBuf,
        #[arg(long,value_parser=positive_bookmark_number)]
        index: usize,
        #[arg(long,value_parser=positive_bookmark_number)]
        parent: Option<usize>,
        #[arg(long,value_parser=positive_bookmark_number)]
        position: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Delete a bookmark and its children in a new copy; the input cannot be replaced.
    BookmarkDelete {
        file: PathBuf,
        #[arg(long,value_parser=positive_bookmark_number)]
        index: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Encrypt with AES-256 (or AES-128 with --aes128): passwords and permissions.
    Protect {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "", help = password_input::ARGV_USER_PASSWORD_HELP)]
        user_password: String,
        #[arg(long, default_value = "", help = password_input::ARGV_OWNER_PASSWORD_HELP)]
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
        /// Owner password of the input file if it is already encrypted.
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Remove encryption (the owner password is required).
    Unprotect {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, required_unless_present_any = ["password_env", "password_stdin"], help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Remove all document metadata (Info dictionary and XMP stream).
    StripMetadata {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Sanitize hidden structures into a verified new copy; existing files are never replaced.
    Sanitize {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        /// Keep hidden metadata, scripts, attachments, bookmarks, named destinations and accessibility data.
        /// These may retain sensitive content. Removed by default; prefer the default cleanup.
        #[arg(long)]
        keep_hidden_data: bool,
        /// Resolution of the image patches and rasterized pages (72 to 600).
        #[arg(long, default_value_t = 200.0)]
        dpi: f32,
        /// Redaction fill colour, RRGGBB (default black).
        #[arg(long)]
        color: Option<String>,
        /// Optional visible replacement text, added only after removal verification.
        #[arg(long)]
        overlay: Option<String>,
        #[arg(long, default_value = "FFFFFF")]
        overlay_color: String,
        #[arg(long, default_value_t = 10.0)]
        overlay_size: f32,
        /// Save visible, unapplied standard /Redact annotations. This does not remove content.
        #[arg(long)]
        marks_only: bool,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Lossless size optimisation (stream recompression, unused objects removed).
    Optimize {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Reduce supported image dimensions/JPEG quality while keeping page text, links and fields.
    /// Documents with digital signatures are refused; the original remains untouched.
    CompressImages {
        file: PathBuf,
        #[arg(long, default_value_t = 2000)]
        max_image_side: u32,
        #[arg(long, default_value_t = 80)]
        jpeg_quality: u8,
        /// New compressed copy; existing files and the input cannot be replaced, even with --overwrite.
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Search text; prints matches (page, rectangles in points, origin bottom-left of the page as
    /// displayed, i.e. after /Rotate and CropBox) as JSON.
    Search {
        file: PathBuf,
        query: String,
        #[arg(long)]
        match_case: bool,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Print document information (page count, page sizes, title) as JSON.
    Info {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// List the content objects of a page (0-based indices, display-space coordinates in points).
    Objects {
        file: PathBuf,
        /// 1-based page number.
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Move content objects together in drawing order, preserving their relative order.
    ReorderObjects {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Comma-separated 0-based object indices from `objects`.
        #[arg(long)]
        objects: String,
        /// Final 0-based index of the first object in the moved block; 0 is the back.
        #[arg(long)]
        at: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        /// Eligible static TrueType/OpenType font or collection, subset when its embedding rights permit.
        #[arg(long, conflicts_with = "font_name")]
        font_file: Option<PathBuf>,
        /// Zero-based collection face (with --font-file).
        #[arg(long, requires = "font_file")]
        font_face_index: Option<u32>,
        /// Name of an installed system font, as listed by `fonts` (e.g. "Arial Bold").
        #[arg(long)]
        font_name: Option<String>,
        #[arg(long)]
        bold: bool,
        #[arg(long)]
        italic: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Rotate objects together (clockwise degrees), preserving their relative positions.
    RotateObjects {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// 0-based object indices, e.g. "0,2".
        #[arg(long)]
        objects: String,
        #[arg(long, allow_negative_numbers = true)]
        degrees: f32,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Resize objects together to displayed page bounds, preserving their relative positions.
    ResizeObjects {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// 0-based object indices, e.g. "0,2".
        #[arg(long)]
        objects: String,
        /// Displayed page coordinates: left,bottom,right,top (PDF points).
        #[arg(long, allow_hyphen_values = true)]
        rect: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Place an image on a page, e.g. a scanned handwritten signature. Visual mark only: this is
    /// NOT a digital signature (no certificate, no integrity protection, removable by anyone).
    AddImage {
        file: PathBuf,
        /// Image file; a PNG with transparency is used as is.
        #[arg(long)]
        image: PathBuf,
        #[arg(long, default_value_t = 1, conflicts_with = "pages")]
        page: usize,
        /// Place once on each selected page: all, 1,3-5. The image shrinks if necessary and its
        /// position is clamped inside each displayed page; --x/--y specify the requested corner.
        #[arg(long)]
        pages: Option<String>,
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        /// Explicitly scale annotation/link geometry too (requires --mode scale).
        #[arg(long)]
        annotations: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// List the AcroForm fields (name, type, value, options, widgets with 1-based pages) as JSON.
    FormFields {
        file: PathBuf,
        /// Print only the {name: value} object (the format accepted by `fill-form --values`;
        /// read-only fields are included with their current value, password fields never).
        #[arg(long)]
        values_only: bool,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Export non-password field values as JSON or XFDF (selected by output extension).
    FormExport {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Fill fields from JSON/XFDF/FDF (--values FILE, "-" = JSON stdin) and/or --set NAME=VALUE
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// List the annotations of one page (--page) or of every page as JSON: index (0-based, the
    /// one the other annotation commands take), subtype, rect (left,bottom,right,top in displayed
    /// page points), text, author, style, link target, reply thread and review state.
    Annotations {
        file: PathBuf,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        /// Dashed border (6pt dash, 3pt gap); square, circle, line or arrow only.
        #[arg(long)]
        dashed: bool,
        /// Opacity (0 to 1); free-text annotations only support 1.
        #[arg(long, default_value_t = 1.0)]
        opacity: f32,
        /// Link to a page (1-based).
        #[arg(long, conflicts_with_all = ["url", "named_destination"])]
        goto_page: Option<usize>,
        /// Link to http, https or mailto address; anything else is refused.
        #[arg(long)]
        url: Option<String>,
        /// Use an existing uniquely named local destination.
        #[arg(long, conflicts_with_all = ["goto_page", "url"])]
        named_destination: Option<String>,
        /// Local viewport: Fit, XYZ or FitH; empty axes retain the reader's current values.
        #[arg(long, requires = "goto_page", value_parser = ["fit", "xyz", "fith"])]
        view: Option<String>,
        #[arg(long, requires = "view", allow_negative_numbers = true)]
        x: Option<f32>,
        #[arg(long, requires = "view", allow_negative_numbers = true)]
        y: Option<f32>,
        #[arg(long, requires = "view")]
        zoom: Option<f32>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Change an existing link's target without replacing its annotation or appearance.
    SetLinkTarget {
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Zero-based annotation index (as reported by annotations).
        #[arg(long)]
        index: usize,
        /// Link to a page (1-based).
        #[arg(long, conflicts_with_all = ["url", "named_destination"], required_unless_present_any = ["url", "named_destination"])]
        goto_page: Option<usize>,
        /// Link to an http, https or mailto address; anything else is refused.
        #[arg(
            long,
            conflicts_with_all = ["goto_page", "named_destination"],
            required_unless_present_any = ["goto_page", "named_destination"]
        )]
        url: Option<String>,
        #[arg(long, conflicts_with_all = ["goto_page", "url"], required_unless_present_any = ["goto_page", "url"])]
        named_destination: Option<String>,
        #[arg(long, requires = "goto_page", value_parser = ["fit", "xyz", "fith"])]
        view: Option<String>,
        #[arg(long, requires = "view", allow_negative_numbers = true)]
        x: Option<f32>,
        #[arg(long, requires = "view", allow_negative_numbers = true)]
        y: Option<f32>,
        #[arg(long, requires = "view")]
        zoom: Option<f32>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
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

fn run(mut cli: Cli) -> Result<serde_json::Value, EngineError> {
    cli.passwords.apply(&mut cli.command)?;
    if let Command::Fonts { filter } = &cli.command {
        let filter = filter.as_deref().unwrap_or("").to_lowercase();
        return Ok(serde_json::json!(sunipdf_core::fonts::system_fonts()
            .into_iter()
            .filter(|f| f.name.to_lowercase().contains(&filter))
            .map(|f| serde_json::json!({
                "name": f.name, "path": f.path, "size_bytes": f.size_bytes, "face_index": f.face_index,
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
    if let Command::Annotate {
        kind,
        opacity,
        dashed,
        ..
    } = &cli.command
    {
        if *dashed && !matches!(kind.as_str(), "square" | "circle" | "line" | "arrow") {
            return Err(EngineError::Other(
                "--dashed only supports square, circle, line or arrow".into(),
            ));
        }
        if !opacity.is_finite() {
            return Err(EngineError::Other("opacity must be finite".into()));
        }
        if kind == "freetext" && *opacity != 1.0 {
            return Err(EngineError::Other(
                "free-text annotations only support --opacity 1".into(),
            ));
        }
    }
    match &cli.command {
        Command::PrepareSignatureImage {
            file,
            out,
            background,
            threshold,
        } => {
            return feature_cmd::prepare_signature_image(file, out, background, *threshold);
        }
        Command::Inspect { file, password } => {
            return sunipdf_core::inspection::inspect_file(file, password.as_deref())
                .map(|report| report.json());
        }
        Command::Signatures {
            file,
            password,
            allow_network,
        } => return feature_cmd::signatures(file, password.as_deref(), *allow_network),
        Command::Sign {
            file,
            certificate,
            certificate_password_env,
            certificate_password_stdin,
            reason,
            location,
            out,
        } => {
            return feature_cmd::sign(
                file,
                certificate,
                certificate_password_env.as_deref(),
                *certificate_password_stdin,
                sunipdf_core::certificate::CertSignOptions {
                    reason: reason.clone(),
                    location: location.clone(),
                },
                out,
            );
        }
        _ => {}
    }
    let engine = Engine::new()?;
    match cli.command {
        Command::Commands
        | Command::PrepareSignatureImage { .. }
        | Command::Inspect { .. }
        | Command::Fonts { .. }
        | Command::Signatures { .. }
        | Command::Sign { .. } => unreachable!(),
        Command::OfficeToPdf { file, out, profile } => {
            use sunipdf_core::office::{Converter, Profile};
            let converter = Converter::detect().ok_or_else(|| EngineError::Other(
                "A supported local LibreOffice is not installed; Office conversion is unavailable".into()
            ))?;
            let profile = Profile::from_id(&profile).ok_or(EngineError::InvalidFormat)?;
            let input = std::path::absolute(file)?;
            let output = std::path::absolute(out)?;
            let report = sunipdf_core::office::convert(
                &engine,
                &converter,
                &input,
                &output,
                profile,
                &std::sync::atomic::AtomicBool::new(false),
            )?;
            let mut json = report.json();
            json["out"] = serde_json::json!(output);
            Ok(json)
        }
        Command::ExportOffice {
            file,
            out,
            format,
            password,
        } => {
            let out = std::path::absolute(out)?;
            let format = sunipdf_core::export_office::Format::from_id(&format)
                .ok_or(EngineError::InvalidFormat)?;
            let doc = engine.open(&file, password.as_deref())?;
            let report = doc.export_office(
                &out,
                format,
                &std::sync::atomic::AtomicBool::new(false),
                |_, _| {},
            )?;
            let mut json = report.json();
            json["out"] = serde_json::json!(out);
            Ok(json)
        }
        Command::AddFormField {
            file,
            kind,
            name,
            options,
            multi_select,
            editable,
            rect,
            page,
            out,
            password,
        } => {
            let rect = annot_cmd::parse_rect(&rect)?;
            let mut doc = engine.open(&file, password.as_deref())?;
            let index = page_index(page, doc.page_count())?;
            let field_kind = match kind.as_str() {
                "checkbox" => sunipdf_core::FieldKind::Checkbox,
                "radio" => sunipdf_core::FieldKind::Radio,
                "combo" => sunipdf_core::FieldKind::Combo,
                "list" => sunipdf_core::FieldKind::List,
                _ => sunipdf_core::FieldKind::Text,
            };
            let options = options
                .iter()
                .map(|spec| {
                    let (value, label) = spec.split_once('=').ok_or_else(|| {
                        EngineError::FormValueInvalid("field options require VALUE=LABEL".into())
                    })?;
                    Ok(sunipdf_core::FieldOption {
                        value: value.to_string(),
                        label: label.to_string(),
                    })
                })
                .collect::<Result<Vec<_>, EngineError>>()?;
            doc.add_form_field_with_flags(
                index,
                &name,
                field_kind,
                rect,
                &options,
                multi_select,
                editable,
            )?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({"out":out,"page":page,"name":name,"kind":kind}))
        }
        Command::Ocr {
            file,
            language,
            pages,
            out,
            password,
        } => {
            use sunipdf_core::ocr::{OcrLanguage, OcrOptions};
            let mut doc = engine.open(&file, password.as_deref())?;
            let pages = unique(parse_pages(&pages, doc.page_count())?);
            let language = match language.as_str() {
                "eng" => OcrLanguage::English,
                "fra" => OcrLanguage::French,
                _ => OcrLanguage::EnglishFrench,
            };
            let report = doc.ocr(
                &OcrOptions { pages, language },
                &std::sync::atomic::AtomicBool::new(false),
                |_, _| {},
            )?;
            if report.recognized_pages == 0 {
                sunipdf_core::write_atomic(&out, &std::fs::read(&file)?)?;
            } else {
                doc.save_as(&out)?;
            }
            Ok(
                serde_json::json!({"out":out,"recognized_pages":report.recognized_pages,"skipped_pages":report.skipped_pages,"words":report.words}),
            )
        }
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
                writeln!(out, "{}", safe_output::terminal_text(&text)).map_err(EngineError::Io)?;
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
            Ok(serde_json::json!({
                "out": out, "page_count": n, "encryption_removed": false,
                "encrypted": doc.is_encrypted()
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
                "files": files, "count": files.len(), "encryption_removed": false,
                "encrypted": doc.is_encrypted()
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
        Command::ImageWatermark {
            file,
            image,
            pages,
            width_percent,
            opacity,
            angle,
            position,
            margin,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let options = sunipdf_core::image_watermark::ImageWatermark {
                width_percent,
                opacity,
                angle,
                margin_pt: margin,
                position: StampPosition::from_id(&position)
                    .ok_or_else(|| EngineError::Other("unknown watermark position".into()))?,
            };
            let count = doc.image_watermark(Some(&selection), &image, &options)?;
            sunipdf_core::write_new_copy(&out, &doc.snapshot()?)?;
            Ok(
                serde_json::json!({"out":out,"watermarked":count,"original_unchanged":true,"foreground":true,"encrypted":doc.properties().encrypted}),
            )
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
        Command::SetPageLabels {
            file,
            from,
            to,
            style,
            start,
            prefix,
            out,
            password,
        } => {
            use sunipdf_core::{PageLabelRange, PageLabelStyle};
            let mut doc = engine.open(&file, password.as_deref())?;
            let last = to.unwrap_or(doc.page_count());
            let range = PageLabelRange {
                first: page_index(from, doc.page_count())?,
                last: page_index(last, doc.page_count())?,
                style: match style.as_str() {
                    "decimal" => PageLabelStyle::Decimal,
                    "roman-upper" => PageLabelStyle::RomanUpper,
                    "roman-lower" => PageLabelStyle::RomanLower,
                    "letters-upper" => PageLabelStyle::LettersUpper,
                    "letters-lower" => PageLabelStyle::LettersLower,
                    "prefix-only" => PageLabelStyle::PrefixOnly,
                    _ => unreachable!("page label style validated by clap"),
                },
                prefix,
                start,
            };
            doc.set_page_labels(&range)?;
            doc.save_as(&out)?;
            Ok(
                serde_json::json!({ "out": out, "from": from, "to": last, "count": range.last - range.first + 1 }),
            )
        }
        Command::ResetPageLabels {
            file,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            doc.reset_page_labels()?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "count": doc.page_count() }))
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
        Command::LinkDestinations { file, password } => {
            let doc = engine.open(&file, password.as_deref())?;
            let names = doc.named_link_destinations()?;
            Ok(serde_json::json!({"named_destinations": names}))
        }
        Command::Bookmarks { file, password } => {
            let doc = engine.open(&file, password.as_deref())?;
            Ok(serde_json::json!(doc
                .bookmarks()
                .iter()
                .map(|b| serde_json::json!({
                    "title": b.title, "page": b.page.map(|p| p + 1), "depth": b.depth,
                    "destination": b.destination.map(annot_cmd::destination_json),
                }))
                .collect::<Vec<_>>()))
        }
        Command::BookmarkAdd {
            file,
            title,
            page,
            parent,
            out,
            password,
        } => bookmark_copy(
            &engine,
            &file,
            &out,
            password.as_deref(),
            sunipdf_core::bookmarks::BookmarkEdit::Add {
                parent: parent.map(bookmark_index).transpose()?,
                title,
                page: bookmark_index(page)?,
            },
        ),
        Command::BookmarkRename {
            file,
            index,
            title,
            out,
            password,
        } => bookmark_copy(
            &engine,
            &file,
            &out,
            password.as_deref(),
            sunipdf_core::bookmarks::BookmarkEdit::Rename {
                index: bookmark_index(index)?,
                title,
            },
        ),
        Command::BookmarkDestination {
            file,
            index,
            page,
            out,
            password,
        } => bookmark_copy(
            &engine,
            &file,
            &out,
            password.as_deref(),
            sunipdf_core::bookmarks::BookmarkEdit::Destination {
                index: bookmark_index(index)?,
                page: bookmark_index(page)?,
            },
        ),
        Command::BookmarkMove {
            file,
            index,
            parent,
            position,
            out,
            password,
        } => bookmark_copy(
            &engine,
            &file,
            &out,
            password.as_deref(),
            sunipdf_core::bookmarks::BookmarkEdit::Move {
                index: bookmark_index(index)?,
                parent: parent.map(bookmark_index).transpose()?,
                position: bookmark_index(position)?,
            },
        ),
        Command::BookmarkDelete {
            file,
            index,
            out,
            password,
        } => bookmark_copy(
            &engine,
            &file,
            &out,
            password.as_deref(),
            sunipdf_core::bookmarks::BookmarkEdit::Delete {
                index: bookmark_index(index)?,
            },
        ),
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
            let bytes = sunipdf_core::security::unprotect(
                &input,
                password.as_deref().ok_or(EngineError::PasswordRequired)?,
            )?;
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
        Command::Sanitize {
            file,
            out,
            password,
        } => {
            let input = std::fs::read(&file)?;
            let (bytes, report) =
                sunipdf_core::sanitize::sanitize_document(&input, password.as_deref())?;
            sunipdf_core::write_new_copy(&out, &bytes)?;
            Ok(serde_json::json!({"out":out,"verified":report.verified,
                "removed_entries":report.removed_entries,"removed_annotations":report.removed_annotations,
                "unused_font_entries":report.unused_font_entries,"unreachable_objects":report.unreachable_objects}))
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
            color,
            overlay,
            overlay_color,
            overlay_size,
            marks_only,
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
            let appearance = sunipdf_core::redact::RedactAppearance {
                fill_rgb: color
                    .as_deref()
                    .map(parse_hex_color)
                    .transpose()?
                    .unwrap_or((0, 0, 0)),
                overlay: overlay
                    .map(|text| -> Result<_, EngineError> {
                        Ok(sunipdf_core::redact::RedactOverlay {
                            text,
                            style: sunipdf_core::TextStyle {
                                rgb: parse_hex_color(&overlay_color)?,
                                size_pt: overlay_size,
                                ..Default::default()
                            },
                        })
                    })
                    .transpose()?,
            };
            if marks_only {
                doc.persist_redaction_marks(&marks, &appearance, None)?;
                sunipdf_core::write_new_copy(&out, &doc.snapshot()?)?;
                return Ok(
                    serde_json::json!({"out":out,"marks":marks.len(),"persisted":true,"content_removed":false}),
                );
            }
            let r = if color.is_some() || appearance.overlay.is_some() {
                let mut preview =
                    doc.preview_redaction(&marks, &options, &appearance, marks[0].page)?;
                let prepared = preview.prepared.take().ok_or_else(|| {
                    EngineError::Other("Redaction preview produced no candidate".into())
                })?;
                doc.commit_redaction_preview(prepared)?
            } else {
                doc.redact(&marks, &options)?
            };
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
                "native_text_runs": r.native_text_runs, "bitmap_objects": r.bitmap_objects,
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
        Command::CompressImages {
            file,
            max_image_side,
            jpeg_quality,
            out,
            password,
        } => v010_cmd::compress(
            &engine,
            &file,
            &out,
            password.as_deref(),
            sunipdf_core::compress::CompressionOptions {
                max_image_side,
                jpeg_quality,
            },
        ),
        Command::SearchFiles {
            files,
            query,
            match_case,
            password,
        } => sunipdf_core::local_review::search_files(
            &engine,
            &files,
            &query,
            match_case,
            password.as_deref(),
            &std::sync::atomic::AtomicBool::new(false),
            |_, _, _, _| {},
        )
        .map(|report| report.json()),
        Command::Compare {
            left,
            right,
            left_start,
            right_start,
            max_pages,
            password,
        } => sunipdf_core::local_review::compare_files(
            &engine,
            &left,
            &right,
            sunipdf_core::local_review::CompareOptions {
                left_start,
                right_start,
                max_pages,
            },
            password.as_deref(),
            &std::sync::atomic::AtomicBool::new(false),
            |_, _| {},
        )
        .map(|report| report.json()),
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
                    "label": info.page_label(i),
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
        Command::ReorderObjects {
            file,
            page,
            objects,
            at,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let indices = parse_indices(&objects)?;
            let new_indices = doc.reorder_objects(p, &indices, sunipdf_core::ZOrder::At(at))?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({ "out": out, "page": page, "new_indices": new_indices }))
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
            font_face_index,
            font_name,
            bold,
            italic,
            out,
            password,
        } => {
            let (font_file, font_face_index) = match (font_file, font_name) {
                (Some(path), _) => (Some(path), font_face_index.unwrap_or(0)),
                (None, Some(name)) => {
                    let selected = sunipdf_core::fonts::system_fonts()
                        .into_iter()
                        .find(|f| f.name.eq_ignore_ascii_case(&name))
                        .ok_or_else(|| {
                            EngineError::Other(format!("unknown system font: {name}"))
                        })?;
                    (Some(selected.path), selected.face_index)
                }
                (None, None) => (None, 0),
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
                font_face_index,
                ..Default::default()
            };
            let added = doc.add_text(p, x, y, &text.replace("\\n", "\n"), &style)?;
            doc.save_as(&out)?;
            Ok(
                serde_json::json!({ "out": out, "added": added, "embedded_font": font_file, "font_face_index": font_face_index }),
            )
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
        Command::RotateObjects {
            file,
            page,
            objects,
            degrees,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let indices = parse_indices(&objects)?;
            doc.rotate_objects(p, &indices, degrees)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({"out":out,"objects":indices.len(),"degrees":degrees}))
        }
        Command::ResizeObjects {
            file,
            page,
            objects,
            rect,
            out,
            password,
        } => {
            let bounds = annot_cmd::parse_rect(&rect)?;
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let indices = parse_indices(&objects)?;
            doc.resize_objects(p, &indices, bounds)?;
            doc.save_as(&out)?;
            Ok(
                serde_json::json!({"out":out,"objects":indices.len(),"bounds":[bounds.left,bounds.bottom,bounds.right,bounds.top]}),
            )
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
            pages,
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
            let selection = pages
                .as_deref()
                .map(|pages| parse_pages(pages, doc.page_count()).map(unique))
                .transpose()?;
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
            if let Some(selection) = selection {
                let height = width * h as f32 / w as f32;
                let count = doc.place_signature_image(
                    &selection,
                    &image::DynamicImage::ImageRgba8(img),
                    (x + width / 2.0, y + height / 2.0),
                    width,
                    &std::sync::atomic::AtomicBool::new(false),
                    |_, _| {},
                )?;
                doc.save_as(&out)?;
                return Ok(serde_json::json!({
                    "out": out,
                    "pages": selection.iter().map(|p| p + 1).collect::<Vec<_>>(),
                    "count": count,
                    "x": x,
                    "y": y,
                    "width_pt": width,
                    "height_pt": height,
                    "background_removed": remove_background.is_some(),
                    "digital_signature": false,
                }));
            }
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
            annotations,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let selection = unique(parse_pages(&pages, doc.page_count())?);
            let k = unit_factor(&unit)?;
            let mode = match mode.as_str() {
                "keep" if !annotations => sunipdf_core::ResizeMode::KeepContent,
                "keep" => {
                    return Err(EngineError::Other(
                        "--annotations requires --mode scale".into(),
                    ))
                }
                "scale" if annotations => sunipdf_core::ResizeMode::ScaleContentWithAnnotations,
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
        Command::FormExport {
            file,
            out,
            password,
        } => {
            let source = std::fs::canonicalize(&file)?;
            if out.exists() && std::fs::canonicalize(&out)? == source {
                return Err(EngineError::FormValueInvalid(
                    "form values cannot replace the input PDF".into(),
                ));
            }
            let doc = engine.open(&file, password.as_deref())?;
            let info = doc.form_info()?;
            let extension = out.extension().and_then(|s| s.to_str()).unwrap_or("");
            let bytes = form_data::export_values(&info, extension)?;
            sunipdf_core::write_atomic(&out, &bytes)?;
            Ok(
                serde_json::json!({"out":out,"fields":form_data::values_to_json(&info).as_object().map_or(0, |v| v.len())}),
            )
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
                let links = doc.page_links(p)?;
                let list: Vec<_> = doc
                    .annotation_details(p)?
                    .iter()
                    .map(|detail| {
                        let mut value = annot_cmd::detail_json(detail);
                        if let Some(link) = links.iter().find(|link| link.index == detail.index) {
                            value["link"] = annot_cmd::link_json(&link.target);
                            if let Some(sunipdf_core::LinkTarget::Named(name)) = &detail.link {
                                value["link"]["named_destination"] = name.clone().into();
                            }
                            if let Some(destination) = link.destination {
                                value["link"]["destination"] =
                                    annot_cmd::destination_json(destination);
                            }
                        }
                        value
                    })
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
            dashed,
            opacity,
            goto_page,
            url,
            named_destination,
            view,
            x,
            y,
            zoom,
            author,
            out,
            password,
        } => {
            use sunipdf_core::{AnnotShape, AnnotStyle, FreeTextStyle};
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
                dash_pattern: if dashed { vec![6.0, 3.0] } else { Vec::new() },
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
                    let target = annot_cmd::link_target(
                        goto_page,
                        url,
                        named_destination,
                        view,
                        x,
                        y,
                        zoom,
                    )?;
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
        Command::SetLinkTarget {
            file,
            page,
            index,
            goto_page,
            url,
            named_destination,
            view,
            x,
            y,
            zoom,
            out,
            password,
        } => {
            let mut doc = engine.open(&file, password.as_deref())?;
            let p = page_index(page, doc.page_count())?;
            let target =
                annot_cmd::link_target(goto_page, url, named_destination, view, x, y, zoom)?;
            doc.set_link_target(p, index, &target)?;
            doc.save_as(&out)?;
            Ok(serde_json::json!({
                "out": out, "page": page, "index": index,
                "link": annot_cmd::link_json(&target),
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
                let json = v010_cmd::read_form_values(&path)?;
                plan = if ignore_unknown {
                    form_data::plan_from_json(&info, &json)?
                } else {
                    form_data::strict_plan_from_json(&info, &json)?
                };
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
        | Command::BookmarkAdd { out, .. }
        | Command::BookmarkRename { out, .. }
        | Command::BookmarkDestination { out, .. }
        | Command::BookmarkMove { out, .. }
        | Command::BookmarkDelete { out, .. }
        | Command::OfficeToPdf { out, .. }
        | Command::ExportOffice { out, .. }
        | Command::PrepareSignatureImage { out, .. }
        | Command::AddFormField { out, .. }
        | Command::Ocr { out, .. }
        | Command::Sign { out, .. }
        | Command::Extract { out, .. }
        | Command::Rotate { out, .. }
        | Command::Stamp { out, .. }
        | Command::ImageWatermark { out, .. }
        | Command::PageNumbers { out, .. }
        | Command::SetPageLabels { out, .. }
        | Command::ResetPageLabels { out, .. }
        | Command::Insert { out, .. }
        | Command::Move { out, .. }
        | Command::Protect { out, .. }
        | Command::Unprotect { out, .. }
        | Command::SetMetadata { out, .. }
        | Command::StripMetadata { out, .. }
        | Command::Sanitize { out, .. }
        | Command::Redact { out, .. }
        | Command::Images2pdf { out, .. }
        | Command::Optimize { out, .. }
        | Command::CompressImages { out, .. }
        | Command::AddText { out, .. }
        | Command::ReorderObjects { out, .. }
        | Command::SetOpacity { out, .. }
        | Command::RotateObject { out, .. }
        | Command::RotateObjects { out, .. }
        | Command::ResizeObjects { out, .. }
        | Command::CopyObjects { out, .. }
        | Command::ReplaceImage { out, .. }
        | Command::AddImage { out, .. }
        | Command::Crop { out, .. }
        | Command::ResizePages { out, .. }
        | Command::InsertBlank { out, .. }
        | Command::FillForm { out, .. }
        | Command::FormExport { out, .. }
        | Command::Flatten { out, .. }
        | Command::Annotate { out, .. }
        | Command::SetLinkTarget { out, .. }
        | Command::AnnotationDelete { out, .. }
        | Command::Reply { out, .. }
        | Command::ReviewState { out, .. }
        | Command::Render { out, .. } => Some(out.as_path()),
        _ => None,
    }
}

/// 90, 180, 270 (ou leurs équivalents négatifs) → quart de tour horaire ; le reste est refusé.
fn positive_bookmark_number(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|number| *number > 0)
        .ok_or_else(|| "bookmark indices, pages and positions start at 1".into())
}
fn bookmark_index(value: usize) -> Result<usize, EngineError> {
    value.checked_sub(1).ok_or_else(|| {
        EngineError::Other("bookmark indices, pages and positions start at 1".into())
    })
}
fn bookmark_copy(
    engine: &Engine,
    input: &std::path::Path,
    output: &std::path::Path,
    password: Option<&str>,
    edit: sunipdf_core::bookmarks::BookmarkEdit,
) -> Result<serde_json::Value, EngineError> {
    if output.exists() {
        return Err(EngineError::AlreadyExists(output.into()));
    }
    let mut doc = engine.open(input, password)?;
    doc.edit_bookmark(&edit)?;
    sunipdf_core::write_new_copy(output, &doc.snapshot()?)?;
    Ok(
        serde_json::json!({"out":output,"bookmarks":doc.bookmarks().len(),"original_unchanged":true,"encrypted":doc.properties().encrypted}),
    )
}

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
    let cli = Cli::parse();
    let signature_check = matches!(&cli.command, Command::Signatures { .. });
    let local_review_check = matches!(
        &cli.command,
        Command::SearchFiles { .. } | Command::Compare { .. } | Command::Inspect { .. }
    );
    match run(cli) {
        Ok(serde_json::Value::Null) => ExitCode::SUCCESS,
        Ok(value) => {
            println!(
                "{}",
                safe_output::escape_c1(&serde_json::to_string_pretty(&value).expect("json"))
            );
            if (signature_check && value["verification_failed"] == true)
                || (local_review_check && value["complete"] == false)
            {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
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
                safe_output::escape_c1(
                    &serde_json::json!({ "error": e.code(), "message": message }).to_string()
                )
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

    fn parse_cli<const N: usize>(args: [&'static str; N]) -> Result<Cli, clap::Error> {
        // Debug clap parsing needs the same stack allocation as the production CLI thread.
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || Cli::try_parse_from(args))
            .expect("CLI parser thread")
            .join()
            .expect("CLI parser result")
    }

    #[test]
    fn bookmark_author_commands_use_one_based_indices_and_guard_all_output_paths() {
        let add = parse_cli([
            "sunipdf-cli",
            "bookmark-add",
            "input.pdf",
            "--title",
            "Chapter",
            "--page",
            "2",
            "--parent",
            "1",
            "--out",
            "added.pdf",
        ])
        .unwrap();
        assert!(matches!(
            add.command,
            Command::BookmarkAdd {
                parent: Some(1),
                page: 2,
                ..
            }
        ));
        assert_eq!(
            output_path(&add.command),
            Some(std::path::Path::new("added.pdf"))
        );
        let rename = parse_cli([
            "sunipdf-cli",
            "bookmark-rename",
            "input.pdf",
            "--index",
            "1",
            "--title",
            "Renamed",
            "--out",
            "renamed.pdf",
        ])
        .unwrap();
        assert_eq!(
            output_path(&rename.command),
            Some(std::path::Path::new("renamed.pdf"))
        );
        let destination = parse_cli([
            "sunipdf-cli",
            "bookmark-destination",
            "input.pdf",
            "--index",
            "1",
            "--page",
            "2",
            "--out",
            "dest.pdf",
        ])
        .unwrap();
        assert_eq!(
            output_path(&destination.command),
            Some(std::path::Path::new("dest.pdf"))
        );
        let moved = parse_cli([
            "sunipdf-cli",
            "bookmark-move",
            "input.pdf",
            "--index",
            "2",
            "--parent",
            "1",
            "--position",
            "1",
            "--out",
            "moved.pdf",
        ])
        .unwrap();
        assert_eq!(
            output_path(&moved.command),
            Some(std::path::Path::new("moved.pdf"))
        );
        let deleted = parse_cli([
            "sunipdf-cli",
            "bookmark-delete",
            "input.pdf",
            "--index",
            "1",
            "--out",
            "deleted.pdf",
            "--overwrite",
        ])
        .unwrap();
        assert!(deleted.overwrite);
        assert_eq!(
            output_path(&deleted.command),
            Some(std::path::Path::new("deleted.pdf"))
        );
        assert!(parse_cli([
            "sunipdf-cli",
            "bookmark-delete",
            "input.pdf",
            "--index",
            "0",
            "--out",
            "bad.pdf"
        ])
        .is_err());
        assert!(parse_cli([
            "sunipdf-cli",
            "bookmark-add",
            "input.pdf",
            "--title",
            "bad",
            "--page",
            "0",
            "--out",
            "bad.pdf"
        ])
        .is_err());
        assert!(bookmark_index(0).is_err());
        assert_eq!(bookmark_index(1).unwrap(), 0);
    }
    #[test]
    fn bookmark_cli_new_copy_helper_preserves_input_and_refuses_replacement_or_bad_edits() {
        let engine = Engine::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.pdf");
        let original = include_bytes!("../../../crates/sunipdf-core/tests/fixtures/two-pages.pdf");
        std::fs::write(&input, original).unwrap();
        let original_bookmarks = engine.open(&input, None).unwrap().bookmarks();
        let output = dir.path().join("output.pdf");
        let result = bookmark_copy(
            &engine,
            &input,
            &output,
            None,
            sunipdf_core::bookmarks::BookmarkEdit::Add {
                parent: None,
                title: "CLI Chapter".into(),
                page: 1,
            },
        )
        .unwrap();
        let bookmarks = engine.open(&output, None).unwrap().bookmarks();
        assert_eq!(result["bookmarks"].as_u64(), Some(bookmarks.len() as u64));
        assert_eq!(bookmarks.len(), original_bookmarks.len() + 1);
        assert_eq!(bookmarks[..original_bookmarks.len()], original_bookmarks);
        let added = bookmarks.last().unwrap();
        assert_eq!(added.title, "CLI Chapter");
        assert_eq!(added.page, Some(1));
        assert_eq!(added.depth, 0);
        assert_eq!(std::fs::read(&input).unwrap(), original);
        assert!(matches!(
            bookmark_copy(
                &engine,
                &input,
                &input,
                None,
                sunipdf_core::bookmarks::BookmarkEdit::Delete { index: 0 }
            ),
            Err(EngineError::AlreadyExists(_))
        ));
        let invalid = dir.path().join("invalid.pdf");
        assert!(bookmark_copy(
            &engine,
            &input,
            &invalid,
            None,
            sunipdf_core::bookmarks::BookmarkEdit::Delete { index: 99 }
        )
        .is_err());
        assert!(!invalid.exists());
        assert_eq!(std::fs::read(&input).unwrap(), original);
    }

    #[test]
    fn office_commands_and_font_faces_keep_existing_defaults_and_new_copy_paths() {
        let import = parse_cli([
            "sunipdf-cli",
            "office-to-pdf",
            "input.docx",
            "--out",
            "new.pdf",
        ])
        .unwrap();
        assert!(
            matches!(import.command, Command::OfficeToPdf { ref profile, .. } if profile == "standard")
        );
        assert_eq!(
            output_path(&import.command),
            Some(std::path::Path::new("new.pdf"))
        );
        let export = parse_cli([
            "sunipdf-cli",
            "export-office",
            "input.pdf",
            "--format",
            "powerpoint",
            "--out",
            "new.pptx",
        ])
        .unwrap();
        assert_eq!(
            output_path(&export.command),
            Some(std::path::Path::new("new.pptx"))
        );
        let text = parse_cli([
            "sunipdf-cli",
            "add-text",
            "input.pdf",
            "--x",
            "10",
            "--y",
            "20",
            "--text",
            "Hello",
            "--out",
            "new.pdf",
        ])
        .unwrap();
        assert!(matches!(
            text.command,
            Command::AddText {
                font_face_index: None,
                font_file: None,
                ..
            }
        ));
        assert!(parse_cli([
            "sunipdf-cli",
            "add-text",
            "input.pdf",
            "--x",
            "10",
            "--y",
            "20",
            "--text",
            "Hello",
            "--font-face-index",
            "1",
            "--out",
            "new.pdf"
        ])
        .is_err());
    }

    #[test]
    fn signature_network_checks_require_explicit_opt_in() {
        let offline = parse_cli(["sunipdf-cli", "signatures", "signed.pdf"]).unwrap();
        assert!(matches!(
            offline.command,
            Command::Signatures {
                allow_network: false,
                ..
            }
        ));
        let online =
            parse_cli(["sunipdf-cli", "signatures", "signed.pdf", "--allow-network"]).unwrap();
        assert!(matches!(
            online.command,
            Command::Signatures {
                allow_network: true,
                ..
            }
        ));
    }

    #[test]
    fn stamp_help_preserves_literal_page_variables() {
        for command in ["stamp", "page-numbers"] {
            for flag in ["-h", "--help"] {
                let error = parse_cli(["sunipdf-cli", command, flag])
                    .err()
                    .expect("help must stop parsing");
                assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
                let help = error.to_string();
                assert!(
                    help.contains("Template variables: {n} = page number, {total} = page count.")
                );
                assert!(
                    help.contains("Usage:")
                        && help.contains("--out")
                        && help.contains("--overwrite")
                );
                if command == "page-numbers" {
                    assert!(help.contains("{n} / {total}"));
                }
            }
        }
    }

    #[test]
    fn annotation_dash_flag_defaults_to_solid_and_keeps_output_guard() {
        let solid = parse_cli([
            "sunipdf-cli",
            "annotate",
            "input.pdf",
            "--kind",
            "square",
            "--out",
            "output.pdf",
        ])
        .unwrap();
        assert!(matches!(
            solid.command,
            Command::Annotate { dashed: false, .. }
        ));
        assert_eq!(
            output_path(&solid.command),
            Some(std::path::Path::new("output.pdf"))
        );
        let dashed = parse_cli([
            "sunipdf-cli",
            "annotate",
            "input.pdf",
            "--kind",
            "arrow",
            "--dashed",
            "--out",
            "output.pdf",
            "--overwrite",
        ])
        .unwrap();
        assert!(dashed.overwrite);
        assert!(matches!(
            dashed.command,
            Command::Annotate { dashed: true, .. }
        ));
    }

    #[test]
    fn link_target_command_requires_one_target_and_guards_output() {
        let parsed = parse_cli([
            "sunipdf-cli",
            "set-link-target",
            "input.pdf",
            "--index",
            "7",
            "--goto-page",
            "2",
            "--out",
            "output.pdf",
        ])
        .unwrap();
        assert_eq!(
            output_path(&parsed.command),
            Some(std::path::Path::new("output.pdf"))
        );
        assert!(matches!(
            parsed.command,
            Command::SetLinkTarget {
                page: 1,
                index: 7,
                goto_page: Some(2),
                url: None,
                ..
            }
        ));
        let parsed = parse_cli([
            "sunipdf-cli",
            "set-link-target",
            "input.pdf",
            "--page",
            "2",
            "--index",
            "0",
            "--url",
            "https://example.org",
            "--out",
            "output.pdf",
            "--password",
            "secret",
            "--overwrite",
        ])
        .unwrap();
        assert!(parsed.overwrite);
        assert!(matches!(
            parsed.command,
            Command::SetLinkTarget {
                page: 2,
                index: 0,
                goto_page: None,
                url: Some(_),
                password: Some(_),
                ..
            }
        ));
        assert!(parse_cli([
            "sunipdf-cli",
            "set-link-target",
            "input.pdf",
            "--index",
            "7",
            "--out",
            "output.pdf",
        ])
        .is_err());
        assert!(parse_cli([
            "sunipdf-cli",
            "set-link-target",
            "input.pdf",
            "--index",
            "7",
            "--goto-page",
            "2",
            "--url",
            "https://example.org",
            "--out",
            "output.pdf",
        ])
        .is_err());
        assert!(parse_cli([
            "sunipdf-cli",
            "set-link-target",
            "input.pdf",
            "--url",
            "https://example.org",
            "--out",
            "output.pdf",
        ])
        .is_err());
    }

    #[test]
    fn page_label_commands_parse_defaults_styles_and_output_guards() {
        let default = parse_cli([
            "sunipdf-cli",
            "set-page-labels",
            "input.pdf",
            "--out",
            "output.pdf",
        ])
        .unwrap();
        assert_eq!(
            output_path(&default.command),
            Some(std::path::Path::new("output.pdf"))
        );
        assert!(
            matches!(default.command, Command::SetPageLabels { from: 1, to: None, start: 1, style, prefix, .. } if style == "decimal" && prefix.is_empty())
        );
        for style in [
            "decimal",
            "roman-upper",
            "roman-lower",
            "letters-upper",
            "letters-lower",
            "prefix-only",
        ] {
            let parsed = parse_cli([
                "sunipdf-cli",
                "set-page-labels",
                "input.pdf",
                "--from",
                "2",
                "--to",
                "4",
                "--style",
                style,
                "--start",
                "27",
                "--prefix",
                "É-",
                "--out",
                "output.pdf",
                "--overwrite",
            ])
            .unwrap();
            assert!(parsed.overwrite);
            assert!(
                matches!(parsed.command, Command::SetPageLabels { from: 2, to: Some(4), start: 27, prefix, .. } if prefix == "É-")
            );
        }
        assert!(parse_cli([
            "sunipdf-cli",
            "set-page-labels",
            "input.pdf",
            "--style",
            "unknown",
            "--out",
            "output.pdf"
        ])
        .is_err());
        assert!(parse_cli([
            "sunipdf-cli",
            "set-page-labels",
            "input.pdf",
            "--start",
            "4294967296",
            "--out",
            "output.pdf"
        ])
        .is_err());
        let reset = parse_cli([
            "sunipdf-cli",
            "reset-page-labels",
            "input.pdf",
            "--out",
            "output.pdf",
        ])
        .unwrap();
        assert!(matches!(reset.command, Command::ResetPageLabels { .. }));
        assert_eq!(
            output_path(&reset.command),
            Some(std::path::Path::new("output.pdf"))
        );
    }

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
        let cli = parse_cli([
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
        assert!(parse_cli([
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
        let cli = parse_cli([
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
            parse_cli([
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
        let cli = parse_cli([
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
        assert!(parse_cli([
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
        let cli = parse_cli([
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
        assert!(parse_cli(["sunipdf-cli", "add-image", "a.pdf", "--out", "o.pdf"]).is_err());
    }

    #[test]
    fn image_watermark_cli_participates_in_password_and_output_guards() {
        let cli = parse_cli([
            "sunipdf-cli",
            "--password-env",
            "TEST_PDF_PASSWORD",
            "image-watermark",
            "source.pdf",
            "--image",
            "logo.png",
            "--width-percent",
            "42",
            "--angle",
            "-45",
            "--out",
            "copy.pdf",
        ])
        .unwrap();
        assert!(
            matches!(&cli.command,Command::ImageWatermark {width_percent,angle,..} if *width_percent==42.0 && *angle== -45.0)
        );
        assert_eq!(
            output_path(&cli.command),
            Some(std::path::Path::new("copy.pdf"))
        );
        assert!(parse_cli([
            "sunipdf-cli",
            "image-watermark",
            "source.pdf",
            "--out",
            "copy.pdf"
        ])
        .is_err());
    }
}
