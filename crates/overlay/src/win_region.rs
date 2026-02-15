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
        radius_pad_px: i32,
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

        let win_w = width.max(1);
        let win_h = height.max(1);

        let mut union_bounds = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);

        let make_round_region = |rect: &PillRect| {
            let mut x0 = rect.x - radius_pad_px;
            let mut y0 = rect.y - radius_pad_px;
            let mut x1 = rect.x + rect.w + radius_pad_px;
            let mut y1 = rect.y + rect.h + radius_pad_px;

            x0 = x0.clamp(0, win_w);
            y0 = y0.clamp(0, win_h);
            x1 = x1.clamp(0, win_w);
            y1 = y1.clamp(0, win_h);

            if x1 <= x0 || y1 <= y0 {
                return std::ptr::null_mut();
            }

            let rr = (rect.radius + radius_pad_px).max(0);
            unsafe { CreateRoundRectRgn(x0, y0, x1, y1, rr * 2, rr * 2) }
        };

        let first = &rects[0];
        eprintln!(
            "[overlay-region] rect[0] x={} y={} w={} h={} r={}",
            first.x, first.y, first.w, first.h, first.radius
        );
        union_bounds.0 = union_bounds.0.min(first.x - radius_pad_px);
        union_bounds.1 = union_bounds.1.min(first.y - radius_pad_px);
        union_bounds.2 = union_bounds.2.max(first.x + first.w + radius_pad_px);
        union_bounds.3 = union_bounds.3.max(first.y + first.h + radius_pad_px);

        let mut accum = make_round_region(first);
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
            union_bounds.0 = union_bounds.0.min(rect.x - radius_pad_px);
            union_bounds.1 = union_bounds.1.min(rect.y - radius_pad_px);
            union_bounds.2 = union_bounds.2.max(rect.x + rect.w + radius_pad_px);
            union_bounds.3 = union_bounds.3.max(rect.y + rect.h + radius_pad_px);

            let next = make_round_region(rect);
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
        let clamp = unsafe { CreateRectRgn(0, 0, win_w, win_h) };
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
    _radius_pad_px: i32,
    _rects: &[PillRect],
) -> Option<isize> {
    None
}
