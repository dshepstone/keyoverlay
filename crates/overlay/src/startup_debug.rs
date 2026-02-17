use std::env;
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupExperiment {
    None,
    ADisableDwmBeforeVisible,
    BRegionNoRedrawThenRedraw,
    CDelayRegion,
    DHiddenUntilPrepared,
    ESkipRegion,
    FSkipDwm,
    GEnableTransparencyAfterFirstShow,
}

impl StartupExperiment {
    pub fn from_env() -> Self {
        let raw = env::var("OVERLAY_STARTUP_EXPERIMENT").unwrap_or_default();
        match raw.trim().to_ascii_uppercase().as_str() {
            "A" => Self::ADisableDwmBeforeVisible,
            "B" => Self::BRegionNoRedrawThenRedraw,
            "C" => Self::CDelayRegion,
            "D" => Self::DHiddenUntilPrepared,
            "E" => Self::ESkipRegion,
            "F" => Self::FSkipDwm,
            "G" => Self::GEnableTransparencyAfterFirstShow,
            _ => Self::None,
        }
    }
}

pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("OVERLAY_DEBUG_STARTUP").is_ok_and(|v| v == "1"))
}

pub fn process_start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

pub fn elapsed_ms() -> u128 {
    process_start().elapsed().as_millis()
}

pub fn log(msg: impl AsRef<str>) {
    if enabled() {
        eprintln!("[overlay-startup +{:>6}ms] {}", elapsed_ms(), msg.as_ref());
    }
}
