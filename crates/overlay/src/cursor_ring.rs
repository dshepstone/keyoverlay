use keyoverlay_core::AppConfig;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorRingSettings {
    pub enabled: bool,
    pub diameter_px: i32,
    pub thickness_px: i32,
    pub opacity: f32,
}

impl CursorRingSettings {
    pub fn from_config(cfg: &AppConfig) -> Self {
        Self {
            enabled: cfg.enable_green_cursor_ring,
            diameter_px: cfg.cursor_ring_size_px.round().clamp(24.0, 120.0) as i32,
            thickness_px: cfg.cursor_ring_thickness_px.round().clamp(2.0, 12.0) as i32,
            opacity: cfg.cursor_ring_opacity.clamp(0.2, 1.0),
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DeleteObject, Ellipse, FillRect, GetDC, GetStockObject,
        ReleaseDC, SelectObject, HGDIOBJ, HOLLOW_BRUSH, PS_SOLID,
    };
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetCursorPos, IsWindow, SetLayeredWindowAttributes,
        SetWindowPos, ShowWindow, HWND_TOPMOST, LWA_ALPHA, LWA_COLORKEY, SWP_NOACTIVATE,
        SWP_NOOWNERZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WINDOW_EX_STYLE,
        WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
        WS_POPUP,
    };

    use super::CursorRingSettings;

