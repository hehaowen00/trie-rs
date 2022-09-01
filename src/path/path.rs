use crate::params::Params;
use slab::Slab;
use std::time::Instant;

#[derive(Debug)]
pub struct PathTrie<T> {
    index: String,
    children: Vec<usize>,
    nodes: Slab<Node<T>>,
}

impl<T> PathTrie<T> {
    pub fn new() -> Self {
        Self {
            index: String::new(),
            children: Vec::new(),
            nodes: Slab::new(),
        }
    }

    pub fn get<'a, 'b>(&'a self, key: &'b str) -> Option<(&T, Params<'a, 'b>)> {
        let mut params = Params::new();

        let key = if key.starts_with("/") { &key[1..] } else { key };

        if key.len() == 0 {
            return None;
        }

        let lut: &str = self.index.as_ref();

        if lut.len() == 0 {
            return None;
        }

        let xs: &[usize] = self.children.as_ref();

        let n = match key.find("/") {
            Some(n) => n,
            None => key.len(),
        };

        match lut.find(&key[0..1]) {
            Some(start) => {
                for idx in start..lut.len() {
                    let idx = xs[idx];
                    let el = &self.nodes[idx].path;

                    if el.len() < n {
                        continue;
                    }

                    if &el[0..1] != &key[0..1] {
                        break;
                    }

                    if key.starts_with(el) {
                        // return self.get_params(idx, &mut params, &key[el.len()..]);
                        return match self.get_params(idx, &mut params, &key[el.len()..]) {
                            Some(data) => Some((data, params)),
                            None => None,
                        };
                        // curr = idx;
                        // key = &key[el.len()..];
                        // continue 'outer;
                    }
                }
            }
            None => {}
        }

        match lut.find(&":") {
            Some(idx) => {
                let idx = xs[idx];
                let node = &self.nodes[idx];
                params.insert(&node.path[1..], &key[0..n]);

                // curr = idx;
                // key = &key[n..];
                // continue 'outer;
                // return self.get_params(idx, &mut params, &key[n..]);

                return match self.get_params(idx, &mut params, &key[n..]) {
                    Some(data) => Some((data, params)),
                    None => None,
                };
            }
            None => {}
        }

        match lut.find(&"*") {
            Some(idx) => {
                let idx = xs[idx];
                let node = &self.nodes[idx];
                params.insert(&node.path, key);
                return match node.data.as_ref() {
                    Some(data) => Some((data, params)),
                    None => None,
                };
            }
            None => return None,
        }
        // match self.get_params(&mut params, key) {
        //     Some(data) => Some((data, params)),
        //     None => None,
        // }
    }

