use crate::trie::TrieNode;
use crate::TrieExt;

#[test]
fn trie_test() {
    let mut trie = TrieNode::new(0);
    trie.insert(&[1, 2, 3], ());
    trie.insert(&[1, 2, 3, 4], ());
    trie.insert(&[1, 2, 4], ());
    trie.insert(&[2, 2, 4], ());
    let removed = trie.remove(&[1, 2, 3], true);
    assert!(removed.is_some());
}
