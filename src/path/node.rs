use super::params::Params;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PathTrie<T> {
    path: String,
    data: Option<T>,
    children: Vec<Self>,
    compressed: bool,
}

impl<T> PathTrie<T> {
    #[inline]
    pub(crate) fn new(path: String) -> Self {
        Self {
            path,
            data: None,
            children: Vec::new(),
            compressed: false,
        }
    }

    pub fn builder() -> TrieBuilder<T> {
        TrieBuilder::new()
    }

    pub fn children(&self) -> &[Self] {
        &self.children
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn get<'a, 'b>(&'a self, key: &'b str) -> Option<(&T, Params<'a, 'b>)> {
        let mut params = Params::new();
        match self.get_params(&mut params, key) {
            Some(data) => Some((data, params)),
            None => None,
        }
    }

    fn get_params<'a, 'b>(&'a self, params: &mut Params<'a, 'b>, key: &'b str) -> Option<&T> {
        let rem = if key.starts_with("/") { &key[1..] } else { key };

        if rem.len() == 0 {
            return self.data.as_ref();
        }

        if self.children.len() == 0 {
            return None;
        }

        for node in &self.children {
            if &node.path[0..1] == ":" || &node.path[0..1] == "*" {
                continue;
            }

            let temp = substr(rem, "/");

            if node.path.starts_with(temp) {
                return node.get_params(params, &rem[node.path.len()..]);
            }
        }

        if self.children[0].path.starts_with(":") {
            params.insert(&self.children[0].path[1..], substr(rem, "/"));

            return match after(rem, "/") {
                "" => self.children[0].data.as_ref(),
                s => self.children[0].get_params(params, s),
            };
        }

        if self.children[0].path == "*" {
            params.insert(&self.children[0].path[0..1], rem);
            return self.children[0].data.as_ref();
        }

        None
    }

    fn insert(&mut self, keys: &[String], value: T) {
        if keys.len() == 0 {
            self.data = Some(value);
            return;
        }

        for node in self.children.iter_mut() {
            if node.path == keys[0] {
                node.insert(&keys[1..], value);
                return;
            }

            match (&node.path[0..1], &keys[0][0..1]) {
                (":", ":") | ("*", ":") => {
                    let mut new = PathTrie::new(keys[0].clone());
                    new.insert(&keys[1..], value);
                    let _ = std::mem::replace(node, new);
                }
                (":", "*") | ("*", "*") => {
                    let mut new = PathTrie::new(keys[0].clone());
                    new.data = Some(value);
                    let _ = std::mem::replace(node, new);
                }
                _ => continue,
            }

            self.children.sort_by_key(|e| e.path.clone());
            return;
        }

        let mut new = PathTrie::new(keys[0].clone());
        new.insert(&keys[1..], value);
        self.children.push(new);
        self.children.sort_by_key(|e| e.path.clone());
    }

    pub(crate) fn compress(&mut self) {
        self.compressed = true;

        for node in self.children.iter_mut() {
            node.compress_node();
        }
    }

    fn compress_node(&mut self) {
        match self.len() {
            1 => {
                let node = &self.children[0];

                if self.data.is_none() && self.path == "*"
                    || &self.path[0..1] == ":"
                    || &node.path[0..1] == ":"
                    || &node.path[0..1] == "*"
                {
                    return;
                }

                let mut node = self.children.remove(0);
                node.compress_node();

                self.path.push_str("/");
                self.path.push_str(&node.path);
                self.data = node.data;
                self.children = node.children;
            }
            _ => self.compress(),
        }
    }
}

pub struct TrieBuilder<T> {
    root: PathTrie<T>,
}

impl<T> TrieBuilder<T> {
    pub fn new() -> Self {
        Self {
            root: PathTrie::new(String::new()),
        }
    }

    pub fn insert<S>(&mut self, key: S, value: T)
    where
        S: AsRef<str>,
    {
        let key = key.as_ref();
        let xs = parse_key(key).unwrap();
        self.root.insert(&xs, value);
    }