    fn repaint_debug_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED
            .get_or_init(|| std::env::var("OVERLAY_REPAINT_DEBUG").is_ok_and(|value| value == "1"))
    }

    fn static_class_name() -> &'static [u16] {
        static CLASS_NAME: OnceLock<Vec<u16>> = OnceLock::new();
        CLASS_NAME
            .get_or_init(|| "STATIC\0".encode_utf16().collect::<Vec<u16>>())
            .as_slice()
    }

    fn create_cursor_ring_window() -> Option<HWND> {
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(
                    WS_EX_LAYERED.0
                        | WS_EX_TRANSPARENT.0
                        | WS_EX_TOOLWINDOW.0
                        | WS_EX_NOACTIVATE.0
                        | WS_EX_TOPMOST.0,
                ),
                PCWSTR(static_class_name().as_ptr()),
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                Some(std::ptr::null::<c_void>()),
            )
        }
        .ok()?;

        unsafe {
            let _ = SetLayeredWindowAttributes(
                hwnd,
                COLORREF(0),
                (255.0 * 0.9) as u8,
                LWA_COLORKEY | LWA_ALPHA,
            );
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }

        Some(hwnd)
    }

    fn draw_ring(hwnd: HWND, settings: CursorRingSettings) {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return;
        }

        let dc = unsafe { GetDC(hwnd) };
        if dc.0.is_null() {
            return;
        }

        let rect = RECT {
            left: 0,
            top: 0,
            right: settings.diameter_px,
            bottom: settings.diameter_px,
        };

        let background = unsafe { CreateSolidBrush(COLORREF(0)) };
        unsafe {
            FillRect(dc, &rect, background);
        }
        let _ = unsafe { DeleteObject(background) };

        let ring_color = COLORREF(0x0000FF00);
        let pen = unsafe { CreatePen(PS_SOLID, settings.thickness_px, ring_color) };
        let old_pen = unsafe { SelectObject(dc, HGDIOBJ(pen.0)) };
        let hollow = unsafe { GetStockObject(HOLLOW_BRUSH) };
        let old_brush = unsafe { SelectObject(dc, hollow) };

        let inset = (settings.thickness_px / 2).max(1);
        unsafe {
            let _ = Ellipse(
                dc,
                inset,
                inset,
                settings.diameter_px - inset,
                settings.diameter_px - inset,
            );
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
        }
        let _ = unsafe { DeleteObject(pen) };
        let _ = unsafe { ReleaseDC(hwnd, dc) };
    }

    fn scale_for_dpi(hwnd: HWND) -> f32 {
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi == 0 {
            1.0
        } else {
            dpi as f32 / 96.0
        }
    }

    fn set_window_alpha(hwnd: HWND, opacity: f32) {
        let alpha = (opacity.clamp(0.2, 1.0) * 255.0).round() as u8;
        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_COLORKEY | LWA_ALPHA);
        }
    }

    pub struct CursorRingController {
        state: Arc<Mutex<CursorRingSettings>>,
        running: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl CursorRingController {
        pub fn new(initial: CursorRingSettings) -> Self {
            let state = Arc::new(Mutex::new(initial));
            let running = Arc::new(AtomicBool::new(true));
            let state_for_thread = Arc::clone(&state);
            let running_for_thread = Arc::clone(&running);

            let thread = thread::spawn(move || {
                let mut hwnd: Option<HWND> = None;
                let mut last_cursor: Option<(i32, i32)> = None;
                let mut last_move = Instant::now() - Duration::from_millis(16);
                let mut last_settings = CursorRingSettings {
                    enabled: false,
                    diameter_px: 48,
                    thickness_px: 4,
                    opacity: 0.9,
                };

                while running_for_thread.load(Ordering::Relaxed) {
                    let settings = *state_for_thread.lock().unwrap();

                    if settings.enabled && hwnd.is_none() {
                        hwnd = create_cursor_ring_window();
                        if repaint_debug_enabled() {
                            eprintln!("[overlay-repaint] cursor-ring enabled");
                        }
                    }

                    if !settings.enabled {
                        if let Some(window) = hwnd.take() {
                            unsafe {
                                ShowWindow(window, SW_HIDE);
                                let _ = DestroyWindow(window);
                            }
                            if repaint_debug_enabled() {
                                eprintln!("[overlay-repaint] cursor-ring disabled");
                            }
                        }
                        thread::sleep(Duration::from_millis(25));
                        continue;
                    }

                    if let Some(window) = hwnd {
                        if settings != last_settings {
                            let scaled_diameter = ((settings.diameter_px as f32)
                                * scale_for_dpi(window))
                            .round() as i32;
                            unsafe {
                                let _ = SetWindowPos(
                                    window,
                                    HWND_TOPMOST,
                                    0,
                                    0,
                                    scaled_diameter,
                                    scaled_diameter,
                                    SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
                                );
                                ShowWindow(window, SW_SHOWNOACTIVATE);
                            }
                            set_window_alpha(window, settings.opacity);
                            draw_ring(
                                window,
                                CursorRingSettings {
                                    diameter_px: scaled_diameter,
                                    ..settings
                                },
                            );
                            last_settings = settings;
                        }

                        let mut point = POINT::default();
                        if unsafe { GetCursorPos(&mut point) }.is_ok() {
                            let moved = last_cursor != Some((point.x, point.y));
                            let due = last_move.elapsed() >= Duration::from_millis(12);
                            if moved || due {
                                let scaled_diameter =
                                    ((settings.diameter_px as f32) * scale_for_dpi(window)).round()
                                        as i32;
                                let x = point.x - (scaled_diameter / 2);
                                let y = point.y - (scaled_diameter / 2);
                                unsafe {
                                    let _ = SetWindowPos(
                                        window,
                                        HWND_TOPMOST,
                                        x,
                                        y,
                                        scaled_diameter,
                                        scaled_diameter,
                                        SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
                                    );
                                }
                                if repaint_debug_enabled() && moved {
                                    eprintln!(
                                        "[overlay-repaint] cursor-ring move x={} y={} diameter={}",
                                        point.x, point.y, scaled_diameter
                                    );
                                }
                                last_cursor = Some((point.x, point.y));
                                last_move = Instant::now();
                            }
                        }
                    }

                    thread::sleep(Duration::from_millis(8));
                }

                if let Some(window) = hwnd {
                    unsafe {
                        ShowWindow(window, SW_HIDE);
                        let _ = DestroyWindow(window);
                    }
                }
            });

            Self {
                state,
                running,
                thread: Some(thread),
            }
        }

        pub fn sync(&self, settings: CursorRingSettings) {
            *self.state.lock().unwrap() = settings;
        }

        pub fn shutdown(&mut self) {
            self.running.store(false, Ordering::Relaxed);
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
        }
    }

    impl Drop for CursorRingController {
        fn drop(&mut self) {
            self.shutdown();
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::CursorRingSettings;

    pub struct CursorRingController;

    impl CursorRingController {
        pub fn new(_initial: CursorRingSettings) -> Self {
            // TODO: Implement platform cursor ring overlays for non-Windows targets.
            Self
        }

        pub fn sync(&self, _settings: CursorRingSettings) {
            // TODO: Implement platform cursor ring overlays for non-Windows targets.
        }

        pub fn shutdown(&mut self) {
            // TODO: Implement platform cursor ring overlays for non-Windows targets.
        }
    }
}

pub use imp::CursorRingController;
