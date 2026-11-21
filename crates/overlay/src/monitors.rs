//! Monitor enumeration and physical-pixel window placement.
//!
//! All coordinates here are physical pixels on the Windows virtual desktop
//! (the primary monitor's top-left is (0, 0)). The app is per-monitor DPI
//! aware, so these values are not virtualised by Windows.

use keyoverlay_core::{MonitorGeometry, MonitorTarget};

use crate::win_region::OverlayHwnd;

/// One connected display.
#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    /// Stable Windows device name, e.g. `\\.\DISPLAY2`.
    pub id: String,
    /// 1-based number in left-to-right, top-to-bottom order.
    pub number: usize,
    pub primary: bool,
    pub geometry: MonitorGeometry,
}

impl MonitorInfo {
    /// Human-readable label, e.g. "Display 2 — 2560 × 1440 @ 150% (Primary)".
    pub fn label(&self) -> String {
        let b = self.geometry.bounds;
        let pct = (self.geometry.scale * 100.0).round() as i32;
        let primary = if self.primary { " (Primary)" } else { "" };
        format!(
            "Display {} \u{2014} {} \u{00d7} {} @ {}%{}",
            self.number, b.w, b.h, pct, primary
        )
    }

    pub fn short_label(&self) -> String {
        if self.primary {
            format!("Display {} (Primary)", self.number)
        } else {
            format!("Display {}", self.number)
        }
    }
}

/// Sort monitors left-to-right then top-to-bottom and number them from 1.
/// Guarantees a non-empty list.
fn finalize(mut monitors: Vec<MonitorInfo>) -> Vec<MonitorInfo> {
    if monitors.is_empty() {
        monitors.push(MonitorInfo {
            id: String::from("primary"),
            number: 1,
            primary: true,
            geometry: MonitorGeometry::FALLBACK,
        });
    }
    monitors.sort_by_key(|m| (m.geometry.bounds.x, m.geometry.bounds.y));
    for (i, m) in monitors.iter_mut().enumerate() {
        m.number = i + 1;
    }
    if !monitors.iter().any(|m| m.primary) {
        // Prefer the monitor containing the origin, else the first one.
        let idx = monitors
            .iter()
            .position(|m| m.geometry.bounds.contains(0, 0))
            .unwrap_or(0);
        monitors[idx].primary = true;
    }
    monitors
}

/// Outcome of resolving the user's [`MonitorTarget`] against the connected
/// monitors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMonitor {
    pub index: usize,
    /// True when a specific display was requested but is not connected, so
    /// the primary display is used instead.
    pub fell_back: bool,
}

pub fn resolve_target(
    monitors: &[MonitorInfo],
    target: &MonitorTarget,
    cursor: Option<(i32, i32)>,
) -> ResolvedMonitor {
    let primary = monitors.iter().position(|m| m.primary).unwrap_or(0);
    match target {
        MonitorTarget::Primary => ResolvedMonitor {
            index: primary,
            fell_back: false,
        },
        MonitorTarget::FollowCursor => {
            let index = cursor
                .and_then(|(x, y)| {
                    monitors
                        .iter()
                        .position(|m| m.geometry.bounds.contains(x, y))
                })
                .unwrap_or(primary);
            ResolvedMonitor {
                index,
                fell_back: false,
            }
        }
        MonitorTarget::Display(id) => match monitors.iter().position(|m| &m.id == id) {
            Some(index) => ResolvedMonitor {
                index,
                fell_back: false,
            },
            None => ResolvedMonitor {
                index: primary,
                fell_back: true,
            },
        },
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::{finalize, MonitorInfo};
    use keyoverlay_core::{MonitorGeometry, PxRect};
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetWindowRect, IsWindow, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE,
        SWP_NOZORDER,
    };

    const MONITORINFOF_PRIMARY: u32 = 1;

    fn rect(r: RECT) -> PxRect {
        PxRect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
    }

    unsafe extern "system" fn enum_proc(
        hmon: HMONITOR,
        _hdc: HDC,
        _clip: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let out = &mut *(data.0 as *mut Vec<MonitorInfo>);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(hmon, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO).as_bool() {
            return TRUE;
        }
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_err() || dpi_x == 0
        {
            dpi_x = 96;
        }
        let len = info
            .szDevice
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(info.szDevice.len());
        let id = String::from_utf16_lossy(&info.szDevice[..len]);
        out.push(MonitorInfo {
            id,
            number: 0,
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            geometry: MonitorGeometry {
                bounds: rect(info.monitorInfo.rcMonitor),
                work: rect(info.monitorInfo.rcWork),
                scale: dpi_x as f32 / 96.0,
            },
        });
        TRUE
    }

    pub fn enumerate_monitors() -> Vec<MonitorInfo> {
        let mut monitors: Vec<MonitorInfo> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(enum_proc),
                LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize),
            );
        }
        finalize(monitors)
    }

    pub fn cursor_position_px() -> Option<(i32, i32)> {
        let mut pt = POINT::default();
        unsafe { GetCursorPos(&mut pt) }.ok()?;
        Some((pt.x, pt.y))
    }

    pub fn window_scale(hwnd: HWND) -> Option<f32> {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return None;
        }
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        (dpi > 0).then(|| dpi as f32 / 96.0)
    }

    pub fn window_origin_px(hwnd: HWND) -> Option<(i32, i32)> {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return None;
        }
        let mut r = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
        Some((r.left, r.top))
    }

    pub fn move_window_px(hwnd: HWND, x: i32, y: i32) {
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
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
    }
}

