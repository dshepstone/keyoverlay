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
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use windows_sys::Win32::Foundation::{GetLastError, HWND};
    use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn};
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    fn hwnd_from_title(title: &str) -> Option<HWND> {
        let title_w = to_wide(title);
        let hwnd = unsafe { FindWindowW(ptr::null(), title_w.as_ptr()) };
        if hwnd.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }

    pub fn apply_test_region(title: &str) -> Option<isize> {
        let hwnd = hwnd_from_title(title)?;
        eprintln!("[overlay-region] Applying TEST region hwnd={hwnd:p}");

        let hrgn = unsafe { CreateRoundRectRgn(0, 0, 240, 80, 40, 40) };
        if hrgn.is_null() {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(TEST) failed err={err}");
            return Some(hwnd as isize);
        }

        let res = unsafe { SetWindowRgn(hwnd, hrgn, 1) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TEST) failed err={err}");
            unsafe { DeleteObject(hrgn as _) };
        } else {
            eprintln!("[overlay-region] SetWindowRgn(TEST) ok");
        }

        Some(hwnd as isize)
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
            "[overlay-region] apply tray hwnd={hwnd:p} window={}x{} radius={}",
            tray_w, tray_h, rr
        );

        let tray_rgn = unsafe { CreateRoundRectRgn(0, 0, tray_w, tray_h, rr * 2, rr * 2) };
        if tray_rgn.is_null() {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(TRAY) failed err={err}");
            return Some(hwnd as isize);
        }

        let res = unsafe { SetWindowRgn(hwnd, tray_rgn, 1) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn(TRAY) failed err={err}");
            unsafe { DeleteObject(tray_rgn as _) };
        }

        Some(hwnd as isize)
    }
}

#[cfg(target_os = "windows")]
pub use imp::{apply_test_region, apply_tray_region};

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
