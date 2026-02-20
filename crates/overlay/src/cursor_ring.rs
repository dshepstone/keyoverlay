use keyoverlay_core::{AppConfig, CursorShape, CursorTheme};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorRingSettings {
    pub enabled: bool,
    pub diameter_px: i32,
    pub thickness_px: i32,
    pub opacity: f32,
    /// RGB color for the ring/dot.
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    /// Whether to render as a filled dot (true) or ring stroke (false).
    pub filled: bool,
    /// Glow intensity 0.0..1.0.
    pub glow: f32,
    /// Hide cursor highlight after this many ms of inactivity (0 = never).
    pub hide_after_ms: u32,
    /// Enable click animation pulse.
    pub click_animation: bool,
    /// Click animation accent color.
    pub click_accent_r: u8,
    pub click_accent_g: u8,
    pub click_accent_b: u8,
    /// When true, the cursor hotspot is at the center of the circle.
    /// When false, the cursor hotspot touches the right edge of the circle
    /// (circle shifts left from the arrow tip).
    pub hotspot_center: bool,
}

impl CursorRingSettings {
    pub fn from_config(cfg: &AppConfig) -> Self {
        let theme = cfg.cursor_theme.unwrap_or(CursorTheme::GreenRing);
        let desc = theme.desc();
        // Use the user's glow setting, but fall back to theme default if user hasn't adjusted
        let glow = if cfg.cursor_glow > 0.0 {
            cfg.cursor_glow
        } else {
            desc.default_glow
        };
        Self {
            enabled: cfg.enable_green_cursor_ring,
            diameter_px: cfg.cursor_ring_size_px.round().clamp(24.0, 200.0) as i32,
            thickness_px: cfg.cursor_ring_thickness_px.round().clamp(2.0, 12.0) as i32,
            opacity: cfg.cursor_ring_opacity.clamp(0.2, 1.0),
            color_r: desc.base_color.r,
            color_g: desc.base_color.g,
            color_b: desc.base_color.b,
            filled: desc.shape == CursorShape::FilledDot,
            glow,
            hide_after_ms: cfg.cursor_hide_after_ms,
            click_animation: cfg.enable_click_animation,
            click_accent_r: desc.click_accent.r,
            click_accent_g: desc.click_accent.g,
            click_accent_b: desc.click_accent.b,
            hotspot_center: cfg.cursor_hotspot_center,
        }
    }
}

/// Signal a mouse click to the cursor ring controller (for click animation).
#[derive(Clone, Copy, Debug)]
pub struct ClickEvent {
    pub is_down: bool,
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

    use super::{ClickEvent, CursorRingSettings};

    fn debug_enabled() -> bool {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| {
            std::env::var("KEYOVERLAY_DEBUG").is_ok_and(|v| v == "1")
                || std::env::var("OVERLAY_REPAINT_DEBUG").is_ok_and(|v| v == "1")
        })
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

    /// Convert RGB to COLORREF (0x00BBGGRR).
    fn rgb_to_colorref(r: u8, g: u8, b: u8) -> COLORREF {
        COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
    }

