#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub radius: i32,
}

#[cfg(target_os = "windows")]
pub type OverlayHwnd = windows::Win32::Foundation::HWND;

#[cfg(not(target_os = "windows"))]
pub type OverlayHwnd = isize;

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, BOOL, HWND, RECT};
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWINDOWATTRIBUTE,
    };
    use windows::Win32::Graphics::Gdi::{
        CreateRoundRectRgn, DeleteObject, InvalidateRect, RedrawWindow, SetWindowRgn,
        RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW,
    };
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetClassNameW, GetClientRect, GetForegroundWindow, GetWindowLongPtrW,
        GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSENDCHANGING, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE,
        SW_RESTORE, SW_SHOWNOACTIVATE,
    };

    use crate::overlay_startup_diagnostics;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    fn from_wide(buf: &[u16]) -> String {
        let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    }

    fn hwnd_in_current_process(hwnd: HWND) -> bool {
        let mut pid = 0u32;
        let _thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid == std::process::id()
    }

    fn hwnd_debug_identity(hwnd: HWND) -> String {
        let mut title_buf = [0u16; 256];
        let mut class_buf = [0u16; 128];
        let _ = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
        let _ = unsafe { GetClassNameW(hwnd, &mut class_buf) };
        let title = from_wide(&title_buf);
        let class = from_wide(&class_buf);
        format!("title='{title}' class='{class}'")
    }

    const WS_EX_TOOLWINDOW: usize = 0x00000080;
    const WS_EX_NOACTIVATE: usize = 0x08000000;

    pub fn find_hwnd_by_title(title: &str) -> Option<HWND> {
        let title_w = to_wide(title);
        let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title_w.as_ptr())) }.ok()?;
        if hwnd.0.is_null() {
            None
        } else if !hwnd_in_current_process(hwnd) {
            overlay_startup_diagnostics::log_event(format!(
                "FindWindowW matched foreign process hwnd={hwnd:?} {}; ignoring",
                hwnd_debug_identity(hwnd)
            ));
            None
        } else {
            overlay_startup_diagnostics::log_event(format!(
                "FindWindowW succeeded for title='{title}' hwnd={hwnd:?} {}",
                hwnd_debug_identity(hwnd)
            ));
            Some(hwnd)
        }
    }

    pub fn apply_tray_region_hwnd_with_redraw(
        hwnd: HWND,
        width: i32,
        height: i32,
        tray_radius_px: i32,
        redraw: bool,
    ) -> bool {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            overlay_startup_diagnostics::log_event(format!(
                "SetWindowRgn skipped: invalid HWND {hwnd:?}"
            ));
            return false;
        }

        let tray_w = width.max(1);
        let tray_h = height.max(1);
        let rr = tray_radius_px.max(0);

        overlay_startup_diagnostics::log_event(format!(
            "SetWindowRgn request hwnd={hwnd:?} size={}x{} radius={} redraw={redraw}",
            tray_w, tray_h, rr
        ));
        snapshot_hwnd_state(hwnd, "before-setwindowrgn");

        let tray_rgn = unsafe { CreateRoundRectRgn(0, 0, tray_w, tray_h, rr * 2, rr * 2) };
        if tray_rgn.0.is_null() {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(TRAY) failed err={err:?}");
            return false;
        }

        let res = unsafe { SetWindowRgn(hwnd, tray_rgn, redraw) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TRAY) failed err={err:?}");
            let _ = unsafe { DeleteObject(tray_rgn) };
            return false;
        }

        snapshot_hwnd_state(hwnd, "after-setwindowrgn");
        true
    }

    pub fn disable_dwm_transitions(hwnd: HWND) -> Result<(), String> {
        overlay_startup_diagnostics::log_event(format!(
            "DwmSetWindowAttribute(DWMWA_TRANSITIONS_FORCEDISABLED=TRUE) hwnd={hwnd:?}"
        ));
        let value = BOOL(1);
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(3),
                &value as *const _ as *const _,
                size_of::<BOOL>() as u32,
            )
        }
        .map_err(|e| format!("DwmSetWindowAttribute failed: {e:?}"))
    }

    pub fn snapshot_hwnd_state(hwnd: HWND, label: &str) {
        let mut wr = RECT::default();
        let mut cr = RECT::default();
        let mut efb = RECT::default();

        let _ = unsafe { GetWindowRect(hwnd, &mut wr) };
        let _ = unsafe { GetClientRect(hwnd, &mut cr) };
        let _ = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWINDOWATTRIBUTE(9),
                &mut efb as *mut _ as *mut _,
                size_of::<RECT>() as u32,
            )
        };

        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as usize;
        let exstyle = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as usize;
        let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
        let mut pid = 0u32;
        let _ = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let layered = exstyle & 0x0008_0000 != 0; // WS_EX_LAYERED
        let transparent = exstyle & 0x0000_0020 != 0; // WS_EX_TRANSPARENT
        let topmost = exstyle & 0x0000_0008 != 0; // WS_EX_TOPMOST

        overlay_startup_diagnostics::log_event(format!(
            "win_state[{label}] hwnd={hwnd:?} pid={pid} visible={visible} dpi={dpi} style=0x{style:08X} exstyle=0x{exstyle:08X} layered={layered} transparent={transparent} topmost={topmost} \
             window_rect=({},{}-{},{}), client_rect=({},{}-{},{}), frame_bounds=({},{}-{}, {})",
            wr.left,
            wr.top,
            wr.right,
            wr.bottom,
            cr.left,
            cr.top,
            cr.right,
            cr.bottom,
            efb.left,
            efb.top,
            efb.right,
            efb.bottom,
        ));
    }

    pub fn apply_tray_region_with_redraw(
        title: &str,
        width: i32,
        height: i32,
        tray_radius_px: i32,
        redraw: bool,
    ) -> Option<HWND> {
        let hwnd = find_hwnd_by_title(title)?;
        let _ = apply_tray_region_hwnd_with_redraw(hwnd, width, height, tray_radius_px, redraw);
        Some(hwnd)
    }

    pub fn force_redraw(hwnd: HWND) {
        overlay_startup_diagnostics::log_event(format!("force redraw hwnd={hwnd:?}"));
        let _ = unsafe { InvalidateRect(hwnd, None, true) };
        let _ = unsafe {
            RedrawWindow(
                hwnd,
                None,
                None,
                RDW_INVALIDATE | RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN | RDW_UPDATENOW,
            )
        };
    }

    pub fn apply_test_region(title: &str) -> Option<HWND> {
        let hwnd = find_hwnd_by_title(title)?;
        eprintln!("[overlay-region] Applying TEST region hwnd={hwnd:?}");

        let hrgn = unsafe { CreateRoundRectRgn(0, 0, 240, 80, 40, 40) };
        if hrgn.0.is_null() {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(TEST) failed err={err:?}");
            return Some(hwnd);
        }

        let res = unsafe { SetWindowRgn(hwnd, hrgn, true) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TEST) failed err={err:?}");
            let _ = unsafe { DeleteObject(hrgn) };
        } else {
            eprintln!("[overlay-region] SetWindowRgn(TEST) ok");
        }

        Some(hwnd)
    }

    pub fn apply_no_activate_styles(hwnd: HWND) {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return;
        }

        let exstyle = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as usize };
        let desired = exstyle | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;
        if desired != exstyle {
            let _ = unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired as isize) };
        }

        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOSENDCHANGING,
            )
        };
    }

    pub fn show_window_no_activate(hwnd: HWND) {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return;
        }
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOZORDER
                    | SWP_NOSENDCHANGING
                    | SWP_SHOWWINDOW,
            )
        };
    }

    pub fn hide_window(hwnd: HWND) {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return;
        }
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    }

    /// Restore a minimized window (to keep the wgpu surface alive) and
    /// immediately move it to (-32000, -32000) so it is invisible to the user
    /// but still present in the taskbar.  Returns the window's position
    /// *before* the move (so the caller can restore it later).
    pub fn restore_and_move_offscreen(hwnd: HWND) -> (i32, i32) {
        // Read the window rect before we do anything.  While the window is
        // minimized Windows reports the pre-minimize position here.
        let mut rect = windows::Win32::Foundation::RECT::default();
        let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
        let saved_x = rect.left;
        let saved_y = rect.top;

        // Restore from minimized state — this sends WM_SIZE(SIZE_RESTORED)
        // which makes winit recreate the wgpu surface at a valid size.
        let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };

        // Move off-screen. (-32000, -32000) is the same position Windows uses
        // internally for minimized windows, so it is guaranteed to be off every
        // monitor.  SWP_NOSIZE keeps the client area intact so the surface
        // stays valid.
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                -32000,
                -32000,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
            )
        };

        (saved_x, saved_y)
    }

    /// Move a window back to a specific screen position and make it visible.
    pub fn restore_to_position(hwnd: HWND, x: i32, y: i32) {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return;
        }
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING | SWP_SHOWWINDOW,
            )
        };
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    }

    pub fn is_foreground_window(hwnd: HWND) -> bool {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return false;
        }
        unsafe { GetForegroundWindow() == hwnd }
    }

    pub fn apply_tray_region(
        title: &str,
        width: i32,
        height: i32,
        tray_radius_px: i32,
    ) -> Option<HWND> {
        apply_tray_region_with_redraw(title, width, height, tray_radius_px, true)
    }

    pub fn hwnd_is_valid(hwnd: HWND) -> bool {
        unsafe { IsWindow(hwnd).as_bool() }
    }
}

