# bloomy

Bloomy is a personal storage engine laboratory written in Rust.

The goal is not to build the fastest or most production-ready database. The
goal is to understand database internals by implementing, testing, and
benchmarking storage engine designs in small, readable pieces.

## workspace

The repository is a Cargo workspace:

- `bloomy/`: the storage engine library, small binary, and benchmark binary.
- `bloomy-viewer/`: a terminal WAL viewer that reuses WAL parsing code from the
  `bloomy` crate.
- `docs/`: architecture, configuration, and roadmap notes for the whole lab.

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
cargo run -p bloomy --release --bin bench -- 10000
```

Example output:

```text
lsm engine benchmark
items: 10000

   writes:      10000 ops in    0.033s =       301536 ops/sec
    reads:      10000 ops in    0.003s =      3005888 ops/sec
    scans:        100 ops in    0.002s =        45336 ops/sec
scan rows:      10000 ops in    0.002s =      4533583 ops/sec
```

## WAL viewer

Run the terminal WAL viewer against a Bloomy WAL file:

```bash
cargo run -p bloomy-viewer -- ./data/bloomy.wal
```

The viewer tails by default. Use `--snapshot` for a one-shot view:

```bash
cargo run -p bloomy-viewer -- --snapshot ./data/bloomy.wal
```

## license

Bloomy is licensed under the [MIT License](LICENSE).
