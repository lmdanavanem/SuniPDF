//! Aides de la CLI pour les annotations : lecture des arguments (rectangle, traits) et JSON de
//! sortie. Les coordonnées sont des points de page affichée (origine en bas à gauche, rotation et
//! rognage appliqués) ; les pages sont en base 1, les indices d'annotation en base 0.

use sunipdf_core::{AnnotDetail, Destination, DestinationView, EngineError, LinkTarget, PointRect};

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

pub fn link_target(
    page: Option<usize>,
    uri: Option<String>,
    named: Option<String>,
    view: Option<String>,
    x: Option<f32>,
    y: Option<f32>,
    zoom: Option<f32>,
) -> Result<LinkTarget, EngineError> {
    let invalid = || {
        EngineError::Other("Use exactly one of --goto-page, --url or --named-destination; coordinates need --view xyz or --view fith.".into())
    };
    if [x, y, zoom].into_iter().flatten().any(|v| !v.is_finite()) || zoom.is_some_and(|v| v <= 0.0)
    {
        return Err(invalid());
    }
    match (page, uri, named) {
        (Some(page), None, None) => {
            let page = page.checked_sub(1).ok_or_else(invalid)?;
            let view = match view.as_deref() {
                None | Some("fit") if x.is_none() && y.is_none() && zoom.is_none() => {
                    return Ok(LinkTarget::Page(page))
                }
                Some("xyz") => DestinationView::Xyz { x, y, zoom },
                Some("fith") if zoom.is_none() => DestinationView::FitWidth { x, y },
                _ => return Err(invalid()),
            };
            Ok(LinkTarget::Destination(Destination { page, view }))
        }
        (None, Some(uri), None)
            if view.is_none() && x.is_none() && y.is_none() && zoom.is_none() =>
        {
            Ok(LinkTarget::Uri(uri.trim().to_string()))
        }
        (None, None, Some(named))
            if view.is_none()
                && x.is_none()
                && y.is_none()
                && zoom.is_none()
                && !named.trim().is_empty() =>
        {
            Ok(LinkTarget::Named(named))
        }
        _ => Err(invalid()),
    }
}

pub fn link_json(t: &LinkTarget) -> serde_json::Value {
    match t {
        LinkTarget::Page(p) => serde_json::json!({ "page": p + 1 }),
        LinkTarget::Named(name) => serde_json::json!({ "named_destination": name }),
        LinkTarget::Destination(destination) => destination_json(*destination),
        LinkTarget::Uri(u) => serde_json::json!({ "uri": u }),
        LinkTarget::RemotePage(target) => serde_json::json!({
            "remote_page": target.page + 1, "relative_path": target.relative_path,
            "sheet_id": target.sheet_id, "sha256": target.sha256,
            "byte_len": target.byte_len, "requires_verified_plan_set": true,
        }),
        LinkTarget::Unsupported => serde_json::json!({ "unsupported": true }),
    }
}

pub fn destination_json(destination: Destination) -> serde_json::Value {
    let mut value = match destination.view {
        DestinationView::Xyz { x, y, zoom } => {
            serde_json::json!({"view":"XYZ","x":x,"y":y,"zoom":zoom})
        }
        DestinationView::FitPage => serde_json::json!({"view":"Fit"}),
        DestinationView::FitWidth { x, y } => {
            serde_json::json!({"view":"FitH","x":x,"y":y})
        }
    };
    value["page"] = (destination.page + 1).into();
    value
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
        "dash_pattern": d.dash_pattern,
        "opacity": d.opacity,
        "font_size": d.font_size,
        "parent": d.parent,
        "state": d.status.map(|s| s.id()),
        "link": d.link.as_ref().map(link_json),
        "arrow": d.arrow,
        "ours": d.ours,
        "locked": d.locked,
        "can_edit_link": d.can_edit_link,
        "replies": d.replies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_destinations_retain_null_axes_and_reject_invalid_combinations() {
        let target = link_target(
            Some(2),
            None,
            None,
            Some("xyz".into()),
            None,
            Some(25.0),
            Some(1.5),
        )
        .unwrap();
        assert_eq!(
            target,
            LinkTarget::Destination(Destination {
                page: 1,
                view: DestinationView::Xyz {
                    x: None,
                    y: Some(25.0),
                    zoom: Some(1.5)
                }
            })
        );
        assert_eq!(
            link_target(None, None, Some("Chapter 1".into()), None, None, None, None).unwrap(),
            LinkTarget::Named("Chapter 1".into())
        );
        for zoom in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(link_target(
                Some(1),
                None,
                None,
                Some("xyz".into()),
                None,
                None,
                Some(zoom)
            )
            .is_err());
        }
        assert!(link_target(
            Some(1),
            None,
            None,
            Some("fit".into()),
            Some(4.0),
            None,
            None
        )
        .is_err());
        assert!(link_target(
            Some(1),
            Some("https://example.org".into()),
            None,
            None,
            None,
            None,
            None
        )
        .is_err());
    }

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
