//! Integration tests for AMD AMF hardware encoder registration and pipeline integration.

#![cfg(target_os = "windows")]

use filmcraft_export::{
    BitrateMode, ColorSignal, ExportSettings, FieldOrder, Format, H264Pass, HardwareEncoding,
};
use filmcraft_platform::amf::{
    amf_encoder_factory, hevc_available, hevc_hdr_available, hevc_hdr_probed, hevc_probed,
    warm_hevc_probe,
};
use filmcraft_platform::register;
use filmcraft_time::FrameRate;

#[test]
fn test_amf_factory_declines_unsupported_settings() {
    let rate = FrameRate::FPS_24;
    let base = ExportSettings {
        format: Format::H264,
        hardware_encoding: HardwareEncoding::Auto,
        bitrate_mode: BitrateMode::Vbr1Pass,
        bitrate_kbps: 5000,
        field_order: FieldOrder::Progressive,
        h264_pass: H264Pass::Single,
        signal: ColorSignal::default(),
        pixel_aspect: None,
        include_audio: false,
        ..Default::default()
    };

    // 1. Declines unsupported formats
    for unsupported in [
        Format::ProRes,
        Format::DnxHr,
        Format::Apv,
        Format::Mjpeg,
    ] {
        assert!(
            amf_encoder_factory(unsupported, 1280, 720, rate, &base).is_none(),
            "AMF factory should decline unsupported format {unsupported:?}"
        );
    }

    // 2. Declines MXF container
    let mxf_settings = ExportSettings {
        format: Format::MxfOp1a,
        ..base.clone()
    };
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, rate, &mxf_settings).is_none(),
        "AMF factory should decline MXF container for H.264"
    );
    assert!(
        amf_encoder_factory(Format::Hevc, 1280, 720, rate, &mxf_settings).is_none(),
        "AMF factory should decline MXF container for HEVC"
    );

    // 3. Declines H.264 when HardwareEncoding is Off
    let hw_off = ExportSettings {
        hardware_encoding: HardwareEncoding::Off,
        ..base.clone()
    };
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, rate, &hw_off).is_none(),
        "AMF factory should decline H.264 when HardwareEncoding is Off"
    );

    // 4. Declines interlaced encoding
    for field_order in [FieldOrder::UpperFirst, FieldOrder::LowerFirst] {
        let interlaced = ExportSettings {
            field_order,
            ..base.clone()
        };
        assert!(
            amf_encoder_factory(Format::H264, 1280, 720, rate, &interlaced).is_none(),
            "AMF factory should decline interlaced field order {field_order:?} for H.264"
        );
        assert!(
            amf_encoder_factory(Format::Hevc, 1280, 720, rate, &interlaced).is_none(),
            "AMF factory should decline interlaced field order {field_order:?} for HEVC"
        );
    }

    // 5. Declines two-pass encoding
    let two_pass_h264 = ExportSettings {
        h264_pass: H264Pass::First,
        ..base.clone()
    };
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, rate, &two_pass_h264).is_none(),
        "AMF factory should decline H264Pass::First"
    );
    let two_pass_vbr = ExportSettings {
        bitrate_mode: BitrateMode::Vbr2Pass,
        ..base.clone()
    };
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, rate, &two_pass_vbr).is_none(),
        "AMF factory should decline BitrateMode::Vbr2Pass for H.264"
    );
    assert!(
        amf_encoder_factory(Format::Hevc, 1280, 720, rate, &two_pass_vbr).is_none(),
        "AMF factory should decline BitrateMode::Vbr2Pass for HEVC"
    );

    // 6. Declines CRF rate control
    let crf = ExportSettings {
        bitrate_mode: BitrateMode::Crf,
        ..base.clone()
    };
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, rate, &crf).is_none(),
        "AMF factory should decline BitrateMode::Crf for H.264"
    );
    assert!(
        amf_encoder_factory(Format::Hevc, 1280, 720, rate, &crf).is_none(),
        "AMF factory should decline BitrateMode::Crf for HEVC"
    );

    // 7. Declines non-square pixels
    let non_square = ExportSettings {
        pixel_aspect: Some((4, 3)),
        ..base.clone()
    };
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, rate, &non_square).is_none(),
        "AMF factory should decline non-square pixel aspect ratio for H.264"
    );
    assert!(
        amf_encoder_factory(Format::Hevc, 1280, 720, rate, &non_square).is_none(),
        "AMF factory should decline non-square pixel aspect ratio for HEVC"
    );

    // 8. Declines odd or zero dimensions
    for (w, h) in [(0, 720), (1280, 0), (1281, 720), (1280, 721), (0, 0)] {
        assert!(
            amf_encoder_factory(Format::H264, w, h, rate, &base).is_none(),
            "AMF factory should decline invalid dimensions ({w}, {h})"
        );
        assert!(
            amf_encoder_factory(Format::Hevc, w, h, rate, &base).is_none(),
            "AMF factory should decline invalid dimensions ({w}, {h})"
        );
    }

    // 9. Declines invalid frame rates
    let zero_num = FrameRate::new(0, 1);
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, zero_num, &base).is_none(),
        "AMF factory should decline zero fps numerator"
    );
    let zero_den = FrameRate::new(24, 0);
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, zero_den, &base).is_none(),
        "AMF factory should decline zero fps denominator"
    );

    // 10. Declines HDR for H.264
    let hdr_h264 = ExportSettings {
        signal: ColorSignal::PQ,
        ..base.clone()
    };
    assert!(
        amf_encoder_factory(Format::H264, 1280, 720, rate, &hdr_h264).is_none(),
        "AMF factory should decline HDR for H.264"
    );
}

#[test]
fn test_amf_pipeline_registration() {
    let avail1 = register();
    let avail2 = register();
    assert_eq!(avail1, avail2, "register() should be idempotent");

    assert!(
        filmcraft_export::encoder_registered(amf_encoder_factory),
        "amf_encoder_factory should be registered in export registry"
    );

    // Format::H264 is always available via built-in software encoder
    assert!(
        filmcraft_export::available(Format::H264),
        "Format::H264 should be available"
    );

    // Format::Hevc availability reflects whether AMF (or NVENC) is available
    let amf_hevc = hevc_available();
    if amf_hevc {
        assert!(
            filmcraft_export::available(Format::Hevc),
            "Format::Hevc should be available when AMF HEVC is available"
        );
    }

    // Format::Hevc HDR availability reflects whether AMF (or NVENC) HDR is available
    let amf_hdr = hevc_hdr_available();
    if amf_hdr {
        assert!(
            filmcraft_export::hdr_available(Format::Hevc),
            "Format::Hevc HDR should be available when AMF HEVC Main 10 is available"
        );
    }
}

#[test]
fn test_amf_hevc_probe() {
    let hevc = hevc_available();
    let hdr = hevc_hdr_available();

    // Main 10 HDR implies basic HEVC capability
    assert!(!hdr || hevc, "HEVC Main 10 HDR availability implies HEVC availability");

    // Probes are cached: subsequent calls return identical values
    assert_eq!(hevc_available(), hevc, "hevc_available() should be cached");
    assert_eq!(hevc_hdr_available(), hdr, "hevc_hdr_available() should be cached");

    // warm_hevc_probe executes safely and populates the cache
    warm_hevc_probe();
    assert!(hevc_probed(), "hevc_probed() should be true after warm probe");
    assert!(hevc_hdr_probed(), "hevc_hdr_probed() should be true after warm probe");
}