#[cfg(target_os = "windows")]
pub use imp::{cursor_position_px, enumerate_monitors, move_window_px, window_origin_px};

/// DPI scale of the monitor a window is currently on.
#[cfg(target_os = "windows")]
pub fn window_scale(hwnd: OverlayHwnd) -> Option<f32> {
    imp::window_scale(hwnd)
}

#[cfg(not(target_os = "windows"))]
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    finalize(Vec::new())
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_position_px() -> Option<(i32, i32)> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn window_scale(_hwnd: OverlayHwnd) -> Option<f32> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn window_origin_px(_hwnd: OverlayHwnd) -> Option<(i32, i32)> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn move_window_px(_hwnd: OverlayHwnd, _x: i32, _y: i32) {}

#[cfg(test)]
mod tests {
    use super::*;
    use keyoverlay_core::PxRect;

    fn m(id: &str, x: i32, w: i32, primary: bool) -> MonitorInfo {
        MonitorInfo {
            id: id.into(),
            number: 0,
            primary,
            geometry: MonitorGeometry {
                bounds: PxRect::new(x, 0, w, 1080),
                work: PxRect::new(x, 0, w, 1040),
                scale: 1.0,
            },
        }
    }

    fn three() -> Vec<MonitorInfo> {
        finalize(vec![
            m("B", 0, 1920, true),
            m("C", 1920, 2560, false),
            m("A", -1280, 1280, false),
        ])
    }

    #[test]
    fn monitors_are_numbered_left_to_right() {
        let mons = three();
        let ids: Vec<_> = mons.iter().map(|m| (m.number, m.id.as_str())).collect();
        assert_eq!(ids, vec![(1, "A"), (2, "B"), (3, "C")]);
        assert!(mons[1].primary);
        assert!(mons[1].label().contains("(Primary)"));
    }

    #[test]
    fn empty_list_gets_a_fallback_primary() {
        let mons = finalize(Vec::new());
        assert_eq!(mons.len(), 1);
        assert!(mons[0].primary);
    }

    #[test]
    fn resolves_primary_specific_and_missing() {
        let mons = three();
        assert_eq!(
            resolve_target(&mons, &MonitorTarget::Primary, None).index,
            1
        );
        let r = resolve_target(&mons, &MonitorTarget::Display("C".into()), None);
        assert_eq!((r.index, r.fell_back), (2, false));
        let r = resolve_target(&mons, &MonitorTarget::Display("gone".into()), None);
        assert_eq!((r.index, r.fell_back), (1, true));
    }

    #[test]
    fn follow_cursor_picks_monitor_under_cursor() {
        let mons = three();
        let t = MonitorTarget::FollowCursor;
        assert_eq!(resolve_target(&mons, &t, Some((-10, 500))).index, 0);
        assert_eq!(resolve_target(&mons, &t, Some((3000, 10))).index, 2);
        // Off every monitor (or unknown) -> primary.
        assert_eq!(resolve_target(&mons, &t, Some((99999, 0))).index, 1);
        assert_eq!(resolve_target(&mons, &t, None).index, 1);
    }
}
