//! Bounded outline editing; existing actions remain inert and survive rename/reorder.

use crate::{Document, EngineError};
use lopdf::{Dictionary, Object, ObjectId};
use std::collections::HashSet;

pub const MAX_BOOKMARKS: usize = 4096;
pub const MAX_DEPTH: usize = 32;
const MAX_TITLE_BYTES: usize = 1024;

#[derive(Debug, Clone)]
pub enum BookmarkEdit {
    Add {
        parent: Option<usize>,
        title: String,
        page: usize,
    },
    Rename {
        index: usize,
        title: String,
    },
    Destination {
        index: usize,
        page: usize,
    },
    Move {
        index: usize,
        parent: Option<usize>,
        position: usize,
    },
    Delete {
        index: usize,
    },
}
#[derive(Clone)]
pub(crate) struct Node {
    pub dict: Dictionary,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}
#[derive(Default)]
pub(crate) struct Tree {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
}
pub(crate) fn invalid(reason: &str) -> EngineError {
    EngineError::Other(format!("Invalid bookmarks: {reason}"))
}
fn title(value: &str, existing: bool) -> Result<(), EngineError> {
    if (!existing && value.trim().is_empty())
        || value.len() > MAX_TITLE_BYTES
        || value.chars().any(|c| {
            let whitespace = matches!(c, '\t' | '\n' | '\r' | '\u{2028}' | '\u{2029}');
            (c.is_control() || whitespace) && !(existing && whitespace)
        })
    {
        return Err(invalid(
            "title must contain 1–1024 UTF-8 bytes without control characters",
        ));
    }
    Ok(())
}
fn reference(dict: &Dictionary, key: &[u8]) -> Result<Option<ObjectId>, EngineError> {
    match dict.get(key) {
        Ok(Object::Null) => Ok(None),
        Ok(object) => object
            .as_reference()
            .map(Some)
            .map_err(|_| invalid("outline links must be indirect references")),
        Err(_) => Ok(None),
    }
}
impl Tree {
    pub(crate) fn read(pdf: &lopdf::Document) -> Result<Self, EngineError> {
        let mut tree = Self::default();
        let catalog = pdf.catalog().map_err(crate::security::lopdf_err)?;
        let Some(root) = reference(catalog, b"Outlines")? else {
            return Ok(tree);
        };
        let dict = pdf
            .get_dictionary(root)
            .map_err(crate::security::lopdf_err)?;
        let mut seen = HashSet::new();
        tree.roots = tree.chain(
            pdf,
            root,
            (reference(dict, b"First")?, reference(dict, b"Last")?),
            None,
            0,
            &mut seen,
        )?;
        Ok(tree)
    }
    fn chain(
        &mut self,
        pdf: &lopdf::Document,
        parent_id: ObjectId,
        ends: (Option<ObjectId>, Option<ObjectId>),
        parent: Option<usize>,
        depth: usize,
        seen: &mut HashSet<ObjectId>,
    ) -> Result<Vec<usize>, EngineError> {
        let (first, last) = ends;
        if first.is_none() != last.is_none() {
            return Err(invalid("First/Last mismatch"));
        }
        if first.is_none() {
            return Ok(Vec::new());
        }
        if depth >= MAX_DEPTH {
            return Err(invalid("outline depth exceeds 32"));
        }
        let mut result = Vec::new();
        let mut next = first;
        let mut previous = None;
        while let Some(id) = next {
            if self.nodes.len() >= MAX_BOOKMARKS || !seen.insert(id) {
                return Err(invalid("outline cycle or 4096-node limit"));
            }
            let dict = pdf
                .get_dictionary(id)
                .map_err(crate::security::lopdf_err)?
                .clone();
            if reference(&dict, b"Parent")? != Some(parent_id)
                || reference(&dict, b"Prev")? != previous
            {
                return Err(invalid("inconsistent outline Parent/Prev links"));
            }
            let raw = dict.get(b"Title").map_err(crate::security::lopdf_err)?;
            let decoded = lopdf::decode_text_string(raw).map_err(crate::security::lopdf_err)?;
            title(&decoded, true)?;
            let index = self.nodes.len();
            self.nodes.push(Node {
                dict: dict.clone(),
                children: Vec::new(),
                parent,
            });
            let children = self.chain(
                pdf,
                id,
                (reference(&dict, b"First")?, reference(&dict, b"Last")?),
                Some(index),
                depth + 1,
                seen,
            )?;
            self.nodes[index].children = children;
            result.push(index);
            previous = Some(id);
            next = reference(&dict, b"Next")?;
        }
        if previous != last {
            return Err(invalid("outline Last does not match sibling chain"));
        }
        Ok(result)
    }
    pub(crate) fn order(&self) -> Vec<usize> {
        fn walk(tree: &Tree, indices: &[usize], out: &mut Vec<usize>) {
            for &index in indices {
                out.push(index);
                walk(tree, &tree.nodes[index].children, out);
            }
        }
        let mut order = Vec::new();
        walk(self, &self.roots, &mut order);
        order
    }
    fn index(&self, flat: usize) -> Result<usize, EngineError> {
        self.order()
            .get(flat)
            .copied()
            .ok_or_else(|| invalid("bookmark index is out of range"))
    }
    fn siblings(&mut self, parent: Option<usize>) -> &mut Vec<usize> {
        match parent {
            Some(parent) => &mut self.nodes[parent].children,
            None => &mut self.roots,
        }
    }
    fn check_depth(&self) -> Result<(), EngineError> {
        for index in self.order() {
            let mut next = Some(index);
            let mut count = 0;
            let mut seen = HashSet::new();
            while let Some(i) = next {
                count += 1;
                if count > MAX_DEPTH || !seen.insert(i) {
                    return Err(invalid("move creates an outline cycle or exceeds depth 32"));
                }
                next = self.nodes[i].parent;
            }
        }
        Ok(())
    }
    pub(crate) fn write(&self, pdf: &mut lopdf::Document) -> Result<(), EngineError> {
        self.check_depth()?;
        if self.roots.is_empty() {
            pdf.catalog_mut()
                .map_err(crate::security::lopdf_err)?
                .remove(b"Outlines");
            return Ok(());
        }
        let root = pdf.new_object_id();
        let ids: Vec<_> = (0..self.nodes.len()).map(|_| pdf.new_object_id()).collect();
        fn emit(
            tree: &Tree,
            pdf: &mut lopdf::Document,
            indices: &[usize],
            parent: ObjectId,
            ids: &[ObjectId],
        ) -> usize {
            let mut visible = 0;
            for (position, &index) in indices.iter().enumerate() {
                let node = &tree.nodes[index];
                let mut dict = node.dict.clone();
                for key in [
                    b"Parent".as_slice(),
                    b"First",
                    b"Last",
                    b"Prev",
                    b"Next",
                    b"Count",
                ] {
                    dict.remove(key);
                }
                dict.set("Parent", parent);
                if position > 0 {
                    dict.set("Prev", ids[indices[position - 1]]);
                }
                if position + 1 < indices.len() {
                    dict.set("Next", ids[indices[position + 1]]);
                }
                if let (Some(first), Some(last)) = (node.children.first(), node.children.last()) {
                    dict.set("First", ids[*first]);
                    dict.set("Last", ids[*last]);
                    let count = emit(tree, pdf, &node.children, ids[index], ids);
                    let collapsed = node
                        .dict
                        .get(b"Count")
                        .ok()
                        .and_then(|v| v.as_i64().ok())
                        .is_some_and(|n| n < 0);
                    dict.set(
                        "Count",
                        if collapsed {
                            -(count as i64)
                        } else {
                            count as i64
                        },
                    );
                    if !collapsed {
                        visible += count;
                    }
                }
                pdf.objects.insert(ids[index], Object::Dictionary(dict));
                visible += 1;
            }
            visible
        }
        let count = emit(self, pdf, &self.roots, root, &ids);
        let mut dict = Dictionary::new();
        dict.set("Type", "Outlines");
        dict.set("First", ids[self.roots[0]]);
        dict.set("Last", ids[*self.roots.last().unwrap()]);
        dict.set("Count", count as i64);
        pdf.objects.insert(root, Object::Dictionary(dict));
        pdf.catalog_mut()
            .map_err(crate::security::lopdf_err)?
            .set("Outlines", root);
        Ok(())
    }
}

