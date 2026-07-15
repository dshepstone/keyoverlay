//! Keystroke sound engine.
//!
//! Plays short click sounds on keydown events. Uses the Windows multimedia API
//! (PlaySoundW) on Windows and is a no-op stub on other platforms.
//!
//! Sound data is generated at compile-time as embedded WAV byte arrays, avoiding
//! external asset files.

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

/// Identifies a sound pack (matches UI preset names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPack {
    None,
    Typewriter,
    Mechanical,
    SoftClick,
    Pop,
}

impl SoundPack {
    pub fn from_preset_name(name: &str) -> Self {
        match name {
            "Typewriter" => SoundPack::Typewriter,
            "Mechanical" => SoundPack::Mechanical,
            "Soft Click" => SoundPack::SoftClick,
            "Pop" => SoundPack::Pop,
            _ => SoundPack::None,
        }
    }

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            SoundPack::None => "None",
            SoundPack::Typewriter => "Typewriter",
            SoundPack::Mechanical => "Mechanical",
            SoundPack::SoftClick => "Soft Click",
            SoundPack::Pop => "Pop",
        }
    }
}

/// Settings snapshot for the sound engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundSettings {
    pub enabled: bool,
    pub volume: f32,
    pub pack: SoundPack,
}

impl SoundSettings {
    pub fn from_config(cfg: &keyoverlay_core::AppConfig) -> Self {
        Self {
            enabled: cfg.enable_keystroke_sounds,
            volume: cfg.sound_volume.clamp(0.0, 1.0),
            pack: SoundPack::from_preset_name(&cfg.sound_preset),
        }
    }
}

// ── WAV generation ──────────────────────────────────────────────────────
// These functions are used on Windows (via `mod imp`) and in tests.

#[cfg(any(target_os = "windows", test))]
/// Generate a minimal 16-bit mono PCM WAV in memory.
fn generate_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;
    let mut buf = Vec::with_capacity(file_size as usize + 8);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }

    buf
}

#[cfg(any(target_os = "windows", test))]
/// Generate a short "typewriter" click sound (~15ms sharp attack, quick decay).
fn gen_typewriter(volume: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let duration_ms = 18;
    let num_samples = (sample_rate as usize * duration_ms) / 1000;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let freq = 3200.0 + (i as f32 / num_samples as f32) * -1800.0; // descending
        let envelope = if i < num_samples / 6 {
            i as f32 / (num_samples as f32 / 6.0)
        } else {
            1.0 - ((i - num_samples / 6) as f32 / (num_samples as f32 * 5.0 / 6.0))
        };
        let noise = ((i as f32 * 17.3).sin() * 0.3) + ((i as f32 * 31.7).sin() * 0.2);
        let wave = (t * freq * std::f32::consts::TAU).sin() * 0.5 + noise;
        let sample = (wave * envelope * volume * 20000.0).clamp(-32000.0, 32000.0) as i16;
        samples.push(sample);
    }

    generate_wav(sample_rate, &samples)
}

#[cfg(any(target_os = "windows", test))]
/// Generate a "mechanical" keyboard click (~25ms, lower pitch, snappy).
fn gen_mechanical(volume: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let duration_ms = 28;
    let num_samples = (sample_rate as usize * duration_ms) / 1000;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let freq = 1800.0;
        let envelope = if i < num_samples / 8 {
            i as f32 / (num_samples as f32 / 8.0)
        } else {
            let decay_pos = (i - num_samples / 8) as f32 / (num_samples as f32 * 7.0 / 8.0);
            (1.0 - decay_pos).max(0.0).powf(1.5)
        };
        let wave = (t * freq * std::f32::consts::TAU).sin() * 0.6
            + (t * freq * 2.0 * std::f32::consts::TAU).sin() * 0.25
            + ((i as f32 * 23.1).sin() * 0.15);
        let sample = (wave * envelope * volume * 22000.0).clamp(-32000.0, 32000.0) as i16;
        samples.push(sample);
    }

    generate_wav(sample_rate, &samples)
}

#[cfg(any(target_os = "windows", test))]
/// Generate a "soft click" sound (~20ms, muffled, gentle).
fn gen_soft_click(volume: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let duration_ms = 22;
    let num_samples = (sample_rate as usize * duration_ms) / 1000;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let freq = 800.0;
        let envelope = {
            let pos = i as f32 / num_samples as f32;
            (1.0 - pos).powf(2.0)
        };
        let wave = (t * freq * std::f32::consts::TAU).sin() * 0.7 + ((i as f32 * 7.3).sin() * 0.3);
        let sample = (wave * envelope * volume * 14000.0).clamp(-32000.0, 32000.0) as i16;
        samples.push(sample);
    }

    generate_wav(sample_rate, &samples)
}