#[cfg(target_os = "windows")]
#[allow(unused_imports)]
pub use imp::{
    apply_no_activate_styles, apply_test_region, apply_tray_region,
    apply_tray_region_hwnd_with_redraw, apply_tray_region_with_redraw, disable_dwm_transitions,
    find_hwnd_by_title, force_redraw, hide_window, hwnd_is_valid, is_foreground_window,
    restore_and_move_offscreen, restore_to_position, show_window_no_activate, snapshot_hwnd_state,
};

#[cfg(not(target_os = "windows"))]
pub fn apply_no_activate_styles(_hwnd: OverlayHwnd) {}

#[cfg(not(target_os = "windows"))]
pub fn show_window_no_activate(_hwnd: OverlayHwnd) {}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn hide_window(_hwnd: OverlayHwnd) {}

#[cfg(not(target_os = "windows"))]
pub fn restore_and_move_offscreen(_hwnd: OverlayHwnd) -> (i32, i32) {
    (0, 0)
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn restore_to_position(_hwnd: OverlayHwnd, _x: i32, _y: i32) {}

#[cfg(not(target_os = "windows"))]
pub fn is_foreground_window(_hwnd: OverlayHwnd) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn apply_test_region(_title: &str) -> Option<OverlayHwnd> {
    None
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn apply_tray_region(
    _title: &str,
    _width: i32,
    _height: i32,
    _tray_radius_px: i32,
) -> Option<OverlayHwnd> {
    None
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn apply_tray_region_with_redraw(
    _title: &str,
    _width: i32,
    _height: i32,
    _tray_radius_px: i32,
    _redraw: bool,
) -> Option<OverlayHwnd> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn apply_tray_region_hwnd_with_redraw(
    _hwnd: OverlayHwnd,
    _width: i32,
    _height: i32,
    _tray_radius_px: i32,
    _redraw: bool,
) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn hwnd_is_valid(_hwnd: OverlayHwnd) -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub fn find_hwnd_by_title(_title: &str) -> Option<OverlayHwnd> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn force_redraw(_hwnd: OverlayHwnd) {}

#[cfg(not(target_os = "windows"))]
pub fn snapshot_hwnd_state(_hwnd: OverlayHwnd, _label: &str) {}

#[cfg(not(target_os = "windows"))]
pub fn disable_dwm_transitions(_hwnd: OverlayHwnd) -> Result<(), String> {
    Ok(())
}
