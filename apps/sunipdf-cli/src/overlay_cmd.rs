use crate::password_input;
use clap::Args;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use sunipdf_core::revision_overlay::{
    prepare_overlay, OverlayAlignment, OverlayOptions, OverlayPasswords, OverlayTransform,
};
use sunipdf_core::{Engine, EngineError};

#[derive(Args)]
pub struct OverlayArgs {
    /// Original PDF, at most 64 MiB; rendered in magenta.
    pub base: PathBuf,
    /// Revised PDF, at most 64 MiB; rendered in cyan.
    pub revision: PathBuf,
    #[arg(long, default_value_t=1, value_parser=positive_page)]
    pub base_page: usize,
    #[arg(long, default_value_t=1, value_parser=positive_page)]
    pub revision_page: usize,
    /// Revision-to-base translation x,y in displayed-page points, origin TOP LEFT; y grows down.
    #[arg(long, value_parser=point, allow_hyphen_values=true, conflicts_with_all=["base_points", "revision_points"])]
    pub translate: Option<[f64; 2]>,
    /// Clockwise rotation in degrees about the revision's displayed top-left origin.
    #[arg(long, value_parser=finite, allow_negative_numbers=true, conflicts_with_all=["base_points", "revision_points"])]
    pub rotation: Option<f64>,
    /// Positive uniform revision-to-base scale; defaults to 1.
    #[arg(long, value_parser=positive_scale, conflicts_with_all=["base_points", "revision_points"])]
    pub scale: Option<f64>,
    /// Two matching base points: "x,y x,y", TOP LEFT origin. Requires --revision-points.
    #[arg(long, value_parser=two_points, allow_hyphen_values=true, requires="revision_points")]
    pub base_points: Option<[[f64; 2]; 2]>,
    /// Two matching revision points in the same order, TOP LEFT origin.
    #[arg(long, value_parser=two_points, allow_hyphen_values=true, requires="base_points")]
    pub revision_points: Option<[[f64; 2]; 2]>,
    /// Cyan revision opacity from 0 to 1; the magenta base remains fully visible.
    #[arg(long, default_value_t=0.65, value_parser=opacity)]
    pub opacity: f32,
    /// Maximum image side in pixels; 64 to 2048 pixels; defaults to 1536.
    #[arg(long, default_value_t=1536, value_parser=clap::value_parser!(u32).range(64..=2048))]
    pub max_side: u32,
    /// Write one PNG or one JSON manifest per call; neither PDF is modified.
    #[arg(long, default_value="png", value_parser=["png", "json"])]
    pub format: String,
    /// New output only; --overwrite never replaces an existing file for this command.
    #[arg(long)]
    pub out: PathBuf,
    /// Optional base SHA-256 from a previous manifest; refuse a changed source.
    #[arg(long, value_parser=sha256)]
    pub expected_base_sha256: Option<String>,
    /// Optional revision SHA-256 from a previous manifest; refuse a changed source.
    #[arg(long, value_parser=sha256)]
    pub expected_revision_sha256: Option<String>,
    #[arg(long, help=password_input::ARGV_PASSWORD_HELP)]
    pub password: Option<String>,
    /// Unsafe in process arguments/history; prefer --revision-password-env or --revision-password-stdin.
    #[arg(long, conflicts_with_all=["revision_password_env", "revision_password_stdin"])]
    pub revision_password: Option<String>,
    /// Read the revision password from this environment variable.
    #[arg(long, conflicts_with = "revision_password_stdin")]
    pub revision_password_env: Option<String>,
    /// Read the revision password from piped UTF-8 stdin; only one password may use stdin.
    #[arg(long)]
    pub revision_password_stdin: bool,
}

fn positive_page(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|page| *page > 0)
        .ok_or_else(|| "page numbers start at 1".into())
}

fn finite(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| "coordinate or angle must be finite".into())
}

fn positive_scale(value: &str) -> Result<f64, String> {
    finite(value).and_then(|number| {
        if number > 0.0 {
            Ok(number)
        } else {
            Err("scale must be positive".into())
        }
    })
}

fn opacity(value: &str) -> Result<f32, String> {
    let value = finite(value)?;
    if !(0.0..=1.0).contains(&value) {
        return Err("opacity must be between 0 and 1".into());
    }
    Ok(value as f32)
}

fn point(value: &str) -> Result<[f64; 2], String> {
    if value.len() > 128 {
        return Err("point exceeds its size limit".into());
    }
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| "point must be x,y".to_string())?;
    Ok([finite(x)?, finite(y)?])
}

fn two_points(value: &str) -> Result<[[f64; 2]; 2], String> {
    if value.len() > 260 {
        return Err("reference points exceed their size limit".into());
    }
    let parts = value.split_whitespace().collect::<Vec<_>>();
    let [first, second] = parts.as_slice() else {
        return Err("provide exactly two points: x,y x,y".into());
    };
    let points = [point(first)?, point(second)?];
    if points[0] == points[1] {
        return Err("reference points must be distinct".into());
    }
    Ok(points)
}

fn sha256(value: &str) -> Result<String, String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("expected source SHA-256 must contain 64 hexadecimal digits".into());
    }
    Ok(value.to_ascii_lowercase())
}

pub fn run(engine: &Engine, args: OverlayArgs) -> Result<serde_json::Value, EngineError> {
    if args.out.exists() {
        return Err(EngineError::AlreadyExists(args.out));
    }
    let alignment = match (args.base_points, args.revision_points) {
        (Some(base), Some(revision)) => OverlayAlignment::TwoPoints { base, revision },
        (None, None) => OverlayAlignment::Manual {
            transform: OverlayTransform {
                translation: args.translate.unwrap_or([0.0, 0.0]),
                rotation_degrees: args.rotation.unwrap_or(0.0),
                scale: args.scale.unwrap_or(1.0),
            },
        },
        _ => {
            return Err(EngineError::Other(
                "provide both base and revision reference points".into(),
            ))
        }
    };
    let cancel = AtomicBool::new(false);
    let preview = prepare_overlay(
        engine,
        &args.base,
        &args.revision,
        OverlayOptions {
            base_page: args.base_page,
            revision_page: args.revision_page,
            alignment,
            opacity: args.opacity,
            long_side: args.max_side,
        },
        OverlayPasswords {
            base: args.password.as_deref(),
            revision: args.revision_password.as_deref(),
        },
        &cancel,
    )?;
    let report = preview.export_json(&cancel)?;
    for (expected, source) in [
        (args.expected_base_sha256.as_deref(), "base"),
        (args.expected_revision_sha256.as_deref(), "revision"),
    ] {
        if let Some(expected) = expected {
            if report[source]["source_sha256"].as_str() != Some(expected) {
                return Err(EngineError::Other(
                    "revision overlay source differs from its expected SHA-256".into(),
                ));
            }
        }
    }
    let bytes = if args.format == "png" {
        preview.export_png(&cancel)?
    } else {
        serde_json::to_vec_pretty(&report).map_err(|error| EngineError::Other(error.to_string()))?
    };
    preview.revalidate_sources(&cancel)?;
    sunipdf_core::write_new_copy(&args.out, &bytes)?;
    Ok(
        serde_json::json!({"out":args.out,"format":args.format,"originals_unchanged":true,"report":report}),
    )
}
