use std::fmt::Debug;
use std::iter::FromIterator;
use std::usize;

use gc_arena::{Collect, Collection};
use intmap::IntMap;
use redscript::bundle::PoolIndex;

use crate::value::Value;

#[derive(Debug, Clone)]
pub struct IndexMap<V> {
    values: IntMap<V>,
}

impl<V> IndexMap<V> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_capacity(len: usize) -> Self {
        Self {
            values: IntMap::with_capacity(len),
        }
    }

    #[inline]
    pub fn get_mut<A>(&mut self, idx: PoolIndex<A>) -> Option<&mut V> {
        let idx: u32 = idx.into();
        self.values.get_mut(idx.into())
    }

    #[inline]
    pub fn get<A>(&self, idx: PoolIndex<A>) -> Option<&V> {
        let idx: u32 = idx.into();
        self.values.get(idx.into())
    }

    #[inline]
    pub fn put<A>(&mut self, idx: PoolIndex<A>, val: V) {
        let idx: u32 = idx.into();
        self.values.insert(idx.into(), val);
    }

    #[inline]
    pub fn iter<A>(&self) -> impl Iterator<Item = (PoolIndex<A>, &V)> {
        self.values.iter().map(|(&key, val)| (PoolIndex::new(key as u32), val))
    }
}

impl<V> Default for IndexMap<V> {
    #[inline]
    fn default() -> Self {
        Self { values: IntMap::new() }
    }
}

impl<A, V> FromIterator<(PoolIndex<A>, V)> for IndexMap<V> {
    fn from_iter<I: IntoIterator<Item = (PoolIndex<A>, V)>>(iter: I) -> Self {
        let values = iter.into_iter().map(|(k, v)| (u32::from(k).into(), v)).collect();
        Self { values }
    }
}

unsafe impl<V: Collect> Collect for IndexMap<V> {
    #[inline]
    fn needs_trace() -> bool {
        Value::needs_trace()
    }

    #[inline]
    fn trace(&self, cc: &Collection) {
        for (k, v) in self.values.iter() {
            k.trace(cc);
            v.trace(cc);
        }
    }
}
