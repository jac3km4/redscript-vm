use std::fmt::Debug;
use std::iter::FromIterator;
use std::usize;

use gc_arena::{Collect, CollectionContext};
use intmap::IntMap;
use redscript::bundle::PoolIndex;

use crate::value::Value;

#[derive(Debug)]
pub struct IndexMap<V> {
    values: IntMap<V>,
}

impl<V> IndexMap<V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(len: usize) -> Self {
        Self {
            values: IntMap::with_capacity(len),
        }
    }

    #[inline(always)]
    pub fn get_mut<A>(&mut self, idx: PoolIndex<A>) -> Option<&mut V> {
        self.values.get_mut(idx.index as u64)
    }

    #[inline(always)]
    pub fn get<A>(&self, idx: PoolIndex<A>) -> Option<&V> {
        self.values.get(idx.index as u64)
    }

    #[inline(always)]
    pub fn put<A>(&mut self, idx: PoolIndex<A>, val: V) {
        self.values.insert(idx.index as u64, val);
    }
}

impl<V> Default for IndexMap<V> {
    fn default() -> Self {
        Self { values: IntMap::new() }
    }
}

impl<A, V> FromIterator<(PoolIndex<A>, V)> for IndexMap<V> {
    fn from_iter<I: IntoIterator<Item = (PoolIndex<A>, V)>>(iter: I) -> Self {
        let values = iter.into_iter().map(|(k, v)| (k.index as u64, v)).collect();
        Self { values }
    }
}

unsafe impl<V: Collect> Collect for IndexMap<V> {
    #[inline]
    fn needs_trace() -> bool {
        Value::needs_trace()
    }

    #[inline]
    fn trace(&self, cc: CollectionContext) {
        for (k, v) in self.values.iter() {
            k.trace(cc);
            v.trace(cc);
        }
    }
}
