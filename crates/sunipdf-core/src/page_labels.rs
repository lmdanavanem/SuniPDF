//! Page-label authoring. Ranges refer to physical page indices, not page contents.

use crate::{Document, EngineError};
use lopdf::{Dictionary, Object, ObjectId};
use std::collections::HashSet;

const MAX_LABEL_BYTES: usize = 4096;
const MAX_PREFIX_BYTES: usize = 256;
const MAX_TREE_DEPTH: usize = 32;
const MAX_TREE_NODES: usize = 131_072;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLabelStyle {
    Decimal,
    RomanUpper,
    RomanLower,
    LettersUpper,
    LettersLower,
    PrefixOnly,
}

impl PageLabelStyle {
    fn name(self) -> Option<&'static str> {
        match self {
            Self::Decimal => Some("D"),
            Self::RomanUpper => Some("R"),
            Self::RomanLower => Some("r"),
            Self::LettersUpper => Some("A"),
            Self::LettersLower => Some("a"),
            Self::PrefixOnly => None,
        }
    }

    fn number_len(self, number: u32) -> usize {
        match self {
            Self::Decimal => number.ilog10() as usize + 1,
            Self::RomanUpper | Self::RomanLower => {
                let digits = [0, 1, 2, 3, 2, 1, 2, 3, 4, 2];
                (number / 1000) as usize
                    + digits[(number / 100 % 10) as usize]
                    + digits[(number / 10 % 10) as usize]
                    + digits[(number % 10) as usize]
            }
            Self::LettersUpper | Self::LettersLower => ((number - 1) / 26 + 1) as usize,
            Self::PrefixOnly => 0,
        }
    }
}

/// Inclusive, zero-based physical page range. `start` is a positive label number, not a page index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLabelRange {
    pub first: usize,
    pub last: usize,
    pub style: PageLabelStyle,
    pub prefix: String,
    pub start: u32,
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("invalid page labels: {reason}"))
}

fn valid_prefix(prefix: &str, limit: usize) -> bool {
    prefix.len() <= limit
        && !prefix
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
}

fn last_number(start: u32, length: usize) -> Result<u32, EngineError> {
    start
        .checked_add(
            u32::try_from(length.saturating_sub(1)).map_err(|_| invalid("number overflow"))?,
        )
        .filter(|n| start != 0 && *n <= i32::MAX as u32)
        .ok_or_else(|| invalid("number overflow or nonpositive start"))
}

fn check_length(
    style: PageLabelStyle,
    prefix_len: usize,
    start: u32,
    length: usize,
) -> Result<(), EngineError> {
    let end = last_number(
        start,
        if style == PageLabelStyle::PrefixOnly {
            1
        } else {
            length
        },
    )?;
    if prefix_len.saturating_add(style.number_len(end)) > MAX_LABEL_BYTES {
        return Err(invalid("generated label exceeds 4096 UTF-8 bytes"));
    }
    // Roman lengths are not monotonic; count glyphs for this bounded page range without allocating labels.
    let number_len = if matches!(
        style,
        PageLabelStyle::RomanUpper | PageLabelStyle::RomanLower
    ) {
        (start..=end)
            .map(|n| style.number_len(n))
            .max()
            .unwrap_or(0)
    } else {
        style.number_len(end)
    };
    if prefix_len.saturating_add(number_len) > MAX_LABEL_BYTES {
        return Err(invalid("generated label exceeds 4096 UTF-8 bytes"));
    }
    Ok(())
}

impl PageLabelRange {
    /// Validates before loading any newly authored labels into PDFium.
    pub fn validate(&self, page_count: usize) -> Result<(), EngineError> {
        if self.first > self.last {
            return Err(invalid("range is reversed"));
        }
        if self.first >= page_count {
            return Err(EngineError::PageOutOfRange(self.first));
        }
        if self.last >= page_count {
            return Err(EngineError::PageOutOfRange(self.last));
        }
        if !valid_prefix(&self.prefix, MAX_PREFIX_BYTES)
            || (self.style == PageLabelStyle::PrefixOnly && self.prefix.trim().is_empty())
        {
            return Err(invalid(
                "prefix must be one line, at most 256 UTF-8 bytes, and nonblank without numbering",
            ));
        }
        check_length(
            self.style,
            self.prefix.len(),
            self.start,
            self.last - self.first + 1,
        )
    }

    fn dictionary(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        if let Some(name) = self.style.name() {
            dict.set("S", Object::Name(name.as_bytes().to_vec()));
            dict.set("St", self.start as i64);
        }
        if !self.prefix.is_empty() {
            dict.set("P", lopdf::text_string(&self.prefix));
        }
        dict
    }
}

