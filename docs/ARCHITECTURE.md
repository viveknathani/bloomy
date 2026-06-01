# ARCHITECTURE

bloomy is organized around swappable storage engine components. The first
implementation will be an LSM key-value store, but the repository should leave
room for alternative engines and internal implementations as the project grows.

## design principles

- Keep the public API stable and small.
- Keep implementation modules explicit and easy to read.
- Introduce traits only where they describe a real seam between components.
- Prefer deterministic tests over complex integration harnesses.
- Make on-disk formats documented before relying on them.
- Make engine and component choices configurable through files, not hard-coded.

## initial module boundaries

### `api`

The public key-value interface lives here. It should describe what a database
can do without exposing how any specific engine stores data.

The expected responsibilities are:

- database options
- `put`, `get`, `delete`, and scan-facing types
- key and value type aliases or wrappers

The initial public API uses owned byte vectors for stored keys and values.
Point reads and deletes accept borrowed byte slices. Range scans use an
inclusive start bound and exclusive end bound when those bounds are present.

### `config`

JSON configuration loading and validation lives here. The configuration file is
named `bloomy.json`. The binary accepts `--config path/to/bloomy.json` and
otherwise prefers `./bloomy.json` from the current working directory when
present. If no local file exists, it uses `~/.config/bloomy/bloomy.json`.

The configuration file should describe engine selection, storage paths,
memtable limits, compaction settings, cache sizes, bloom filter options, and
I/O backend choices.

The code should keep parsed configuration separate from runtime state. Defaults
should be explicit, validated, and easy to print for debugging.

### `engine`

Concrete engines live here. The first engine will be `engine::lsm`.

The expected responsibilities are:

- engine-level orchestration
- write path coordination
- read path coordination
- recovery flow
- compaction scheduling in a simple synchronous form

### `storage`

Reusable storage components live here.

The expected responsibilities are:

- WAL records and replay
- memtable implementations
- SSTable builders and readers
- manifest records
- bloom filters
- compaction helpers

### `io`

I/O backends live here. The first backend should be normal synchronous file I/O.

The expected responsibilities are:

- file creation/opening
- append and read helpers
- sync/fsync behavior
- eventually, alternate backends for experiments

## initial LSM Shape

The first LSM should use a simple architecture:

1. Append mutations to the WAL.
2. Apply mutations to an in-memory memtable.
3. Flush the memtable into an immutable SSTable when it reaches a size limit.
4. Record SSTable metadata in the manifest.
5. Serve reads from the active memtable, immutable memtables, and SSTables.
6. Compact SSTables with a basic policy.
7. Recover by loading the manifest and replaying WAL records.

## non-goals for the first version

- async I/O
- lock-free data structures
- multi-version concurrency control
- transactions
- background compaction threads
- highly generic storage traits
- production-grade crash consistency

These can be explored later once the basic implementation is easy to reason
about and covered by tests.
