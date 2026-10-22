use crate::password_input;
use clap::{Args, Subcommand};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use sunipdf_core::annotation_library::{
    AnnotationLibrary, LibraryEntry, LibraryPlacement, MAX_LIBRARY_BYTES, MAX_LIBRARY_ENTRIES,
};
use sunipdf_core::{Engine, EngineError};

#[derive(Args)]
pub struct LibraryArgs {
    #[command(subcommand)]
    pub command: LibraryCommand,
}

#[derive(Subcommand)]
pub enum LibraryCommand {
    /// Validate and inspect a self-contained annotation library without opening a PDF.
    Inspect { file: PathBuf },
    /// Add one validated template or group JSON entry to a new library copy.
    Add {
        /// Existing library; omit to create the first library.
        #[arg(long)]
        library: Option<PathBuf>,
        /// One self-contained template or group entry, using the library version 1 schema.
        #[arg(long)]
        entry: PathBuf,
        /// New JSON file; existing destinations are always refused.
        #[arg(long)]
        out: PathBuf,
    },
    /// Remove one exact entry ID and save a new library copy.
    Remove {
        file: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate and export a complete library or explicitly selected entry IDs.
    Export {
        file: PathBuf,
        /// Repeat to select entries. Omit to export all entries.
        #[arg(long)]
        id: Vec<String>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Place one library entry in a new PDF copy through one atomic group placement.
    Place {
        /// Validated version 1 library JSON.
        library: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        file: PathBuf,
        /// Physical page number, starting at 1.
        #[arg(long, value_parser = positive_page)]
        page: usize,
        /// Displayed-page lower-left origin as x,y, in points.
        #[arg(long, value_parser = origin)]
        origin: (f32, f32),
        /// Explicit finite positive uniform scale.
        #[arg(long, value_parser = positive_scale)]
        scale: f32,
        #[arg(long)]
        author: Option<String>,
        /// New PDF copy; existing destinations are refused even with --overwrite.
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
}

impl LibraryArgs {
    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            LibraryCommand::Inspect { .. } => None,
            LibraryCommand::Add { out, .. }
            | LibraryCommand::Remove { out, .. }
            | LibraryCommand::Export { out, .. }
            | LibraryCommand::Place { out, .. } => Some(out),
        }
    }

    pub fn password(&mut self) -> Option<&mut Option<String>> {
        match &mut self.command {
            LibraryCommand::Place { password, .. } => Some(password),
            _ => None,
        }
    }
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|page| *page > 0)
        .ok_or_else(|| "page numbers start at 1".into())
}

fn positive_scale(value: &str) -> Result<f32, String> {
    value
        .parse::<f32>()
        .ok()
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .ok_or_else(|| "scale must be finite and positive".into())
}

