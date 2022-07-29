use super::params::Params;

#[derive(Debug, Eq, PartialEq)]
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
        if key.len() == 0 {
            match &self.data {
                Some(data) => return Some(&data),
                None => return None,
            }
        }

        // search for a static string that matches the key
        // search for parameter node
        // search for wildcard node

        if self.children.len() == 0 {
            return None;
        }

        let rem = if key.starts_with("/") { &key[1..] } else { key };

        for node in &self.children {
            if &node.path[0..1] == ":" {
                continue;
            }

            if &node.path[0..1] == "*" {
                continue;
            }

            if rem.starts_with(&node.path) {
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

    fn insert_xs(&mut self, keys: &[String], value: T) {
        if keys.len() == 0 {
            self.data = Some(value);
            return;
        }

        for node in self.children.iter_mut() {
            if node.path == keys[0] {
                node.insert_xs(&keys[1..], value);
                return;
            }

            match (&node.path[0..1], &keys[0][0..1]) {
                (":", ":") | ("*", ":") => {
                    let mut new = PathTrie::new(keys[0].clone());
                    new.insert_xs(&keys[1..], value);
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
        new.insert_xs(&keys[1..], value);
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
        if self.children.len() == 1 {
            let node = &self.children[0];
            if self.data.is_none() && &node.path[0..1] == ":" || &node.path[0..1] == "*" {
                return;
            }

            let mut node = self.children.remove(0);
            node.compress_node();

            self.path.push_str("/");
            self.path.push_str(&node.path);
            self.data = node.data;
            self.children = node.children;
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
        self.root.insert_xs(&xs, value);
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

// #[test]
fn test_parse() {
    let res = parse_key("/api/hello/:name/:age/*").unwrap();
    assert_eq!(
        res,
        vec![
            "api/hello/".to_string(),
            ":name".to_string(),
            ":age".to_string(),
            "*".to_string()
        ]
    );

    let res = parse_key("/api/hello/*").unwrap();
    assert_eq!(res, vec!["api/hello/".to_string(), "*".to_string(),]);

    let res = parse_key("/api/hello/*/err").unwrap();
    assert_eq!(
        res,
        vec![
            "api/hello/".to_string(),
            "*".to_string(),
            "err/".to_string()
        ]
    );

    let res = parse_key("/query/*").unwrap();
    assert_eq!(res, vec!["query/".to_string(), "*".to_string()]);
}

// #[test]
// fn test_node() {
//     let mut node = Node::new(Vec::new());
//
//     let keys = parse_key("/api/hello/:name").unwrap();
//     node.insert(&keys, 1);
//
//     let keys = parse_key("/api/goodbye/:name/:age").unwrap();
//     node.insert(&keys, 2);
//
//     let keys = parse_key("/api/hello/:name/:age").unwrap();
//     node.insert(&keys, 3);
//
//     let keys = parse_key("/api/hello/:name/:age").unwrap();
//     node.insert(&keys, 6);
//
//     let keys = parse_key("/a/b/*").unwrap();
//     node.insert(&keys, 4);
//
//     let keys = parse_key("/api/hello").unwrap();
//     node.insert(&keys, 0);
//
//     let keys = parse_key("/:id/collections").unwrap();
//     node.insert(&keys, 8);
//
//     // let keys = parse_key("/:name/collections").unwrap();
//     // node.insert(&keys, 8);
//
//     let res = Node {
//         path: vec![],
//         data: None,
//         children: vec![
//             Node {
//                 path: vec![":id".to_string(), "collections/".to_string()],
//                 data: Some(8),
//                 children: vec![],
//             },
//             Node {
//                 path: vec!["a".to_string()],
//                 data: None,
//                 children: vec![
//                     Node {
//                         path: vec!["/b/".to_string(), "*".to_string()],
//                         data: Some(4),
//                         children: vec![],
//                     },
//                     Node {
//                         path: vec!["pi/".to_string()],
//                         data: None,
//                         children: vec![
//                             Node {
//                                 path: vec![
//                                     "goodbye/".to_string(),
//                                     ":name".to_string(),
//                                     ":age".to_string(),
//                                 ],
//                                 data: Some(2),
//                                 children: vec![],
//                             },
//                             Node {
//                                 path: vec!["hello/".to_string()],
//                                 data: Some(0),
//                                 children: vec![Node {
//                                     path: vec![":name".to_string()],
//                                     data: Some(1),
//                                     children: vec![Node {
//                                         path: vec![":age".to_string()],
//                                         data: Some(6),
//                                         children: vec![],
//                                     }],
//                                 }],
//                             },
//                         ],
//                     },
//                 ],
//             },
//         ],
//     };
//
//     assert_eq!(node, res);
// }

// #[test]
// fn test_node_get() {
//     let trie = Node {
//         path: vec![],
//         data: None,
//         children: vec![Node {
//             path: vec!["/a".to_string()],
//             data: None,
//             children: vec![
//                 Node {
//                     path: vec!["/b/".to_string(), "*".to_string()],
//                     data: Some(6),
//                     children: vec![],
//                 },
//                 Node {
//                     path: vec!["pi/".to_string()],
//                     data: None,
//                     children: vec![
//                         Node {
//                             path: vec![
//                                 "goodbye/".to_string(),
//                                 ":name".to_string(),
//                                 ":age".to_string(),
//                             ],
//                             data: Some(2),
//                             children: vec![],
//                         },
//                         Node {
//                             path: vec!["hello".to_string()],
//                             data: Some(0),
//                             children: vec![Node {
//                                 path: vec![":name".to_string()],
//                                 data: Some(1),
//                                 children: vec![Node {
//                                     path: vec![":age".to_string()],
//                                     data: Some(3),
//                                     children: vec![],
//                                 }],
//                             }],
//                         },
//                     ],
//                 },
//             ],
//         }],
//     };
//
//     let (r, params) = trie.get("/api/hello/world").unwrap();
//     assert_eq!(r, &1);
//     assert_eq!(params.get("name"), Some("world"));
//
//     let (r, params) = trie.get("/api/goodbye/world/2").unwrap();
//     assert_eq!(r, &2);
//     assert_eq!(params.get("name"), Some("world"));
//     assert_eq!(params.get("age"), Some("2"));
//
//     let (r, params) = trie.get("/api/hello/world/2").unwrap();
//     assert_eq!(r, &3);
//     assert_eq!(params.get("name"), Some("world"));
//     assert_eq!(params.get("age"), Some("2"));
//
//     let (r, _params) = trie.get("/a/b/string").unwrap();
//     assert_eq!(r, &4);
// }
