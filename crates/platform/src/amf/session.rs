//! AMD Advanced Media Framework (AMF) component encoder session lifecycle.
//!
//! FFI module (docs/adr/0001-platform-ffi.md): every `unsafe` block has a `// SAFETY:` comment,
//! public APIs are safe and return `Result<_, String>`, native AMF pointers are managed via RAII.
//! Hostile inputs (zero dimensions, zero frame rates, buffer overflows) are strictly guarded.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::Arc;

use super::device::AmfDevice;
use super::ffi::{
    AMFBuffer, AMFComponent, AMFContext, AMFData, AMFSurface, AMFVariantStruct, AMF_COLOR_BIT_DEPTH_10,
    AMF_COLOR_BIT_DEPTH_8, AMF_DATA_DTS, AMF_EOF, AMF_INPUT_FULL, AMF_MEMORY_HOST, AMF_OK, AMF_REPEAT,
    AMF_SURFACE_FORMAT, AMF_SURFACE_NV12, AMF_SURFACE_P010, AMF_VIDEO_ENCODER_FRAMESIZE,
    AMF_VIDEO_ENCODER_FRAMERATE, AMF_VIDEO_ENCODER_GOP_SIZE, AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH,
    AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, AMF_VIDEO_ENCODER_HEVC_FRAMERATE, AMF_VIDEO_ENCODER_HEVC_GOP_SIZE,
    AMF_VIDEO_ENCODER_HEVC_NUM_GOPS_PER_IDR, AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE,
    AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE, AMF_VIDEO_ENCODER_HEVC_PROFILE,
    AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN, AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN_10,
    AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR,
    AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_PEAK_CONSTRAINED_VBR,
    AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE, AMF_VIDEO_ENCODER_HEVC_USAGE,
    AMF_VIDEO_ENCODER_IDR_PERIOD_SIZE, AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE,
    AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_I, AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_IDR,
    AMF_VIDEO_ENCODER_PEAK_BITRATE, AMF_VIDEO_ENCODER_PROFILE, AMF_VIDEO_ENCODER_PROFILE_BASELINE,
    AMF_VIDEO_ENCODER_PROFILE_HIGH, AMF_VIDEO_ENCODER_PROFILE_MAIN,
    AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD, AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR,
    AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_PEAK_CONSTRAINED_VBR, AMF_VIDEO_ENCODER_TARGET_BITRATE,
    AMF_VIDEO_ENCODER_USAGE, AMF_VIDEO_ENCODER_USAGE_TRANSCODING, AMFVideoEncoderVCE_AVC,
    AMFVideoEncoder_HEVC, IID_AMFBuffer,
};

/// Target video codec for the AMF hardware encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmfCodec {
    H264,
    Hevc,
}

/// Encoding profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmfProfile {
    Baseline,
    Main,
    High,
    Main10,
}

/// Complete configuration for an AMF hardware encoder session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmfConfig {
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub bitrate_kbps: u32,
    pub max_bitrate_kbps: u32,
    pub keyframe_interval: u32,
    pub codec: AmfCodec,
    pub profile: AmfProfile,
    pub cbr: bool,
}

/// Compressed bitstream packet produced by the AMF hardware encoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmfPacket {
    pub data: Vec<u8>,
    pub key: bool,
    pub pts: i64,
    pub dts: i64,
}

/// RAII guard for raw `AMFComponent` pointer to ensure cleanup on error during setup.
struct ComponentGuard(*mut AMFComponent);

