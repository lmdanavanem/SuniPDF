use std::io::Read;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use sunipdf_core::{
    compress::{self, CompressionOptions},
    form_data, Engine, EngineError,
};

pub fn read_form_values(path: &Path) -> Result<serde_json::Value, EngineError> {
    let mut bytes = Vec::new();
    let stdin = path.as_os_str() == "-";
    let limit = form_data::IMPORT_LIMIT as u64 + 1;
    if stdin {
        std::io::stdin().take(limit).read_to_end(&mut bytes)?;
    } else {
        std::fs::File::open(path)?
            .take(limit)
            .read_to_end(&mut bytes)?;
    }
    let extension = if stdin {
        "json"
    } else {
        path.extension().and_then(|s| s.to_str()).unwrap_or("")
    };
    form_data::parse_values(&bytes, extension)
}

pub fn compress(
    engine: &Engine,
    file: &Path,
    out: &Path,
    password: Option<&str>,
    options: CompressionOptions,
) -> Result<serde_json::Value, EngineError> {
    options.validate()?;
    let source = std::fs::canonicalize(file)?;
    if out.exists() && std::fs::canonicalize(out)? == source {
        return Err(EngineError::Other(
            "image compression must be saved to a new copy; the input cannot be replaced".into(),
        ));
    }
    if out.exists() {
        return Err(EngineError::AlreadyExists(out.to_path_buf()));
    }
    let input = std::fs::read(&source)?;
    let original = engine.open_bytes(input.clone(), password, &source)?;
    let properties = original.properties();
    if !properties.can_modify {
        return Err(EngineError::SecurityRestricted);
    }
    let (bytes, report) = compress::compress_images(
        &input,
        password,
        options,
        &AtomicBool::new(false),
        |_, _| {},
    )?;
    if bytes != input {
        let reopened = engine.open_bytes(bytes.clone(), password, out)?;
        let result_properties = reopened.properties();
        if reopened.info().page_sizes != original.info().page_sizes
            || reopened.form_info()? != original.form_info()?
            || properties.encrypted != result_properties.encrypted
            || properties.can_print != result_properties.can_print
            || properties.can_copy != result_properties.can_copy
            || properties.can_modify != result_properties.can_modify
            || properties.can_annotate != result_properties.can_annotate
        {
            return Err(EngineError::Other(
                "compressed PDF did not preserve document structure or security".into(),
            ));
        }
        for page in 0..original.page_count() {
            if original.page_text(page)? != reopened.page_text(page)?
                || original.page_links(page)? != reopened.page_links(page)?
            {
                return Err(EngineError::Other(
                    "compressed PDF did not preserve text or links".into(),
                ));
            }
        }
    }
    sunipdf_core::write_new_copy(out, &bytes)?;
    Ok(
        serde_json::json!({"out":out,"before":report.before,"after":report.after,
        "images_changed":report.images_changed,"images_skipped":report.images_skipped,
        "saved_percent":if report.before>0 {100.0 * (1.0-report.after as f64/report.before as f64)} else {0.0}}),
    )
}
