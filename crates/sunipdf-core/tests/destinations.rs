#[path = "fixtures/destinations.rs"]
mod fixture;

use std::path::Path;
use sunipdf_core::{
    security, Destination, DestinationView, Document, Engine, LinkTarget, Protection,
};

fn axes(page: usize, x: Option<f32>, y: Option<f32>) -> (Option<f32>, Option<f32>) {
    match page {
        0 => (x.map(|v| v - 100.0), y.map(|v| v - 50.0)),
        1 => (y.map(|v| v - 50.0), x.map(|v| 500.0 - v)),
        2 => (x.map(|v| 500.0 - v), y.map(|v| 650.0 - v)),
        3 => (y.map(|v| 650.0 - v), x.map(|v| v - 100.0)),
        _ => unreachable!(),
    }
}

fn expected(page: usize, row: usize) -> Option<Destination> {
    let xyz = |x, y, zoom| {
        let (x, y) = axes(page, x, y);
        DestinationView::Xyz { x, y, zoom }
    };
    let view = match row {
        0 => xyz(Some(200.0), Some(400.0), Some(1.5)),
        1 => xyz(None, Some(400.0), None),
        2 => xyz(Some(150.0), Some(450.0), Some(2.0)),
        3 | 4 => xyz(Some(300.0), None, None),
        5 => DestinationView::FitPage,
        6 => {
            let (x, y) = axes(page, None, Some(400.0));
            DestinationView::FitWidth { x, y }
        }
        7..=11 => return None,
        _ => unreachable!(),
    };
    Some(Destination { page, view })
}

fn assert_destinations(doc: &Document<'_>) {
    let links = doc.page_links(0).unwrap();
    let bookmarks = doc.bookmarks();
    assert_eq!(links.len(), 4 * fixture::PER_PAGE + 7);
    assert_eq!(bookmarks.len(), links.len());
    for page in 0..4 {
        for row in 0..fixture::PER_PAGE {
            let index = page * fixture::PER_PAGE + row;
            let link = &links[index];
            assert_eq!(link.index, index);
            assert_eq!(link.target, LinkTarget::Page(page), "page {page} row {row}");
            assert_eq!(
                link.destination,
                expected(page, row),
                "page {page} row {row}"
            );
            let bookmark = &bookmarks[index];
            assert_eq!(bookmark.title, format!("Page {page} destination {row}"));
            assert_eq!(bookmark.depth, 0);
            assert_eq!(bookmark.page, Some(page), "bookmark page {page} row {row}");
            assert_eq!(
                bookmark.destination, link.destination,
                "bookmark page {page} row {row}"
            );
        }
    }
    let start = 4 * fixture::PER_PAGE;
    for index in start..start + 5 {
        assert_eq!(
            links[index].target,
            LinkTarget::Unsupported,
            "index {index}"
        );
        assert_eq!(links[index].destination, None);
        assert_eq!(bookmarks[index].page, None);
        assert_eq!(bookmarks[index].destination, None);
    }
    assert_eq!(
        links[start + 5].target,
        LinkTarget::Uri("https://example.org".into())
    );
    assert_eq!(
        links[start + 6].target,
        LinkTarget::Uri("javascript:alert(1)".into())
    );
    assert!(!sunipdf_core::is_safe_external_uri("javascript:alert(1)"));
    for index in start + 5..links.len() {
        assert_eq!(links[index].destination, None);
        assert_eq!(bookmarks[index].page, None);
        assert_eq!(bookmarks[index].destination, None);
    }
}

#[test]
fn direct_named_action_and_bookmark_destinations_survive_crops_rotations_aes_and_snapshot() {
    let engine = Engine::new().unwrap();
    let original = fixture::document();
    let directory = tempfile::tempdir().unwrap();
    for encrypted in [false, true] {
        let bytes = if encrypted {
            security::protect(
                &original,
                None,
                &Protection {
                    user_password: "user".into(),
                    owner_password: "owner".into(),
                    aes256: true,
                    ..Default::default()
                },
            )
            .unwrap()
        } else {
            original.clone()
        };
        let password = encrypted.then_some("user");
        let mut doc = engine
            .open_bytes(bytes, password, Path::new("destinations.pdf"))
            .unwrap();
        assert_destinations(&doc);
        let snapshot = doc.snapshot().unwrap();
        doc.restore(snapshot).unwrap();
        assert_destinations(&doc);
        let saved = directory.path().join(if encrypted {
            "encrypted.pdf"
        } else {
            "plain.pdf"
        });
        doc.save_as(&saved).unwrap();
        let reopened = engine.open(&saved, password).unwrap();
        assert_destinations(&reopened);
    }
}
