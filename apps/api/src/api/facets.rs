//! Per-cycle facet cache. Teams and collaborators are encrypted per entry, so listing them
//! means decrypting every entry; the old pages did that on each request. One short-lived
//! cached copy per cycle keeps filter dropdowns cheap.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Cached value shape lives in `entries::Facets`; this module only stores it.
pub type FacetMap<T> = Arc<DashMap<i64, (Instant, Arc<T>)>>;

pub fn new_facet_map<T>() -> FacetMap<T> {
    Arc::new(DashMap::new())
}

/// Returns the cached value for `cycle_id` when it is younger than `ttl`.
pub fn get_fresh<T>(map: &FacetMap<T>, cycle_id: i64, ttl: Duration) -> Option<Arc<T>> {
    map.get(&cycle_id)
        .filter(|e| e.0.elapsed() < ttl)
        .map(|e| e.1.clone())
}

pub fn put<T>(map: &FacetMap<T>, cycle_id: i64, value: Arc<T>) {
    map.insert(cycle_id, (Instant::now(), value));
}

pub fn invalidate<T>(map: &FacetMap<T>, cycle_id: i64) {
    map.remove(&cycle_id);
}
