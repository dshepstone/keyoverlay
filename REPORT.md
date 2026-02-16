# Overlay Startup Diagnostics Report

## Symptom classification

The diagnostics instrumentation is now focused on distinguishing these startup artifacts:

1. **Hide/show flicker**: rapid `Show requested` / `Hide requested` sequence with `IsWindowVisible` toggling.
2. **Position jump**: `GetWindowRect` origin changes after first visible frame.
3. **Size snap**: `GetWindowRect` / `GetClientRect` size changes after first show.
4. **Shape snap**: `SetWindowRgn` applied after window is visible.
5. **Compositor transition/fade**: first visible frame occurs before DWM transitions are force-disabled.

## Timeline evidence

Enable diagnostics:

### PowerShell
```powershell
$env:OVERLAY_DEBUG = "1"
$env:OVERLAY_EXPERIMENT = "baseline"
cargo run
```

### cmd.exe
```cmd
set OVERLAY_DEBUG=1
set OVERLAY_EXPERIMENT=baseline
cargo run
```

Sample expected timeline excerpt (real logs will vary by machine):

```text
[overlay-diag +    0ms] overlay diagnostics enabled; experiment=Baseline
[overlay-diag +   12ms] first update/frame
[overlay-diag +   14ms] frame input screen_rect=1920x1080 px_per_point=1.250
[overlay-diag +   16ms] viewport config title=KeyOverlayOverlay decorations=false transparent=true always_on_top=true
[overlay-diag +   17ms] startup geometry size_pt=420.0x94.0 size_px=525x118 pos=(1382.0,948.0)
[overlay-diag +   18ms] Show requested
[overlay-diag +   19ms] FindWindowW succeeded for title='KeyOverlayOverlay' hwnd=HWND(...)
[overlay-diag +   20ms] win_state[before-setwindowrgn] ... visible=false ... layered=true transparent=true topmost=true ...
[overlay-diag +   20ms] SetWindowRgn request hwnd=... size=525x118 radius=22 redraw=true
[overlay-diag +   21ms] DwmSetWindowAttribute(DWMWA_TRANSITIONS_FORCEDISABLED=TRUE) hwnd=...
[overlay-diag +   22ms] win_state[after-setwindowrgn] ... visible=true ...
```

## Root cause ranking (what to validate with logs)

1. **Window shown before final geometry/state is applied**
   - Evidence: `Show requested` appears before stable `win_state` rect/style values.
2. **Region applied while visible with redraw**
   - Evidence: `win_state[before-setwindowrgn] visible=true` and `redraw=true`.
   - Win32 fact: `SetWindowRgn(..., TRUE)` triggers redraw; this can produce shape snap/flicker.
3. **DWM/transparency applied too late**
   - Evidence: first visible snapshot occurs before early DWM disable or before transparency settles.
   - Win32 fact: late `DWMWA_TRANSITIONS_FORCEDISABLED` cannot prevent a transition that already started.

## Experiments and how to run

Set both env vars and run `cargo run`:

```cmd
set OVERLAY_DEBUG=1
set OVERLAY_EXPERIMENT=<name>
cargo run
```

Supported experiment values:

- `baseline`
- `no_region`
- `region_no_redraw`
- `hidden_until_ready`
- `no_transparency_until_ready`
- `early_dwm_disable`
- `no_autoposition`
- `delay_region`
- `no_dwm_disable`

Legacy aliases are still accepted (`A..G`, old debug env names) for compatibility.

## Recommendations / next fixes (priority order)

1. If logs show flicker from visibility transitions, make `hidden_until_ready` the default on Windows startup.
2. If logs show shape snap, use `region_no_redraw` startup sequence (`SetWindowRgn(..., false)` + one explicit `RedrawWindow`).
3. If logs show compositor-like fade, apply `early_dwm_disable` immediately when HWND first becomes available.
4. If logs show first-frame jump, use `no_autoposition` evidence to isolate and then defer/rework auto-position logic until stable frame metrics are available.

## Notes on evidence quality

This report includes expected sample logs because this environment cannot run native Windows GUI behavior. The code now emits the required HWND snapshots and lifecycle markers to capture real evidence on Windows machines.

