//! AMD AMF as an Export video encoder: [`amf_encoder_factory`] adapts [`AmfSession`]
//! behind [`filmcraft_export::VideoEncoder`].
//!
//! Safe code: no unsafe blocks. Validates hostile inputs, bounded allocations, and division-by-zero guards.

use std::sync::Arc;

use filmcraft_export::{
    BitrateMode, ColorSignal, EncodedPacket, EncoderFrame, ExportError, ExportSettings, FieldOrder,
    Format, H264Pass, H264Profile, HardwareEncoding, Result, VideoEncoder,
};
use filmcraft_isobmff::{AvcConfig, HevcConfig, SampleEntry};
use filmcraft_time::FrameRate;

use super::device::{AmfDevice, AmfRuntime};
use super::session::{AmfCodec, AmfConfig, AmfPacket, AmfProfile, AmfSession};

/// Extract NAL units from an Annex B byte stream.
pub fn annex_b_nals(data: &[u8]) -> Vec<&[u8]> {
    crate::nvenc::annex_b_nals(data)
}

/// Convert an Annex B byte stream to length-prefixed NAL units (4-byte big-endian length prefix),
/// dropping parameter sets and AUDs.
pub fn annex_b_to_length_prefixed(codec: AmfCodec, data: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let nv_codec = match codec {
        AmfCodec::H264 => crate::nvenc::Codec::H264,
        AmfCodec::Hevc => crate::nvenc::Codec::Hevc,
    };
    crate::nvenc::annex_b_to_length_prefixed_for(nv_codec, data)
}

/// Extracts SPS and PPS parameter sets from an Annex B byte stream.
pub fn split_parameter_sets(data: &[u8]) -> std::result::Result<(Vec<u8>, Vec<u8>), String> {
    crate::nvenc::split_parameter_sets(data)
}

/// Extracts VPS, SPS, and PPS parameter sets from an Annex B byte stream (HEVC).
pub fn split_hevc_parameter_sets(data: &[u8]) -> std::result::Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    crate::nvenc::split_hevc_parameter_sets(data)
}

/// Calculates `(pts - dts) * den` as an MP4 composition offset.
/// Uses saturating arithmetic to prevent overflow with hostile inputs.
pub fn composition_offset(pts: i64, dts: i64, den: i64) -> i32 {
    let offset = pts.saturating_sub(dts).saturating_mul(den);
    i32::try_from(offset).unwrap_or(if offset < 0 { i32::MIN } else { i32::MAX })
}

/// The AMF hardware export video encoder wrapping [`AmfSession`].
pub struct AmfVideoEncoder {
    session: AmfSession,
    w: u32,
    h: u32,
    rate: FrameRate,
    codec: AmfCodec,
    signal: ColorSignal,
    // 8-bit conversion buffers
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    nv12_y: Vec<u8>,
    nv12_uv: Vec<u8>,
    // 10-bit conversion buffers
    y16: Vec<u16>,
    u16: Vec<u16>,
    v16: Vec<u16>,
    p010_y: Vec<u8>,
    p010_uv: Vec<u8>,
    // Cached parameter sets / headers
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
    hvcc: Option<HevcConfig>,
}

impl AmfVideoEncoder {
    /// Creates a new `AmfVideoEncoder`.
    pub fn new(
        session: AmfSession,
        w: u32,
        h: u32,
        rate: FrameRate,
        codec: AmfCodec,
        signal: ColorSignal,
    ) -> Self {
        Self {
            session,
            w,
            h,
            rate,
            codec,
            signal,
            y: Vec::new(),
            u: Vec::new(),
            v: Vec::new(),
            nv12_y: Vec::new(),
            nv12_uv: Vec::new(),
            y16: Vec::new(),
            u16: Vec::new(),
            v16: Vec::new(),
            p010_y: Vec::new(),
            p010_uv: Vec::new(),
            sps: None,
            pps: None,
            hvcc: None,
        }
    }

    /// The video codec handled by this encoder.
    pub fn codec(&self) -> AmfCodec {
        self.codec
    }

    /// Reference to the underlying active AMF session.
    pub fn session(&self) -> &AmfSession {
        &self.session
    }

    /// Mutable reference to the underlying active AMF session.
    pub fn session_mut(&mut self) -> &mut AmfSession {
        &mut self.session
    }

