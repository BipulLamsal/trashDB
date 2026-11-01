# TrashDB

An exploratory, experimental database. The goal is to learn from the best existing ideas by building them: the ups and downs, the IO, the concurrency, and the internals behind the policies and algorithms.

## Roadmap

- [x] LRU replacement
- [x] `Replacer` eviction trait
- [x] Tests for LRU
- [x] LRU approximation with a clock hand and reference bit with unit test 
- [x] [ARC](https://www.cs.cmu.edu/~natassa/courses/15-721/papers/arcfast.pdf), covering both frequency and recency

