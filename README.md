<h1 align="center">&#5842; bluey</h1>
<p align="center">A beautiful, keyboard-driven Bluetooth manager for your terminal.<br>
Built with <a href="https://ratatui.rs">Ratatui</a> and <a href="https://www.bluez.org">BlueZ</a>.</p>

---

```
╭──────────────────────────────────────────────────────────────────────────────╮
│ bluey - v0.1.0                                    hci0  power on  scanning   │
╰──────────────────────────────────────────────────────────────────────────────╯
╭ Devices ───────────────────────────╮╭ Details ─────────────────────────────╮
│   Name            Type       Sig   ││ WH-1000XM5                           │
│ ▌ ● WH-1000XM5    Headphones ▂▄▆█  ││ Headphones                           │
│   ● MX Keys       Keyboard   ▂▄▆█  ││ Address     AA:BB:CC:DD:EE:FF        │
│   ○ Pixel 8       Phone      ▂▄    ││ Signal      ▂▄▆█  -48 dBm  excellent │
╰────────────────────────────────────╯╰──────────────────────────────────────╯
```

## Features

- Live device list with signal bars, type, address and state, sorted so the rows never jump
- Connect, disconnect, pair, trust and forget devices
- Adapter controls: power, scan, discoverable
- Built-in pairing agent (just-works and confirmation pairing)
- Details, adapter and activity panels
- Three themes (Midnight, Nord, Ember), filters, scrollbar, help overlay
- Config file for theme and transparent background
- Non-blocking: slow operations run in the background

## Requirements

- Linux with BlueZ 5.x (`bluetooth.service` running)
- A true-colour terminal (`COLORTERM=truecolor`) and a font with Braille and block glyphs
- Rust 1.80+ and the D-Bus development files to build

| Distro        | Packages                                |
| ------------- | --------------------------------------- |
| Debian/Ubuntu | `sudo apt install libdbus-1-dev pkg-config bluez` |
| Fedora        | `sudo dnf install dbus-devel pkgconf bluez`       |
| Arch          | `sudo pacman -S dbus pkgconf bluez`               |

## Build and run

```sh
git clone https://github.com/dummy3ye/bluey
cd bluey
cargo run --release
# or install system-wide
make install            # PREFIX=/usr/local by default
```

## Keys

| Key            | Action                      |
| -------------- | --------------------------- |
| `j` `k` / arrows | Move selection            |
| `g` `G`        | First / last device         |
| `Enter` / `c`  | Connect or disconnect       |
| `p`            | Pair (and trust)            |
| `t`            | Toggle trust                |
| `b`            | Toggle blocked              |
| `x`            | Remove (asks to confirm)    |
| `f`            | Cycle filter                |
| `s`            | Start / stop scanning       |
| `o`            | Power adapter on / off      |
| `d`            | Toggle discoverable         |
| `T`            | Cycle theme                 |
| `?`            | Help                        |
| `q` / `Ctrl-C` | Quit                        |

## Architecture

```
src/
├── main.rs        entry point, terminal setup, async event loop
├── bluetooth.rs   BlueZ backend: Command in, Update (Snapshot/Notice) out
├── app.rs         state + key handling (no I/O, no drawing; unit tested)
├── input.rs       terminal input thread
├── theme.rs       palettes
└── ui/            rendering, one module per panel
```

The UI never calls BlueZ directly. It sends `Command`s through a `Handle` and
renders the latest `Snapshot`, which keeps state, rendering and I/O separated.

## Configuration

bluey reads an optional config file from `$XDG_CONFIG_HOME/bluey/config.toml`
(or `~/.config/bluey/config.toml`).

```toml
# Use the terminal's default background. Set to false for a solid theme background.
transparent_background = true

# Theme name: "midnight", "nord", or "ember"
theme = "midnight"
```

## Troubleshooting

- **"could not reach the BlueZ daemon"**: `sudo systemctl enable --now bluetooth`
- **"no Bluetooth adapter found"**: check `rfkill list` and `sudo rfkill unblock bluetooth`
- **Pairing fails**: another agent (e.g. your desktop environment) may hold the default slot; bluey registers its own on start.
- **Colours look off**: use a truecolor terminal or set `COLORTERM=truecolor`.

## Development

```sh
make check    # fmt, clippy, tests
make run
```

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CHANGELOG.md](CHANGELOG.md).

## License

MIT, see [LICENSE](LICENSE).