impl Drop for ComponentGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: self.0 is a valid AMFComponent pointer being released on initialization failure.
            unsafe {
                let vtbl = &*(*self.0).pVtbl;
                let _ = (vtbl.Terminate)(self.0);
                let _ = (vtbl.Release)(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

/// RAII guard for raw `AMFSurface` pointer.
struct SurfaceGuard(*mut AMFSurface);

impl Drop for SurfaceGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: self.0 is a valid AMFSurface pointer.
            unsafe {
                let vtbl = &*(*self.0).pVtbl;
                let _ = (vtbl.Release)(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

/// RAII guard for raw `AMFData` pointer.
struct DataGuard(*mut AMFData);

impl Drop for DataGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: self.0 is a valid AMFData pointer.
            unsafe {
                let vtbl = &*(*self.0).pVtbl;
                let _ = (vtbl.Release)(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

/// RAII guard for raw `AMFBuffer` pointer returned by `QueryInterface`.
struct BufferGuard(*mut AMFBuffer);

impl Drop for BufferGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: self.0 is a valid AMFBuffer pointer acquired via QueryInterface.
            unsafe {
                let vtbl = &*(*self.0).pVtbl;
                let _ = (vtbl.Release)(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

enum RawOutputResult {
    Packet(AmfPacket),
    Repeat,
    Eof,
}

/// Active AMD AMF hardware encoder session.
pub struct AmfSession {
    component: *mut AMFComponent,
    context: *mut AMFContext,
    config: AmfConfig,
    _device: Arc<AmfDevice>,
    pending_packets: VecDeque<AmfPacket>,
}

// SAFETY: AMF component and context pointers are encapsulated within AmfSession and accessed only
// through &mut self calls, making it safe to transfer ownership across threads.
unsafe impl Send for AmfSession {}

impl std::fmt::Debug for AmfSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AmfSession")
            .field("config", &self.config)
            .field("pending_packets", &self.pending_packets.len())
            .finish()
    }
}

impl AmfSession {
    /// Opens a new AMF hardware encoder session for the specified configuration.
    pub fn open(device: &Arc<AmfDevice>, config: AmfConfig) -> Result<Self, String> {
        // Validate hostile inputs: dimensions
        if config.width == 0 || config.height == 0 {
            return Err("Invalid dimensions: width and height must be non-zero".into());
        }
        if !config.width.is_multiple_of(2) || !config.height.is_multiple_of(2) {
            return Err(format!(
                "Invalid dimensions: width ({}) and height ({}) must be even numbers",
                config.width, config.height
            ));
        }
        if config.width > 8192 || config.height > 8192 {
            return Err(format!(
                "Dimensions {}x{} exceed maximum supported resolution (8192x8192)",
                config.width, config.height
            ));
        }

        // Validate hostile inputs: frame rate (never divide by zero)
        if config.fps_den == 0 {
            return Err("Invalid frame rate: denominator must be non-zero (division by zero guard)".into());
        }
        if config.fps_num == 0 {
            return Err("Invalid frame rate: numerator must be non-zero".into());
        }

        // Validate hostile inputs: bitrate
        if config.bitrate_kbps == 0 {
            return Err("Invalid bitrate: bitrate_kbps must be non-zero".into());
        }

        // Validate codec and profile compatibility
        match (config.codec, config.profile) {
            (AmfCodec::H264, AmfProfile::Main10) => {
                return Err("Main10 profile is not supported for H.264 encoding".into());
            }
            (AmfCodec::Hevc, AmfProfile::Baseline) => {
                return Err("Baseline profile is not supported for HEVC encoding".into());
            }
            _ => {}
        }

        let keyframe_interval = if config.keyframe_interval == 0 {
            (config.fps_num / config.fps_den).max(1)
        } else {
            config.keyframe_interval
        };

        // Select component ID
        let component_id = match config.codec {
            AmfCodec::H264 => AMFVideoEncoderVCE_AVC,
            AmfCodec::Hevc => AMFVideoEncoder_HEVC,
        };

        let factory_ptr = device.factory().as_raw();
        let context_ptr = device.context().as_raw();
        if factory_ptr.is_null() || context_ptr.is_null() {
            return Err("Invalid AMF device: factory or context pointer is null".into());
        }

        let mut component: *mut AMFComponent = std::ptr::null_mut();
        // SAFETY: factory_ptr and context_ptr are valid AMF pointers; component_id is null-terminated UTF-16; component is an out-pointer.
        let res = unsafe {
            let vtbl = &*(*factory_ptr).pVtbl;
            (vtbl.CreateComponent)(factory_ptr, context_ptr, component_id.as_ptr(), &mut component)
        };
        if res != AMF_OK || component.is_null() {
            return Err(format!("AMFFactory::CreateComponent failed with result code {res}"));
        }

        let mut comp_guard = ComponentGuard(component);
        // SAFETY: component was created by CreateComponent above and is non-null.
        let vtbl = unsafe { &*(*component).pVtbl };

        let set_prop = |name: &[u16], val: AMFVariantStruct| -> Result<(), String> {
            // SAFETY: component is a valid AMFComponent pointer; name is null-terminated UTF-16.
            let res = unsafe { (vtbl.SetProperty)(component, name.as_ptr(), val) };
            if res != AMF_OK {
                return Err(format!("SetProperty failed with result code {res}"));
            }
            Ok(())
        };

        let size_var = AMFVariantStruct::from_size(config.width as i32, config.height as i32);
        let rate_var = AMFVariantStruct::from_rate(config.fps_num, config.fps_den);
        let rc_method = match (config.codec, config.cbr) {
            (AmfCodec::H264, true) => AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR,
            (AmfCodec::H264, false) => AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_PEAK_CONSTRAINED_VBR,
            (AmfCodec::Hevc, true) => AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_CBR,
            (AmfCodec::Hevc, false) => AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD_PEAK_CONSTRAINED_VBR,
        };
        let target_bitrate = config.bitrate_kbps as i64 * 1000;
        let peak_kbps = if config.max_bitrate_kbps > 0 {
            config.max_bitrate_kbps.max(config.bitrate_kbps)
        } else {
            config.bitrate_kbps
        };
        let peak_bitrate = peak_kbps as i64 * 1000;

        match config.codec {
            AmfCodec::H264 => {
                set_prop(
                    AMF_VIDEO_ENCODER_USAGE,
                    AMFVariantStruct::from_int64(AMF_VIDEO_ENCODER_USAGE_TRANSCODING),
                )?;
                set_prop(AMF_VIDEO_ENCODER_FRAMESIZE, size_var)?;
                set_prop(AMF_VIDEO_ENCODER_FRAMERATE, rate_var)?;
                set_prop(
                    AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD,
                    AMFVariantStruct::from_int64(rc_method),
                )?;
                set_prop(
                    AMF_VIDEO_ENCODER_TARGET_BITRATE,
                    AMFVariantStruct::from_int64(target_bitrate),
                )?;
                set_prop(
                    AMF_VIDEO_ENCODER_PEAK_BITRATE,
                    AMFVariantStruct::from_int64(peak_bitrate),
                )?;

                let h264_profile = match config.profile {
                    AmfProfile::Baseline => AMF_VIDEO_ENCODER_PROFILE_BASELINE,
                    AmfProfile::Main => AMF_VIDEO_ENCODER_PROFILE_MAIN,
                    AmfProfile::High | AmfProfile::Main10 => AMF_VIDEO_ENCODER_PROFILE_HIGH,
                };
                set_prop(
                    AMF_VIDEO_ENCODER_PROFILE,
                    AMFVariantStruct::from_int64(h264_profile),
                )?;
                set_prop(
                    AMF_VIDEO_ENCODER_IDR_PERIOD_SIZE,
                    AMFVariantStruct::from_int64(keyframe_interval as i64),
                )?;
                let _ = set_prop(
                    AMF_VIDEO_ENCODER_GOP_SIZE,
                    AMFVariantStruct::from_int64(keyframe_interval as i64),
                );
            }
            AmfCodec::Hevc => {
                set_prop(
                    AMF_VIDEO_ENCODER_HEVC_USAGE,
                    AMFVariantStruct::from_int64(AMF_VIDEO_ENCODER_USAGE_TRANSCODING),
                )?;
                set_prop(AMF_VIDEO_ENCODER_HEVC_FRAMESIZE, size_var)?;
                set_prop(AMF_VIDEO_ENCODER_HEVC_FRAMERATE, rate_var)?;
                set_prop(
                    AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD,
                    AMFVariantStruct::from_int64(rc_method),
                )?;
                set_prop(
                    AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE,
                    AMFVariantStruct::from_int64(target_bitrate),
                )?;
                set_prop(
                    AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE,
                    AMFVariantStruct::from_int64(peak_bitrate),
                )?;

                let (hevc_profile, bit_depth) = match config.profile {
                    AmfProfile::Main10 => {
                        (AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN_10, AMF_COLOR_BIT_DEPTH_10)
                    }
                    _ => (AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN, AMF_COLOR_BIT_DEPTH_8),
                };
                set_prop(
                    AMF_VIDEO_ENCODER_HEVC_PROFILE,
                    AMFVariantStruct::from_int64(hevc_profile),
                )?;
                set_prop(
                    AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH,
                    AMFVariantStruct::from_int64(bit_depth),
                )?;
                set_prop(
                    AMF_VIDEO_ENCODER_HEVC_GOP_SIZE,
                    AMFVariantStruct::from_int64(keyframe_interval as i64),
                )?;
                let _ = set_prop(
                    AMF_VIDEO_ENCODER_HEVC_NUM_GOPS_PER_IDR,
                    AMFVariantStruct::from_int64(1),
                );
            }
        }

        let format = if config.codec == AmfCodec::Hevc && config.profile == AmfProfile::Main10 {
            AMF_SURFACE_P010
        } else {
            AMF_SURFACE_NV12
        };

        // SAFETY: component is a valid AMFComponent pointer; format and dimensions were validated.
        let init_res = unsafe {
            (vtbl.Init)(component, format, config.width as i32, config.height as i32)
        };
        if init_res != AMF_OK {
            return Err(format!("AMFComponent::Init failed with result code {init_res}"));
        }

        // SAFETY: context_ptr is valid; Acquire increments reference count so session retains it.
        unsafe {
            let ctx_vtbl = &*(*context_ptr).pVtbl;
            (ctx_vtbl.Acquire)(context_ptr);
        }

        // Disarm guard since ownership is transferred to the AmfSession struct
        comp_guard.0 = std::ptr::null_mut();

        Ok(Self {
            component,
            context: context_ptr,
            config,
            _device: Arc::clone(device),
            pending_packets: VecDeque::new(),
        })
    }

    /// Configuration used to open this encoder session.
    pub fn config(&self) -> &AmfConfig {
        &self.config
    }

    #[allow(clippy::too_many_arguments)]
    fn submit_surface(
        &mut self,
        format: AMF_SURFACE_FORMAT,
        bytes_per_pixel: usize,
        pts: i64,
        dts: i64,
        y_plane: &[u8],
        uv_plane: &[u8],
        y_stride: usize,
        uv_stride: usize,
    ) -> Result<(), String> {
        let width = self.config.width as usize;
        let height = self.config.height as usize;

        let y_row_bytes = width
            .checked_mul(bytes_per_pixel)
            .ok_or_else(|| "Width calculation overflow".to_string())?;

        if y_stride < y_row_bytes {
            return Err(format!(
                "y_stride ({y_stride}) is smaller than row width in bytes ({y_row_bytes})"
            ));
        }
        let y_min_len = (height.saturating_sub(1))
            .checked_mul(y_stride)
            .and_then(|offset| offset.checked_add(y_row_bytes))
            .ok_or_else(|| "Y plane size overflow".to_string())?;
        if y_plane.len() < y_min_len {
            return Err(format!(
                "y_plane buffer too small: expected at least {y_min_len} bytes, got {}",
                y_plane.len()
            ));
        }

        let chroma_height = height.div_ceil(2);
        let uv_row_bytes = y_row_bytes;
        if uv_stride < uv_row_bytes {
            return Err(format!(
                "uv_stride ({uv_stride}) is smaller than row width in bytes ({uv_row_bytes})"
            ));
        }
        let uv_min_len = (chroma_height.saturating_sub(1))
            .checked_mul(uv_stride)
            .and_then(|offset| offset.checked_add(uv_row_bytes))
            .ok_or_else(|| "UV plane size overflow".to_string())?;
        if uv_plane.len() < uv_min_len {
            return Err(format!(
                "uv_plane buffer too small: expected at least {uv_min_len} bytes, got {}",
                uv_plane.len()
            ));
        }

        let mut surface: *mut AMFSurface = std::ptr::null_mut();
        // SAFETY: self.context is valid; dimensions and format are validated; surface is a valid out-pointer.
        let res = unsafe {
            let ctx_vtbl = &*(*self.context).pVtbl;
            (ctx_vtbl.AllocSurface)(
                self.context,
                AMF_MEMORY_HOST,
                format,
                self.config.width as i32,
                self.config.height as i32,
                &mut surface,
            )
        };
        if res != AMF_OK || surface.is_null() {
            return Err(format!("AMFContext::AllocSurface failed with result code {res}"));
        }
        let surface_guard = SurfaceGuard(surface);

        // Copy host frame data into surface planes
        // SAFETY: surface is allocated with AMF_MEMORY_HOST; planes are valid host memory with HPitch strides.
        unsafe {
            let surf_vtbl = &*(*surface).pVtbl;
            let planes_count = (surf_vtbl.GetPlanesCount)(surface);
            if planes_count < 2 {
                return Err(format!("Allocated AMFSurface has fewer than 2 planes: {planes_count}"));
            }

            let p_plane_y = (surf_vtbl.GetPlaneAt)(surface, 0);
            if p_plane_y.is_null() {
                return Err("Failed to get Y plane from AMFSurface".into());
            }
            let plane_y_vtbl = &*(*p_plane_y).pVtbl;
            let y_native = (plane_y_vtbl.GetNative)(p_plane_y) as *mut u8;
            let y_hpitch = (plane_y_vtbl.GetHPitch)(p_plane_y) as usize;
            if y_native.is_null() || y_hpitch < y_row_bytes {
                return Err("Invalid native buffer or pitch on Y plane".into());
            }

            for r in 0..height {
                let src_offset = r
                    .checked_mul(y_stride)
                    .ok_or_else(|| "Y source stride offset overflow".to_string())?;
                let dst_offset = r
                    .checked_mul(y_hpitch)
                    .ok_or_else(|| "Y destination stride offset overflow".to_string())?;
                std::ptr::copy_nonoverlapping(
                    y_plane.as_ptr().add(src_offset),
                    y_native.add(dst_offset),
                    y_row_bytes,
                );
            }

            let p_plane_uv = (surf_vtbl.GetPlaneAt)(surface, 1);
            if p_plane_uv.is_null() {
                return Err("Failed to get UV plane from AMFSurface".into());
            }
            let plane_uv_vtbl = &*(*p_plane_uv).pVtbl;
            let uv_native = (plane_uv_vtbl.GetNative)(p_plane_uv) as *mut u8;
            let uv_hpitch = (plane_uv_vtbl.GetHPitch)(p_plane_uv) as usize;
            if uv_native.is_null() || uv_hpitch < uv_row_bytes {
                return Err("Invalid native buffer or pitch on UV plane".into());
            }

            for r in 0..chroma_height {
                let src_offset = r
                    .checked_mul(uv_stride)
                    .ok_or_else(|| "UV source stride offset overflow".to_string())?;
                let dst_offset = r
                    .checked_mul(uv_hpitch)
                    .ok_or_else(|| "UV destination stride offset overflow".to_string())?;
                std::ptr::copy_nonoverlapping(
                    uv_plane.as_ptr().add(src_offset),
                    uv_native.add(dst_offset),
                    uv_row_bytes,
                );
            }
        }

        // SAFETY: surface is valid; SetPts sets PTS timestamp.
        unsafe {
            let surf_vtbl = &*(*surface).pVtbl;
            (surf_vtbl.SetPts)(surface, pts);
            let _ = (surf_vtbl.SetProperty)(
                surface,
                AMF_DATA_DTS.as_ptr(),
                AMFVariantStruct::from_int64(dts),
            );
        }

        let mut submitted = false;
        // Bounded retry loop up to 100 iterations for AMF_INPUT_FULL
        for _ in 0..100 {
            // SAFETY: self.component and surface are valid AMF pointers; AMFSurface inherits from AMFData.
            let res = unsafe {
                let comp_vtbl = &*(*self.component).pVtbl;
                (comp_vtbl.SubmitInput)(self.component, surface as *mut AMFData)
            };
            if res == AMF_OK {
                submitted = true;
                break;
            } else if res == AMF_INPUT_FULL {
                match self.query_output_raw()? {
                    RawOutputResult::Packet(pkt) => {
                        self.pending_packets.push_back(pkt);
                    }
                    RawOutputResult::Repeat | RawOutputResult::Eof => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
            } else {
                return Err(format!("AMFComponent::SubmitInput failed with result code {res}"));
            }
        }

        if !submitted {
            return Err(
                "AMF component SubmitInput rejected: input queue remained full after 100 attempts"
                    .into(),
            );
        }

        drop(surface_guard);
        Ok(())
    }

    /// Submits an NV12 frame to the encoder.
    pub fn submit_nv12(
        &mut self,
        pts: i64,
        dts: i64,
        y_plane: &[u8],
        uv_plane: &[u8],
        y_stride: usize,
        uv_stride: usize,
    ) -> Result<(), String> {
        if self.config.codec == AmfCodec::Hevc && self.config.profile == AmfProfile::Main10 {
            return Err(
                "Cannot submit NV12 frame to a 10-bit Main10 HEVC session (expected P010)".into(),
            );
        }
        self.submit_surface(
            AMF_SURFACE_NV12,
            1,
            pts,
            dts,
            y_plane,
            uv_plane,
            y_stride,
            uv_stride,
        )
    }

    /// Submits a P010 (10-bit) frame to the encoder.
    pub fn submit_p010(
        &mut self,
        pts: i64,
        dts: i64,
        y_plane: &[u8],
        uv_plane: &[u8],
        y_stride: usize,
        uv_stride: usize,
    ) -> Result<(), String> {
        if !(self.config.codec == AmfCodec::Hevc && self.config.profile == AmfProfile::Main10) {
            return Err("Cannot submit P010 frame to an 8-bit session (expected NV12)".into());
        }
        self.submit_surface(
            AMF_SURFACE_P010,
            2,
            pts,
            dts,
            y_plane,
            uv_plane,
            y_stride,
            uv_stride,
        )
    }

    fn query_output_raw(&mut self) -> Result<RawOutputResult, String> {
        let mut data: *mut AMFData = std::ptr::null_mut();
        // SAFETY: self.component is a valid AMFComponent pointer; data is a valid out-pointer.
        let res = unsafe {
            let vtbl = &*(*self.component).pVtbl;
            (vtbl.QueryOutput)(self.component, &mut data)
        };
        if res == AMF_REPEAT || (res == AMF_OK && data.is_null()) {
            return Ok(RawOutputResult::Repeat);
        }
        if res == AMF_EOF {
            return Ok(RawOutputResult::Eof);
        }
        if res != AMF_OK || data.is_null() {
            return Err(format!("AMFComponent::QueryOutput failed with result code {res}"));
        }

        let _data_guard = DataGuard(data);

        // Query AMFBuffer interface
        let mut buffer_ptr: *mut c_void = std::ptr::null_mut();
        // SAFETY: data is a valid AMFData pointer; buffer_ptr is an out-pointer.
        let qi_res = unsafe {
            let data_vtbl = &*(*data).pVtbl;
            (data_vtbl.QueryInterface)(data, &IID_AMFBuffer, &mut buffer_ptr)
        };
        let (buffer, buffer_guard) = if qi_res == AMF_OK && !buffer_ptr.is_null() {
            let b = buffer_ptr as *mut AMFBuffer;
            (b, Some(BufferGuard(b)))
        } else {
            (data as *mut AMFBuffer, None)
        };

        // SAFETY: buffer is a valid AMFBuffer pointer.
        let (native_ptr, size) = unsafe {
            let buf_vtbl = &*(*buffer).pVtbl;
            let ptr = (buf_vtbl.GetNative)(buffer);
            let sz = (buf_vtbl.GetSize)(buffer);
            (ptr, sz)
        };

        // Cap output packet size against corrupted driver values (max 128 MB for a single compressed video frame)
        const MAX_PACKET_SIZE: usize = 128 * 1024 * 1024;
        if size > MAX_PACKET_SIZE {
            return Err(format!(
                "Encoded packet size ({size} bytes) exceeds maximum limit ({MAX_PACKET_SIZE} bytes)"
            ));
        }

        let packet_data = if !native_ptr.is_null() && size > 0 {
            // SAFETY: native_ptr is non-null and valid for size bytes as reported by GetSize().
            unsafe {
                let slice = std::slice::from_raw_parts(native_ptr as *const u8, size);
                slice.to_vec()
            }
        } else {
            Vec::new()
        };

        // SAFETY: data is a valid AMFData pointer; GetPts reads display timestamp.
        let pts = unsafe {
            let data_vtbl = &*(*data).pVtbl;
            (data_vtbl.GetPts)(data)
        };

        // Read DTS if property is present, otherwise default to PTS
        let mut dts = pts;
        // SAFETY: data is a valid AMFData pointer; AMF_DATA_DTS is null-terminated UTF-16.
        unsafe {
            let data_vtbl = &*(*data).pVtbl;
            let mut var = AMFVariantStruct::empty();
            if (data_vtbl.GetProperty)(data, AMF_DATA_DTS.as_ptr(), &mut var) == AMF_OK {
                dts = var.as_int64().unwrap_or(dts);
            }
        }

        // Check keyframe via property and bitstream inspection
        let mut key = false;
        let prop_name = match self.config.codec {
            AmfCodec::H264 => AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE.as_ptr(),
            AmfCodec::Hevc => AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE.as_ptr(),
        };
        // SAFETY: data is a valid AMFData pointer; prop_name is null-terminated UTF-16.
        unsafe {
            let data_vtbl = &*(*data).pVtbl;
            let mut var = AMFVariantStruct::empty();
            if (data_vtbl.GetProperty)(data, prop_name, &mut var) == AMF_OK
                && matches!(
                    var.as_int64(),
                    Some(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_IDR | AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_I)
                )
            {
                key = true;
            }
        }

        if !key && !packet_data.is_empty() {
            key = detect_keyframe(self.config.codec, &packet_data);
        }

        drop(buffer_guard);
        drop(_data_guard);

        Ok(RawOutputResult::Packet(AmfPacket {
            data: packet_data,
            key,
            pts,
            dts,
        }))
    }

    /// Queries the next encoded packet from the encoder.
    ///
    /// Returns `Ok(Some(packet))` if a packet is ready, or `Ok(None)` if no packet is ready yet.
    pub fn query_output(&mut self) -> Result<Option<AmfPacket>, String> {
        if let Some(pkt) = self.pending_packets.pop_front() {
            return Ok(Some(pkt));
        }
        match self.query_output_raw()? {
            RawOutputResult::Packet(pkt) => Ok(Some(pkt)),
            RawOutputResult::Repeat | RawOutputResult::Eof => Ok(None),
        }
    }

    /// Drains all remaining frames from the encoder pipeline.
    ///
    /// The drain loop is bounded up to 1000 iterations to guarantee termination.
    pub fn drain(&mut self) -> Result<Vec<AmfPacket>, String> {
        // SAFETY: self.component is a valid AMFComponent pointer.
        let res = unsafe {
            let vtbl = &*(*self.component).pVtbl;
            (vtbl.Drain)(self.component)
        };
        if res != AMF_OK {
            return Err(format!("AMFComponent::Drain failed with result code {res}"));
        }

        let mut packets = Vec::new();
        while let Some(pkt) = self.pending_packets.pop_front() {
            packets.push(pkt);
        }

        // Bounded drain loop up to 1000 iterations
        for _ in 0..1000 {
            match self.query_output_raw()? {
                RawOutputResult::Packet(pkt) => packets.push(pkt),
                RawOutputResult::Repeat => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                RawOutputResult::Eof => break,
            }
        }

        Ok(packets)
    }
}

impl Drop for AmfSession {
    fn drop(&mut self) {
        if !self.component.is_null() {
            // SAFETY: self.component is a valid AMFComponent pointer; Terminate stops encoding and Release frees the native component.
            unsafe {
                let vtbl = &*(*self.component).pVtbl;
                let _ = (vtbl.Terminate)(self.component);
                let _ = (vtbl.Release)(self.component);
            }
            self.component = std::ptr::null_mut();
        }
        if !self.context.is_null() {
            // SAFETY: self.context was acquired via Acquire in open() and is released here.
            unsafe {
                let vtbl = &*(*self.context).pVtbl;
                let _ = (vtbl.Release)(self.context);
            }
            self.context = std::ptr::null_mut();
        }
    }
}

/// Detects whether an Annex B bitstream contains an IDR or parameter set keyframe NAL unit.
fn detect_keyframe(codec: AmfCodec, data: &[u8]) -> bool {
    let mut i = 0;
    while i + 4 <= data.len() {
        let is_3byte = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1;
        let is_4byte = data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1;
        if is_3byte || is_4byte {
            let nal_offset = if is_4byte { i + 4 } else { i + 3 };
            if nal_offset < data.len() {
                let nal_header = data[nal_offset];
                match codec {
                    AmfCodec::H264 => {
                        let nal_type = nal_header & 0x1F;
                        // 5 = IDR picture, 7 = SPS (sequence parameter set, preceding IDR)
                        if nal_type == 5 || nal_type == 7 {
                            return true;
                        }
                    }
                    AmfCodec::Hevc => {
                        let nal_type = (nal_header >> 1) & 0x3F;
                        // 19 = IDR_W_RADL, 20 = IDR_N_LP, 21 = CRA_NUT, 32 = VPS, 33 = SPS
                        if nal_type == 19
                            || nal_type == 20
                            || nal_type == 21
                            || nal_type == 32
                            || nal_type == 33
                        {
                            return true;
                        }
                    }
                }
            }
            i = nal_offset;
        } else {
            i += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::device::AmfRuntime;

    #[test]
    fn test_config_validation_zero_dimensions() {
        let device_dummy = std::ptr::null_mut() as *mut c_void;
        let _ = device_dummy;

        let cfg_zero_w = AmfConfig {
            width: 0,
            height: 1080,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 5000,
            max_bitrate_kbps: 8000,
            keyframe_interval: 30,
            codec: AmfCodec::H264,
            profile: AmfProfile::High,
            cbr: false,
        };
        // Creating session with null device / zero width must fail at dimension validation
        let res = match AmfRuntime::load() {
            Ok(rt) => match AmfDevice::new(&rt) {
                Ok(dev) => {
                    let dev = Arc::new(dev);
                    AmfSession::open(&dev, cfg_zero_w)
                }
                Err(_) => return,
            },
            Err(_) => return,
        };
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid dimensions"));
    }

    #[test]
    fn test_config_validation_zero_fps() {
        let cfg_zero_fps = AmfConfig {
            width: 1920,
            height: 1080,
            fps_num: 30,
            fps_den: 0,
            bitrate_kbps: 5000,
            max_bitrate_kbps: 8000,
            keyframe_interval: 30,
            codec: AmfCodec::H264,
            profile: AmfProfile::High,
            cbr: false,
        };
        let res = match AmfRuntime::load() {
            Ok(rt) => match AmfDevice::new(&rt) {
                Ok(dev) => {
                    let dev = Arc::new(dev);
                    AmfSession::open(&dev, cfg_zero_fps)
                }
                Err(_) => return,
            },
            Err(_) => return,
        };
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("division by zero"));
    }

    #[test]
    fn test_config_validation_incompatible_profile() {
        let cfg_h264_main10 = AmfConfig {
            width: 1920,
            height: 1080,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 5000,
            max_bitrate_kbps: 8000,
            keyframe_interval: 30,
            codec: AmfCodec::H264,
            profile: AmfProfile::Main10,
            cbr: false,
        };
        let res = match AmfRuntime::load() {
            Ok(rt) => match AmfDevice::new(&rt) {
                Ok(dev) => {
                    let dev = Arc::new(dev);
                    AmfSession::open(&dev, cfg_h264_main10)
                }
                Err(_) => return,
            },
            Err(_) => return,
        };
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Main10 profile is not supported"));
    }

    #[test]
    fn test_detect_keyframe_h264() {
        // H.264 Annex-B start code + NAL header
        // NAL type 1: non-IDR slice
        let non_key = [0x00, 0x00, 0x00, 0x01, 0x21, 0x00, 0x00];
        assert!(!detect_keyframe(AmfCodec::H264, &non_key));

        // NAL type 5: IDR slice
        let key_idr = [0x00, 0x00, 0x01, 0x25, 0x88, 0x80];
        assert!(detect_keyframe(AmfCodec::H264, &key_idr));

        // NAL type 7: SPS
        let key_sps = [0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00];
        assert!(detect_keyframe(AmfCodec::H264, &key_sps));
    }

    #[test]
    fn test_detect_keyframe_hevc() {
        // HEVC Annex-B start code + NAL header
        // NAL type 1 (TRAIL_R): (1 << 1) = 0x02
        let non_key = [0x00, 0x00, 0x00, 0x01, 0x02, 0x01, 0x00];
        assert!(!detect_keyframe(AmfCodec::Hevc, &non_key));

        // NAL type 19 (IDR_W_RADL): (19 << 1) = 0x26
        let key_idr = [0x00, 0x00, 0x00, 0x01, 0x26, 0x01, 0x00];
        assert!(detect_keyframe(AmfCodec::Hevc, &key_idr));

        // NAL type 32 (VPS): (32 << 1) = 0x40
        let key_vps = [0x00, 0x00, 0x01, 0x40, 0x01, 0x0c];
        assert!(detect_keyframe(AmfCodec::Hevc, &key_vps));
    }

    #[test]
    fn test_amf_hardware_session_if_present() {
        let runtime = match AmfRuntime::load() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("AMF runtime not present, skipping hardware encode test: {e}");
                return;
            }
        };

        let device = match AmfDevice::new(&runtime) {
            Ok(d) => Arc::new(d),
            Err(e) => {
                eprintln!("AMF device failed: {e}");
                return;
            }
        };

        let config = AmfConfig {
            width: 1280,
            height: 720,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 4_000,
            max_bitrate_kbps: 6_000,
            keyframe_interval: 30,
            codec: AmfCodec::H264,
            profile: AmfProfile::Main,
            cbr: true,
        };

        let mut session = match AmfSession::open(&device, config) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open AMF session: {e}");
                panic!("Failed to open AMF session: {e}");
            }
        };

        // Submit 10 synthetic frames
        let width = 1280usize;
        let height = 720usize;
        let y_plane = vec![128u8; width * height];
        let uv_plane = vec![128u8; width * (height / 2)];

        for i in 0..10 {
            let pts = i * 333333;
            let dts = pts;
            session.submit_nv12(pts, dts, &y_plane, &uv_plane, width, width).expect("submit_nv12");
        }

        let output_packets = session.drain().expect("drain");

        println!("Successfully encoded {} H.264 packets with AMF hardware!", output_packets.len());
        assert!(!output_packets.is_empty(), "Expected at least 1 encoded packet from AMF");
        assert!(output_packets[0].key, "First packet should be a keyframe");

        // HEVC Main (8-bit)
        let hevc_config = AmfConfig {
            width: 1280,
            height: 720,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 4_000,
            max_bitrate_kbps: 6_000,
            keyframe_interval: 30,
            codec: AmfCodec::Hevc,
            profile: AmfProfile::Main,
            cbr: true,
        };
        let mut hevc_session = AmfSession::open(&device, hevc_config).expect("open hevc");
        for i in 0..10 {
            let pts = i * 333333;
            hevc_session.submit_nv12(pts, pts, &y_plane, &uv_plane, width, width).expect("hevc submit_nv12");
        }
        let hevc_packets = hevc_session.drain().expect("hevc drain");
        assert!(!hevc_packets.is_empty(), "Expected at least 1 HEVC packet from AMF");
        assert!(hevc_packets[0].key, "First HEVC packet should be a keyframe");
        println!("Successfully encoded {} HEVC Main packets with AMF hardware!", hevc_packets.len());

        // HEVC Main 10 (10-bit P010)
        let hevc10_config = AmfConfig {
            width: 1280,
            height: 720,
            fps_num: 30,
            fps_den: 1,
            bitrate_kbps: 6_000,
            max_bitrate_kbps: 8_000,
            keyframe_interval: 30,
            codec: AmfCodec::Hevc,
            profile: AmfProfile::Main10,
            cbr: true,
        };
        let mut hevc10_session = AmfSession::open(&device, hevc10_config).expect("open hevc main10");
        let p010_y = vec![0u8; width * height * 2];
        let p010_uv = vec![0u8; width * (height / 2) * 2];
        for i in 0..10 {
            let pts = i * 333333;
            hevc10_session.submit_p010(pts, pts, &p010_y, &p010_uv, width * 2, width * 2).expect("hevc10 submit_p010");
        }
        let hevc10_packets = hevc10_session.drain().expect("hevc10 drain");
        assert!(!hevc10_packets.is_empty(), "Expected at least 1 HEVC Main10 packet from AMF");
        assert!(hevc10_packets[0].key, "First HEVC Main10 packet should be a keyframe");
        println!("Successfully encoded {} HEVC Main10 packets with AMF hardware!", hevc10_packets.len());
    }
}