    fn get_params<'a, 'b>(
        &'a self,
        mut curr: usize,
        params: &mut Params<'a, 'b>,
        key: &'b str,
    ) -> Option<&T> {
        let mut key = key;

        'outer: loop {
            key = if key.starts_with("/") { &key[1..] } else { key };

            if key.len() == 0 {
                return self.nodes[curr].data.as_ref();
            }

            let lut: &str = self.nodes[curr].index.as_ref();

            if lut.len() == 0 {
                return None;
            }

            let xs: &[usize] = self.nodes[curr].children.as_ref();

            let n = match key.find("/") {
                Some(n) => n,
                None => key.len(),
            };

            match lut.find(&key[0..1]) {
                Some(start) => {
                    for idx in start..lut.len() {
                        let idx = xs[idx];
                        let el = &self.nodes[idx].path;

                        if el.len() < n {
                            continue;
                        }

                        if &el[0..1] != &key[0..1] {
                            break;
                        }

                        if key.starts_with(el) {
                            curr = idx;
                            key = &key[el.len()..];
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
                    params.insert(&node.path[1..], &key[0..n]);

                    curr = idx;
                    key = &key[n..];
                    continue 'outer;
                }
                None => {}
            }

            match lut.find(&"*") {
                Some(idx) => {
                    let idx = xs[idx];
                    let node = &self.nodes[idx];
                    params.insert(&node.path, key);
                    return node.data.as_ref();
                }
                None => return None,
            }
        }
    }

    pub fn sort_self(&mut self) {
        if self.children.len() == 1 {
            let xs = [self.children[0]]
                .iter()
                .map(|i| self.nodes[*i].path[0..1].to_string())
                .collect::<Vec<_>>()
                .join("");
            self.index = xs;
            return;
        }

        let mut children = self.children.clone();

        children.sort_by(|a, b| {
            let p_a = &self.nodes[*a].path;
            let p_b = &self.nodes[*b].path;
            if &p_a[0..1] == ":" || &p_a[0..1] == "*" && &p_b[0..1] != ":" || &p_b[0..1] != "*" {
                return std::cmp::Ordering::Greater;
            }
            if &p_b[0..1] == ":" || &p_b[0..1] == "*" && &p_a[0..1] != ":" || &p_a[0..1] != "*" {
                return std::cmp::Ordering::Less;
            }
            if &p_a[0..1] == &p_b[0..1] {
                return self.count_children(*a).cmp(&self.count_children(*b));
            }
            p_a.cmp(p_b)
        });

        let index = children
            .iter()
            .map(|i| self.nodes[*i].path[0..1].to_string())
            .collect::<Vec<_>>()
            .join("");

        self.index = index;
        self.children = children;
    }

    pub fn insert<S>(&mut self, key: S, value: T)
    where
        S: AsRef<str>,
    {
        let key: Vec<_> = key.as_ref().split("/").filter(|s| s.len() != 0).collect();
        let mut active = key.as_slice();

        if self.children.len() == 0 {
            let (start, rem) = longest(active);
            let node = Node::new(start, Vec::new());

            let pos = self.nodes.insert(node);
            self.children.push(pos);

            // active = &active[rem..];
            self.insert_child(pos, &active[rem..], value);
            return;
        }

        let xs = self.children.clone();

        for idx in xs {
            let num = lcs(&self.nodes[idx].path, active);
            let equal = eq(&self.nodes[idx].path, active);

            if num > 0 && !equal {
                if self.nodes[idx].path.length() == num {
                    // curr = idx;
                    // active = &active[num..];
                    // continue 'outer;
                    self.insert_child(idx, &active[num..], value);
                    return;
                }

                let subpath = self.nodes[idx].path.after(num).to_string();
                let children = std::mem::replace(&mut self.nodes[idx].children, Vec::new());

                let mut right = Node::from(subpath, children);
                right.data = self.nodes[idx].data.take();

                let pos = self.nodes.insert(right);

                self.nodes[idx].path = self.nodes[idx].path.from(num).to_string();
                self.nodes[idx].children.push(pos);

                active = &active[num..];
                let (joined, rem) = longest(active);
                let node = Node::new(joined, Vec::new());

                let pos = self.nodes.insert(node);
                self.nodes[idx].children.push(pos);

                self.insert_child(pos, &active[rem..], value);
                return;
                // curr = pos;
                // active = &active[rem..];

                // continue 'outer;
            }

            if equal {
                self.nodes[idx].data = Some(value);
                return;
                // break 'outer;
            }

            let p = &self.nodes[idx].path.at(0)[0..1];
            match (p, &active[0][0..1]) {
                (":", ":") | ("*", ":") => {
                    if &self.nodes[idx].path == &active[0] {
                        // curr = idx;
                        // active = &active[1..];
                        // continue 'outer;

                        self.insert_child(idx, &active[1..], value);
                        return;
                    }
                    let node = Node::new(&active[0..1], self.nodes[idx].children.clone());
                    // curr = idx;
                    // active = &active[1..];

                    let prev = std::mem::replace(&mut self.nodes[idx], node);
                    for sub in &prev.children {
                        self.delete(*sub);
                    }
                    self.insert_child(idx, &active[1..], value);
                    return;
                }
                (":", "*") | ("*", "*") => {
                    let node = Node::new(&active[0..1], self.nodes[idx].children.clone());
                    // curr = idx;
                    // active = &active[1..];

                    let prev = std::mem::replace(&mut self.nodes[idx], node);
                    for sub in &prev.children {
                        self.delete(*sub);
                    }
                    self.insert_child(idx, &active[1..], value);
                    return;
                }
                _ => continue,
            }

            // continue 'outer;
        }
        let (start, rem) = longest(active);
        let node = Node::new(start, Vec::new());

        let pos = self.nodes.insert(node);
        self.children.push(pos);
        self.insert_child(pos, &active[rem..], value);

        // curr = pos;
        // active = &active[rem..];
    }

    pub fn insert_child(&mut self, mut curr: usize, key: &[&str], value: T) {
        let mut active = key;

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
                let num = lcs(&self.nodes[idx].path, active);
                let equal = eq(&self.nodes[idx].path, active);

                if num > 0 && !equal {
                    if self.nodes[idx].path.length() == num {
                        curr = idx;
                        active = &active[num..];
                        continue 'outer;
                    }

                    let subpath = self.nodes[idx].path.after(num).to_string();
                    let children = std::mem::replace(&mut self.nodes[idx].children, Vec::new());

                    let mut right = Node::from(subpath, children);
                    right.data = self.nodes[idx].data.take();

                    let pos = self.nodes.insert(right);

                    self.nodes[idx].path = self.nodes[idx].path.from(num).to_string();
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

                let p = &self.nodes[idx].path.at(0)[0..1];
                match (p, &active[0][0..1]) {
                    (":", ":") | ("*", ":") => {
                        if &self.nodes[idx].path == &active[0] {
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
        self.sort_self();
    }

    pub fn remove(&mut self, curr: usize, key: &str) -> u8 {
        let key = if key.starts_with("/") { &key[1..] } else { key };

        if key.len() == 0 {
            if self.nodes[curr].children.len() > 0 {
                return 0;
            }
            return 1;
        }

        let mut found = false;
        let mut index = 0;
        let mut p = 0;

        for (i, idx) in self.nodes[curr].children.iter().enumerate() {
            let temp = find(key);
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
                .map(|i| self.nodes[*i].path[0..1].to_string())
                .collect::<Vec<_>>()
                .join("");
            self.nodes[idx].index = xs;
            return;
        }

        let mut children = self.nodes[idx].children.clone();

        children.sort_by(|a, b| {
            let p_a = &self.nodes[*a].path;
            let p_b = &self.nodes[*b].path;
            if &p_a[0..1] == ":" || &p_a[0..1] == "*" && &p_b[0..1] != ":" || &p_b[0..1] != "*" {
                return std::cmp::Ordering::Greater;
            }
            if &p_b[0..1] == ":" || &p_b[0..1] == "*" && &p_a[0..1] != ":" || &p_a[0..1] != "*" {
                return std::cmp::Ordering::Less;
            }
            if &p_a[0..1] == &p_b[0..1] {
                return self.count_children(*a).cmp(&self.count_children(*b));
            }
            p_a.cmp(p_b)
        });

        let index = children
            .iter()
            .map(|i| self.nodes[*i].path[0..1].to_string())
            .collect::<Vec<_>>()
            .join("");

        self.nodes[idx].index = index;
        self.nodes[idx].children = children;
    }
}

pub fn parse_xs<'a>(path: &'a str) -> Vec<&'a str> {
    let mut idx = 0;
    let mut xs = Vec::new();
    let mut rem = path;

    while idx != path.len() {
        if let Some(mut n) = &rem[1..].find("/") {
            let mut el = &rem[0..n + 2];
            if el.starts_with(":") {
                el = &rem[0..n + 1];
            }
            xs.push(el);
            rem = &rem[n + 2..];
            println!("{:?} {}", xs, rem);
            idx = n + 1;
        } else {
            xs.push(&rem);
            break;
        }
    }

    xs
}

#[derive(Debug)]
struct Node<T> {
    path: String,
    index: String,
    data: Option<T>,
    children: Vec<usize>,
}

impl<T> Node<T> {
    pub fn new(path: &[&str], children: Vec<usize>) -> Self {
        Self {
            path: path.join("/"),
            data: None,
            index: String::new(),
            children,
        }
    }

    pub fn from(path: String, children: Vec<usize>) -> Self {
        Self {
            path,
            data: None,
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

#[inline]
fn find<'a>(a: &'a str) -> &'a str {
    match a.find("/") {
        Some(n) => &a[0..n],
        None => a,
    }
}

fn after<'a>(a: &'a str) -> &'a str {
    match a.find("/") {
        Some(n) => &a[n..],
        None => &a[a.len()..],
    }
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
