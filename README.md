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

This opens the current UI shell with a control menu and presentation popups.

Current scaffold behavior:
- Control window for keyboard/mouse filters, popup X/Y placement, visible seconds, scale, and click-through behavior.
- Separate transparent overlay viewport (floating, borderless, always-on-top) that renders keycaps.
- Live pressed-state display (held keys/buttons only) with fade-out after `visible_seconds`.

## Next steps: test and verify it works

Because this project is currently a scaffold plus a Phase 4 UI shell, the best verification
flow is:

1. **Validate the workspace compiles and tests pass**

   ```bash
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

2. **Run the app binary**

   ```bash
   cargo run -p keyoverlay-app
   ```

3. **Confirm expected Phase 4 behavior**
   - Terminal prints: `KeyOverlay – live overlay + controls scaffold`
   - Overlay window appears and stays on top.
   - The overlay keycaps update based on held sample inputs and fade out when nothing is pressed.

4. **If testing on non-Windows**
   - You can still validate build, tests, and the sample-input overlay shell.
   - Real global key capture is a later phase and Windows-specific.

5. **If testing on Windows (recommended for upcoming phases)**
   - Run from a normal user session (not headless/SSH).
   - Keep this phase focused on window/render behavior; global hooks are not implemented yet.

## Contributing

- Follow Rust 2021 edition conventions.
- Run `cargo fmt`, `cargo clippy`, and `cargo test` before submitting changes.
- Keep dependencies minimal and prefer Windows-native APIs for the final implementation.

## License

MIT. See [LICENSE](LICENSE).