fn origin(value: &str) -> Result<(f32, f32), String> {
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| "origin must be x,y in displayed-page points".to_string())?;
    let coordinate = |part: &str| {
        part.trim()
            .parse::<f32>()
            .ok()
            .filter(|number| number.is_finite() && *number >= 0.0)
            .ok_or_else(|| "origin coordinates must be finite and nonnegative".to_string())
    };
    Ok((coordinate(x)?, coordinate(y)?))
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, EngineError> {
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(EngineError::Other(
            "library input must be a regular file".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(EngineError::Other(
            "annotation library JSON exceeds its size limit".into(),
        ));
    }
    Ok(bytes
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .unwrap_or(&bytes)
        .to_vec())
}

fn ensure_new(out: &Path) -> Result<(), EngineError> {
    if out.exists() {
        return Err(EngineError::AlreadyExists(out.to_path_buf()));
    }
    Ok(())
}

fn read_library(path: &Path) -> Result<AnnotationLibrary, EngineError> {
    AnnotationLibrary::from_json(&read_bounded(path, MAX_LIBRARY_BYTES)?)
}

fn write_library(out: &Path, library: &AnnotationLibrary) -> Result<(), EngineError> {
    sunipdf_core::write_new_copy(out, &library.to_json()?)
}

pub fn run(args: &LibraryArgs) -> Result<Value, EngineError> {
    if let Some(out) = args.output_path() {
        ensure_new(out)?;
    }
    match &args.command {
        LibraryCommand::Inspect { file } => {
            let library = read_library(file)?;
            serde_json::from_slice(&library.to_json()?)
                .map_err(|_| EngineError::Other("could not serialize annotation library".into()))
        }
        LibraryCommand::Add {
            library,
            entry,
            out,
        } => {
            let mut library = match library {
                Some(file) => read_library(file)?,
                None => AnnotationLibrary::default(),
            };
            let entry = LibraryEntry::from_json(&read_bounded(entry, MAX_LIBRARY_BYTES)?)?;
            let id = entry.id.clone();
            library.insert(entry)?;
            write_library(out, &library)?;
            Ok(serde_json::json!({"out":out,"entries":library.entries().len(),"entry_id":id}))
        }
        LibraryCommand::Remove { file, id, out } => {
            let mut library = read_library(file)?;
            library.remove(id)?;
            write_library(out, &library)?;
            Ok(serde_json::json!({"out":out,"entries":library.entries().len(),"removed_id":id}))
        }
        LibraryCommand::Export { file, id, out } => {
            let library = read_library(file)?;
            if id.len() > MAX_LIBRARY_ENTRIES {
                return Err(EngineError::Other(
                    "too many annotation library selections".into(),
                ));
            }
            let selected = if id.is_empty() {
                library
            } else {
                let mut selected = AnnotationLibrary::default();
                for id in id {
                    let entry = library.entry(id).ok_or_else(|| {
                        EngineError::Other("annotation library entry was not found".into())
                    })?;
                    selected.insert(entry.clone())?;
                }
                selected
            };
            write_library(out, &selected)?;
            Ok(serde_json::json!({"out":out,"entries":selected.entries().len()}))
        }
        LibraryCommand::Place {
            library,
            id,
            file,
            page,
            origin,
            scale,
            author,
            out,
            password,
        } => {
            let library = read_library(library)?;
            let entry = library.entry(id).ok_or_else(|| {
                EngineError::Other("annotation library entry was not found".into())
            })?;
            let engine = Engine::new()?;
            let mut document = engine.open(file, password.as_deref())?;
            let selected_page = crate::page_index(*page, document.page_count())?;
            let indices = document.place_library_entry(
                selected_page,
                entry,
                LibraryPlacement {
                    origin: *origin,
                    scale: *scale,
                },
                author.as_deref(),
            )?;
            sunipdf_core::write_new_copy(out, &document.snapshot()?)?;
            // The result describes only this requested insertion. It never exports PDF text,
            // geometry, source annotations, metadata or rendered page content.
            Ok(serde_json::json!({
                "out":out,"page":page,"entry_id":id,"count":indices.len(),"indices":indices,
                "original_unchanged":true,"encrypted":document.properties().encrypted,
                "source_content_exported":false,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::parse_cli;

    #[test]
    fn placement_requires_explicit_finite_geometry_and_accepts_common_password_sources() {
        let base = [
            "sunipdf-cli",
            "annotation-library",
            "place",
            "symbols.json",
            "--id",
            "review",
            "--file",
            "plan.pdf",
            "--out",
            "placed.pdf",
        ];
        assert!(parse_cli(base).is_err());
        for (page, origin, scale) in [
            ("0", "10,20", "1"),
            ("1", "NaN,20", "1"),
            ("1", "-1,20", "1"),
            ("1", "10,20,30", "1"),
            ("1", "10,20", "NaN"),
            ("1", "10,20", "0"),
        ] {
            assert!(parse_cli(
                base.into_iter()
                    .chain(["--page", page, "--origin", origin, "--scale", scale])
            )
            .is_err());
        }
        let parsed = parse_cli(base.into_iter().chain([
            "--page",
            "2",
            "--origin",
            "10,20",
            "--scale",
            "0.5",
            "--password-stdin",
        ]))
        .unwrap();
        assert_eq!(
            crate::output_path(&parsed.command),
            Some(Path::new("placed.pdf"))
        );
        assert!(
            matches!(parsed.command, crate::Command::AnnotationLibrary(LibraryArgs {
            command: LibraryCommand::Place { page: 2, origin, scale, password: None, .. }
        }) if origin == (10.0, 20.0) && scale == 0.5)
        );
    }

    #[test]
    fn management_commands_reject_pdf_password_input_before_reading_stdin() {
        let mut parsed = parse_cli([
            "sunipdf-cli",
            "annotation-library",
            "inspect",
            "symbols.json",
            "--password-stdin",
        ])
        .unwrap();
        assert!(parsed.passwords.apply(&mut parsed.command).is_err());
    }
}
