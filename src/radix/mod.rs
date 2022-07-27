#[cfg(test)]
mod tests;

use crate::TrieExt;
use std::borrow::Borrow;
use std::cmp::Ordering;

#[derive(Debug, Eq, PartialEq)]
pub struct RadixNode<K, V> {
    key: Vec<K>,
    value: Option<V>,
    children: Vec<RadixNode<K, V>>,
}

impl<K, V> RadixNode<K, V>
where
    K: Clone + Ord,
{
    pub fn new() -> Self {
        Self {
            key: Vec::new(),
            value: None,
            children: Vec::new(),
        }
    }

    pub fn from<T>(key: T, value: V) -> Self
    where
        T: AsRef<[K]>,
    {
        Self {
            key: key.as_ref().to_vec(),
            value: Some(value),
            children: Vec::new(),
        }
    }

    pub fn from_key<T>(key: T) -> Self
    where
        T: AsRef<[K]>,
    {
        Self {
            key: key.as_ref().to_vec(),
            value: None,
            children: Vec::new(),
        }
    }

    pub fn key(&self) -> &[K] {
        &self.key
    }

    pub fn value(&self) -> Option<&V> {
        self.value.as_ref()
    }

    pub fn children(&self) -> &Vec<RadixNode<K, V>> {
        &self.children
    }

    pub fn to_parts(self) -> (Vec<K>, Option<V>, Vec<RadixNode<K, V>>) {
        (self.key, self.value, self.children)
    }
}

impl<K, V> TrieExt<K, V> for RadixNode<K, V>
where
    K: Clone + Ord,
{
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: Borrow<[K]>,
    {
        let k = key.borrow();
        assert!(k.len() > 0);

        let mut rem = k;
        let mut nodes = &self.children;

        let mut i = 0;

        loop {
            if nodes.len() == 0 {
                return None;
            }

            let node_key = &nodes[i].key;
            let lcs = longest_match(node_key, &rem);

            if lcs == 0 {
                i += 1;
                if i >= nodes.len() {
                    break;
                }
                continue;
            }

            if node_key == rem {
                return nodes[i].value.as_ref();
            }

            if node_key.len() == lcs && rem.len() > lcs {
                nodes = &nodes[i].children;
                rem = &rem[lcs..];
                i = 0;
                continue;
            }

            i += 1;

            if i == nodes.len() {
                break;
            }
        }

        None
    }

    fn insert<Q>(&mut self, key: &Q, value: V) -> Result<Option<V>, ()>
    where
        Q: Borrow<[K]> + ?Sized,
    {
        let mut nodes = &mut self.children;
        let mut k = key.borrow();

        let mut i = 0;
        loop {
            if nodes.len() == 0 {
                nodes.push(RadixNode::from(k, value));
                return Ok(None);
            }

            let node_key = &nodes[i].key;
            let lcs = longest_match(node_key, k);

            if lcs == 0 {
                i += 1;
                if i >= nodes.len() {
                    break;
                }
                continue;
            }

            let res = node_key.len().cmp(&lcs);

            match res {
                Ordering::Equal => match k[lcs..].len() {
                    0 => {
                        let val = std::mem::replace(&mut nodes[i].value, Some(value));
                        return Ok(val);
                    }
                    _ => {
                        nodes = &mut nodes[i].children;
                        i = 0;
                        k = &k[lcs..];
                        continue;
                    }
                },
                Ordering::Greater => {
                    let mut new_root = RadixNode::from_key(&k[0..lcs]);
                    let mut old = nodes.remove(i);

                    match old.key[lcs..] == new_root.key {
                        true => {
                            new_root.value = old.value.take();
                            std::mem::swap(&mut new_root.children, &mut old.children);
                            drop(old);
                        }
                        false => {
                            old.key = old.key[lcs..].to_vec();
                            new_root.children.push(old);
                        }
                    }

                    match &k[lcs..].len() {
                        0 => {
                            new_root.value = Some(value);
                            nodes.push(new_root);
                            nodes.sort_by(|a, b| a.key.cmp(&b.key));
                            return Ok(None);
                        }
                        _ => {
                            let mut child = RadixNode::from_key(&k[lcs..]);
                            child.value = Some(value);
                            new_root.children.push(child);

                            nodes.push(new_root);
                            nodes.sort_by(|a, b| a.key.cmp(&b.key));

                            return Ok(None);
                        }
                    }
                }
                Ordering::Less => unreachable!(),
            }
        }

        nodes.push(RadixNode::from(k, value));
        nodes.sort_by(|a, b| a.key.cmp(&b.key));

        Ok(None)
    }

    fn remove<Q>(&mut self, key: &Q, prune: bool) -> Option<Self>
    where
        Q: Borrow<[K]>,
    {
        let key = key.borrow();
        assert!(key.len() > 0);

        let mut rem = key;
        let mut nodes = &mut self.children;

        let mut idx = 0;

        loop {
            let node_key = &nodes[idx].key;
            let lcs = longest_match(node_key, &rem);

            if lcs == 0 {
                idx += 1;
                if idx == nodes.len() {
                    break;
                }
                continue;
            }

            if node_key == rem {
                if prune {
                    let removed = nodes.remove(idx);
                    return Some(removed);
                }

                match nodes[idx].children.len() {
                    0 => {
                        let removed = nodes.remove(idx);
                        return Some(removed);
                    }
                    1 => {
                        // a node with one child is only possible if the node has a value
                        let node = &mut nodes[idx];

                        let mut child = node.children.pop().unwrap();
                        let mut prefix = node.key.clone();
                        let prefix = prefix.drain(0..node.key.len());
                        child.key.splice(0..0, prefix);

                        let removed = std::mem::replace(node, child);
                        return Some(removed);
                    }
                    _ => {
                        let node = &mut nodes[idx];
                        let key = node.key.clone();
                        let res = match node.value.take() {
                            Some(v) => RadixNode::from(key, v),
                            None => RadixNode::from_key(key),
                        };
                        return Some(res);
                    }
                }
            }

            if node_key.len() == lcs && rem.len() > lcs {
                nodes = &mut nodes[idx].children;
                rem = &rem[lcs..];
                idx = 0;
                continue;
            }

            idx += 1;

            if idx == nodes.len() {
                break;
            }
        }

        None
    }
}

fn longest_match<T>(a: &[T], b: &[T]) -> usize
where
    T: Ord,
{
    let mut len = 0;

    match (a.len(), b.len()) {
        (0, _) => return 0,
        (_, 0) => return 0,
        (_, _) => (),
    }

    let min = usize::min(a.len(), b.len());

    for i in 0..min {
        if a[i] == b[i] {
            len += 1;
        } else {
            break;
        }
    }

    len
}
