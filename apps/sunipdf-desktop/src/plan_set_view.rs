use std::cmp::Ordering;
use std::collections::HashMap;
use sunipdf_core::plan_set::{PlanSet, PlanSetReport, PlanSheet, ReferenceStatus};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Group {
    #[default]
    None,
    Discipline,
    Revision,
    File,
}

impl Group {
    pub const ALL: [Self; 4] = [Self::None, Self::Discipline, Self::Revision, Self::File];

    pub fn key(self) -> &'static str {
        match self {
            Self::None => "plan_sets.group_none",
            Self::Discipline => "plan_sets.discipline",
            Self::Revision => "plan_sets.revision",
            Self::File => "plan_sets.group_file",
        }
    }

    pub fn label<'a>(self, sheet: &'a PlanSheet, set: &'a PlanSet) -> &'a str {
        match self {
            Self::None => "",
            Self::Discipline => sheet.discipline.trim(),
            Self::Revision => sheet.revision.trim(),
            Self::File => set
                .files
                .iter()
                .find(|file| file.id == sheet.file_id)
                .map_or(sheet.file_id.as_str(), |file| file.path.as_str()),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Number,
    Title,
    Discipline,
    Revision,
    Page,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Options {
    pub query: String,
    pub group: Group,
    pub sort: Sort,
    pub descending: bool,
    pub issues_only: bool,
}

#[derive(Default)]
pub struct View {
    pub options: Options,
    cached_options: Options,
    cached_set: Option<PlanSet>,
    cached_status: Option<Vec<(String, ReferenceStatus)>>,
    indices: Vec<usize>,
}

impl View {
    /// Only sorting/filtering changes rebuild the index; opening a sheet still uses its stable ID.
    pub fn indices(&mut self, set: &PlanSet, report: Option<&PlanSetReport>) -> &[usize] {
        let same_status = match (&self.cached_status, report) {
            (None, None) => true,
            (Some(cached), Some(report)) => {
                cached.len() == report.sheets.len()
                    && cached
                        .iter()
                        .zip(&report.sheets)
                        .all(|((id, status), row)| id == &row.sheet_id && status == &row.status)
            }
            _ => false,
        };
        if self.cached_set.as_ref() != Some(set)
            || self.options != self.cached_options
            || !same_status
        {
            self.indices = filtered_indices(set, report, &self.options);
            self.cached_set = Some(set.clone());
            self.cached_options = self.options.clone();
            self.cached_status = report.map(|report| {
                report
                    .sheets
                    .iter()
                    .map(|row| (row.sheet_id.clone(), row.status))
                    .collect()
            });
        }
        &self.indices
    }

    pub fn toggle_sort(&mut self, sort: Sort) {
        self.options.descending = self.options.sort == sort && !self.options.descending;
        self.options.sort = sort;
    }
}

/// Digit runs compare by magnitude without parsing an integer or assuming revision chronology.
fn natural_cmp(left: &str, right: &str) -> Ordering {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let (mut a, mut b) = (0, 0);
    while a < left.len() && b < right.len() {
        if left[a].is_ascii_digit() && right[b].is_ascii_digit() {
            let (start_a, start_b) = (a, b);
            while a < left.len() && left[a].is_ascii_digit() {
                a += 1;
            }
            while b < right.len() && right[b].is_ascii_digit() {
                b += 1;
            }
            let mut significant_a = start_a;
            let mut significant_b = start_b;
            while significant_a + 1 < a && left[significant_a] == b'0' {
                significant_a += 1;
            }
            while significant_b + 1 < b && right[significant_b] == b'0' {
                significant_b += 1;
            }
            let order = (a - significant_a)
                .cmp(&(b - significant_b))
                .then_with(|| left[significant_a..a].cmp(&right[significant_b..b]))
                .then_with(|| (a - start_a).cmp(&(b - start_b)));
            if order != Ordering::Equal {
                return order;
            }
        } else {
            let order = left[a].cmp(&right[b]);
            if order != Ordering::Equal {
                return order;
            }
            a += 1;
            b += 1;
        }
    }
    (left.len() - a).cmp(&(right.len() - b))
}

fn filtered_indices(
    set: &PlanSet,
    report: Option<&PlanSetReport>,
    options: &Options,
) -> Vec<usize> {
    let statuses: HashMap<_, _> = report
        .into_iter()
        .flat_map(|r| &r.sheets)
        .map(|row| (row.sheet_id.as_str(), row.status))
        .collect();
    let query = options.query.to_lowercase();
    let terms: Vec<_> = query.split_whitespace().collect();
    let mut rows: Vec<_> = set
        .sheets
        .iter()
        .enumerate()
        .filter_map(|(index, sheet)| {
            if options.issues_only
                && statuses.get(sheet.id.as_str()) == Some(&ReferenceStatus::Ready)
            {
                return None;
            }
            let file = Group::File.label(sheet, set);
            if !terms.is_empty() {
                let text = format!(
                    "{} {} {} {} {} {}",
                    sheet.number, sheet.title, sheet.discipline, sheet.revision, file, sheet.page
                )
                .to_lowercase();
                if !terms.iter().all(|term| text.contains(term)) {
                    return None;
                }
            }
            let primary = match options.sort {
                Sort::Number => sheet.number.clone(),
                Sort::Title => sheet.title.clone(),
                Sort::Discipline => sheet.discipline.clone(),
                Sort::Revision => sheet.revision.clone(),
                Sort::Page => sheet.page.to_string(),
            }
            .to_lowercase();
            Some((
                index,
                options.group.label(sheet, set).to_lowercase(),
                primary,
                sheet.number.to_lowercase(),
            ))
        })
        .collect();
    rows.sort_by(|a, b| {
        let mut primary = natural_cmp(&a.2, &b.2);
        if options.descending {
            primary = primary.reverse();
        }
        natural_cmp(&a.1, &b.1)
            .then(primary)
            .then_with(|| natural_cmp(&a.3, &b.3))
            .then_with(|| set.sheets[a.0].page.cmp(&set.sheets[b.0].page))
            .then_with(|| set.sheets[a.0].id.cmp(&set.sheets[b.0].id))
    });
    rows.into_iter().map(|row| row.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunipdf_core::plan_set::{PlanFile, SheetReport};

    fn sample() -> PlanSet {
        PlanSet {
            version: 1,
            id: "set".into(),
            name: "Plans".into(),
            files: vec![PlanFile {
                id: "f".into(),
                path: "plans/Architecture.pdf".into(),
                sha256: "a".repeat(64),
                byte_len: 10,
                page_count: 3,
            }],
            sheets: [
                ("one", "A-10", "Structure", "B"),
                ("two", "A-2", "Architecture", "A"),
                ("three", "A-1", "Architecture", "B"),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (id, number, discipline, revision))| PlanSheet {
                id: id.into(),
                file_id: "f".into(),
                page: i + 1,
                number: number.into(),
                title: "Étage façade".into(),
                discipline: discipline.into(),
                revision: revision.into(),
            })
            .collect(),
        }
    }

    #[test]
    fn grouping_filtering_and_sorting_preserve_physical_identity_and_input_order() {
        let set = sample();
        let original = set.clone();
        let mut view = View::default();
        assert_eq!(view.indices(&set, None), &[2, 1, 0]);
        view.options.group = Group::Discipline;
        view.toggle_sort(Sort::Number);
        assert_eq!(view.indices(&set, None), &[1, 2, 0]);
        view.options.query = "ÉTAGE architecture.pdf A-2".into();
        assert_eq!(view.indices(&set, None), &[1]);
        assert_eq!(set.sheets[1].id, "two");
        assert_eq!(set.sheets[1].page, 2);
        assert_eq!(set, original);
        view.options.query = "absent".into();
        assert!(view.indices(&set, None).is_empty());
    }

    #[test]
    fn issue_filter_retains_unknown_missing_changed_and_duplicate_references_and_refreshes() {
        let set = sample();
        let mut view = View::default();
        view.options.issues_only = true;
        assert_eq!(view.indices(&set, None).len(), 3);
        let mut report = PlanSetReport {
            files: vec![],
            sheets: set
                .sheets
                .iter()
                .map(|s| SheetReport {
                    sheet_id: s.id.clone(),
                    file_id: s.file_id.clone(),
                    page: s.page,
                    path: None,
                    status: ReferenceStatus::Ready,
                    issues: vec![],
                })
                .collect(),
            cancelled: false,
        };
        assert!(view.indices(&set, Some(&report)).is_empty());
        for status in [
            ReferenceStatus::Missing,
            ReferenceStatus::Changed,
            ReferenceStatus::Duplicate,
        ] {
            report.sheets[1].status = status;
            assert_eq!(view.indices(&set, Some(&report)), &[1]);
        }
        let mut edited = set.clone();
        edited.sheets[1].number = "Z-99".into();
        view.options.query = "Z-99".into();
        assert_eq!(view.indices(&edited, Some(&report)), &[1]);
        assert!(view.indices(&set, Some(&report)).is_empty());
    }

    #[test]
    fn number_sort_handles_large_digit_runs_without_overflow() {
        assert_eq!(natural_cmp("a-2", "a-10"), Ordering::Less);
        assert_eq!(natural_cmp("a-2", "a-02"), Ordering::Less);
        assert_eq!(
            natural_cmp("a-999999999999999999999", "a-1000000000000000000000"),
            Ordering::Less
        );
        assert_eq!(natural_cmp("étage-2", "étage-10"), Ordering::Less);
    }
}
