use crate::Command;
use std::io::{IsTerminal, Read};
use sunipdf_core::EngineError;
use zeroize::Zeroizing;

pub const ARGV_PASSWORD_HELP: &str =
    "Unsafe in process arguments/history; prefer --password-env NAME or --password-stdin";
pub const ARGV_USER_PASSWORD_HELP: &str =
    "Unsafe in process arguments/history; prefer --user-password-env NAME or --user-password-stdin";
pub const ARGV_OWNER_PASSWORD_HELP: &str = "Unsafe in process arguments/history; prefer --owner-password-env NAME or --owner-password-stdin";

#[derive(clap::Args, Default)]
pub struct PasswordSources {
    /// Read the PDF password from this environment variable, without putting its value in argv.
    #[arg(long, global = true, conflicts_with = "password_stdin")]
    password_env: Option<String>,
    /// Read the PDF password from piped UTF-8 stdin; remove one trailing line ending.
    #[arg(long, global = true)]
    password_stdin: bool,
    /// For protect: read the new opening password from this environment variable.
    #[arg(long, global = true, conflicts_with = "user_password_stdin")]
    user_password_env: Option<String>,
    /// For protect: read the new opening password from piped UTF-8 stdin.
    #[arg(long, global = true)]
    user_password_stdin: bool,
    /// For protect: read the new owner password from this environment variable.
    #[arg(long, global = true, conflicts_with = "owner_password_stdin")]
    owner_password_env: Option<String>,
    /// For protect: read the new owner password from piped UTF-8 stdin.
    #[arg(long, global = true)]
    owner_password_stdin: bool,
}

fn invalid(message: &str) -> EngineError {
    EngineError::Other(message.into())
}

fn read_secret(variable: Option<&str>, stdin: bool) -> Result<String, EngineError> {
    let value = if let Some(name) = variable {
        let value = Zeroizing::new(std::env::var(name).map_err(|_| {
            invalid("PDF password environment variable is missing or is not Unicode")
        })?);
        if value.len() > 65_536 || value.contains('\0') {
            return Err(invalid(
                "PDF password exceeds limits or contains a null character",
            ));
        }
        value
    } else {
        if !stdin || std::io::stdin().is_terminal() {
            return Err(invalid("PDF password stdin must be piped"));
        }
        let mut bytes = Zeroizing::new(Vec::new());
        std::io::stdin().take(65_537).read_to_end(&mut bytes)?;
        if bytes.len() > 65_536 {
            return Err(invalid("PDF password input exceeds 65536 bytes"));
        }
        if bytes.ends_with(b"\r\n") {
            let length = bytes.len() - 2;
            bytes.truncate(length);
        } else if bytes.ends_with(b"\n") || bytes.ends_with(b"\r") {
            bytes.pop();
        }
        let value =
            std::str::from_utf8(&bytes).map_err(|_| invalid("PDF password stdin must be UTF-8"))?;
        if value.contains('\0') {
            return Err(invalid("PDF password contains a null character"));
        }
        Zeroizing::new(value.to_owned())
    };
    Ok(value.to_string())
}

