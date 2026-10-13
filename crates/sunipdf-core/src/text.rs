//! Extraction et recherche de texte (PDFium `FPDFText_*`).

use crate::{Document, EngineError};
use pdfium_render::prelude::*;

/// Rectangle en points PDF, origine en bas à gauche de la page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointRect {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextMatch {
    pub page: usize,
    /// Un rectangle par segment de ligne couvert par l'occurrence.
    pub rects: Vec<PointRect>,
}

impl Document<'_> {
    /// Texte complet d'une page, dans l'ordre fourni par PDFium.
    pub fn page_text(&self, page_index: usize) -> Result<String, EngineError> {
        let page = self.page(page_index)?;
        let text = page.text()?.all();
        Ok(text)
    }

    /// Occurrences de `query` sur une page. Requête vide → aucune occurrence.
    pub fn search_page(
        &self,
        page_index: usize,
        query: &str,
        match_case: bool,
    ) -> Result<Vec<TextMatch>, EngineError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let page = self.page(page_index)?;
        let text = page.text()?;
        let options = PdfSearchOptions::new().match_case(match_case);
        let search = text.search(query, &options)?;
        let mut matches = Vec::new();
        while let Some(segments) = search.find_next() {
            let rects = segments
                .iter()
                .map(|s| {
                    let b = s.bounds();
                    PointRect {
                        left: b.left().value,
                        bottom: b.bottom().value,
                        right: b.right().value,
                        top: b.top().value,
                    }
                })
                .collect();
            matches.push(TextMatch {
                page: page_index,
                rects,
            });
        }
        Ok(matches)
    }

    /// Occurrences sur tout le document, dans l'ordre des pages.
    pub fn search(&self, query: &str, match_case: bool) -> Result<Vec<TextMatch>, EngineError> {
        let mut all = Vec::new();
        for page in 0..self.page_count() {
            all.extend(self.search_page(page, query, match_case)?);
        }
        Ok(all)
    }

    fn page(&self, page_index: usize) -> Result<PdfPage<'_>, EngineError> {
        let index = PdfPageIndex::try_from(page_index)
            .map_err(|_| EngineError::PageOutOfRange(page_index))?;
        if page_index >= self.page_count() {
            return Err(EngineError::PageOutOfRange(page_index));
        }
        Ok(self.inner.pages().get(index)?)
    }
}
