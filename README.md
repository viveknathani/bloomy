# bloomy

bloomy is a personal storage engine laboratory written in Rust.

The goal is not to build the fastest or most production-ready database. The
goal is to understand database internals by implementing, testing, and
benchmarking storage engine designs in small, readable pieces.

Bloomy will grow as a collection of interchangeable components and engines:

- B+Trees
- LSM Trees
- buffer pools
- write-ahead logs
- bloom filters
- page managers
- compression strategies
- caching layers
- I/O backends

bloomy should be highly configurable through a JSON configuration file so storage
engine choices, component settings, and benchmark setups can be changed without
rewriting code.

A custom path can be supplied with `--config path/to/bloomy.json`. Without that,
Bloomy prefers `./bloomy.json` in the current working directory when present,
then falls back to `~/.config/bloomy/bloomy.json`. See
[docs/CONFIGURATION.md](docs/CONFIGURATION.md).

The first target is a simple LSM-based key-value store supporting:

- `put(key, value)`
- `get(key)`
- `delete(key)`
- range scans

The first LSM implementation should include:

- write-ahead log (WAL)
- in-memory memtable
- SSTable generation
- manifest metadata
- basic compaction
- crash recovery
- configuration file loading and validation

## license

Bloomy is licensed under the [MIT License](LICENSE).
