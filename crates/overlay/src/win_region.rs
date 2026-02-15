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
    use super::PillRect;
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use windows_sys::Win32::Foundation::{GetLastError, HWND};
    use windows_sys::Win32::Graphics::Gdi::{
        CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, RGN_AND, RGN_OR,
    };
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

    pub fn apply_pill_region(
        title: &str,
        width: i32,
        height: i32,
        rects: &[PillRect],
    ) -> Option<isize> {
        let hwnd = hwnd_from_title(title)?;
        eprintln!(
            "[overlay-region] apply hwnd={hwnd:p} window={}x{} rects={}",
            width,
            height,
            rects.len()
        );

        if rects.is_empty() {
            eprintln!("[overlay-region] skip empty rects");
            return Some(hwnd as isize);
        }

        let mut union_bounds = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);

        let first = &rects[0];
        eprintln!(
            "[overlay-region] rect[0] x={} y={} w={} h={} r={}",
            first.x, first.y, first.w, first.h, first.radius
        );
        union_bounds.0 = union_bounds.0.min(first.x);
        union_bounds.1 = union_bounds.1.min(first.y);
        union_bounds.2 = union_bounds.2.max(first.x + first.w);
        union_bounds.3 = union_bounds.3.max(first.y + first.h);

        let mut accum = unsafe {
            CreateRoundRectRgn(
                first.x,
                first.y,
                first.x + first.w,
                first.y + first.h,
                first.radius * 2,
                first.radius * 2,
            )
        };
        if accum.is_null() {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] CreateRoundRectRgn(first) failed err={err}");
            return Some(hwnd as isize);
        }

        for (idx, rect) in rects.iter().enumerate().skip(1) {
            eprintln!(
                "[overlay-region] rect[{idx}] x={} y={} w={} h={} r={}",
                rect.x, rect.y, rect.w, rect.h, rect.radius
            );
            union_bounds.0 = union_bounds.0.min(rect.x);
            union_bounds.1 = union_bounds.1.min(rect.y);
            union_bounds.2 = union_bounds.2.max(rect.x + rect.w);
            union_bounds.3 = union_bounds.3.max(rect.y + rect.h);

            let next = unsafe {
                CreateRoundRectRgn(
                    rect.x,
                    rect.y,
                    rect.x + rect.w,
                    rect.y + rect.h,
                    rect.radius * 2,
                    rect.radius * 2,
                )
            };
            if next.is_null() {
                continue;
            }
            unsafe {
                CombineRgn(accum, accum, next, RGN_OR);
                DeleteObject(next as _);
            }
        }

        eprintln!(
            "[overlay-region] union bounds left={} top={} right={} bottom={}",
            union_bounds.0, union_bounds.1, union_bounds.2, union_bounds.3
        );

        // Ensure region cannot exceed current window client size.
        let clamp = unsafe { CreateRectRgn(0, 0, width.max(1), height.max(1)) };
        if !clamp.is_null() {
            unsafe {
                CombineRgn(accum, accum, clamp, RGN_AND);
                DeleteObject(clamp as _);
            }
        }

        let res = unsafe { SetWindowRgn(hwnd, accum, 1) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            eprintln!("[overlay-region] SetWindowRgn failed err={err}");
            unsafe { DeleteObject(accum as _) };
        }

        Some(hwnd as isize)
    }
}

#[cfg(target_os = "windows")]
pub use imp::{apply_pill_region, apply_test_region};

#[cfg(not(target_os = "windows"))]
pub fn apply_test_region(_title: &str) -> Option<isize> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn apply_pill_region(
    _title: &str,
    _width: i32,
    _height: i32,
    _rects: &[PillRect],
) -> Option<isize> {
    None
}