#[cfg(any(target_os = "windows", test))]
/// Generate a "pop" sound (~12ms, round, bubbly).
fn gen_pop(volume: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let duration_ms = 15;
    let num_samples = (sample_rate as usize * duration_ms) / 1000;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let progress = i as f32 / num_samples as f32;
        let freq = 2400.0 * (1.0 - progress * 0.6); // descending pitch
        let envelope = if progress < 0.1 {
            progress / 0.1
        } else {
            (1.0 - (progress - 0.1) / 0.9).powf(1.2)
        };
        let wave = (t * freq * std::f32::consts::TAU).sin();
        let sample = (wave * envelope * volume * 24000.0).clamp(-32000.0, 32000.0) as i16;
        samples.push(sample);
    }

    generate_wav(sample_rate, &samples)
}

#[cfg(any(target_os = "windows", test))]
/// Pre-generate all sound pack WAVs at a given volume level.
fn generate_pack_wav(pack: SoundPack, volume: f32) -> Vec<u8> {
    match pack {
        SoundPack::None => Vec::new(),
        SoundPack::Typewriter => gen_typewriter(volume),
        SoundPack::Mechanical => gen_mechanical(volume),
        SoundPack::SoftClick => gen_soft_click(volume),
        SoundPack::Pop => gen_pop(volume),
    }
}

#[cfg(target_os = "windows")]
fn debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("KEYOVERLAY_DEBUG").is_ok_and(|v| v == "1"))
}

