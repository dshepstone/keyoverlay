use std::env;
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayExperiment {
    Baseline,
    NoRegion,
    RegionNoRedraw,
    HiddenUntilReady,
    NoTransparencyUntilReady,
    EarlyDwmDisable,
    NoAutoPosition,
    DelayRegion,
    NoDwmDisable,
}

impl OverlayExperiment {
    pub fn from_env() -> Self {
        let raw = env::var("OVERLAY_EXPERIMENT")
            .or_else(|_| env::var("OVERLAY_STARTUP_EXPERIMENT"))
            .unwrap_or_default();

        match raw.trim().to_ascii_lowercase().as_str() {
            "baseline" | "" => Self::Baseline,
            "no_region" | "e" => Self::NoRegion,
            "region_no_redraw" | "b" => Self::RegionNoRedraw,
            "hidden_until_ready" | "d" => Self::HiddenUntilReady,
            "no_transparency_until_ready" | "g" => Self::NoTransparencyUntilReady,
            "early_dwm_disable" | "a" => Self::EarlyDwmDisable,
            "no_autoposition" => Self::NoAutoPosition,
            "delay_region" | "c" => Self::DelayRegion,
            "no_dwm_disable" | "f" => Self::NoDwmDisable,
            _ => Self::Baseline,
        }
    }
}

pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        env::var("OVERLAY_DEBUG").is_ok_and(|v| v == "1")
            || env::var("OVERLAY_DEBUG_STARTUP").is_ok_and(|v| v == "1")
    })
}

pub fn process_start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

pub fn elapsed_ms() -> u128 {
    process_start().elapsed().as_millis()
}

pub fn log_event(msg: impl AsRef<str>) {
    if enabled() {
        eprintln!("[overlay-diag +{:>6}ms] {}", elapsed_ms(), msg.as_ref());
    }
}
