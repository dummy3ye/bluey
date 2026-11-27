# Contributing to bluey

Thanks for helping out. The short version:

1. Fork, branch, hack.
2. Run `make check` (format, clippy, tests) before opening a pull request.
3. Keep commits focused and write a clear description.

## Project layout

| Path                | Responsibility                                           |
| ------------------- | -------------------------------------------------------- |
| `src/main.rs`       | Entry point, terminal setup, the async event loop        |
| `src/bluetooth.rs`  | BlueZ backend: commands in, snapshots and notices out    |
| `src/app.rs`        | Application state and key handling (no I/O, no drawing) |
| `src/input.rs`      | Terminal input thread                                    |
| `src/theme.rs`      | Colour palettes                                          |
| `src/ui/`           | Pure rendering; one file per panel                       |

## Guidelines

- `app.rs` never touches the terminal and `ui/` never mutates state other than
  scroll/selection widgets. This keeps both easy to test.
- All Bluetooth I/O goes through `bluetooth::Handle`. Slow operations (connect,
  pair) must run as background jobs so the UI never blocks.
- New colours belong in `theme.rs`; do not hard-code colours in `ui/`.
- No emoji in the interface. Use plain Unicode box/block/geometric glyphs.

## Adding a theme

Add an entry to `THEMES` in `src/theme.rs` and bump the array length.
