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

## Install (Windows 10/11)

**[⬇ Download KeyOverlay-Setup-0.3.0-x64.exe](https://github.com/dshepstone/keyoverlay/releases/download/v0.3.0/KeyOverlay-Setup-0.3.0-x64.exe)** and run it, or pick any version from the
[Releases page](https://github.com/dshepstone/keyoverlay/releases). (Releases up to
v0.2.0 ship only the program itself, `KeyOverlay-v0.2.0-Windows-x64.exe`, which runs without
installing.)

The installer:

- installs for the current user by default (no admin rights needed), or for all users if you choose;
- adds **KeyOverlay** to the Start menu, with an uninstall shortcut;
- can optionally add a desktop shortcut and start KeyOverlay when you sign in;
- registers an uninstaller in **Settings → Apps → Installed apps**.

A portable `KeyOverlay-<version>-x64-portable.exe` is attached to each release too, if you
would rather not install anything. Releases are not code-signed yet, so SmartScreen may warn
"Windows protected your PC" — click **More info → Run anyway**.

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

## Building the installer

The installer is built with [Inno Setup 6](https://jrsoftware.org/isinfo.php) from
[`installer/keyoverlay.iss`](installer/keyoverlay.iss):

```powershell
cargo build --release -p keyoverlay-app
iscc /DAppVersion=0.2.0 installer\keyoverlay.iss   # output: target\installer\
```

### Publishing a release

1. Bump `version` under `[workspace.package]` in `Cargo.toml` (e.g. `0.3.0`) and commit.
2. Tag and push: `git tag v0.3.0 && git push origin v0.3.0`.

The [Release workflow](.github/workflows/release.yml) checks the tag matches `Cargo.toml`, builds
the app and installer on Windows, and publishes a GitHub Release with the installer, the
portable `.exe`, and `SHA256SUMS.txt`.

## Contributing

- Follow Rust 2021 edition conventions.
- Run `cargo fmt`, `cargo clippy`, and `cargo test` before submitting changes.
- Keep dependencies minimal and prefer Windows-native APIs for the final implementation.

## License

MIT. See [LICENSE](LICENSE).