    pub fn finalize(mut self) -> PathTrie<T> {
        self.root.compress();
        self.root
    }
}

pub fn substr<'a, 'b>(a: &'a str, b: &'b str) -> &'a str {
    for i in 0..a.len() {
        if a[i..].starts_with(b) {
            return &a[0..i];
        }
    }

    a
}

pub fn after<'a, 'b>(a: &'a str, b: &'b str) -> &'a str {
    for i in 0..(a.len() - b.len()) {
        if a[i..].starts_with(b) {
            return &a[i..];
        }
    }
    &a[a.len()..]
}

pub fn lcs<'a, 'b>(a: &'a str, b: &'b str) -> usize {
    let min = std::cmp::min(a.len(), b.len());
    for i in 0..min {
        if &a[i..i + 1] != &b[i..i + 1] {
            return i;
        }
    }

    min
}

#[derive(Debug)]
pub enum PathParseError<'a> {
    InsufficientLength,
    UnexpectedToken(&'a str),
}

// assumes key always starts with '/'
pub fn parse_key<'a>(key: &'a str) -> Result<Vec<String>, PathParseError<'a>> {
    let parts = key.split("/");
    let mut end = false;
    let mut xs = Vec::new();

    for p in parts.filter(|s| s != &"").filter(|s| s != &"/") {
        if end {
            return Err(PathParseError::UnexpectedToken(p));
        }

        if p == "" {
            continue;
        }

        match &p[0..1] {
            ":" => {
                if p.len() == 1 {
                    return Err(PathParseError::InsufficientLength);
                }
                xs.push(p.to_string());
            }
            "*" => {
                xs.push(p.to_string());
                end = true;
            }
            _ => {
                xs.push(p.trim_start_matches('/').trim_end_matches('/').to_string());
            }
        }
    }

    Ok(xs)
}

#[test]
fn test_node_get() {
    let mut builder = PathTrie::builder();

    builder.insert("/api/todos", 1);
    builder.insert("/api/todo/:id", 2);

    builder.insert("/api/lists", 3);
    builder.insert("/api/list/:id", 4);

    builder.insert("/api/auth/register", 5);
    builder.insert("/api/auth/login", 6);
    builder.insert("/api/auth/logout", 7);

    builder.insert("/a/b/c/d/e/*", 8);
    builder.insert("/a/d/c/d/e/*", -8);
    builder.insert("/a/de/c/d/e/*", -9);
    builder.insert("/a/de/d/d/e/*", -9);
    builder.insert("/a/def/d/d/e/*", -10);
    builder.insert("/api/hello/:name", 9);
    builder.insert("/api/hello/:name/:addr", 10);
    builder.insert("/api/hello/:name/:age", 11);
    builder.insert("/api/hello/:name/addr", 12);
    builder.insert("/api/goodbye/:name", 13);

    builder.insert("*", 404);
    builder.insert("/:user/profile", 14);

    let trie = builder.finalize();
    println!("{:#?}", trie);

    assert_eq!(trie.children()[0].path(), ":user");

    match trie.get("/name/profile") {
        Some((14, p)) if p.get("user") == Some("name") => assert!(true),
        n => panic!("{:?}", n),
    };

    match trie.get("/api/todos") {
        Some((1, _)) => assert!(true),
        n => panic!("{:?}", n),
    };

    match trie.get("/api/todo/5") {
        Some((2, p)) if p.get("id") == Some("5") => assert!(true),
        n => panic!("{:?}", n),
    };

    match trie.get("/api/todo/a2") {
        Some((2, p)) if p.get("id") == Some("a2") => assert!(true),
        n => panic!("{:?}", n),
    };

    match trie.get("/a/b/c/d/e/wildcard") {
        Some((8, p)) if p.get("*") == Some("wildcard") => assert!(true),
        n => panic!("{:?}", n),
    }
    match trie.get("/a/def/d/d/e/wildcard") {
        Some((-10, p)) if p.get("*") == Some("wildcard") => assert!(true),
        n => panic!("{:?}", n),
    }

    match trie.get("/api/hello/world/1") {
        Some((11, p)) => {
            let name = p.get("name");
            assert_eq!(name, Some("world"));

            let age = p.get("age");
            assert_eq!(age, Some("1"));
        }
        n => panic!("{:?}", n),
    }
}
