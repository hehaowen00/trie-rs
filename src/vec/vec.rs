use crate::path::{
    node::{after, substr},
    Params,
};
use slab::Slab;

#[derive(Debug)]
pub struct VecPathTrie<T> {
    nodes: Slab<Node>,
    data: Slab<T>,
}

impl<T> VecPathTrie<T> {
    pub fn new() -> Self {
        let mut slab = Slab::new();

        let root = Node::from(String::new(), None, Vec::new());
        slab.insert(root);

        Self {
            nodes: slab,
            data: Slab::new(),
        }
    }

    pub fn get<'a, 'b>(&'a self, key: &'b str) -> Option<(&T, Params<'a, 'b>)> {
        let mut params = Params::new();
        match self.get_params(&mut params, key) {
            Some(data) => Some((data, params)),
            None => None,
        }
    }

    fn get_params<'a, 'b>(&'a self, params: &mut Params<'a, 'b>, key: &'b str) -> Option<&T> {
        let mut curr = 0;
        let mut key = key;

        'outer: loop {
            key = if key.starts_with("/") { &key[1..] } else { key };

            if key.len() == 0 {
                let idx = self.nodes[curr].data?;
                return Some(&self.data[idx]);
            }

            let cs = &self.nodes[curr].children;

            if cs.len() == 0 {
                return None;
            }

            let lut = &self.nodes[curr].index;

            let xs = &self.nodes[curr].children;
            let temp = substr(key, "/");

            match lut.find(&key[0..1]) {
                Some(start) => {
                    for idx in start..lut.len() {
                        let idx = xs[idx];
                        let p = &self.nodes[idx].path;

                        if &p[0..1] != &temp[0..1] {
                            break;
                        }

                        if p.len() < temp.len() {
                            continue;
                        }

                        if key.starts_with(p) {
                            curr = idx;
                            key = &key[p.len()..];
                            continue 'outer;
                        }

                        let a = match_left(p, temp);

                        if a == 0 {
                            break;
                        }

                        if a == p.len() && a == temp.len() {
                            curr = idx;
                            key = &key[a..];
                            continue 'outer;
                        }
                    }
                }
                None => {}
            }

            match lut.find(&":") {
                Some(idx) => {
                    let idx = xs[idx];
                    let node = &self.nodes[idx];
                    params.insert(&self.nodes[idx].path[1..], temp);
                    match after(key, "/") {
                        "" => match node.data {
                            Some(idx) => return Some(&self.data[idx]),
                            None => return None,
                        },
                        s => {
                            curr = idx;
                            key = s;
                            continue 'outer;
                        }
                    }
                }
                None => {}
            }

            match lut.find(&"*") {
                Some(idx) => {
                    let idx = xs[idx];
                    let node = &self.nodes[idx];
                    params.insert(&self.nodes[idx].path, temp);
                    match after(key, "/") {
                        "" => match node.data {
                            Some(idx) => return Some(&self.data[idx]),
                            None => return None,
                        },
                        s => {
                            curr = idx;
                            key = s;
                            continue 'outer;
                        }
                    }
                }
                None => return None,
            }
        }
    }

    pub fn insert<S>(&mut self, key: S, value: T)
    where
        S: AsRef<str>,
    {
        let key: Vec<_> = key.as_ref().split("/").filter(|s| s.len() != 0).collect();
        let mut active = key.as_slice();
        let mut curr = 0;

        'outer: loop {
            if active.len() == 0 {
                match self.nodes[curr].data {
                    Some(idx) => {
                        self.data[idx] = value;
                    }
                    None => {
                        let idx = self.data.insert(value);
                        self.nodes[curr].data = Some(idx);
                    }
                }
                break 'outer;
            }

            if self.nodes[curr].children.len() == 0 {
                let (start, rem) = longest(active);
                let node = Node::new(start, None, Vec::new());

                let pos = self.add_node(node);
                self.nodes[curr].children.push(pos);

                curr = pos;
                active = &active[rem..];

                continue 'outer;
            }

            let xs = self.nodes[curr].children.clone();

            for idx in xs {
                let num = lcs(&self.nodes[idx].path, active);
                let equal = eq(&self.nodes[idx].path, active);

                if num > 0 && !equal {
                    if self.nodes[idx].path.length() == num {
                        curr = idx;
                        active = &active[num..];
                        continue 'outer;
                    }

                    let sp = self.nodes[idx].path.after(num).to_string();
                    let rn = Node::from(
                        sp,
                        self.nodes[idx].data.take(),
                        self.nodes[idx].children.clone(),
                    );

                    let pos = self.add_node(rn);

                    self.nodes[idx].path = self.nodes[idx].path.from(num).to_string();
                    self.nodes[idx].children.clear();
                    self.nodes[idx].children.push(pos);

                    active = &active[num..];
                    let (joined, rem) = longest(active);
                    let node = Node::new(joined, None, Vec::new());

                    let pos = self.add_node(node);
                    self.nodes[idx].children.push(pos);

                    curr = pos;
                    active = &active[rem..];

                    continue 'outer;
                }

                if equal {
                    match self.nodes[idx].data {
                        Some(n) => {
                            self.data[n] = value;
                        }
                        None => {
                            let pos = self.add_data(value);
                            self.nodes[idx].data = Some(pos);
                        }
                    }

                    break 'outer;
                }

                let p = &self.nodes[idx].path.at(0)[0..1];
                match (p, &active[0][0..1]) {
                    (":", ":") | ("*", ":") => {
                        if &self.nodes[idx].path == &active[0] {
                            curr = idx;
                            active = &active[1..];
                            continue 'outer;
                        }
                        let node = Node::new(&active[0..1], None, self.nodes[idx].children.clone());
                        curr = idx;
                        active = &active[1..];

                        let prev = std::mem::replace(&mut self.nodes[idx], node);
                        if let Some(v) = prev.data {
                            self.data.remove(v);
                        }
                        for sub in &prev.children {
                            self.delete(*sub);
                        }
                    }
                    (":", "*") | ("*", "*") => {
                        let node = Node::new(&active[0..1], None, self.nodes[idx].children.clone());
                        curr = idx;
                        active = &active[1..];
                        let prev = std::mem::replace(&mut self.nodes[idx], node);
                        if let Some(v) = prev.data {
                            self.data.remove(v);
                        }
                        for sub in &prev.children {
                            self.delete(*sub);
                        }
                    }
                    _ => continue,
                }

                continue 'outer;
            }

            let (start, rem) = longest(active);
            let node = Node::new(start, None, Vec::new());

            let pos = self.add_node(node);
            self.nodes[curr].children.push(pos);

            curr = pos;
            active = &active[rem..];
        }
        self.sort_all();
    }

    pub fn remove(&mut self, curr: usize, key: &str) -> u8 {
        let key = if key.starts_with("/") { &key[1..] } else { key };

        if key.len() == 0 {
            if self.nodes[curr].children.len() > 0 {
                self.nodes[curr].data = None;
                return 0;
            }
            return 1;
        }

        let mut found = false;
        let mut index = 0;
        let mut p = 0;

        for (i, idx) in self.nodes[curr].children.iter().enumerate() {
            let temp = substr(key, "/");
            if self.nodes[*idx].path == temp {
                index = *idx;
                p = i;
                found = true;
                break;
            }
        }

        if !found {
            return 0;
        }

        let res = self.remove(index, &key[self.nodes[index].path.len()..]);

        if res == 1 {
            self.delete(index);
            self.nodes[curr].children.remove(p);
            return 2;
        }

        if res == 2 {
            self.nodes[curr].children.remove(p);
            if self.nodes[index].data.is_none() && self.nodes[index].children.len() == 0 {
                self.nodes.remove(index);
                return 2;
            }
        }

        0
    }

    fn delete(&mut self, idx: usize) {
        for sub in self.nodes[idx].children.clone() {
            self.delete(sub);
        }
        if let Some(v) = self.nodes[idx].data {
            self.data.remove(v);
        }
        self.nodes.remove(idx);
    }

    fn add_node(&mut self, node: Node) -> usize {
        self.nodes.insert(node)
    }

    fn add_data(&mut self, data: T) -> usize {
        self.data.insert(data)
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
                .map(|i| self.nodes[*i].path[0..1].to_string())
                .collect::<Vec<_>>()
                .join("");
            self.nodes[idx].index = xs;
            return;
        }

        let mut temp = self.nodes[idx].children.clone();
        temp.sort_by(|a, b| {
            let p_a = &self.nodes[*a].path;
            let p_b = &self.nodes[*b].path;
            if &p_a[0..1] == ":" || &p_a[0..1] == "*" && &p_b[0..1] != ":" || &p_b[0..1] != "*" {
                return std::cmp::Ordering::Greater;
            }
            if &p_b[0..1] == ":" || &p_b[0..1] == "*" && &p_a[0..1] != ":" || &p_a[0..1] != "*" {
                return std::cmp::Ordering::Less;
            }
            p_a.cmp(p_b)
        });

        let xs = temp
            .iter()
            .map(|i| self.nodes[*i].path[0..1].to_string())
            .collect::<Vec<_>>()
            .join("");

        // println!("xs {}", xs);
        self.nodes[idx].index = xs;
        self.nodes[idx].children = temp;
    }
}

#[derive(Debug)]
struct Node {
    path: String,
    index: String,
    data: Option<usize>,
    children: Vec<usize>,
}

impl Node {
    pub fn new(path: &[&str], data: Option<usize>, children: Vec<usize>) -> Self {
        Self {
            path: path.join("/"),
            data,
            index: String::new(),
            children,
        }
    }

    pub fn from(path: String, data: Option<usize>, children: Vec<usize>) -> Self {
        Self {
            path,
            data,
            index: String::new(),
            children,
        }
    }
}

fn match_left(a: &str, b: &str) -> usize {
    let min = std::cmp::min(a.len(), b.len());
    for i in 0..min {
        if &a[i..i + 1] != &b[i..i + 1] {
            return i;
        }
    }
    min
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
