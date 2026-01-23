# KeyOverlay

KeyOverlay is a Windows 10/11 desktop utility (working title) for visualizing input and
highlighting the cursor for demos, training, and recordings. This repository currently contains
a clean scaffold and build tooling only.

## Project goals

- Capture global keystrokes and display them in a minimal overlay.
- Highlight the cursor position for improved visibility.
- Provide system tray controls and a lightweight settings UI.
- Keep the application fast, unobtrusive, and easy to configure.

## Non-goals

- This is **not** a Keystro clone or a UI-heavy customization tool.
- This phase intentionally avoids implementation of hooks, overlays, or persistence.

## Planned phases

See [docs/PHASES.md](docs/PHASES.md) for the full roadmap.

## Workspace layout

```
crates/
  core/    # domain types + config
  input/   # keyboard/mouse hooks (later)
  overlay/ # rendering + window (later)
  app/     # binary, lifecycle, tray, wiring
```

## Build & run

```bash
cargo build
cargo run -p keyoverlay-app
```

## Running the overlay

```bash
cargo run -p keyoverlay-app
```

This opens a small always-on-top overlay window showing "KeyOverlay – overlay shell" and a list
of recent key events.

## Contributing

- Follow Rust 2021 edition conventions.
- Run `cargo fmt`, `cargo clippy`, and `cargo test` before submitting changes.
- Keep dependencies minimal and prefer Windows-native APIs for the final implementation.

## License

MIT. See [LICENSE](LICENSE).
