#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub radius: i32,
}

#[cfg(target_os = "windows")]
pub fn apply_window_region_by_title(title: &str, rects: &[RegionRect]) {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{
        CombineRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, RGN_OR,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    unsafe {
        let title_w = to_wide(title);
        let hwnd: HWND = FindWindowW(std::ptr::null(), title_w.as_ptr());
        if hwnd == 0 {
            return;
        }

        if rects.is_empty() {
            SetWindowRgn(hwnd, 0, 1);
            return;
        }

        let first = &rects[0];
        let mut accum = CreateRoundRectRgn(
            first.x,
            first.y,
            first.x + first.w,
            first.y + first.h,
            first.radius * 2,
            first.radius * 2,
        );
        if accum == 0 {
            return;
        }

        for r in &rects[1..] {
            let next =
                CreateRoundRectRgn(r.x, r.y, r.x + r.w, r.y + r.h, r.radius * 2, r.radius * 2);
            if next != 0 {
                CombineRgn(accum, accum, next, RGN_OR);
                DeleteObject(next as _);
            }
        }

        // Region ownership is transferred to the window on success.
        if SetWindowRgn(hwnd, accum, 1) == 0 {
            DeleteObject(accum as _);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_window_region_by_title(_title: &str, _rects: &[RegionRect]) {}