    fn process_packets(&mut self, ps: Vec<AmfPacket>) -> Result<Vec<EncodedPacket>> {
        let den = self.rate.den;
        let duration = u32::try_from(den).unwrap_or(u32::MAX);
        let mut out = Vec::with_capacity(ps.len());
        for p in ps {
            match self.codec {
                AmfCodec::H264 => {
                    if self.sps.is_none() || self.pps.is_none() {
                        let _ = split_parameter_sets(&p.data).map(|(sps, pps)| {
                            self.sps = Some(sps);
                            self.pps = Some(pps);
                        });
                    }
                }
                AmfCodec::Hevc => {
                    if self.hvcc.is_none() {
                        let _ = split_hevc_parameter_sets(&p.data).map(|(vps, sps, pps)| {
                            let bit_depth = if self.session.config().profile == AmfProfile::Main10 {
                                10
                            } else {
                                8
                            };
                            if let Ok(hvcc) = crate::nvenc::hevc::hevc_config(
                                &vps,
                                &sps,
                                &pps,
                                (self.w, self.h),
                                bit_depth,
                            ) {
                                self.hvcc = Some(hvcc);
                            }
                        });
                    }
                }
            }

            let data = annex_b_to_length_prefixed(self.codec, &p.data)
                .map_err(|e| ExportError::Encode(format!("AMF: bitstream conversion error: {e}")))?;
            let composition_offset = composition_offset(p.pts, p.dts, den);
            out.push(EncodedPacket {
                data,
                key: p.key,
                duration,
                composition_offset,
            });
        }
        Ok(out)
    }
}

impl VideoEncoder for AmfVideoEncoder {
    fn timescale(&self) -> u32 {
        u32::try_from(self.rate.num).unwrap_or(u32::MAX)
    }

    fn sample_entry(&self) -> SampleEntry {
        let (w, h) = (
            u16::try_from(self.w).unwrap_or(u16::MAX),
            u16::try_from(self.h).unwrap_or(u16::MAX),
        );
        match self.codec {
            AmfCodec::H264 => {
                let sps_list = self.sps.as_ref().map(|s| vec![s.clone()]).unwrap_or_default();
                let pps_list = self.pps.as_ref().map(|p| vec![p.clone()]).unwrap_or_default();
                SampleEntry::avc(AvcConfig::new(sps_list, pps_list, 4), w, h)
            }
            AmfCodec::Hevc => {
                let mut entry = SampleEntry::hevc(self.hvcc.clone().unwrap_or_default(), w, h);
                self.signal.apply_to(&mut entry, false);
                entry
            }
        }
    }

