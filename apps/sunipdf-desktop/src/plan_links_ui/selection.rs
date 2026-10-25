use super::{core, Context};
use std::collections::{BTreeSet, HashSet};
use sunipdf_core::plan_set::PlanSheet;
use sunipdf_i18n::{tr, Language};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectionError {
    Source,
    Pages,
    PageLimit,
    Targets,
    Sources,
    SourceLimit,
    TotalPages,
    IndexedPages,
}

impl SelectionError {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Source => "plan_links.selection.source",
            Self::Pages => "plan_links.selection.pages",
            Self::PageLimit => "plan_links.selection.page_limit",
            Self::Targets => "plan_links.selection.targets",
            Self::Sources => "plan_links.selection.sources",
            Self::SourceLimit => "plan_links.selection.source_limit",
            Self::TotalPages => "plan_links.selection.total_pages",
            Self::IndexedPages => "plan_links.selection.indexed_pages",
        }
    }
}

/// Physical page numbers remain one-based throughout the plan-link workflow.
pub(super) fn parse_pages(value: &str, page_count: usize) -> Result<Vec<usize>, SelectionError> {
    if value.is_empty() || value.len() > 4096 || page_count == 0 {
        return Err(SelectionError::Pages);
    }
    fn number(value: &str) -> Result<usize, SelectionError> {
        let value = value.trim();
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(SelectionError::Pages);
        }
        value.parse().map_err(|_| SelectionError::Pages)
    }
    let mut pages = Vec::new();
    let mut seen = HashSet::new();
    for item in value.split(',') {
        let mut limits = item.split('-');
        let first = number(limits.next().ok_or(SelectionError::Pages)?)?;
        let last = limits.next().map(number).transpose()?.unwrap_or(first);
        if limits.next().is_some() || first == 0 || last < first || last > page_count {
            return Err(SelectionError::Pages);
        }
        // Bound the range before expanding, including deliberately enormous endpoints.
        let count = last
            .checked_sub(first)
            .and_then(|n| n.checked_add(1))
            .ok_or(SelectionError::PageLimit)?;
        if count > core::MAX_SCAN_PAGES.saturating_sub(pages.len()) {
            return Err(SelectionError::PageLimit);
        }
        for page in first..=last {
            if !seen.insert(page) {
                return Err(SelectionError::Pages);
            }
            pages.push(page);
        }
    }
    Ok(pages)
}

pub(super) fn indexed_pages(context: &Context, file_id: &str) -> Result<String, SelectionError> {
    let file = context
        .set
        .files
        .iter()
        .find(|file| file.id == file_id)
        .ok_or(SelectionError::Source)?;
    let mut pages = BTreeSet::new();
    for sheet in context
        .set
        .sheets
        .iter()
        .filter(|sheet| sheet.file_id == file_id)
    {
        if sheet.page == 0 || sheet.page > file.page_count {
            return Err(SelectionError::Pages);
        }
        pages.insert(sheet.page);
        if pages.len() > core::MAX_SCAN_PAGES {
            return Err(SelectionError::PageLimit);
        }
    }
    if pages.is_empty() {
        return Err(SelectionError::IndexedPages);
    }
    Ok(pages
        .into_iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(","))
}

pub(super) fn target_matches(sheet: &PlanSheet, source: &str, query: &str) -> bool {
    if sheet.file_id == source {
        return false;
    }
    query.is_empty()
        || [
            &sheet.number,
            &sheet.title,
            &sheet.discipline,
            &sheet.revision,
        ]
        .iter()
        .any(|value| value.to_lowercase().contains(query))
}

pub(super) fn select_visible_targets(
    targets: &mut HashSet<String>,
    context: &Context,
    source: &str,
    query: &str,
    selected: bool,
) {
    let query = query.trim().to_lowercase();
    for sheet in context
        .set
        .sheets
        .iter()
        .filter(|sheet| target_matches(sheet, source, &query))
    {
        if selected {
            targets.insert(sheet.id.clone());
        } else {
            targets.remove(&sheet.id);
        }
    }
}

