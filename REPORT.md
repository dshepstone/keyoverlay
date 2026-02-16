# Overlay Startup Animation Investigation Report

## Symptoms

I could not directly reproduce the visual startup animation in this Linux-based CI/container environment, but I added Windows-specific startup instrumentation and experiment switches so you can characterize it on Windows precisely.

Based on your description and current startup order, the likely visible symptom classes are:

1. **Position/size jump**: overlay window appears at a default size/position and is then moved/resized.
2. **Shape transition**: rectangular window appears first, then rounded region is applied.
3. **OS transition/fade**: DWM transition runs before `DWMWA_TRANSITIONS_FORCEDISABLED` takes effect.
4. **Transparency redraw artifact**: first draw with layered/transparent surface causes a flash/fade-like effect.

## Timeline (instrumented)

Enable logging:

```powershell
$env:OVERLAY_DEBUG_STARTUP = "1"
cargo run
```

Sample startup log lines now emitted (timestamps are monotonic ms since process start):

```text
[overlay-startup +   0ms] overlay process startup; experiment=None
[overlay-startup +  15ms] first update/frame
[overlay-startup +  16ms] frame input screen_rect=1920x1080 px_per_point=1.250
[overlay-startup +  17ms] viewport config title=KeyOverlayOverlay decorations=false transparent=true always_on_top=true
[overlay-startup +  17ms] overlay title assigned in ViewportBuilder
[overlay-startup +  18ms] startup geometry size_pt=420.0x94.0 size_px=525x118 pos=(1382.0,948.0)
[overlay-startup +  19ms] FindWindowW succeeded for title='KeyOverlayOverlay' hwnd=HWND(0x....)
[overlay-startup +  19ms] win_state[before-setwindowrgn] ... visible=false style=... exstyle=...
[overlay-startup +  20ms] DwmSetWindowAttribute(DWMWA_TRANSITIONS_FORCEDISABLED=TRUE) hwnd=...
[overlay-startup +  21ms] win_state[after-setwindowrgn] ... visible=true ...
[overlay-startup +  21ms] visibility toggle false -> true (requested=true)
```

Additional logging now includes:

- first frame
- first input event
- per-frame screen rect + DPI scale factor
- viewport creation/config/title timing
- `FindWindowW` success timing
- `SetWindowRgn` timing and `bRedraw` mode
- `DwmSetWindowAttribute` timing
- full Win32 state snapshots (`GetWindowRect`, `GetClientRect`, `GWL_STYLE`, `GWL_EXSTYLE`, `IsWindowVisible`, DWM frame bounds)

## Startup Experiments (A/B)

Use `OVERLAY_STARTUP_EXPERIMENT` with values `A`..`G`:

```powershell
$env:OVERLAY_DEBUG_STARTUP = "1"
$env:OVERLAY_STARTUP_EXPERIMENT = "A"   # or B,C,D,E,F,G
cargo run
```

Implemented variants:

- **A**: disable DWM transitions ASAP once HWND exists, before visibility.
- **B**: apply region with `SetWindowRgn(..., bRedraw=false)`, then force redraw.
- **C**: delay region application by 200ms.
- **D**: keep overlay hidden until region prep is complete, then show.
- **E**: skip `SetWindowRgn` entirely.
- **F**: skip `DwmSetWindowAttribute` entirely.
- **G**: first show non-transparent, enable transparency after startup prep.

## Most likely causes (ranked)

1. **Region applied after/while visible (shape snap)**
   - Evidence to look for: `IsWindowVisible=true` before `SetWindowRgn` and immediate geometry change afterward.
   - Win32 fact: `SetWindowRgn(..., TRUE)` triggers redraw; if visible, this can look like shape animation.

2. **DWM transition not disabled early enough**
   - Evidence: first visibility on window occurs before `DwmSetWindowAttribute` call.
   - Win32 fact: `DWMWA_TRANSITIONS_FORCEDISABLED` helps suppress transitions, but applying it late cannot retroactively prevent initial transition.

3. **Initial move/resize after first frame (position/size jump)**
   - Evidence: first `GetWindowRect` differs from intended geometry/position, followed by correction next frame.

4. **Transparent/layered first-frame artifact**
   - Evidence: issue disappears or reduces under experiment G.

## Recommendations / next fixes

1. Prefer **hidden-first startup** (experiment D) as default on Windows:
   - create viewport/window,
   - resolve HWND,
   - apply DWM transition disable,
   - apply final region,
   - then show.

2. Keep region apply on startup with **`bRedraw=false` + one explicit redraw** (experiment B) to reduce intermediate animation.

3. If startup flash persists, keep **transparency disabled for first show** and enable after first prepared frame (experiment G).

4. Longer-term robustness:
   - avoid relying primarily on `FindWindowW(title)` for HWND discovery when direct window handle access becomes available from integration layer.

## Files touched for this investigation

- `crates/overlay/src/startup_debug.rs`
- `crates/overlay/src/win_region.rs`
- `crates/overlay/src/lib.rs`