// ── Platform implementation ─────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod imp {
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT, SND_SYNC};

    use super::{debug_enabled, generate_pack_wav, SoundPack, SoundSettings};

    /// Commands sent to the dedicated audio thread.
    enum AudioCmd {
        /// Play the given WAV data synchronously.
        Play(Arc<Vec<u8>>),
        /// Shut down the audio thread.
        Stop,
    }

    struct EngineState {
        settings: SoundSettings,
        /// Cached WAV data for current pack + volume (shared with audio thread).
        cached_wav: Arc<Vec<u8>>,
        /// Cache key: (enabled, pack, quantized volume). `enabled` must be part
        /// of the key — the cache is intentionally left empty while sounds are
        /// disabled, so re-enabling with an unchanged pack/volume still has to
        /// regenerate the WAV.
        cached_key: (bool, SoundPack, u8),
    }

    fn quantize_volume(volume: f32) -> u8 {
        (volume.clamp(0.0, 1.0) * 20.0).round() as u8
    }

    pub struct SoundEngine {
        state: Mutex<EngineState>,
        audio_tx: mpsc::Sender<AudioCmd>,
    }

    impl SoundEngine {
        pub fn new(initial: SoundSettings) -> Self {
            let wav = if initial.enabled && initial.pack != SoundPack::None {
                generate_pack_wav(initial.pack, initial.volume)
            } else {
                Vec::new()
            };

            let (tx, rx) = mpsc::channel::<AudioCmd>();

            // Dedicated audio thread — stays alive for the lifetime of the engine.
            // Receives Play commands and calls PlaySoundW with SND_SYNC so each
            // sound plays to completion.  Short sounds (15-30 ms) finish quickly;
            // if keystrokes arrive faster, intermediate sounds are skipped by
            // draining the channel to the most recent command.
            thread::Builder::new()
                .name("sound-engine".into())
                .spawn(move || {
                    if debug_enabled() {
                        eprintln!("[sound-engine] audio thread started");
                    }
                    // Block until a command arrives; exit when the channel closes.
                    while let Ok(cmd) = rx.recv() {
                        // Drain to the newest Play command so we don't queue up
                        // a backlog of sounds during fast typing.
                        let cmd = rx.try_iter().fold(cmd, |_prev, newer| newer);

                        match cmd {
                            AudioCmd::Play(wav_data) => {
                                // SAFETY: PlaySoundW with SND_MEMORY interprets
                                // the first parameter as a pointer to WAV data.
                                // SND_SYNC blocks until playback is complete.
                                unsafe {
                                    let _ = PlaySoundW(
                                        PCWSTR(wav_data.as_ptr() as *const u16),
                                        None,
                                        SND_MEMORY | SND_SYNC | SND_NODEFAULT,
                                    );
                                }
                            }
                            AudioCmd::Stop => break,
                        }
                    }
                    if debug_enabled() {
                        eprintln!("[sound-engine] audio thread stopped");
                    }
                })
                .expect("failed to spawn sound-engine thread");

            Self {
                state: Mutex::new(EngineState {
                    settings: initial,
                    cached_wav: Arc::new(wav),
                    cached_key: (
                        initial.enabled,
                        initial.pack,
                        quantize_volume(initial.volume),
                    ),
                }),
                audio_tx: tx,
            }
        }

        pub fn update_settings(&self, settings: SoundSettings) {
            let mut state = self.state.lock().unwrap();
            let key = (
                settings.enabled,
                settings.pack,
                quantize_volume(settings.volume),
            );
            if state.cached_key != key {
                let wav = if settings.enabled && settings.pack != SoundPack::None {
                    generate_pack_wav(settings.pack, settings.volume)
                } else {
                    Vec::new()
                };
                state.cached_wav = Arc::new(wav);
                state.cached_key = key;
                if debug_enabled() {
                    eprintln!(
                        "[sound-engine] regenerated wav pack={:?} vol={:.2}",
                        settings.pack, settings.volume
                    );
                }
            }
            state.settings = settings;
        }

        pub fn play_keystroke(&self) {
            let state = self.state.lock().unwrap();
            if !state.settings.enabled || state.settings.pack == SoundPack::None {
                return;
            }
            if state.cached_wav.is_empty() {
                return;
            }

            let wav = Arc::clone(&state.cached_wav);
            drop(state);

            // Throttle logging
            static LAST_LOG: Mutex<Option<Instant>> = Mutex::new(None);
            if debug_enabled() {
                let mut last = LAST_LOG.lock().unwrap();
                let should_log = last.map(|t| t.elapsed().as_millis() > 200).unwrap_or(true);
                if should_log {
                    eprintln!("[sound-engine] play keystroke ({} bytes)", wav.len());
                    *last = Some(Instant::now());
                }
            }

            // Send to the audio thread — non-blocking for the caller.
            let _ = self.audio_tx.send(AudioCmd::Play(wav));
        }
    }

    impl Drop for SoundEngine {
        fn drop(&mut self) {
            let _ = self.audio_tx.send(AudioCmd::Stop);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::SoundSettings;

    pub struct SoundEngine;

    impl SoundEngine {
        pub fn new(_initial: SoundSettings) -> Self {
            Self
        }

        pub fn update_settings(&self, _settings: SoundSettings) {}

        pub fn play_keystroke(&self) {}
    }
}

pub use imp::SoundEngine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_pack_from_preset_name() {
        assert_eq!(
            SoundPack::from_preset_name("Typewriter"),
            SoundPack::Typewriter
        );
        assert_eq!(
            SoundPack::from_preset_name("Mechanical"),
            SoundPack::Mechanical
        );
        assert_eq!(
            SoundPack::from_preset_name("Soft Click"),
            SoundPack::SoftClick
        );
        assert_eq!(SoundPack::from_preset_name("Pop"), SoundPack::Pop);
        assert_eq!(SoundPack::from_preset_name("None"), SoundPack::None);
        assert_eq!(SoundPack::from_preset_name("unknown"), SoundPack::None);
    }

    #[test]
    fn generated_wav_has_valid_header() {
        for pack in [
            SoundPack::Typewriter,
            SoundPack::Mechanical,
            SoundPack::SoftClick,
            SoundPack::Pop,
        ] {
            let wav = generate_pack_wav(pack, 0.5);
            assert!(
                wav.len() > 44,
                "{:?} wav too short: {} bytes",
                pack,
                wav.len()
            );

            // Check RIFF header
            assert_eq!(&wav[0..4], b"RIFF", "{:?} missing RIFF", pack);
            assert_eq!(&wav[8..12], b"WAVE", "{:?} missing WAVE", pack);
            assert_eq!(&wav[12..16], b"fmt ", "{:?} missing fmt", pack);
            assert_eq!(&wav[36..40], b"data", "{:?} missing data", pack);

            // Check PCM format (1)
            let format = u16::from_le_bytes([wav[20], wav[21]]);
            assert_eq!(format, 1, "{:?} not PCM", pack);

            // Check mono (1 channel)
            let channels = u16::from_le_bytes([wav[22], wav[23]]);
            assert_eq!(channels, 1, "{:?} not mono", pack);

            // Check 16-bit
            let bits = u16::from_le_bytes([wav[34], wav[35]]);
            assert_eq!(bits, 16, "{:?} not 16-bit", pack);
        }
    }

    #[test]
    fn none_pack_generates_empty_wav() {
        let wav = generate_pack_wav(SoundPack::None, 1.0);
        assert!(wav.is_empty());
    }

    #[test]
    fn volume_affects_amplitude() {
        let loud = generate_pack_wav(SoundPack::Typewriter, 1.0);
        let quiet = generate_pack_wav(SoundPack::Typewriter, 0.1);

        // Both should be valid WAVs with same sample count
        assert_eq!(loud.len(), quiet.len());

        // Compare peak amplitudes in the sample data
        fn peak_amplitude(wav: &[u8]) -> i16 {
            let samples = &wav[44..];
            samples
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]).abs())
                .max()
                .unwrap_or(0)
        }

        let loud_peak = peak_amplitude(&loud);
        let quiet_peak = peak_amplitude(&quiet);
        assert!(
            loud_peak > quiet_peak,
            "loud peak {} should be > quiet peak {}",
            loud_peak,
            quiet_peak
        );
    }

    #[test]
    fn sound_settings_from_config() {
        let cfg = keyoverlay_core::AppConfig {
            enable_keystroke_sounds: true,
            sound_volume: 0.75,
            sound_preset: "Pop".to_string(),
            ..keyoverlay_core::AppConfig::default()
        };

        let settings = SoundSettings::from_config(&cfg);
        assert!(settings.enabled);
        assert!((settings.volume - 0.75).abs() < f32::EPSILON);
        assert_eq!(settings.pack, SoundPack::Pop);
    }
}
