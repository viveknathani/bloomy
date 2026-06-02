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

## configuration

bloomy should be highly configurable through a JSON configuration file so storage
engine choices, component settings, and benchmark setups can be changed without
rewriting code.

A custom path can be supplied with `--config path/to/bloomy.json`. Without that,
Bloomy prefers `./bloomy.json` in the current working directory when present,
then falls back to `~/.config/bloomy/bloomy.json`. See
[docs/CONFIGURATION.md](docs/CONFIGURATION.md).

## benchmarks

Run the simple LSM engine benchmark with:

```bash
cargo run --release --bin bench -- 10000
```

Example output:

```text
lsm engine benchmark
items: 10000

   writes:      10000 ops in    0.011s =       871707 ops/sec
    reads:      10000 ops in    0.010s =      1042264 ops/sec
    scans:        100 ops in    0.001s =        71713 ops/sec
scan rows:      10000 ops in    0.001s =      7171317 ops/sec
```

## license

Bloomy is licensed under the [MIT License](LICENSE).