fn document_password(command: &mut Command) -> Option<&mut Option<String>> {
    match command {
        Command::RevisionOverlay(args) => Some(&mut args.password),
        Command::AnnotationLibrary(args) => args.password(),
        Command::Count(args) => Some(&mut args.password),
        Command::AnnotationRegister(args) => Some(&mut args.password),
        Command::AnnotationFields(args) => Some(&mut args.password),
        Command::PageCalibration(args) => Some(&mut args.password),
        Command::SetPageCalibration(args) => Some(&mut args.password),
        Command::RemovePageCalibration(args) => Some(&mut args.password),
        Command::EditMeasurement(args) => Some(&mut args.password),
        Command::AddFormField { password, .. }
        | Command::Inspect { password, .. }
        | Command::Ocr { password, .. }
        | Command::ExportOffice { password, .. }
        | Command::Signatures { password, .. }
        | Command::Text { password, .. }
        | Command::Extract { password, .. }
        | Command::Split { password, .. }
        | Command::Rotate { password, .. }
        | Command::Stamp { password, .. }
        | Command::ImageWatermark { password, .. }
        | Command::ImageStamp { password, .. }
        | Command::Measure { password, .. }
        | Command::Measurements { password, .. }
        | Command::HeadersFooters { password, .. }
        | Command::PageNumbers { password, .. }
        | Command::SetPageLabels { password, .. }
        | Command::ResetPageLabels { password, .. }
        | Command::Insert { password, .. }
        | Command::Move { password, .. }
        | Command::Properties { password, .. }
        | Command::LinkDestinations { password, .. }
        | Command::Bookmarks { password, .. }
        | Command::BookmarkAdd { password, .. }
        | Command::BookmarkRename { password, .. }
        | Command::BookmarkDestination { password, .. }
        | Command::BookmarkMove { password, .. }
        | Command::BookmarkDelete { password, .. }
        | Command::Protect { password, .. }
        | Command::Unprotect { password, .. }
        | Command::SetMetadata { password, .. }
        | Command::StripMetadata { password, .. }
        | Command::Sanitize { password, .. }
        | Command::Redact { password, .. }
        | Command::ExportImages { password, .. }
        | Command::Optimize { password, .. }
        | Command::CompressImages { password, .. }
        | Command::Search { password, .. }
        | Command::SearchFiles { password, .. }
        | Command::Compare { password, .. }
        | Command::Info { password, .. }
        | Command::Objects { password, .. }
        | Command::ReorderObjects { password, .. }
        | Command::AddText { password, .. }
        | Command::SetOpacity { password, .. }
        | Command::RotateObject { password, .. }
        | Command::RotateObjects { password, .. }
        | Command::ResizeObjects { password, .. }
        | Command::CopyObjects { password, .. }
        | Command::ReplaceImage { password, .. }
        | Command::AddImage { password, .. }
        | Command::Crop { password, .. }
        | Command::ResizePages { password, .. }
        | Command::InsertBlank { password, .. }
        | Command::Render { password, .. }
        | Command::FormFields { password, .. }
        | Command::FormExport { password, .. }
        | Command::FillForm { password, .. }
        | Command::Flatten { password, .. }
        | Command::Annotations { password, .. }
        | Command::Annotate { password, .. }
        | Command::SetLinkTarget { password, .. }
        | Command::AnnotationDelete { password, .. }
        | Command::Reply { password, .. }
        | Command::ReviewState { password, .. } => Some(password),
        _ => None,
    }
}

impl PasswordSources {
    pub fn apply(&self, command: &mut Command) -> Result<(), EngineError> {
        if [
            matches!(command, Command::RevisionOverlay(args) if args.revision_password_stdin),
            self.password_stdin,
            self.user_password_stdin,
            self.owner_password_stdin,
        ]
        .into_iter()
        .filter(|value| *value)
        .count()
            > 1
        {
            return Err(invalid(
                "only one PDF password can be read from stdin per call",
            ));
        }
        let new_user = self.user_password_env.is_some() || self.user_password_stdin;
        let new_owner = self.owner_password_env.is_some() || self.owner_password_stdin;
        if (new_user || new_owner) && !matches!(command, Command::Protect { .. }) {
            return Err(invalid(
                "new user/owner password sources require the protect command",
            ));
        }
        if self.password_env.is_some() || self.password_stdin {
            let target = document_password(command)
                .ok_or_else(|| invalid("this command does not accept a PDF password"))?;
            if target.is_some() {
                return Err(invalid(
                    "choose an argv password or an environment/stdin source, not both",
                ));
            }
            *target = Some(read_secret(
                self.password_env.as_deref(),
                self.password_stdin,
            )?);
        }
        if let Command::RevisionOverlay(args) = command {
            if args.revision_password_env.is_some() || args.revision_password_stdin {
                if args.revision_password.is_some() {
                    return Err(invalid("choose one source for the revision PDF password"));
                }
                args.revision_password = Some(read_secret(
                    args.revision_password_env.as_deref(),
                    args.revision_password_stdin,
                )?);
            }
        }
        if let Command::Protect {
            user_password,
            owner_password,
            ..
        } = command
        {
            for (target, environment, stdin, selected) in [
                (
                    user_password,
                    self.user_password_env.as_deref(),
                    self.user_password_stdin,
                    new_user,
                ),
                (
                    owner_password,
                    self.owner_password_env.as_deref(),
                    self.owner_password_stdin,
                    new_owner,
                ),
            ] {
                if selected {
                    if !target.is_empty() {
                        return Err(invalid("choose one source for each new PDF password"));
                    }
                    *target = read_secret(environment, stdin)?;
                }
            }
        }
        Ok(())
    }
}