    /// Draw the cursor highlight on the window DC.
    fn draw_highlight(hwnd: HWND, settings: &CursorRingSettings, click_expand: f32) {
        if !unsafe { IsWindow(hwnd).as_bool() } {
            return;
        }

        let dc = unsafe { GetDC(hwnd) };
        if dc.0.is_null() {
            return;
        }

        // Total window size includes glow margin + click expand
        let glow_margin = if settings.glow > 0.0 {
            (settings.diameter_px as f32 * settings.glow * 0.5).round() as i32
        } else {
            0
        };
        let expand_margin = (click_expand * settings.diameter_px as f32 * 0.3).round() as i32;
        let total_size = settings.diameter_px + glow_margin * 2 + expand_margin * 2;

        // Clear with transparent (black = color key)
        let rect = RECT {
            left: 0,
            top: 0,
            right: total_size,
            bottom: total_size,
        };
        let background = unsafe { CreateSolidBrush(COLORREF(0)) };
        unsafe {
            FillRect(dc, &rect, background);
        }
        let _ = unsafe { DeleteObject(background) };

        let center = total_size / 2;
        let base_radius = settings.diameter_px / 2;

        // Draw glow layers (concentric rings with decreasing alpha effect via thinner pens)
        if settings.glow > 0.0 {
            let glow_steps = 3;
            for i in 1..=glow_steps {
                let t = i as f32 / glow_steps as f32;
                let glow_radius = base_radius + (glow_margin as f32 * t).round() as i32;
                // Blend color toward background (darken for glow fade)
                let fade = (1.0 - t * 0.7) * settings.glow;
                let gr = (settings.color_r as f32 * fade).round() as u8;
                let gg = (settings.color_g as f32 * fade).round() as u8;
                let gb = (settings.color_b as f32 * fade).round() as u8;
                // Avoid pure black (color key)
                let gr = gr.max(1);
                let gg = gg.max(1);

                let glow_color = rgb_to_colorref(gr, gg, gb);
                let pen_thick = ((settings.thickness_px as f32) * (1.0 - t * 0.5))
                    .round()
                    .max(1.0) as i32;
                let pen = unsafe { CreatePen(PS_SOLID, pen_thick, glow_color) };
                let old_pen = unsafe { SelectObject(dc, HGDIOBJ(pen.0)) };
                let hollow = unsafe { GetStockObject(HOLLOW_BRUSH) };
                let old_brush = unsafe { SelectObject(dc, hollow) };

                let inset = pen_thick / 2;
                let left = center - glow_radius + inset;
                let top = center - glow_radius + inset;
                let right = center + glow_radius - inset;
                let bottom = center + glow_radius - inset;
                unsafe {
                    let _ = Ellipse(dc, left, top, right, bottom);
                    SelectObject(dc, old_pen);
                    SelectObject(dc, old_brush);
                }
                let _ = unsafe { DeleteObject(pen) };
            }
        }

        // Draw click animation expanding ring
        if click_expand > 0.01 {
            let expand_radius = base_radius + (expand_margin as f32 * click_expand).round() as i32;
            let cr = settings.click_accent_r.max(1);
            let cg = settings.click_accent_g.max(1);
            let cb = settings.click_accent_b;
            let anim_color = rgb_to_colorref(cr, cg, cb);
            let pen_thick = ((settings.thickness_px as f32) * (1.0 - click_expand * 0.6))
                .round()
                .max(1.0) as i32;
            let pen = unsafe { CreatePen(PS_SOLID, pen_thick, anim_color) };
            let old_pen = unsafe { SelectObject(dc, HGDIOBJ(pen.0)) };
            let hollow = unsafe { GetStockObject(HOLLOW_BRUSH) };
            let old_brush = unsafe { SelectObject(dc, hollow) };

            let inset = pen_thick / 2;
            let left = center - expand_radius + inset;
            let top = center - expand_radius + inset;
            let right = center + expand_radius - inset;
            let bottom = center + expand_radius - inset;
            unsafe {
                let _ = Ellipse(dc, left, top, right, bottom);
                SelectObject(dc, old_pen);
                SelectObject(dc, old_brush);
            }
            let _ = unsafe { DeleteObject(pen) };
        }

        // Draw main highlight (ring or filled dot)
        // Ensure color is not pure black (color key = 0x000000)
        let r = settings.color_r.max(1);
        let g = settings.color_g.max(1);
        let b = settings.color_b;
        let ring_color = rgb_to_colorref(r, g, b);

        if settings.filled {
            // Filled dot
            let brush = unsafe { CreateSolidBrush(ring_color) };
            let pen = unsafe { CreatePen(PS_SOLID, 1, ring_color) };
            let old_pen = unsafe { SelectObject(dc, HGDIOBJ(pen.0)) };
            let old_brush = unsafe { SelectObject(dc, HGDIOBJ(brush.0 as _)) };

            let left = center - base_radius;
            let top = center - base_radius;
            let right = center + base_radius;
            let bottom = center + base_radius;
            unsafe {
                let _ = Ellipse(dc, left, top, right, bottom);
                SelectObject(dc, old_pen);
                SelectObject(dc, old_brush);
            }
            let _ = unsafe { DeleteObject(pen) };
            let _ = unsafe { DeleteObject(brush) };
        } else {
            // Ring stroke
            let pen = unsafe { CreatePen(PS_SOLID, settings.thickness_px, ring_color) };
            let old_pen = unsafe { SelectObject(dc, HGDIOBJ(pen.0)) };
            let hollow = unsafe { GetStockObject(HOLLOW_BRUSH) };
            let old_brush = unsafe { SelectObject(dc, hollow) };

            let inset = (settings.thickness_px / 2).max(1);
            let left = center - base_radius + inset;
            let top = center - base_radius + inset;
            let right = center + base_radius - inset;
            let bottom = center + base_radius - inset;
            unsafe {
                let _ = Ellipse(dc, left, top, right, bottom);
                SelectObject(dc, old_pen);
                SelectObject(dc, old_brush);
            }
            let _ = unsafe { DeleteObject(pen) };
        }

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

    /// Internal shared state between controller and render thread.
    struct SharedState {
        settings: CursorRingSettings,
        pending_click: bool,
    }

    pub struct CursorRingController {
        state: Arc<Mutex<SharedState>>,
        running: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl CursorRingController {
        pub fn new(initial: CursorRingSettings) -> Self {
            let state = Arc::new(Mutex::new(SharedState {
                settings: initial,
                pending_click: false,
            }));
            let running = Arc::new(AtomicBool::new(true));
            let state_for_thread = Arc::clone(&state);
            let running_for_thread = Arc::clone(&running);

            let thread = thread::spawn(move || {
                let mut hwnd: Option<HWND> = None;
                let mut last_cursor: Option<(i32, i32)> = None;
                let mut last_move = Instant::now() - Duration::from_millis(16);
                let mut last_motion_at = Instant::now();
                let mut hidden_by_idle = false;
                let mut last_settings = CursorRingSettings {
                    enabled: false,
                    diameter_px: 48,
                    thickness_px: 4,
                    opacity: 0.9,
                    color_r: 100,
                    color_g: 220,
                    color_b: 100,
                    filled: false,
                    glow: 0.0,
                    hide_after_ms: 0,
                    click_animation: false,
                    click_accent_r: 180,
                    click_accent_g: 255,
                    click_accent_b: 180,
                    hotspot_center: true,
                };

                // Click animation state
                let mut click_anim_start: Option<Instant> = None;
                const CLICK_ANIM_DURATION_MS: f32 = 160.0;

                // Throttle debug logging
                let mut last_debug_log = Instant::now() - Duration::from_secs(1);

                while running_for_thread.load(Ordering::Relaxed) {
                    let (settings, pending_click) = {
                        let mut st = state_for_thread.lock().unwrap();
                        let s = st.settings;
                        let click = st.pending_click;
                        st.pending_click = false;
                        (s, click)
                    };

                    // Handle click animation trigger
                    if pending_click && settings.click_animation {
                        click_anim_start = Some(Instant::now());
                        if debug_enabled() {
                            eprintln!("[cursor-ring] click animation triggered");
                        }
                    }

                    // Calculate click expand factor (0..1)
                    let click_expand = if let Some(start) = click_anim_start {
                        let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
                        if elapsed_ms >= CLICK_ANIM_DURATION_MS {
                            click_anim_start = None;
                            0.0
                        } else {
                            let t = elapsed_ms / CLICK_ANIM_DURATION_MS;
                            // Quick rise, smooth fade: ease-out
                            let rise = 1.0 - (1.0 - t).powi(3);
                            // Fade: starts at 1 and goes to 0
                            let fade = 1.0 - t;
                            rise * fade
                        }
                    } else {
                        0.0
                    };

                    if settings.enabled && hwnd.is_none() {
                        hwnd = create_cursor_ring_window();
                        last_motion_at = Instant::now();
                        hidden_by_idle = false;
                        if debug_enabled() {
                            eprintln!("[cursor-ring] enabled, window created");
                        }
                    }

                    if !settings.enabled {
                        if let Some(window) = hwnd.take() {
                            unsafe {
                                ShowWindow(window, SW_HIDE);
                                let _ = DestroyWindow(window);
                            }
                            if debug_enabled() {
                                eprintln!("[cursor-ring] disabled, window destroyed");
                            }
                        }
                        thread::sleep(Duration::from_millis(25));
                        continue;
                    }

                    if let Some(window) = hwnd {
                        let settings_changed = settings != last_settings;
                        let needs_redraw = settings_changed || click_expand > 0.01;

                        if needs_redraw {
                            let dpi_scale = scale_for_dpi(window);
                            let glow_margin = if settings.glow > 0.0 {
                                (settings.diameter_px as f32 * settings.glow * 0.5).round() as i32
                            } else {
                                0
                            };
                            let expand_margin = if settings.click_animation {
                                (settings.diameter_px as f32 * 0.3).round() as i32
                            } else {
                                0
                            };
                            let total_size_base =
                                settings.diameter_px + glow_margin * 2 + expand_margin * 2;
                            let scaled_total = (total_size_base as f32 * dpi_scale).round() as i32;
                            unsafe {
                                let _ = SetWindowPos(
                                    window,
                                    HWND_TOPMOST,
                                    0,
                                    0,
                                    scaled_total,
                                    scaled_total,
                                    SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
                                );
                                ShowWindow(window, SW_SHOWNOACTIVATE);
                            }
                            set_window_alpha(window, settings.opacity);
                            // Draw with scaled pixel dimensions
                            let scaled_settings = CursorRingSettings {
                                diameter_px: (settings.diameter_px as f32 * dpi_scale).round()
                                    as i32,
                                thickness_px: (settings.thickness_px as f32 * dpi_scale).round()
                                    as i32,
                                ..settings
                            };
                            draw_highlight(window, &scaled_settings, click_expand);
                            last_settings = settings;
                        }

                        // Track cursor position
                        let mut point = POINT::default();
                        if unsafe { GetCursorPos(&mut point) }.is_ok() {
                            let moved = last_cursor != Some((point.x, point.y));
                            let due = last_move.elapsed() >= Duration::from_millis(12);

                            if moved {
                                last_motion_at = Instant::now();
                                if hidden_by_idle {
                                    hidden_by_idle = false;
                                    unsafe {
                                        ShowWindow(window, SW_SHOWNOACTIVATE);
                                    }
                                    if debug_enabled() {
                                        eprintln!("[cursor-ring] shown (motion resumed)");
                                    }
                                }
                            }

                            // Also show on click
                            if pending_click && hidden_by_idle {
                                hidden_by_idle = false;
                                last_motion_at = Instant::now();
                                unsafe {
                                    ShowWindow(window, SW_SHOWNOACTIVATE);
                                }
                                if debug_enabled() {
                                    eprintln!("[cursor-ring] shown (click while idle)");
                                }
                            }

                            // Hide-after-idle logic
                            if settings.hide_after_ms > 0
                                && !hidden_by_idle
                                && last_motion_at.elapsed()
                                    > Duration::from_millis(settings.hide_after_ms as u64)
                            {
                                hidden_by_idle = true;
                                unsafe {
                                    ShowWindow(window, SW_HIDE);
                                }
                                if debug_enabled() {
                                    eprintln!(
                                        "[cursor-ring] hidden (idle {}ms)",
                                        settings.hide_after_ms
                                    );
                                }
                            }

                            if (moved || due) && !hidden_by_idle {
                                let dpi_scale = scale_for_dpi(window);
                                let glow_margin = if settings.glow > 0.0 {
                                    (settings.diameter_px as f32 * settings.glow * 0.5).round()
                                        as i32
                                } else {
                                    0
                                };
                                let expand_margin = if settings.click_animation {
                                    (settings.diameter_px as f32 * 0.3).round() as i32
                                } else {
                                    0
                                };
                                let total_size_base =
                                    settings.diameter_px + glow_margin * 2 + expand_margin * 2;
                                let scaled_total =
                                    (total_size_base as f32 * dpi_scale).round() as i32;
                                let radius =
                                    (settings.diameter_px as f32 * dpi_scale * 0.5).round() as i32;
                                let (x, y) = if settings.hotspot_center {
                                    // Centered: top-left = mouse - radius.
                                    (point.x - radius, point.y - radius)
                                } else {
                                    // Non-centered mode: cursor hotspot touches the circle's right edge.
                                    // circle_center = hotspot - (radius, 0)
                                    // window_top_left = circle_center - scaled_total/2
                                    (
                                        point.x - radius - (scaled_total / 2),
                                        point.y - (scaled_total / 2),
                                    )
                                };
                                unsafe {
                                    let _ = SetWindowPos(
                                        window,
                                        HWND_TOPMOST,
                                        x,
                                        y,
                                        scaled_total,
                                        scaled_total,
                                        SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
                                    );
                                }
                                if debug_enabled()
                                    && moved
                                    && last_debug_log.elapsed() >= Duration::from_millis(500)
                                {
                                    let mode = if settings.hotspot_center {
                                        "centered"
                                    } else {
                                        "right-edge-touch"
                                    };
                                    eprintln!(
                                        "[cursor-ring] mode={} hotspot=({},{}) radius={} window=({},{}) total_size={}",
                                        mode, point.x, point.y, radius, x, y, scaled_total
                                    );
                                    last_debug_log = Instant::now();
                                }
                                last_cursor = Some((point.x, point.y));
                                last_move = Instant::now();
                            }
                        }
                    }

                    // Faster loop during click animation for smooth rendering
                    if click_anim_start.is_some() {
                        thread::sleep(Duration::from_millis(4));
                    } else {
                        thread::sleep(Duration::from_millis(8));
                    }
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
            self.state.lock().unwrap().settings = settings;
        }

        pub fn notify_click(&self, _event: ClickEvent) {
            self.state.lock().unwrap().pending_click = true;
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
    use super::{ClickEvent, CursorRingSettings};

    pub struct CursorRingController;

    impl CursorRingController {
        pub fn new(_initial: CursorRingSettings) -> Self {
            Self
        }

        pub fn sync(&self, _settings: CursorRingSettings) {}

        pub fn notify_click(&self, _event: ClickEvent) {}

        pub fn shutdown(&mut self) {}
    }
}

pub use imp::CursorRingController;
