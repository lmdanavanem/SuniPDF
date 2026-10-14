//! Cache LRU de rendus, plafonné en octets. Partagé par les miniatures (pages entières) et les
//! tuiles de la vue continue (régions).

use crate::tiles::{PixelRect, TileJob};
use crate::RenderedPage;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// (page, échelle en millièmes, région ; `None` = page entière).
pub type CacheKey = (usize, u32, Option<PixelRect>);

/// Clé stable d'une page entière : page + échelle quantifiée au millième.
pub fn cache_key(page: usize, scale: f32) -> CacheKey {
    (page, (scale * 1000.0).round() as u32, None)
}

/// Clé d'une tuile.
pub fn tile_key(job: &TileJob) -> CacheKey {
    (job.page, job.scale_milli, Some(job.region))
}

pub struct RenderCache {
    entries: HashMap<CacheKey, Arc<RenderedPage>>,
    lru: VecDeque<CacheKey>,
    bytes: usize,
    max_bytes: usize,
}

impl RenderCache {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            lru: VecDeque::new(),
            bytes: 0,
            max_bytes,
        }
    }

    pub fn contains(&self, page: usize, scale: f32) -> bool {
        self.entries.contains_key(&cache_key(page, scale))
    }

    pub fn contains_tile(&self, job: &TileJob) -> bool {
        self.entries.contains_key(&tile_key(job))
    }

    pub fn get(&mut self, page: usize, scale: f32) -> Option<Arc<RenderedPage>> {
        self.get_keyed(cache_key(page, scale))
    }

    pub fn get_tile(&mut self, job: &TileJob) -> Option<Arc<RenderedPage>> {
        self.get_keyed(tile_key(job))
    }

    /// Insère (ou remplace) une page entière.
    pub fn insert(&mut self, scale: f32, page: RenderedPage) -> Arc<RenderedPage> {
        self.insert_keyed(cache_key(page.page_index, scale), page)
    }

    pub fn insert_tile(&mut self, job: TileJob, image: RenderedPage) -> Arc<RenderedPage> {
        self.insert_keyed(tile_key(&job), image)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru.clear();
        self.bytes = 0;
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn get_keyed(&mut self, key: CacheKey) -> Option<Arc<RenderedPage>> {
        let hit = self.entries.get(&key).cloned()?;
        self.touch(key);
        Some(hit)
    }

    /// Insère puis évince les entrées les moins récentes jusqu'à respecter le plafond.
    /// L'entrée insérée n'est jamais évincée, même si elle dépasse seule le plafond.
    fn insert_keyed(&mut self, key: CacheKey, page: RenderedPage) -> Arc<RenderedPage> {
        let page = Arc::new(page);
        if let Some(old) = self.entries.insert(key, page.clone()) {
            self.bytes -= old.rgba.len();
        }
        self.bytes += page.rgba.len();
        self.touch(key);
        while self.bytes > self.max_bytes && self.lru.len() > 1 {
            let Some(victim) = self.lru.pop_front() else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&victim) {
                self.bytes -= evicted.rgba.len();
            }
        }
        page
    }

    fn touch(&mut self, key: CacheKey) {
        if let Some(pos) = self.lru.iter().position(|k| *k == key) {
            self.lru.remove(pos);
        }
        self.lru.push_back(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(index: usize, bytes: usize) -> RenderedPage {
        RenderedPage {
            page_index: index,
            width: bytes as u32 / 4,
            height: 1,
            rgba: vec![0; bytes],
        }
    }

    fn job(page: usize, scale_milli: u32, x: u32) -> TileJob {
        TileJob {
            page,
            scale_milli,
            region: PixelRect::new(x, 0, 10, 10),
        }
    }

    #[test]
    fn evicts_least_recently_used_within_budget() {
        let mut cache = RenderCache::new(100);
        cache.insert(1.0, page(0, 40));
        cache.insert(1.0, page(1, 40));
        assert!(cache.get(0, 1.0).is_some()); // page 0 devient la plus récente
        cache.insert(1.0, page(2, 40)); // dépasse 100 → évince la page 1
        assert!(cache.contains(0, 1.0));
        assert!(!cache.contains(1, 1.0));
        assert!(cache.contains(2, 1.0));
        assert_eq!(cache.bytes(), 80);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn oversized_entry_is_kept_alone_and_scale_is_part_of_key() {
        let mut cache = RenderCache::new(50);
        cache.insert(1.0, page(0, 40));
        cache.insert(2.0, page(0, 400));
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(0, 2.0));
        assert!(!cache.contains(0, 1.0));
        assert_eq!(cache_key(3, 1.33349), (3, 1333, None));
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.bytes(), 0);
    }

    #[test]
    fn tiles_have_distinct_keys_and_share_the_byte_budget() {
        let mut cache = RenderCache::new(200);
        cache.insert_tile(job(0, 1000, 0), page(0, 40));
        cache.insert_tile(job(0, 1000, 10), page(0, 40));
        cache.insert_tile(job(0, 2000, 0), page(0, 40));
        cache.insert(1.0, page(0, 40)); // page entière, même page et même échelle : autre clé
        assert_eq!(cache.len(), 4);
        assert!(cache.contains_tile(&job(0, 1000, 10)));
        assert!(!cache.contains_tile(&job(1, 1000, 10)));
        // La tuile relue survit à l'éviction ; le plafond n'est jamais dépassé.
        assert!(cache.get_tile(&job(0, 1000, 0)).is_some());
        for x in 100..120 {
            cache.insert_tile(job(5, 1000, x), page(5, 30));
            assert!(cache.bytes() <= 200, "{}", cache.bytes());
            if x == 101 {
                assert!(cache.contains_tile(&job(0, 1000, 0)));
                assert!(!cache.contains_tile(&job(0, 1000, 10)));
            }
        }
        cache.clear();
        assert_eq!(cache.bytes(), 0);
    }
}
