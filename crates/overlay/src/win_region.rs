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
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetClientRect, GetWindowLongPtrW, GetWindowRect, IsWindowVisible, GWL_EXSTYLE,
        GWL_STYLE,
    };

    use crate::overlay_startup_diagnostics;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    pub fn find_hwnd_by_title(title: &str) -> Option<HWND> {
        let title_w = to_wide(title);
        let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title_w.as_ptr())) }.ok()?;
        if hwnd.0.is_null() {
            None
        } else {
            overlay_startup_diagnostics::log_event(format!(
                "FindWindowW succeeded for title='{title}' hwnd={hwnd:?}"
            ));
            Some(hwnd)
        }
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
        let layered = exstyle & 0x0008_0000 != 0; // WS_EX_LAYERED
        let transparent = exstyle & 0x0000_0020 != 0; // WS_EX_TRANSPARENT
        let topmost = exstyle & 0x0000_0008 != 0; // WS_EX_TOPMOST

        overlay_startup_diagnostics::log_event(format!(
            "win_state[{label}] hwnd={hwnd:?} visible={visible} style=0x{style:08X} exstyle=0x{exstyle:08X} layered={layered} transparent={transparent} topmost={topmost} \
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
            return Some(hwnd);
        }

        let res = unsafe { SetWindowRgn(hwnd, tray_rgn, redraw) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TRAY) failed err={err:?}");
            let _ = unsafe { DeleteObject(tray_rgn) };
        }

        snapshot_hwnd_state(hwnd, "after-setwindowrgn");
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

    pub fn apply_tray_region(
        title: &str,
        width: i32,
        height: i32,
        tray_radius_px: i32,
    ) -> Option<HWND> {
        apply_tray_region_with_redraw(title, width, height, tray_radius_px, true)
    }
}

#[cfg(target_os = "windows")]
pub use imp::{
    apply_test_region, apply_tray_region, apply_tray_region_with_redraw, disable_dwm_transitions,
    find_hwnd_by_title, force_redraw, snapshot_hwnd_state,
};

#[cfg(not(target_os = "windows"))]
pub fn apply_test_region(_title: &str) -> Option<OverlayHwnd> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn apply_tray_region(
    _title: &str,
    _width: i32,
    _height: i32,
    _tray_radius_px: i32,
) -> Option<OverlayHwnd> {
    None
}

#[cfg(not(target_os = "windows"))]
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
