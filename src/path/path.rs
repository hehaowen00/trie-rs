use crate::params::Params;
use slab::Slab;

#[derive(Debug)]
pub struct PathTrie<T> {
    nodes: Slab<Node<T>>,
}

impl<T> PathTrie<T> {
    pub fn new() -> Self {
        let root = Node::from(String::new(), Vec::new());

        let mut slab = Slab::new();
        slab.insert(root);

        Self { nodes: slab }
    }

    pub fn get<'a, 'b>(&'a self, key: &'b str) -> Option<(&T, Params<'a, 'b>)> {
        let mut params = Params::new();
        match self.get_params(&mut params, key) {
            Some(data) => Some((data, params)),
            None => None,
        }
    }

    fn get_params<'a, 'b>(&'a self, params: &mut Params<'a, 'b>, key: &'b str) -> Option<&T> {
        let mut key = key.as_bytes();
        let mut curr = 0;

        'outer: loop {
            key = if key.starts_with(b"/") {
                key.split_at(1).1
            } else {
                key
            };

            let node = &self.nodes[curr];

            if key.len() == 0 {
                return node.data.as_ref();
            }

            let lut: &[u8] = node.index.as_ref();

            if lut.len() == 0 {
                return None;
            }

            let xs: &[usize] = node.children.as_ref();

            let n = match find(key, b'/') {
                Some(n) => n,
                None => key.len(),
            };

            match find(lut, key[0]) {
                Some(start) => {
                    for idx in start..lut.len() {
                        let idx = xs[idx];
                        let el: &[u8] = self.nodes[idx].path.as_ref();

                        if el[0] != key[0] {
                            break;
                        }

                        if el.len() < n {
                            continue;
                        }

                        if key.starts_with(el) {
                            let (_, rem) = key.split_at(el.len());
                            curr = idx;
                            key = rem;
                            continue 'outer;
                        }
                    }
                }
                None => {}
            }

            match find(lut, b':') {
                Some(idx) => {
                    let idx = xs[idx];
                    let node = &self.nodes[idx];

                    let (_, k) = node.path.split_at(1);
                    let (v, rem) = key.split_at(n);
                    let k = to_str(k);
                    let v = to_str(v);
                    params.insert(k, v);

                    curr = idx;
                    key = rem;
                    continue 'outer;
                }
                None => {}
            }

            match find(lut, b'*') {
                Some(idx) => {
                    let idx = xs[idx];
                    let node = &self.nodes[idx];

                    let k = to_str(&node.path);
                    let v = to_str(key);
                    params.insert(k, v);

                    return node.data.as_ref();
                }
                None => return None,
            }
        }
    }

    pub fn insert<S>(&mut self, key: S, value: T)
    where
        S: AsRef<str>,
    {
        let key: Vec<_> = key.as_ref().split('/').filter(|s| s.len() != 0).collect();
        let mut active = key.as_slice();
        let mut curr = 0;

        'outer: loop {
            if active.len() == 0 {
                self.nodes[curr].data = Some(value);
                break 'outer;
            }

            if self.nodes[curr].children.len() == 0 {
                let (start, rem) = longest(active);
                let node = Node::new(start, Vec::new());

                let pos = self.nodes.insert(node);
                self.nodes[curr].children.push(pos);

                curr = pos;
                active = &active[rem..];

                continue 'outer;
            }

            let xs = self.nodes[curr].children.clone();

            for idx in xs {
                let n_p = to_str(&self.nodes[idx].path).to_owned();
                let num = lcs(&n_p, active);
                let equal = eq(&n_p, active);

                if num > 0 && !equal {
                    if n_p.length() == num {
                        curr = idx;
                        active = &active[num..];
                        continue 'outer;
                    }

                    let subpath = n_p.after(num).to_string();
                    let children = std::mem::replace(&mut self.nodes[idx].children, Vec::new());

                    let mut right = Node::from(subpath, children);
                    right.data = self.nodes[idx].data.take();

                    let pos = self.nodes.insert(right);

                    self.nodes[idx].path = n_p.from(num).to_string().into_bytes();
                    self.nodes[idx].children.push(pos);

                    active = &active[num..];
                    let (joined, rem) = longest(active);
                    let node = Node::new(joined, Vec::new());

                    let pos = self.nodes.insert(node);
                    self.nodes[idx].children.push(pos);

                    curr = pos;
                    active = &active[rem..];

                    continue 'outer;
                }

                if equal {
                    self.nodes[idx].data = Some(value);
                    break 'outer;
                }

                let p = &unsafe { std::str::from_utf8_unchecked(&self.nodes[idx].path) };
                let p = p.to_string();
                let p = &p.at(0)[0..1];
                match (p, &active[0][0..1]) {
                    (":", ":") | ("*", ":") => {
                        if &self.nodes[idx].path == &active[0].as_bytes() {
                            curr = idx;
                            active = &active[1..];
                            continue 'outer;
                        }
                        let node = Node::new(&active[0..1], self.nodes[idx].children.clone());
                        curr = idx;
                        active = &active[1..];

                        let prev = std::mem::replace(&mut self.nodes[idx], node);
                        for sub in &prev.children {
                            self.delete(*sub);
                        }
                    }
                    (":", "*") | ("*", "*") => {
                        let node = Node::new(&active[0..1], self.nodes[idx].children.clone());
                        curr = idx;
                        active = &active[1..];

                        let prev = std::mem::replace(&mut self.nodes[idx], node);
                        for sub in &prev.children {
                            self.delete(*sub);
                        }
                    }
                    _ => continue,
                }

                continue 'outer;
            }

            let (start, rem) = longest(active);
            let node = Node::new(start, Vec::new());

            let pos = self.nodes.insert(node);
            self.nodes[curr].children.push(pos);

            curr = pos;
            active = &active[rem..];
        }
        self.sort_all();
    }

    fn delete(&mut self, idx: usize) {
        for sub in self.nodes[idx].children.clone() {
            self.delete(sub);
        }
        self.nodes.remove(idx);
    }

    fn count_children(&self, idx: usize) -> usize {
        let mut count = self.nodes[idx].children.len();
        for child in &self.nodes[idx].children {
            count += self.count_children(*child);
        }
        count
    }

    fn sort_all(&mut self) {
        let mut keys = Vec::new();
        for (idx, _) in &self.nodes {
            keys.push(idx);
        }
        for i in keys {
            self.sort(i);
        }
    }

    fn sort(&mut self, idx: usize) {
        if self.nodes[idx].children.len() == 0 {
            return;
        }

        if self.nodes[idx].children.len() == 1 {
            let xs = [self.nodes[idx].children[0]]
                .iter()
                .map(|i| self.nodes[*i].path[0])
                .collect::<Vec<_>>();
            self.nodes[idx].index = xs;
            return;
        }

        let mut children = self.nodes[idx].children.clone();

        children.sort_by(|a, b| {
            let p_a = &self.nodes[*a].path;
            let p_b = &self.nodes[*b].path;
            if p_a[0] == b':' || p_a[0] == b'*' && p_b[0] != b':' || p_b[0] != b'*' {
                return std::cmp::Ordering::Greater;
            }
            if p_b[0] == b':' || p_b[0] == b'*' && p_a[0] != b':' || p_a[0] != b'*' {
                return std::cmp::Ordering::Less;
            }
            if p_a[0] == p_b[0] {
                return self.count_children(*a).cmp(&self.count_children(*b));
            }
            p_a.cmp(p_b)
        });

        let index = children
            .iter()
            .map(|i| self.nodes[*i].path[0])
            .collect::<Vec<_>>();

        self.nodes[idx].index = index;
        self.nodes[idx].children = children;
    }
}

