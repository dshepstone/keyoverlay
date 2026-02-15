#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub radius: i32,
}

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GetLastError, BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED};
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn};
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    fn hwnd_from_title(title: &str) -> Option<HWND> {
        let title_w = to_wide(title);
        let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title_w.as_ptr())) };
        if hwnd.0 == 0 {
            None
        } else {
            Some(hwnd)
        }
    }

    pub fn disable_window_transitions(hwnd_raw: isize) -> Result<(), String> {
        let hwnd = HWND(hwnd_raw);
        let disable = BOOL::from(true);

        let result = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &disable as *const _ as *const _,
                size_of::<BOOL>() as u32,
            )
        };

        result
            .ok()
            .map_err(|e| format!("DwmSetWindowAttribute failed: {e}"))
    }

    pub fn apply_test_region(title: &str) -> Option<isize> {
        let hwnd = hwnd_from_title(title)?;
        eprintln!("[overlay-region] Applying TEST region hwnd={hwnd:?}");

        let hrgn = unsafe { CreateRoundRectRgn(0, 0, 240, 80, 40, 40) };
        if hrgn.0 == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(TEST) failed err={err}");
            return Some(hwnd.0);
        }

        let res = unsafe { SetWindowRgn(hwnd, hrgn, true) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TEST) failed err={err}");
            let _ = unsafe { DeleteObject(hrgn) };
        } else {
            eprintln!("[overlay-region] SetWindowRgn(TEST) ok");
        }

        Some(hwnd.0)
    }

    pub fn apply_tray_region(
        title: &str,
        width: i32,
        height: i32,
        tray_radius_px: i32,
    ) -> Option<isize> {
        let hwnd = hwnd_from_title(title)?;
        let tray_w = width.max(1);
        let tray_h = height.max(1);
        let rr = tray_radius_px.max(0);

        eprintln!(
            "[overlay-region] apply tray hwnd={hwnd:?} window={}x{} radius={}",
            tray_w, tray_h, rr
        );

        let tray_rgn = unsafe { CreateRoundRectRgn(0, 0, tray_w, tray_h, rr * 2, rr * 2) };
        if tray_rgn.0 == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(TRAY) failed err={err}");
            return Some(hwnd.0);
        }

        let res = unsafe { SetWindowRgn(hwnd, tray_rgn, true) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TRAY) failed err={err}");
            let _ = unsafe { DeleteObject(tray_rgn) };
        }

        Some(hwnd.0)
    }
}

#[cfg(target_os = "windows")]
pub use imp::{apply_test_region, apply_tray_region, disable_window_transitions};

#[cfg(not(target_os = "windows"))]
pub fn apply_test_region(_title: &str) -> Option<isize> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn apply_tray_region(
    _title: &str,
    _width: i32,
    _height: i32,
    _tray_radius_px: i32,
) -> Option<isize> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn disable_window_transitions(_hwnd_raw: isize) -> Result<(), String> {
    Ok(())
}
