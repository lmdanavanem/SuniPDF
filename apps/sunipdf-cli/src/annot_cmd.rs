//! Aides de la CLI pour les annotations : lecture des arguments (rectangle, traits) et JSON de
//! sortie. Les coordonnées sont des points de page affichée (origine en bas à gauche, rotation et
//! rognage appliqués) ; les pages sont en base 1, les indices d'annotation en base 0.

use sunipdf_core::{AnnotDetail, EngineError, LinkTarget, PointRect};

fn number(s: &str) -> Result<f32, EngineError> {
    s.trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| EngineError::Other(format!("invalid number: {s}")))
}

/// `left,bottom,right,top`.
pub fn parse_rect(spec: &str) -> Result<PointRect, EngineError> {
    let v: Vec<&str> = spec.split(',').collect();
    let [l, b, r, t] = v.as_slice() else {
        return Err(EngineError::Other(
            "rect must be left,bottom,right,top".into(),
        ));
    };
    Ok(PointRect {
        left: number(l)?,
        bottom: number(b)?,
        right: number(r)?,
        top: number(t)?,
    })
}

/// `"x,y x,y x,y"` : un trait d'encre.
pub fn parse_stroke(spec: &str) -> Result<Vec<(f32, f32)>, EngineError> {
    spec.split_whitespace()
        .map(|p| {
            let (x, y) = p
                .split_once(',')
                .ok_or_else(|| EngineError::Other(format!("invalid stroke point: {p}")))?;
            Ok((number(x)?, number(y)?))
        })
        .collect()
}

fn hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub fn link_json(t: &LinkTarget) -> serde_json::Value {
    match t {
        LinkTarget::Page(p) => serde_json::json!({ "page": p + 1 }),
        LinkTarget::Uri(u) => serde_json::json!({ "uri": u }),
        LinkTarget::Unsupported => serde_json::json!({ "unsupported": true }),
    }
}

pub fn detail_json(d: &AnnotDetail) -> serde_json::Value {
    serde_json::json!({
        "index": d.index,
        "subtype": d.subtype,
        "rect": [d.rect.left, d.rect.bottom, d.rect.right, d.rect.top],
        "contents": d.contents,
        "author": d.author,
        "modified": d.modified,
        "name": d.name,
        "color": d.rgb.map(hex),
        "fill": d.fill_rgb.map(hex),
        "width": d.width_pt,
        "opacity": d.opacity,
        "font_size": d.font_size,
        "parent": d.parent,
        "state": d.status.map(|s| s.id()),
        "link": d.link.as_ref().map(link_json),
        "arrow": d.arrow,
        "ours": d.ours,
        "locked": d.locked,
        "replies": d.replies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_and_strokes_parse_strictly() {
        let r = parse_rect("10, 20,30.5,40").unwrap();
        assert_eq!((r.left, r.bottom, r.right, r.top), (10.0, 20.0, 30.5, 40.0));
        for bad in ["1,2,3", "1,2,3,4,5", "a,b,c,d", "1,2,3,nan", ""] {
            assert!(parse_rect(bad).is_err(), "{bad:?}");
        }
        assert_eq!(
            parse_stroke("1,2 3.5,-4").unwrap(),
            [(1.0, 2.0), (3.5, -4.0)]
        );
        assert!(parse_stroke("1,2 3").is_err());
        assert!(parse_stroke("1,x").is_err());
        assert!(parse_stroke("").unwrap().is_empty());
    }

    #[test]
    fn links_are_reported_with_one_based_pages() {
        assert_eq!(
            link_json(&LinkTarget::Page(1)),
            serde_json::json!({ "page": 2 })
        );
        assert_eq!(
            link_json(&LinkTarget::Uri("https://a.b".into())),
            serde_json::json!({ "uri": "https://a.b" })
        );
    }
}
