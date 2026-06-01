# CONFIGURATION

bloomy uses a JSON configuration file named `bloomy.json`.

The binary loads configuration in this order:

1. A supplied path: `bloomy --config path/to/bloomy.json`
2. `./bloomy.json` in the current working directory, if present
3. Default path: `~/.config/bloomy/bloomy.json`

If the selected file does not exist, Bloomy creates it with default settings.

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
