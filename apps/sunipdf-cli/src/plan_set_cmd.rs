use crate::password_input;
use clap::{Args, Subcommand};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use sunipdf_core::plan_set::{self, AddOptions, PlanSet};
use sunipdf_core::{Engine, EngineError};

#[derive(Args)]
pub struct PlanSetArgs {
    #[command(subcommand)]
    pub command: PlanSetCommand,
}

#[derive(Subcommand)]
pub enum PlanSetCommand {
    /// Index explicitly selected PDFs into a new local plan-set JSON file, without modifying PDFs.
    Create {
        #[arg(required = true, num_args = 1..=64)]
        files: Vec<PathBuf>,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "")]
        discipline: String,
        #[arg(long, default_value = "")]
        revision: String,
        /// New JSON file; existing destinations are refused even with --overwrite.
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Inspect stored sheet metadata and check the current local PDF references.
    Inspect {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Validate local references; incomplete reports remain JSON and return exit code 2.
    Validate {
        file: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Add all pages of explicitly selected PDFs to a new copy of a plan set.
    Add {
        file: PathBuf,
        /// Repeat for each PDF; no directory scanning or implicit file discovery.
        #[arg(long, required = true)]
        pdf: Vec<PathBuf>,
        #[arg(long, default_value = "")]
        discipline: String,
        #[arg(long, default_value = "")]
        revision: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Edit one sheet's metadata in a new JSON copy; the sheet number must remain nonblank.
    EditSheet {
        file: PathBuf,
        #[arg(long)]
        id: String,
        /// Nonblank displayed sheet number; this does not change the physical page.
        #[arg(long)]
        number: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        discipline: Option<String>,
        #[arg(long)]
        revision: Option<String>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Explicitly accept the current bytes of one PDF reference or relink that reference.
    Refresh {
        file: PathBuf,
        #[arg(long)]
        file_id: String,
        /// Replacement local PDF; omit to reread the existing referenced path.
        #[arg(long)]
        source: Option<PathBuf>,
        /// Required acknowledgement; no page matching or annotation transfer is performed.
        #[arg(long, required = true)]
        accept_changed: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, help = password_input::ARGV_PASSWORD_HELP)]
        password: Option<String>,
    },
    /// Remove one indexed sheet from a new JSON copy, leaving the source PDF untouched.
    RemoveSheet {
        file: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Export the validated manifest to a new path, rebasing its PDF references.
    Export {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

impl PlanSetArgs {
    pub fn output_path(&self) -> Option<&Path> {
        match &self.command {
            PlanSetCommand::Inspect { .. } | PlanSetCommand::Validate { .. } => None,
            PlanSetCommand::Create { out, .. }
            | PlanSetCommand::Add { out, .. }
            | PlanSetCommand::EditSheet { out, .. }
            | PlanSetCommand::Refresh { out, .. }
            | PlanSetCommand::RemoveSheet { out, .. }
            | PlanSetCommand::Export { out, .. } => Some(out),
        }
    }

    pub fn password(&mut self) -> Option<&mut Option<String>> {
        match &mut self.command {
            PlanSetCommand::Create { password, .. }
            | PlanSetCommand::Inspect { password, .. }
            | PlanSetCommand::Validate { password, .. }
            | PlanSetCommand::Add { password, .. }
            | PlanSetCommand::Refresh { password, .. } => Some(password),
            _ => None,
        }
    }

    pub fn is_inspection(&self) -> bool {
        matches!(
            self.command,
            PlanSetCommand::Inspect { .. } | PlanSetCommand::Validate { .. }
        )
    }
}

fn saved(set: &PlanSet, original: &Path, out: &Path) -> Result<Value, EngineError> {
    plan_set::save_new(set, original, out)?;
    Ok(json!({
        "out": out, "plan_set_id": set.id, "name": set.name,
        "files": set.files.len(), "sheets": set.sheets.len(),
        "source_documents_unchanged": true
    }))
}

pub fn run(args: &PlanSetArgs) -> Result<Value, EngineError> {
    // This workflow always writes a new manifest, independently of the global overwrite flag.
    if let Some(out) = args.output_path() {
        if out.exists() {
            return Err(EngineError::AlreadyExists(out.to_path_buf()));
        }
    }
    match &args.command {
        PlanSetCommand::Create {
            files,
            name,
            discipline,
            revision,
            out,
            password,
        } => {
            let mut set = PlanSet::new(name)?;
            let engine = Engine::new()?;
            let options = AddOptions {
                discipline: discipline.clone(),
                revision: revision.clone(),
            };
            for source in files {
                plan_set::add_pdf(
                    &mut set,
                    &engine,
                    out,
                    source,
                    password.as_deref(),
                    &options,
                )?;
            }
            saved(&set, out, out)
        }
        PlanSetCommand::Inspect { file, password }
        | PlanSetCommand::Validate { file, password } => {
            let set = plan_set::load(file)?;
            let engine = Engine::new()?;
            let report = plan_set::inspect(
                &engine,
                file,
                &set,
                password.as_deref(),
                &AtomicBool::new(false),
                |_, _| {},
            )?;
            let mut value = report.json();
            value["complete"] = json!(report.complete());
            value["manifest"] = json!(set);
            value["manifest_path"] = json!(file);
            value["page_index_base"] = json!(1);
            value["source_documents_unchanged"] = json!(true);
            Ok(value)
        }
        PlanSetCommand::Add {
            file,
            pdf,
            discipline,
            revision,
            out,
            password,
        } => {
            let mut set = plan_set::load(file)?;
            let engine = Engine::new()?;
            let options = AddOptions {
                discipline: discipline.clone(),
                revision: revision.clone(),
            };
            for source in pdf {
                plan_set::add_pdf(
                    &mut set,
                    &engine,
                    file,
                    source,
                    password.as_deref(),
                    &options,
                )?;
            }
            saved(&set, file, out)
        }
        PlanSetCommand::EditSheet {
            file,
            id,
            number,
            title,
            discipline,
            revision,
            out,
        } => {
            if [number, title, discipline, revision]
                .iter()
                .all(|field| field.is_none())
            {
                return Err(EngineError::Other(
                    "edit-sheet requires at least one metadata field".into(),
                ));
            }
            let mut set = plan_set::load(file)?;
            let sheet = set
                .sheets
                .iter_mut()
                .find(|sheet| sheet.id == *id)
                .ok_or_else(|| EngineError::Other("plan-set sheet ID was not found".into()))?;
            for (destination, value) in [
                (&mut sheet.number, number),
                (&mut sheet.title, title),
                (&mut sheet.discipline, discipline),
                (&mut sheet.revision, revision),
            ] {
                if let Some(value) = value {
                    *destination = value.clone();
                }
            }
            plan_set::validate(&set)?;
            saved(&set, file, out)
        }
        PlanSetCommand::Refresh {
            file,
            file_id,
            source,
            accept_changed,
            out,
            password,
        } => {
            if !accept_changed {
                return Err(EngineError::Other(
                    "refresh requires explicit --accept-changed".into(),
                ));
            }
            let mut set = plan_set::load(file)?;
            let engine = Engine::new()?;
            plan_set::refresh_exact(
                &mut set,
                &engine,
                file,
                file_id,
                source.as_deref(),
                password.as_deref(),
            )?;
            saved(&set, file, out)
        }
        PlanSetCommand::RemoveSheet { file, id, out } => {
            let mut set = plan_set::load(file)?;
            let index = set
                .sheets
                .iter()
                .position(|sheet| sheet.id == *id)
                .ok_or_else(|| EngineError::Other("plan-set sheet ID was not found".into()))?;
            let removed = set.sheets.remove(index);
            if !set
                .sheets
                .iter()
                .any(|sheet| sheet.file_id == removed.file_id)
            {
                set.files.retain(|entry| entry.id != removed.file_id);
            }
            saved(&set, file, out)
        }
        PlanSetCommand::Export { file, out } => saved(&plan_set::load(file)?, file, out),
    }
}
