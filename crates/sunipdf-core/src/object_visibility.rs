//! Temporary native visibility for reader rendering. Never regenerate or save hidden content.

use crate::EngineError;
use pdfium_render::prelude::*;

pub(crate) struct HideGuard<'a> {
    objects: Vec<PdfPageObject<'a>>,
}

impl<'a> HideGuard<'a> {
    pub(crate) fn new(page: &'a PdfPage<'a>, hidden: &[usize]) -> Result<Self, EngineError> {
        if hidden.is_empty() {
            return Ok(Self {
                objects: Vec::new(),
            });
        }
        let count = page.objects().len();
        if hidden.len() > count || hidden.iter().any(|&index| index >= count) {
            return Err(EngineError::Other(
                "hidden object index is out of range".into(),
            ));
        }
        let mut indices = hidden.to_vec();
        indices.sort_unstable();
        indices.dedup();
        let mut guard = Self {
            objects: Vec::new(),
        };
        for index in indices {
            let mut object = page.objects().get(index as PdfPageObjectIndex)?;
            if object.is_active()? {
                object.set_inactive()?;
                guard.objects.push(object);
            }
        }
        Ok(guard)
    }

    pub(crate) fn restore(&mut self) -> Result<(), EngineError> {
        for object in &mut self.objects {
            object.set_active()?;
        }
        self.objects.clear();
        Ok(())
    }
}

impl Drop for HideGuard<'_> {
    fn drop(&mut self) {
        for object in &mut self.objects {
            if let Err(error) = object.set_active() {
                log::error!("Unable to restore reader object visibility: {error}");
            }
        }
    }
}
