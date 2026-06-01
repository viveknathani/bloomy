# ROADMAP

This roadmap is a working checklist for Bloomy. Items can be reordered as the
project teaches us more.

- [x] Create repository layout.
- [x] Add initial architecture docs.
- [x] Define the initial public API shape.
- [x] Add `Bloomy` open/close flow.
- [x] Choose JSON as the initial configuration file format.
- [x] Add configuration loading.
- [x] Add explicit defaults for all configurable settings.
- [x] Validate configuration before opening storage files.
- [x] Document a sample Bloomy configuration file.
- [ ] Add simple `put`, `get`, and `delete` API.
- [ ] Store data in an in-memory memtable only.
- [ ] Add unit tests for API behavior.
- [ ] Define WAL record format.
- [ ] Append mutations before applying them to the memtable.
- [ ] Replay WAL during startup.
- [ ] Add corruption and partial-record tests.
- [ ] Add sorted string table builder.
- [ ] Add SSTable reader.
- [ ] Flush memtables to SSTables.
- [ ] Support point lookups from SSTables.
- [ ] Define manifest record format.
- [ ] Track generated SSTables.
- [ ] Recover engine metadata on startup.
- [ ] Add memtable scans.
- [ ] Add SSTable scans.
- [ ] Merge visible records across active and immutable sources.
- [ ] Respect tombstones.
- [ ] Add a simple compaction policy.
- [ ] Merge SSTables and discard shadowed records.
- [ ] Update manifest atomically enough for the educational first version.
- [ ] Swap memtable implementations.
- [ ] Add bloom filters to SSTables.
- [ ] Try block caching.
- [ ] Try compression.
- [ ] Make engine/component selection configurable.
- [ ] Add benchmark configuration files for repeatable experiments.
- [ ] Add benchmarks for write-heavy, read-heavy, and scan-heavy workloads.