#[derive(Debug)]
struct Node<T> {
    path: Vec<u8>,
    index: Vec<u8>,
    data: Option<T>,
    children: Vec<usize>,
}

impl<T> Node<T> {
    pub fn new(path: &[&str], children: Vec<usize>) -> Self {
        Self {
            path: path.join("/").into_bytes(),
            data: None,
            index: Vec::new(),
            children,
        }
    }

    pub fn from(path: String, children: Vec<usize>) -> Self {
        Self {
            path: path.into_bytes(),
            data: None,
            index: Vec::new(),
            children,
        }
    }
}

fn longest<'a>(key: &'a [&str]) -> (&'a [&'a str], usize) {
    if key.len() == 0 {
        return (&key[0..0], 0);
    }
    let mut len = 0;
    for i in 0..key.len() {
        if &key[i][0..1] == "*" || &key[i][0..1] == ":" {
            break;
        }
        len = i;
    }
    (&key[0..len + 1], len + 1)
}

fn lcs(a: &String, b: &[&str]) -> usize {
    let min = std::cmp::min(a.length(), b.len());
    let mut last = 0;
    for i in 0..min {
        if &a.at(i)[0..1] == ":" && &b[i][0..1] == ":" {
            return last;
        }
        if a.at(i) != b[i] {
            return i;
        }
        last = i;
    }
    min
}

fn eq(s: &String, xs: &[&str]) -> bool {
    if s.length() < xs.len() {
        return false;
    }
    for i in 0..xs.len() {
        if s.at(i) != xs[i] {
            return false;
        }
    }
    true
}

#[inline]
fn find(a: &[u8], b: u8) -> Option<usize> {
    a.iter().position(|&a| a == b)
}

#[inline]
fn to_str<'a>(bytes: &'a [u8]) -> &'a str {
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

trait Segmented {
    fn length(&self) -> usize;
    fn at(&self, idx: usize) -> &str;
    fn after(&self, idx: usize) -> &str;
    fn from(&self, idx: usize) -> &str;
}

impl Segmented for String {
    fn length(&self) -> usize {
        self.split("/").count()
    }

    fn at(&self, idx: usize) -> &str {
        self.split("/").nth(idx).unwrap()
    }

    fn after(&self, idx: usize) -> &str {
        let mut s;
        let mut c = 0;
        for i in 0..self.len() {
            if &self[i..i + 1] == "/" {
                s = i + 1;
                c = c + 1;
                if c == idx {
                    return &self[s..];
                }
            }
        }
        &self[self.len() - 1..self.len() - 1]
    }

    fn from(&self, idx: usize) -> &str {
        let mut c = 0;
        for i in 0..self.len() {
            if &self[i..i + 1] == "/" {
                c = c + 1;
                if c == idx {
                    return &self[0..i];
                }
            }
        }
        &self
    }
}
