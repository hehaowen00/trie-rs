#[cfg(test)]
mod tests;

use std::borrow::Borrow;

#[derive(Debug)]
pub struct TrieNode<K, V> {
    key: K,
    value: Option<V>,
    children: Vec<TrieNode<K, V>>,
}

impl<K, V> TrieNode<K, V> 
where
    K: Clone + Ord + PartialOrd,
{
    pub fn new<T>(key: T) -> Self
    where
        T: Borrow<K>,
    {
        Self {
            key: key.borrow().clone(),
            value: None,
            children: Vec::new(),
        }
    }

    pub fn from<T>(key: T, value: V) -> Self
    where
        T: Borrow<K>
    {
        Self {
            key: key.borrow().clone(),
            value: Some(value),
            children: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.children.clear();
        self.value = None;
    }

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn value(&self) -> Option<&V> {
        self.value.as_ref()
    }

    pub fn children(&self) -> &Vec<TrieNode<K, V>> {
        &self.children
    }

    pub fn get(&self, key: &[K]) -> Option<&V> {
        assert!(key.len() > 0);

        let mut node = self;
        let mut i = 0;

        loop {
            let res = node.children
                .binary_search_by(|e| e.key().cmp(&key[i]));

            match res {
                Ok(idx) => {
                    node = &node.children[idx];
                    i += 1;
                },
                Err(_) => {
                    return None;
                },
            }

            if i == key.len() {
                return node.value.as_ref();
            }
        }
    }

    pub fn insert<Q>(&mut self, key: Q, value: V) -> Result<Option<V>, ()>
    where
        Q: Borrow<[K]>,
    {
        let key = key.borrow();

        if key.len() == 0 {
            let value = ::std::mem::replace(&mut self.value, Some(value));
            return Ok(value);
        }

        let res = self.children
            .binary_search_by(|e| e.key().cmp(&key[0]));

        match res {
            Ok(idx) => {
                let node = &mut self.children[idx];
                match key.len() {
                    1 => {
                        let value = ::std::mem::replace(&mut node.value, Some(value));
                        return Ok(value);
                    },
                    _ => {
                        node.insert(&key[1..], value)?;
                        return Ok(None);
                    }
                }
            },
            Err(idx) => {
                let mut xs = vec![];

                for k in key {
                    xs.push(TrieNode::new(k));
                }

                xs[key.len() - 1].value = Some(value);

                let mut temp = xs.pop().unwrap();

                while let Some(mut n) = xs.pop() {
                    n.children.push(temp);
                    temp = n;
                }

                self.children.insert(idx, temp);
                return Ok(None);
            }
        }
    }

    pub fn remove(&mut self, key: &[K], prune: bool) -> Option<TrieNode<K, V>> {
        assert!(key.len() > 0);

        let mut nodes = &mut self.children;
        let mut i = 0;
        let mut rem = key;

        loop {
            let node_key = &nodes[i].key;

            if node_key == &rem[0] {
                rem = &rem[1..];
            }
            match rem.len() {
                0 => match prune {
                    true => {
                        let removed = nodes.remove(i);
                        return Some(removed);
                    },
                    false => {
                        let mut node = TrieNode::new(&nodes[i].key);
                        node.value = nodes[i].value.take();
                        return Some(node);
                    }
                },
                _ => {
                    nodes = &mut nodes[i].children;
                    i = 0;
                    continue;
                }
            }
        }
    }
}