    #[allow(clippy::chunks_exact_to_as_chunks)]
    fn encode(&mut self, f: &EncoderFrame<'_>) -> Result<Vec<EncodedPacket>> {
        if f.width != self.w || f.height != self.h {
            return Err(ExportError::Encode(format!(
                "AMF: a {}x{} picture for a {}x{} encoder",
                f.width, f.height, self.w, self.h
            )));
        }

        let (w, h) = (self.w as usize, self.h as usize);
        if w == 0 || h == 0 || !w.is_multiple_of(2) || !h.is_multiple_of(2) {
            return Err(ExportError::Encode(
                "AMF: encoder requires non-zero even dimensions".into(),
            ));
        }

        let pts = i64::try_from(f.index).unwrap_or(i64::MAX);
        let dts = pts;

        let is_main10 = self.session.config().profile == AmfProfile::Main10;
        match (is_main10, f.hdr) {
            (true, Some(rgb)) => {
                let required_hdr_len = w.checked_mul(h).and_then(|n| n.checked_mul(3)).ok_or_else(|| {
                    ExportError::Encode("frame dimensions overflow".into())
                })?;
                if rgb.len() < required_hdr_len {
                    return Err(ExportError::Encode(format!(
                        "HDR float buffer too small: expected {} floats, got {}",
                        required_hdr_len,
                        rgb.len()
                    )));
                }

                let (kr, kb) = self.signal.kr_kb();
                filmcraft_export::timed(filmcraft_export::Stage::Convert, || {
                    filmcraft_export::rgbf_to_yuv420_10(
                        rgb,
                        w,
                        h,
                        filmcraft_export::Matrix::new(kr, kb),
                        &mut self.y16,
                        &mut self.u16,
                        &mut self.v16,
                    )
                })?;

                let luma_samples = w.saturating_mul(h);
                let chroma_samples = (w / 2).saturating_mul(h / 2);
                if self.y16.len() < luma_samples
                    || self.u16.len() < chroma_samples
                    || self.v16.len() < chroma_samples
                {
                    return Err(ExportError::Encode(
                        "10-bit YUV conversion produced insufficient data".into(),
                    ));
                }

                let y_stride = w.saturating_mul(2);
                let y_size = y_stride.saturating_mul(h);
                self.p010_y.resize(y_size, 0);
                for (src_row, dst_row) in self.y16.chunks_exact(w).zip(self.p010_y.chunks_exact_mut(y_stride)) {
                    for (dst, &code) in dst_row.chunks_exact_mut(2).zip(src_row) {
                        let bytes = ((code & 0x3ff) << 6).to_le_bytes();
                        dst[0] = bytes[0];
                        dst[1] = bytes[1];
                    }
                }

                let cw = w / 2;
                let ch = h / 2;
                let uv_stride = w.saturating_mul(2);
                let uv_size = uv_stride.saturating_mul(ch);
                self.p010_uv.resize(uv_size, 0);
                for ((u_chunk, v_chunk), dst_chunk) in self.u16.chunks_exact(cw).zip(self.v16.chunks_exact(cw)).zip(self.p010_uv.chunks_exact_mut(uv_stride)) {
                    for ((&u_val, &v_val), dst_quad) in u_chunk.iter().zip(v_chunk.iter()).zip(dst_chunk.chunks_exact_mut(4)) {
                        let u_bytes = ((u_val & 0x3ff) << 6).to_le_bytes();
                        let v_bytes = ((v_val & 0x3ff) << 6).to_le_bytes();
                        dst_quad[0] = u_bytes[0];
                        dst_quad[1] = u_bytes[1];
                        dst_quad[2] = v_bytes[0];
                        dst_quad[3] = v_bytes[1];
                    }
                }

                self.session
                    .submit_p010(pts, dts, &self.p010_y, &self.p010_uv, y_stride, uv_stride)
                    .map_err(|e| ExportError::Encode(format!("AMF: {e}")))?;
            }
            (true, None) => {
                return Err(ExportError::Unsupported(
                    "AMF Main 10 needs the HDR picture (EncoderFrame::hdr)".into(),
                ));
            }
            (false, Some(_)) => {
                return Err(ExportError::Unsupported(
                    "AMF Main (8-bit) does not take HDR pictures".into(),
                ));
            }
            (false, None) => {
                let required_rgba_len = w.checked_mul(h).and_then(|n| n.checked_mul(4)).ok_or_else(|| {
                    ExportError::Encode("frame dimensions overflow".into())
                })?;
                if f.rgba.len() < required_rgba_len {
                    return Err(ExportError::Encode(format!(
                        "RGBA buffer too small: expected {} bytes, got {}",
                        required_rgba_len,
                        f.rgba.len()
                    )));
                }

                filmcraft_export::timed(filmcraft_export::Stage::Convert, || {
                    filmcraft_export::rgba_to_yuv420_8(f.rgba, w, h, &mut self.y, &mut self.u, &mut self.v);
                });

                let luma_samples = w.saturating_mul(h);
                let chroma_samples = (w / 2).saturating_mul(h / 2);
                if self.y.len() < luma_samples
                    || self.u.len() < chroma_samples
                    || self.v.len() < chroma_samples
                {
                    return Err(ExportError::Encode(
                        "YUV conversion produced insufficient data".into(),
                    ));
                }

                let y_stride = w;
                self.nv12_y.resize(w.saturating_mul(h), 0);
                self.nv12_y.copy_from_slice(&self.y[..luma_samples]);

                let cw = w / 2;
                let ch = h / 2;
                let uv_stride = w;
                let uv_size = uv_stride.saturating_mul(ch);
                self.nv12_uv.resize(uv_size, 0);
                for ((u_chunk, v_chunk), dst_chunk) in self.u.chunks_exact(cw).zip(self.v.chunks_exact(cw)).zip(self.nv12_uv.chunks_exact_mut(uv_stride)) {
                    for ((&u_val, &v_val), dst_pair) in u_chunk.iter().zip(v_chunk.iter()).zip(dst_chunk.chunks_exact_mut(2)) {
                        dst_pair[0] = u_val;
                        dst_pair[1] = v_val;
                    }
                }

                self.session
                    .submit_nv12(pts, dts, &self.nv12_y, &self.nv12_uv, y_stride, uv_stride)
                    .map_err(|e| ExportError::Encode(format!("AMF: {e}")))?;
            }
        }

        filmcraft_export::note_hw_encode_frame();

        let mut raw_packets = Vec::new();
        while let Some(pkt) = self
            .session
            .query_output()
            .map_err(|e| ExportError::Encode(format!("AMF: {e}")))?
        {
            raw_packets.push(pkt);
        }

        self.process_packets(raw_packets)
    }

