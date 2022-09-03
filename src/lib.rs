pub mod params;
pub mod path;
pub mod radix;
pub mod trie;

use std::borrow::Borrow;

pub trait TrieExt<K, V>: Sized
where
    K: Clone + Ord,
{
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: Borrow<[K]>;

    fn insert<Q>(&mut self, key: &Q, value: V) -> Result<Option<V>, ()>
    where
        Q: Borrow<[K]> + ?Sized;

    fn remove<Q>(&mut self, key: &Q, prune: bool) -> Option<Self>
    where
        Q: Borrow<[K]>;
}