pub(super) fn target_controls(
    ui: &mut egui::Ui,
    context: &Context,
    source: &str,
    query: &mut String,
    targets: &mut HashSet<String>,
    lang: Language,
) -> bool {
    let mut changed = false;
    let label = ui.label(tr(lang, "plan_links.target_filter"));
    ui.add(
        egui::TextEdit::singleline(query)
            .char_limit(256)
            .desired_width(320.0)
            .hint_text(tr(lang, "plan_links.target_filter_hint")),
    )
    .labelled_by(label.id);
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr(lang, "plan_links.select_visible_targets"))
            .clicked()
        {
            select_visible_targets(targets, context, source, query, true);
            changed = true;
        }
        if ui
            .button(tr(lang, "plan_links.clear_visible_targets"))
            .clicked()
        {
            select_visible_targets(targets, context, source, query, false);
            changed = true;
        }
    });
    let normalized_query = query.trim().to_lowercase();
    egui::ScrollArea::vertical()
        .id_salt(("plan-links-targets", source))
        .max_height(135.0)
        .show(ui, |ui| {
            let mut found = false;
            for sheet in context
                .set
                .sheets
                .iter()
                .filter(|sheet| target_matches(sheet, source, &normalized_query))
            {
                found = true;
                let mut selected = targets.contains(&sheet.id);
                if ui
                    .checkbox(
                        &mut selected,
                        format!(
                            "{} · {} · {} · {}",
                            sheet.number, sheet.title, sheet.discipline, sheet.revision
                        ),
                    )
                    .changed()
                {
                    if selected {
                        targets.insert(sheet.id.clone());
                    } else {
                        targets.remove(&sheet.id);
                    }
                    changed = true;
                }
            }
            if !found {
                ui.small(tr(lang, "plan_links.no_visible_targets"));
            }
        });
    changed
}

#[cfg(test)]
mod tests {
    use super::super::tests::context;
    use super::*;

    #[test]
    fn physical_page_ranges_are_bounded_before_expansion() {
        assert_eq!(
            parse_pages(" 1 - 3, 8, 10-11 ", 11).unwrap(),
            vec![1, 2, 3, 8, 10, 11]
        );
        for value in [
            "",
            " ",
            "0",
            "3-2",
            "1-",
            "-2",
            "1-2-3",
            "1,,2",
            "1,",
            "+1",
            "1.0",
            "1,1",
            "1-3,3",
            "12",
            "1-12",
            "184467440737095516160",
        ] {
            assert!(parse_pages(value, 11).is_err(), "{value}");
        }
        assert_eq!(parse_pages("1-32", 100).unwrap().len(), 32);
        assert_eq!(parse_pages("1-33", 100), Err(SelectionError::PageLimit));
        assert_eq!(parse_pages("1-32,33", 100), Err(SelectionError::PageLimit));
        assert_eq!(
            parse_pages(&format!("1-{}", usize::MAX), usize::MAX),
            Err(SelectionError::PageLimit)
        );
    }

    #[test]
    fn indexed_selection_is_exact_sorted_deduplicated_and_never_truncated() {
        let mut context = context();
        context.set.files[0].page_count = 40;
        let template = context.set.sheets[0].clone();
        context.set.sheets.retain(|s| s.file_id != "source");
        for page in [8, 2, 8, 4] {
            let mut sheet = template.clone();
            sheet.page = page;
            context.set.sheets.push(sheet);
        }
        assert_eq!(indexed_pages(&context, "source").unwrap(), "2,4,8");
        assert_eq!(
            indexed_pages(&context, "missing"),
            Err(SelectionError::Source)
        );
        context.set.sheets.retain(|s| s.file_id != "source");
        assert_eq!(
            indexed_pages(&context, "source"),
            Err(SelectionError::IndexedPages)
        );
        for page in 1..=33 {
            let mut sheet = template.clone();
            sheet.page = page;
            context.set.sheets.push(sheet);
        }
        assert_eq!(
            indexed_pages(&context, "source"),
            Err(SelectionError::PageLimit)
        );
    }

    #[test]
    fn visible_target_actions_keep_hidden_selections_and_exclude_source_sheets() {
        let mut context = context();
        let mut other = context.set.sheets[1].clone();
        other.id = "structure".into();
        other.number = "S-20".into();
        other.discipline = "Structure".into();
        context.set.sheets.push(other);
        let mut targets = HashSet::new();
        select_visible_targets(&mut targets, &context, "source", "s-20", true);
        assert_eq!(targets, HashSet::from(["structure".to_owned()]));
        select_visible_targets(&mut targets, &context, "source", "Architecture", true);
        assert_eq!(targets.len(), 2);
        assert!(!targets.contains("source-sheet"));
        select_visible_targets(&mut targets, &context, "source", "A-10", false);
        assert_eq!(targets, HashSet::from(["structure".to_owned()]));
        assert!(target_matches(
            &context.set.sheets[1],
            "source",
            "floor.pdf"
        ));
        assert!(target_matches(&context.set.sheets[1], "source", "a"));
    }
}