    fn flush(&mut self) -> Result<Vec<EncodedPacket>> {
        let drained = self
            .session
            .drain()
            .map_err(|e| ExportError::Encode(format!("AMF: {e}")))?;
        self.process_packets(drained)
    }

    fn media_start(&self) -> Option<i64> {
        None
    }
}

/// Factory function to create an AMF hardware encoder for export.
pub fn amf_encoder_factory(
    format: Format,
    w: u32,
    h: u32,
    rate: FrameRate,
    s: &ExportSettings,
) -> Option<Result<Box<dyn VideoEncoder>>> {
    if !(format == Format::H264 || format == Format::Hevc) || s.format.is_mxf() {
        return None;
    }

    if format == Format::H264 && s.hardware_encoding != HardwareEncoding::Auto {
        return None;
    }

    let declined = |why: &str| {
        log::info!("AMF hardware encoding declined: {why}");
        filmcraft_export::note_hw_encode_declined();
        None
    };

    if s.field_order != FieldOrder::Progressive {
        return declined("interlaced output");
    }

    if !matches!(s.h264_pass, H264Pass::Single) || s.bitrate_mode == BitrateMode::Vbr2Pass {
        return declined("two-pass encoding");
    }

    if s.bitrate_mode == BitrateMode::Crf {
        return declined("CRF (constant quality)");
    }

    if s.pixel_aspect.is_some_and(|(n, d)| n != d) {
        return declined("non-square pixels");
    }

    if w == 0 || h == 0 || !w.is_multiple_of(2) || !h.is_multiple_of(2) {
        return declined("non-zero even dimensions required");
    }

    if w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return declined("dimensions exceed u16::MAX");
    }

    if rate.num <= 0 || rate.den <= 0 || rate.num > i64::from(u32::MAX) || rate.den > i64::from(u32::MAX) {
        return declined("invalid frame rate");
    }

    let (Ok(fps_num), Ok(fps_den)) = (u32::try_from(rate.num), u32::try_from(rate.den)) else {
        return declined("invalid frame rate");
    };
    if fps_num == 0 || fps_den == 0 {
        return declined("frame rate components must be non-zero");
    }

    let hdr = s.signal.is_hdr();
    if hdr && format != Format::Hevc {
        return declined("HDR is only supported for HEVC Main 10");
    }

    let kbps = s.bitrate_kbps.max(100);
    let max_bitrate_kbps = s
        .max_bitrate_kbps
        .filter(|m| *m >= kbps)
        .unwrap_or_else(|| kbps.saturating_add(kbps / 2));
    let cbr = s.bitrate_mode == BitrateMode::Cbr;
    let keyframe_interval = s.keyframe_distance.filter(|k| *k > 0).unwrap_or_else(|| {
        (f64::from(fps_num) / f64::from(fps_den) * 2.0).round().max(1.0) as u32
    });

    let codec = match format {
        Format::H264 => AmfCodec::H264,
        Format::Hevc => AmfCodec::Hevc,
        _ => return None,
    };

    let profile = match (format == Format::Hevc, s.h264_profile) {
        (true, _) if hdr => AmfProfile::Main10,
        (true, _) => AmfProfile::Main,
        (false, H264Profile::Baseline) => AmfProfile::Baseline,
        (false, H264Profile::Main) => AmfProfile::Main,
        (false, H264Profile::High) => AmfProfile::High,
    };

    let config = AmfConfig {
        width: w,
        height: h,
        fps_num,
        fps_den,
        bitrate_kbps: kbps,
        max_bitrate_kbps,
        keyframe_interval,
        codec,
        profile,
        cbr,
    };

    let runtime = match AmfRuntime::load() {
        Ok(rt) => rt,
        Err(why) => {
            log::info!("AMF runtime not present: {why}");
            return None;
        }
    };

    let device = match AmfDevice::new(&runtime) {
        Ok(dev) => Arc::new(dev),
        Err(why) => {
            return declined(&format!("failed to initialize Direct3D 11 / AMF device: {why}"));
        }
    };

    let session = match AmfSession::open(&device, config) {
        Ok(sess) => sess,
        Err(why) => {
            return declined(&format!("failed to open AMF encoder session: {why}"));
        }
    };

    filmcraft_export::note_hw_encode_session();
    Some(Ok(Box::new(AmfVideoEncoder::new(
        session,
        w,
        h,
        rate,
        codec,
        s.signal,
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annexb_to_avcc_conversion() {
        let sps = vec![0x67, 0x42, 0x00, 0x0a];
        let pps = vec![0x68, 0xce, 0x38, 0x80];
        let slice = vec![0x65, 0x88, 0x84, 0x00, 0x10];

        let mut annex_b = Vec::new();
        // 4-byte start code + SPS
        annex_b.extend_from_slice(&[0, 0, 0, 1]);
        annex_b.extend_from_slice(&sps);
        // 3-byte start code + PPS
        annex_b.extend_from_slice(&[0, 0, 1]);
        annex_b.extend_from_slice(&pps);
        // 4-byte start code + IDR slice
        annex_b.extend_from_slice(&[0, 0, 0, 1]);
        annex_b.extend_from_slice(&slice);

        let (extracted_sps, extracted_pps) =
            split_parameter_sets(&annex_b).expect("split_parameter_sets failed");
        assert_eq!(extracted_sps, sps);
        assert_eq!(extracted_pps, pps);

        let length_prefixed = annex_b_to_length_prefixed(AmfCodec::H264, &annex_b)
            .expect("annex_b_to_length_prefixed failed");

        // Length prefix is 4 bytes big endian for slice (5 bytes)
        let mut expected = vec![0, 0, 0, 5];
        expected.extend_from_slice(&slice);
        assert_eq!(length_prefixed, expected);

        let avcc = AvcConfig::new(vec![extracted_sps], vec![extracted_pps], 4);
        let sample_entry = SampleEntry::avc(avcc, 1920, 1080);
        let video = sample_entry.video.as_ref().expect("video sample entry");
        assert_eq!(video.width, 1920);
        assert_eq!(video.height, 1080);
    }

    #[test]
    fn test_annexb_to_hvcc_conversion() {
        fn unhex(h: &str) -> Vec<u8> {
            (0..h.len())
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(h.get(i..i + 2)?, 16).ok())
                .collect()
        }

        const VPS_HEX: &str = "40010c01ffff01600000030090000003000003005d9940c0000003004000000614";
        const SPS_HEX: &str = "42010101600000030090000003000003005da00280802e1f1396654a421191bff0c05a8080808a0000030002000003003010";
        const PPS_HEX: &str = "4401c1937c0cc9";

        let vps = unhex(VPS_HEX);
        let sps = unhex(SPS_HEX);
        let pps = unhex(PPS_HEX);
        let slice = vec![0x26, 0x01, 0xaf, 0x05]; // HEVC CRA/IDR slice

        let mut annex_b = Vec::new();
        annex_b.extend_from_slice(&[0, 0, 0, 1]);
        annex_b.extend_from_slice(&vps);
        annex_b.extend_from_slice(&[0, 0, 0, 1]);
        annex_b.extend_from_slice(&sps);
        annex_b.extend_from_slice(&[0, 0, 0, 1]);
        annex_b.extend_from_slice(&pps);
        annex_b.extend_from_slice(&[0, 0, 0, 1]);
        annex_b.extend_from_slice(&slice);

        let (extracted_vps, extracted_sps, extracted_pps) =
            split_hevc_parameter_sets(&annex_b).expect("split_hevc_parameter_sets failed");
        assert_eq!(extracted_vps, vps);
        assert_eq!(extracted_sps, sps);
        assert_eq!(extracted_pps, pps);

        let hvcc = crate::nvenc::hevc::hevc_config(
            &extracted_vps,
            &extracted_sps,
            &extracted_pps,
            (1280, 720),
            8,
        )
        .expect("hevc_config failed");
        assert_eq!(hvcc.length_size, 4);

        let length_prefixed = annex_b_to_length_prefixed(AmfCodec::Hevc, &annex_b)
            .expect("annex_b_to_length_prefixed failed");

        let mut expected = vec![0, 0, 0, 4];
        expected.extend_from_slice(&slice);
        assert_eq!(length_prefixed, expected);

        let sample_entry = SampleEntry::hevc(hvcc, 1280, 720);
        let video = sample_entry.video.as_ref().expect("video sample entry");
        assert_eq!(video.width, 1280);
        assert_eq!(video.height, 720);
    }

    #[test]
    fn test_composition_offset_calculation() {
        assert_eq!(composition_offset(10, 10, 1000), 0);
        assert_eq!(composition_offset(15, 10, 1000), 5000);
        assert_eq!(composition_offset(8, 10, 1000), -2000);
        assert_eq!(composition_offset(0, 0, 0), 0);

        // Saturation bounds
        assert_eq!(composition_offset(i64::MAX, 0, 1), i32::MAX);
        assert_eq!(composition_offset(0, i64::MAX, 1), i32::MIN);
        assert_eq!(composition_offset(i64::MAX, i64::MIN, 1), i32::MAX);
        assert_eq!(composition_offset(i64::MIN, i64::MAX, 1), i32::MIN);
        assert_eq!(composition_offset(i64::MAX / 2, -i64::MAX / 2, 1000), i32::MAX);
        assert_eq!(composition_offset(-i64::MAX / 2, i64::MAX / 2, 1000), i32::MIN);
    }

    #[test]
    fn test_amf_factory_declines_unsupported() {
        let base_settings = ExportSettings {
            hardware_encoding: HardwareEncoding::Auto,
            field_order: FieldOrder::Progressive,
            h264_pass: H264Pass::Single,
            bitrate_mode: BitrateMode::Cbr,
            ..Default::default()
        };

        // Unsupported formats
        assert!(amf_encoder_factory(Format::ProRes, 1920, 1080, FrameRate::FPS_24, &base_settings).is_none());
        assert!(amf_encoder_factory(Format::Mjpeg, 1920, 1080, FrameRate::FPS_24, &base_settings).is_none());

        // MXF format
        let mut mxf_settings = base_settings.clone();
        mxf_settings.format = Format::MxfOp1a;
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate::FPS_24, &mxf_settings).is_none());

        // Hardware encoding off for H.264
        let mut hw_off = base_settings.clone();
        hw_off.hardware_encoding = HardwareEncoding::Off;
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate::FPS_24, &hw_off).is_none());

        // Interlaced
        let mut interlaced = base_settings.clone();
        interlaced.field_order = FieldOrder::UpperFirst;
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate::FPS_24, &interlaced).is_none());
        assert!(amf_encoder_factory(Format::Hevc, 1920, 1080, FrameRate::FPS_24, &interlaced).is_none());

        // Two-pass
        let mut two_pass_first = base_settings.clone();
        two_pass_first.h264_pass = H264Pass::First;
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate::FPS_24, &two_pass_first).is_none());

        let mut two_pass_vbr = base_settings.clone();
        two_pass_vbr.bitrate_mode = BitrateMode::Vbr2Pass;
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate::FPS_24, &two_pass_vbr).is_none());
        assert!(amf_encoder_factory(Format::Hevc, 1920, 1080, FrameRate::FPS_24, &two_pass_vbr).is_none());

        // CRF
        let mut crf = base_settings.clone();
        crf.bitrate_mode = BitrateMode::Crf;
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate::FPS_24, &crf).is_none());

        // Non-square pixels
        let mut non_square = base_settings.clone();
        non_square.pixel_aspect = Some((4, 3));
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate::FPS_24, &non_square).is_none());

        // Odd and zero dimensions
        assert!(amf_encoder_factory(Format::H264, 0, 1080, FrameRate::FPS_24, &base_settings).is_none());
        assert!(amf_encoder_factory(Format::H264, 1920, 0, FrameRate::FPS_24, &base_settings).is_none());
        assert!(amf_encoder_factory(Format::H264, 1921, 1080, FrameRate::FPS_24, &base_settings).is_none());
        assert!(amf_encoder_factory(Format::H264, 1920, 1081, FrameRate::FPS_24, &base_settings).is_none());
        assert!(amf_encoder_factory(Format::H264, 70000, 1080, FrameRate::FPS_24, &base_settings).is_none());

        // Invalid frame rates
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate { num: 0, den: 1 }, &base_settings).is_none());
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate { num: 24, den: 0 }, &base_settings).is_none());
        assert!(amf_encoder_factory(Format::H264, 1920, 1080, FrameRate { num: -24, den: 1 }, &base_settings).is_none());
    }
}