fn entry_values(
    pdf: &lopdf::Document,
    dict: &Dictionary,
) -> Result<(PageLabelStyle, usize, u32), EngineError> {
    let style = match dict.get(b"S") {
        Ok(value) => match pdf
            .dereference(value)
            .map_err(|_| invalid("style reference"))?
            .1
            .as_name()
        {
            Ok(b"D") => PageLabelStyle::Decimal,
            Ok(b"R") => PageLabelStyle::RomanUpper,
            Ok(b"r") => PageLabelStyle::RomanLower,
            Ok(b"A") => PageLabelStyle::LettersUpper,
            Ok(b"a") => PageLabelStyle::LettersLower,
            _ => return Err(invalid("unsupported numbering style")),
        },
        Err(_) => PageLabelStyle::PrefixOnly,
    };
    let prefix = match dict.get(b"P") {
        Ok(value) => {
            let value = pdf
                .dereference(value)
                .map_err(|_| invalid("prefix reference"))?
                .1;
            if value
                .as_str()
                .map_err(|_| invalid("prefix is not a text string"))?
                .len()
                > MAX_LABEL_BYTES * 2 + 4
            {
                return Err(invalid("encoded existing prefix is too long"));
            }
            let text = lopdf::decode_text_string(value).map_err(|_| invalid("prefix encoding"))?;
            if !valid_prefix(&text, MAX_LABEL_BYTES) {
                return Err(invalid("existing prefix is unsafe or too long"));
            }
            text.len()
        }
        Err(_) => 0,
    };
    let start = match dict.get(b"St") {
        Ok(value) => pdf
            .dereference(value)
            .map_err(|_| invalid("start reference"))?
            .1
            .as_i64()
            .ok()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n > 0 && *n <= i32::MAX as u32)
            .ok_or_else(|| invalid("existing start is not a positive supported integer"))?,
        Err(_) => 1,
    };
    Ok((style, prefix, start))
}

fn collect_ranges(
    pdf: &lopdf::Document,
    object: &Object,
    page_count: usize,
    depth: usize,
    seen: &mut HashSet<ObjectId>,
    nodes: &mut usize,
    ranges: &mut Vec<(usize, Dictionary)>,
) -> Result<(), EngineError> {
    *nodes += 1;
    if depth > MAX_TREE_DEPTH || *nodes > MAX_TREE_NODES {
        return Err(invalid("number tree exceeds traversal limits"));
    }
    let (id, object) = pdf
        .dereference(object)
        .map_err(|_| invalid("number-tree reference"))?;
    if id.is_some_and(|id| !seen.insert(id)) {
        return Err(invalid("cyclic or repeated number-tree node"));
    }
    let dict = object
        .as_dict()
        .map_err(|_| invalid("number-tree node is not a dictionary"))?;
    let begin = ranges.len();
    match (dict.get(b"Nums").ok(), dict.get(b"Kids").ok()) {
        (Some(nums), None) => {
            let nums = pdf
                .dereference(nums)
                .map_err(|_| invalid("Nums reference"))?
                .1
                .as_array()
                .map_err(|_| invalid("Nums is not an array"))?;
            if nums.is_empty() || nums.len() % 2 != 0 || nums.len() / 2 > page_count {
                return Err(invalid("Nums must contain bounded key/dictionary pairs"));
            }
            for pair in nums.chunks(2) {
                let key = pair[0]
                    .as_i64()
                    .ok()
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|n| *n < page_count)
                    .ok_or_else(|| invalid("page-label key out of range"))?;
                if ranges.last().is_some_and(|(previous, _)| *previous >= key) {
                    return Err(invalid("duplicate or unordered page-label keys"));
                }
                let value = pdf
                    .dereference(&pair[1])
                    .map_err(|_| invalid("label dictionary reference"))?
                    .1
                    .as_dict()
                    .map_err(|_| invalid("page label is not a dictionary"))?;
                entry_values(pdf, value)?;
                ranges.push((key, value.clone()));
                if ranges.len() > page_count {
                    return Err(invalid("too many page-label ranges"));
                }
            }
        }
        (None, Some(kids)) => {
            let kids = pdf
                .dereference(kids)
                .map_err(|_| invalid("Kids reference"))?
                .1
                .as_array()
                .map_err(|_| invalid("Kids is not an array"))?;
            if kids.is_empty() || kids.len() > MAX_TREE_NODES {
                return Err(invalid("Kids is empty or too large"));
            }
            for child in kids {
                collect_ranges(pdf, child, page_count, depth + 1, seen, nodes, ranges)?;
            }
        }
        _ => return Err(invalid("number-tree node must have either Kids or Nums")),
    }
    if let Ok(limits) = dict.get(b"Limits") {
        let limits = pdf
            .dereference(limits)
            .map_err(|_| invalid("Limits reference"))?
            .1
            .as_array()
            .map_err(|_| invalid("Limits is not an array"))?;
        let key = |index: usize| {
            limits
                .get(index)
                .and_then(|v| v.as_i64().ok())
                .and_then(|n| usize::try_from(n).ok())
        };
        if limits.len() != 2
            || key(0) != ranges.get(begin).map(|r| r.0)
            || key(1) != ranges.last().map(|r| r.0)
        {
            return Err(invalid("number-tree Limits do not match its keys"));
        }
    }
    Ok(())
}

