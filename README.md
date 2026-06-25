# MUDular Client

A cross-platform MUD client with Lua scripting, built in Rust.

- Multi-tab parallel connections
- Full Lua scripting: panes, gauges, triggers, aliases, timers, keymaps
- ANSI color (256 + truecolor), MSDP, GMCP, MCCP2, MSSP
- 550+ built-in terminal themes
- TLS support

## Download

| Platform | Link |
|----------|------|
| Linux x86_64 | [mudular-linux-x86_64](https://github.com/peachpearorange/MUDular-Client/releases/latest/download/mudular-linux-x86_64) |
| Windows x86_64 | [mudular-windows-x86_64.exe](https://github.com/peachpearorange/MUDular-Client/releases/latest/download/mudular-windows-x86_64.exe) |
| macOS Apple Silicon | [mudular-macos-aarch64](https://github.com/peachpearorange/MUDular-Client/releases/latest/download/mudular-macos-aarch64) |
| Web (WASM) | [Play in browser](https://peachpearorange.github.io/MUDular-Client/) |

## Building from source

Requires nightly Rust (edition 2024).

```
cargo build --release
```
