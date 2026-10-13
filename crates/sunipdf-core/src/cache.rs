//! Cache LRU de pages rendues, plafonné en octets. Partagé par le rendu principal, les miniatures
//! et le pré-rendu des pages voisines.

use crate::RenderedPage;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub type CacheKey = (usize, u32);

/// Clé stable : page + échelle quantifiée au millième.
pub fn cache_key(page: usize, scale: f32) -> CacheKey {
    (page, (scale * 1000.0).round() as u32)
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

    pub fn get(&mut self, page: usize, scale: f32) -> Option<Arc<RenderedPage>> {
        let key = cache_key(page, scale);
        let hit = self.entries.get(&key).cloned()?;
        self.touch(key);
        Some(hit)
    }

    /// Insère (ou remplace) puis évince les entrées les moins récentes jusqu'à respecter le plafond.
    /// L'entrée insérée n'est jamais évincée, même si elle dépasse seule le plafond.
    pub fn insert(&mut self, scale: f32, page: RenderedPage) -> Arc<RenderedPage> {
        let key = cache_key(page.page_index, scale);
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
        assert_eq!(cache_key(3, 1.33349), (3, 1333));
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.bytes(), 0);
    }
}