impl Document<'_> {
    /// Replaces only the inclusive physical range. Existing label rules outside it remain effective.
    pub fn set_page_labels(&mut self, range: &PageLabelRange) -> Result<(), EngineError> {
        range.validate(self.page_count())?;
        let mut pdf = crate::security::load(&self.snapshot()?, self.password())?;
        let original = pdf
            .catalog()
            .map_err(crate::security::lopdf_err)?
            .get(b"PageLabels")
            .ok()
            .cloned();
        let mut ranges = Vec::new();
        let mut tree = Dictionary::new();
        if let Some(original) = original {
            collect_ranges(
                &pdf,
                &original,
                self.page_count(),
                0,
                &mut HashSet::new(),
                &mut 0,
                &mut ranges,
            )?;
            if ranges.first().map(|r| r.0) != Some(0) {
                return Err(invalid("number tree must start at physical page zero"));
            }
            for (index, &(first, ref dict)) in ranges.iter().enumerate() {
                let last = ranges.get(index + 1).map_or(self.page_count(), |r| r.0);
                let (style, prefix_len, start) = entry_values(&pdf, dict)?;
                check_length(style, prefix_len, start, last - first)?;
            }
            tree = pdf
                .dereference(&original)
                .map_err(|_| invalid("number-tree root reference"))?
                .1
                .as_dict()
                .map_err(|_| invalid("number-tree root is not a dictionary"))?
                .clone();
        } else {
            let mut physical = Dictionary::new();
            physical.set("S", "D");
            ranges.push((0, physical));
        }
        let after = range.last + 1;
        let restore = if after < self.page_count() && !ranges.iter().any(|r| r.0 == after) {
            let (first, original) = ranges
                .iter()
                .rev()
                .find(|r| r.0 < after)
                .ok_or_else(|| invalid("missing preceding range"))?;
            let mut original = original.clone();
            let (style, _, start) = entry_values(&pdf, &original)?;
            if style != PageLabelStyle::PrefixOnly {
                original.set("St", last_number(start, after - first + 1)? as i64);
            }
            Some((after, original))
        } else {
            None
        };
        ranges.retain(|(first, _)| *first < range.first || *first > range.last);
        ranges.push((range.first, range.dictionary()));
        if let Some(restore) = restore {
            ranges.push(restore);
        }
        ranges.sort_unstable_by_key(|r| r.0);
        tree.remove(b"Kids");
        tree.remove(b"Limits");
        tree.set(
            "Nums",
            ranges
                .into_iter()
                .flat_map(|(first, dict)| [Object::Integer(first as i64), Object::Dictionary(dict)])
                .collect::<Vec<_>>(),
        );
        let tree = pdf.add_object(tree);
        pdf.catalog_mut()
            .map_err(crate::security::lopdf_err)?
            .set("PageLabels", tree);
        self.commit_page_labels(crate::security::save(pdf)?)
    }

    /// Removes all label rules, including malformed trees. Physical page numbering becomes the fallback.
    /// This is not secure erasure of old metadata objects.
    pub fn reset_page_labels(&mut self) -> Result<(), EngineError> {
        let mut pdf = crate::security::load(&self.snapshot()?, self.password())?;
        let catalog = pdf.catalog_mut().map_err(crate::security::lopdf_err)?;
        if catalog.remove(b"PageLabels").is_none() {
            return Ok(());
        }
        self.commit_page_labels(crate::security::save(pdf)?)
    }

    fn commit_page_labels(&mut self, bytes: Vec<u8>) -> Result<(), EngineError> {
        let mut candidate = Document {
            inner: self.pdfium.load_pdf_from_byte_vec(bytes, self.password())?,
            pdfium: self.pdfium,
            info: self.info.clone(),
            page_rotations: self.page_rotations.clone(),
            password: self.password.clone(),
            font_cache: Default::default(),
        };
        candidate.refresh_page_sizes()?;
        self.inner = candidate.inner;
        self.info = candidate.info;
        self.page_rotations = candidate.page_rotations;
        self.font_cache.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    #[test]
    fn range_validation_bounds_input_and_generated_label_lengths() {
        let mut range = PageLabelRange {
            first: 0,
            last: 1,
            style: PageLabelStyle::Decimal,
            prefix: String::new(),
            start: 1,
        };
        assert!(range.validate(2).is_ok());
        for (first, last, count, start) in [
            (1, 0, 2, 1),
            (0, 2, 2, 1),
            (0, 0, 0, 1),
            (0, 1, 2, 0),
            (0, 1, 2, i32::MAX as u32),
            (0, 0, 1, u32::MAX),
        ] {
            let invalid = PageLabelRange {
                first,
                last,
                start,
                ..range.clone()
            };
            assert!(invalid.validate(count).is_err());
        }
        for prefix in [
            "bad\nlabel".to_owned(),
            "bad\u{2028}label".to_owned(),
            "x".repeat(257),
            "É".repeat(129),
        ] {
            range.prefix = prefix;
            assert!(range.validate(2).is_err());
        }
        range.prefix = "É".repeat(128);
        assert!(range.validate(2).is_ok());
        range.style = PageLabelStyle::PrefixOnly;
        range.prefix = " ".into();
        assert!(range.validate(2).is_err());
        range.prefix = "Annexe".into();
        range.start = i32::MAX as u32;
        assert!(range.validate(2).is_ok());
        range.prefix.clear();
        range.last = 0;
        range.style = PageLabelStyle::LettersUpper;
        range.start = 26 * 4096;
        assert!(range.validate(1).is_ok());
        range.start += 1;
        assert!(range.validate(1).is_err());
        assert_eq!(PageLabelStyle::LettersUpper.number_len(27), 2);
        range.style = PageLabelStyle::RomanUpper;
        range.start = 4_096_000;
        assert!(range.validate(1).is_ok());
        range.start += 1;
        assert!(range.validate(1).is_err());
        // A range endpoint can have a shorter Roman representation than an earlier page.
        assert!(check_length(PageLabelStyle::RomanUpper, 4093, 8, 2).is_err());
        assert!(check_length(PageLabelStyle::RomanUpper, 4095, 1, 1).is_ok());
    }

    #[test]
    fn number_tree_validation_refuses_malformed_and_unbounded_structures() {
        let mut pdf = lopdf::Document::with_version("1.7");
        let pair = || vec![0.into(), dictionary! {"S" => "D"}.into()];
        let mut bad = vec![
            Object::Null,
            dictionary! {"Nums" => vec![Object::Integer(0)]}.into(),
            dictionary! {"Nums" => Vec::<Object>::new()}.into(),
            dictionary! {"Nums" => vec![0.into(), dictionary! {"S"=>"D"}.into(), 0.into(), dictionary! {"S"=>"D"}.into()]}.into(),
            dictionary! {"Nums" => vec![1.into(), dictionary! {"S"=>"D"}.into(), 0.into(), dictionary! {"S"=>"D"}.into()]}.into(),
            dictionary! {"Nums" => vec![9.into(), dictionary! {"S"=>"D"}.into()]}.into(),
            dictionary! {"Nums" => vec![0.into(), dictionary! {"S"=>"Unknown"}.into()]}.into(),
            dictionary! {"Nums" => vec![0.into(), dictionary! {"St"=>0}.into()]}.into(),
            dictionary! {"Nums" => vec![0.into(), dictionary! {"P"=>5}.into()]}.into(),
            dictionary! {"Nums"=>pair(), "Kids"=>Vec::<Object>::new()}.into(),
            dictionary! {"Kids"=>Vec::<Object>::new()}.into(),
            dictionary! {"Kids"=>Object::Integer(3)}.into(),
            dictionary! {"Nums"=>pair(), "Limits"=>vec![0.into(), 1.into()]}.into(),
        ];
        let cycle = pdf.new_object_id();
        pdf.objects.insert(
            cycle,
            dictionary! {"Kids"=>vec![Object::Reference(cycle)]}.into(),
        );
        bad.push(cycle.into());
        let mut deep = Object::Dictionary(dictionary! {"Nums"=>pair()});
        for _ in 0..MAX_TREE_DEPTH + 1 {
            deep = dictionary! {"Kids"=>vec![deep]}.into();
        }
        bad.push(deep);
        for object in bad {
            assert!(
                collect_ranges(
                    &pdf,
                    &object,
                    9,
                    0,
                    &mut HashSet::new(),
                    &mut 0,
                    &mut Vec::new()
                )
                .is_err(),
                "{object:?}"
            );
        }
        let valid = dictionary! {"Nums"=>pair(), "Limits"=>vec![0.into(),0.into()]}.into();
        assert!(collect_ranges(
            &pdf,
            &valid,
            9,
            0,
            &mut HashSet::new(),
            &mut 0,
            &mut Vec::new()
        )
        .is_ok());
    }
}
