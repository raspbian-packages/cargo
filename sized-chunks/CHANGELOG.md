# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2019-04-12

### ADDED

- `SparseChunk` now has a default length of `U64`.
- `Chunk` now has `PartialEq` defined for anything that can be borrowed as a
  slice.
- `SparseChunk<A>` likewise has `PartialEq` defined for `BTreeMap<usize, A>` and
  `HashMap<usize, A>`. These are intended for debugging and aren't optimally
  `efficient.
- `Chunk` and `SparseChunk` now have a new method `capacity()` which returns its
  maximum capacity (the number in the type) as a usize.
- Added an `entries()` method to `SparseChunk`.
- `SparseChunk` now has a `Debug` implementation.

### FIXED

- Extensive integration tests were added for `Chunk` and `SparseChunk`.
- `Chunk::clear` is now very slightly faster.

## [0.1.2] - 2019-03-11

### FIXED

- Fixed an alignment issue in `Chunk::drain_from_back`. (#1)

## [0.1.1] - 2019-02-19

### FIXED

- Some 2018 edition issues.

## [0.1.0] - 2019-02-19

Initial release.