pub fn edit(
    input: &[u8],
    password: Option<&str>,
    edit: &BookmarkEdit,
) -> Result<Vec<u8>, EngineError> {
    if input.len() > 128 * 1024 * 1024 {
        return Err(invalid("input exceeds 128 MiB"));
    }
    let mut pdf = crate::security::load(input, password)?;
    let pages: Vec<_> = pdf.get_pages().into_values().collect();
    let mut tree = Tree::read(&pdf)?;
    let destination = |page: usize| -> Result<Object, EngineError> {
        let id = pages.get(page).ok_or(EngineError::PageOutOfRange(page))?;
        Ok(Object::Array(vec![
            Object::Reference(*id),
            Object::Name(b"Fit".to_vec()),
        ]))
    };
    match edit {
        BookmarkEdit::Add {
            parent,
            title: label,
            page,
        } => {
            title(label, false)?;
            if tree.nodes.len() >= MAX_BOOKMARKS {
                return Err(invalid("4096-node limit"));
            }
            let parent = parent.map(|i| tree.index(i)).transpose()?;
            let mut dict = Dictionary::new();
            dict.set("Title", lopdf::text_string(label));
            dict.set("Dest", destination(*page)?);
            let index = tree.nodes.len();
            tree.nodes.push(Node {
                dict,
                children: Vec::new(),
                parent,
            });
            tree.siblings(parent).push(index);
        }
        BookmarkEdit::Rename {
            index,
            title: label,
        } => {
            title(label, false)?;
            let index = tree.index(*index)?;
            tree.nodes[index]
                .dict
                .set("Title", lopdf::text_string(label));
        }
        BookmarkEdit::Destination { index, page } => {
            let index = tree.index(*index)?;
            tree.nodes[index].dict.remove(b"A");
            tree.nodes[index].dict.set("Dest", destination(*page)?);
        }
        BookmarkEdit::Delete { index } => {
            let index = tree.index(*index)?;
            let parent = tree.nodes[index].parent;
            tree.siblings(parent).retain(|i| *i != index);
        }
        BookmarkEdit::Move {
            index,
            parent,
            position,
        } => {
            let index = tree.index(*index)?;
            let parent = parent.map(|i| tree.index(i)).transpose()?;
            let mut ancestor = parent;
            while let Some(i) = ancestor {
                if i == index {
                    return Err(invalid("a bookmark cannot move under its own descendant"));
                }
                ancestor = tree.nodes[i].parent;
            }
            let old = tree.nodes[index].parent;
            tree.siblings(old).retain(|i| *i != index);
            if *position > tree.siblings(parent).len() {
                return Err(invalid("sibling position is out of range"));
            }
            tree.nodes[index].parent = parent;
            tree.siblings(parent).insert(*position, index);
        }
    }
    tree.write(&mut pdf)?;
    pdf.prune_objects();
    crate::security::save(pdf)
}

impl Document<'_> {
    pub fn edit_bookmark(&mut self, edit: &BookmarkEdit) -> Result<(), EngineError> {
        if !self.properties().can_modify {
            return Err(EngineError::SecurityRestricted);
        }
        let bytes = crate::bookmarks::edit(&self.snapshot()?, self.password(), edit)?;
        self.restore(bytes)
    }
}

#[cfg(test)]
#[path = "bookmarks_tests.rs"]
mod tests;
