//! Synthetic test outputs only, retained for independent qpdf/MuPDF release checks.

use std::path::{Path, PathBuf};
use sunipdf_core::Document;

pub struct Evidence {
    group: &'static str,
    directory: PathBuf,
    cases: Vec<serde_json::Value>,
}
impl Evidence {
    pub fn new(group: &'static str) -> Self {
        assert!(matches!(group, "rich" | "structure"));
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let directory = root.join("target/feature-030-evidence").join(group);
        std::fs::create_dir_all(&directory).unwrap();
        Self {
            group,
            directory,
            cases: Vec::new(),
        }
    }
    pub fn save(&mut self, name: &str, pdf: &Path, doc: &Document<'_>, password: Option<&str>) {
        assert!(
            !name.is_empty()
                && name.len() <= 64
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        );
        assert!(self.cases.len() < 12 && !self.cases.iter().any(|case| case["name"] == name));
        let bytes = std::fs::read(pdf).unwrap();
        assert!(bytes.starts_with(b"%PDF-") && bytes.len() <= 128 * 1024 * 1024);
        let rendered = doc.render_page(0, 1.0).unwrap();
        assert_eq!(
            rendered.rgba.len(),
            rendered.width as usize * rendered.height as usize * 4
        );
        let mut ppm = format!("P6\n{} {}\n255\n", rendered.width, rendered.height).into_bytes();
        for pixel in rendered.rgba.as_chunks::<4>().0 {
            ppm.extend_from_slice(&pixel[..3]);
        }
        sunipdf_core::write_atomic(&self.directory.join(format!("{name}.pdf")), &bytes).unwrap();
        sunipdf_core::write_atomic(&self.directory.join(format!("{name}.ppm")), &ppm).unwrap();
        let base = format!("target/feature-030-evidence/{}/{name}", self.group);
        self.cases.push(serde_json::json!({"name":name,"pdf":format!("{base}.pdf"),"password":password,"readable":true,"baseline_ppm":format!("{base}.ppm"),"page":1,"scale":1}));
    }
    pub fn finish(self) {
        assert!(!self.cases.is_empty());
        let manifest = serde_json::json!({"schema":1,"cases":self.cases});
        sunipdf_core::write_atomic(
            &self.directory.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }
}
