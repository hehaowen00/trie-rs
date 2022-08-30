# trie-rs

Implementation of trie data structures

## VecPathTrie
Implementation of `PathTrie` except using a single vector to hold all nodes to improve cache locality. Unlike `PathTrie`, `VecPathTrie` supports removal of paths.

```
vec-path-trie-get       time:   [65.588 µs 65.909 µs 66.237 µs]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

matchit_at              time:   [22.023 µs 22.133 µs 22.250 µs]
Found 5 outliers among 100 measurements (5.00%)
  2 (2.00%) high mild
  3 (3.00%) high severe
```
