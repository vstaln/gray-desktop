<div align="center">
  <img alt="gray-desktop" src="assets/icon.svg" width="120" height="120" />
  <h1>gray-desktop</h1>
  <p><strong>Desktop UI automation — screenshot, click, type, key chords, window list.</strong></p>
  <p>
    <a href="https://gray.alignment.id">Website</a> ·
    <a href="https://github.com/vstaln/gray-desktop">Source</a> ·
    <a href="https://github.com/vstaln/gray">gray</a>
  </p>
  <p>
    <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-1c1c20?style=flat-square&labelColor=0a0a0b" /></a>
    <a href="https://www.rust-lang.org"><img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-1c1c20?style=flat-square&labelColor=0a0a0b&logo=rust&logoColor=d4a373" /></a>
    <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-1c1c20?style=flat-square&labelColor=0a0a0b&color=7aa2f7" />
  </p>
</div>

<br/>
```bash
gray plugin install desktop
```

`gray-desktop` gives the agent eyes and hands on the GUI session.

## Tools

| Tool | What it does |
|---|---|
| `desktop_shot` | Screenshot to a PNG. |
| `desktop_click` | Click at coordinates / on a window. |
| `desktop_type` | Type text into the focused window. |
| `desktop_key` | Press a key chord (`ctrl+l`, `return`, …). |
| `desktop_windows` | List open windows. |

## Platform support

| OS | Mechanism |
|---|---|
| Linux/X11 | `scrot`\|`import` (ImageMagick) + `xdotool`, `wmctrl` for windows |
| macOS | `screencapture` + `cliclick` or `osascript` for input |
| Wayland / headless / other | honest unsupported error — nothing faked |

Missing binaries report install hints (`apt install xdotool`, `brew
install cliclick`, …) instead of failing opaquely.

## Build from source

```sh
cargo build --release
gray plugin check ./target/release/desktop
```

## License

MIT
