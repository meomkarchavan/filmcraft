//! AMD Advanced Media Framework (AMF) hardware encoding support.

#[allow(unsafe_code)]
pub mod device;
pub mod export;
#[allow(unsafe_code)]
pub mod ffi;
#[allow(unsafe_code)]
pub mod session;

#[cfg(test)]
mod abi_tests;

use std::sync::Arc;

pub use device::{AmfDevice, AmfRuntime};
pub use export::{amf_encoder_factory, AmfVideoEncoder};
pub use session::{AmfCodec, AmfConfig, AmfPacket, AmfProfile, AmfSession};

/// The answer of [`hevc_available`], once asked.
static HEVC_PROBE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// The answer of [`hevc_hdr_available`], once asked.
static HEVC_HDR_PROBE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Whether this system's AMD AMF can encode HEVC: a runtime loaded, device created, and an HEVC session
/// opened for a small picture, once (the answer is cached).
pub fn hevc_available() -> bool {
    *HEVC_PROBE.get_or_init(|| {
        let runtime = match AmfRuntime::load() {
            Ok(rt) => rt,
            Err(why) => {
                log::info!("no AMF HEVC hardware encoder (runtime load failed): {why}");
                return false;
            }
        };
        let device = match AmfDevice::new(&runtime) {
            Ok(dev) => Arc::new(dev),
            Err(why) => {
                log::info!("no AMF HEVC hardware encoder (device creation failed): {why}");
                return false;
            }
        };
        let probe = AmfConfig {
            width: 640,
            height: 360,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 2000,
            max_bitrate_kbps: 3000,
            keyframe_interval: 30,
            codec: AmfCodec::Hevc,
            profile: AmfProfile::Main,
            cbr: false,
        };
        let result = AmfSession::open(&device, probe);
        if let Err(why) = &result {
            log::info!("no AMF HEVC hardware encoder: {why}");
        }
        result.is_ok()
    })
}

/// Whether this system's AMD AMF can encode HEVC Main 10 (10-bit HDR): HEVC is available and an
/// `AmfProfile::Main10` session opens for a small picture, once (the answer is cached).
pub fn hevc_hdr_available() -> bool {
    *HEVC_HDR_PROBE.get_or_init(|| {
        if !hevc_available() {
            return false;
        }
        let runtime = match AmfRuntime::load() {
            Ok(rt) => rt,
            Err(why) => {
                log::info!("no AMF HEVC Main 10 hardware encoder (runtime load failed): {why}");
                return false;
            }
        };
        let device = match AmfDevice::new(&runtime) {
            Ok(dev) => Arc::new(dev),
            Err(why) => {
                log::info!("no AMF HEVC Main 10 hardware encoder (device creation failed): {why}");
                return false;
            }
        };
        let probe = AmfConfig {
            width: 640,
            height: 360,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 2000,
            max_bitrate_kbps: 3000,
            keyframe_interval: 30,
            codec: AmfCodec::Hevc,
            profile: AmfProfile::Main10,
            cbr: false,
        };
        let result = AmfSession::open(&device, probe);
        if let Err(why) = &result {
            log::info!("no AMF HEVC Main 10 hardware encoder: {why}");
        }
        result.is_ok()
    })
}

/// Ask [`hevc_available`] and [`hevc_hdr_available`] on a background thread (once),
/// so the first draw of Export's format list does not wait for the probe session.
pub fn warm_hevc_probe() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let spawned = std::thread::Builder::new()
            .name("filmcraft-amf-hevc-probe".into())
            .spawn(|| {
                if std::panic::catch_unwind(hevc_available).is_err() {
                    log::warn!("the AMF HEVC probe panicked; the format list will ask again");
                }
                if std::panic::catch_unwind(hevc_hdr_available).is_err() {
                    log::warn!("the AMF HEVC Main 10 probe panicked; export will ask again");
                }
            });
        if let Err(e) = spawned {
            log::info!("AMF HEVC probe thread not started: {e}");
        }
    });
}

/// Whether [`hevc_available`] has its answer yet (diagnostics and tests).
pub fn hevc_probed() -> bool {
    HEVC_PROBE.get().is_some()
}

/// Whether [`hevc_hdr_available`] has its answer yet (diagnostics and tests).
pub fn hevc_hdr_probed() -> bool {
    HEVC_HDR_PROBE.get().is_some()
}
