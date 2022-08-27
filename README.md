# trie-rs

Implementation of trie data structures

## VecPathTrie
Implementation of `PathTrie` except using a single vector to hold all nodes to improve cache locality. Unlike `PathTrie`, `VecPathTrie` supports removal of paths.

```
vec-path-trie-get       time:   [86.922 µs 87.539 µs 88.167 µs]                              
Found 7 outliers among 100 measurements (7.00%)
  7 (7.00%) high mild

matchit_at              time:   [22.023 µs 22.133 µs 22.250 µs]
Found 5 outliers among 100 measurements (5.00%)
  2 (2.00%) high mild
  3 (3.00%) high severe
```