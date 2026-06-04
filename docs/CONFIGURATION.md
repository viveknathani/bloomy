# CONFIGURATION

Bloomy uses a JSON configuration file named `bloomy.json`.

The binary loads configuration in this order:

1. A supplied path: `bloomy --config path/to/bloomy.json`
2. `./bloomy.json` in the current working directory, if present
3. Default path: `~/.config/bloomy/bloomy.json`

If the selected file does not exist, Bloomy creates it with default settings.

In the workspace, run the engine binary with `cargo run -p bloomy`. The
configuration lookup still uses the process current working directory, so a
root-level `./bloomy.json` is used when running commands from the repository
root.

## sample

```json
{
  "storage_path": "./data",
  "memtable_bytes": 4194304
}
```

## fields

- `storage_path`: directory where Bloomy stores engine files.
- `memtable_bytes`: maximum target size of the active memtable before flush, must be greater than zero.

The active LSM engine writes `bloomy.wal` inside `storage_path`. The terminal
viewer reads WAL files directly and does not load `bloomy.json`.
