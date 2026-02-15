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
    use windows::Win32::Foundation::{GetLastError, BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn};
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    fn hwnd_from_title(title: &str) -> Option<HWND> {
        let title_w = to_wide(title);
        let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title_w.as_ptr())) }.ok()?;
        if hwnd.0.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }

    pub fn disable_dwm_transitions(hwnd: HWND) -> Result<(), String> {
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

    pub fn apply_test_region(title: &str) -> Option<HWND> {
        let hwnd = hwnd_from_title(title)?;
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
        let hwnd = hwnd_from_title(title)?;
        let tray_w = width.max(1);
        let tray_h = height.max(1);
        let rr = tray_radius_px.max(0);

        eprintln!(
            "[overlay-region] apply tray hwnd={hwnd:?} window={}x{} radius={}",
            tray_w, tray_h, rr
        );

        let tray_rgn = unsafe { CreateRoundRectRgn(0, 0, tray_w, tray_h, rr * 2, rr * 2) };
        if tray_rgn.0.is_null() {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(TRAY) failed err={err:?}");
            return Some(hwnd);
        }

        let res = unsafe { SetWindowRgn(hwnd, tray_rgn, true) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TRAY) failed err={err:?}");
            let _ = unsafe { DeleteObject(tray_rgn) };
        }

        Some(hwnd)
    }
}

#[cfg(target_os = "windows")]
pub use imp::{apply_test_region, apply_tray_region, disable_dwm_transitions};

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
pub fn disable_dwm_transitions(_hwnd: OverlayHwnd) -> Result<(), String> {
    Ok(())
}
