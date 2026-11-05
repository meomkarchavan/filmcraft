# AMD AMF Hardware Video Encoder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement native AMD AMF hardware video encoding (H.264 & HEVC) for Windows in FilmCraft, unlocking GPU acceleration for AMD Radeon GPUs (such as RX 6700 XT).

**Architecture:** Dynamically load `amfrt64.dll` in `crates/platform/src/amf`, initialize an `AMFContext` with Direct3D 11, configure `AMFVideoEncoderVCE_AVC` and `AMFVideoEncoder_HEVC` components, feed NV12/P010 surfaces, extract Annex-B NAL units, and package into length-prefixed MP4 samples (`avcC` / `hvcC`).

**Tech Stack:** Rust, Windows D3D11 / DXGI (`windows` crate), AMD Advanced Media Framework (AMF C-ABI), `filmcraft_export`, `filmcraft_isobmff`.

---

## File Structure & Responsibilities

- `crates/platform/src/amf/ffi.rs`: C-ABI declarations for AMD AMF runtime interfaces (`AMFFactory`, `AMFContext`, `AMFComponent`, `AMFSurface`, `AMFBuffer`, `AMFVariant`, property keys, GUIDs, and return codes).
- `crates/platform/src/amf/abi_tests.rs`: ABI layout verification tests checking sizes, field offsets, and alignments against the official AMF specification.
- `crates/platform/src/amf/device.rs`: Runtime loader for `amfrt64.dll`, initialization via `AMFInit`, and binding `AMFContext` to the process Direct3D 11 device.
- `crates/platform/src/amf/session.rs`: High-level safe wrapper driving the `AMFComponent` pipeline, managing parameter bags (bitrate, GOP size, profile), submitting NV12/P010 surfaces, and polling/draining output packets.
- `crates/platform/src/amf/export.rs`: Implements `filmcraft_export::VideoEncoder`. Converts RGBA/HDR frames to planar/semi-planar buffers, drives the AMF session, and extracts parameter sets (`avcC` / `hvcC`) and length-prefixed MP4 samples.
- `crates/platform/src/amf/mod.rs`: Safe public API, capability probes (`hevc_available`), background probe warming, and factory constructor.
- `crates/platform/src/lib.rs`: Registers the AMF encoder and HEVC probes on Windows.
- `docs/gaps.md`: Documents closing the AMD AMF hardware encoding gap.

---

### Task 1: AMD AMF C-ABI Type Definitions & Vtables

**Files:**
- Create: `crates/platform/src/amf/ffi.rs`
- Modify: `crates/platform/src/lib.rs` (conditional module declaration)

- [ ] **Step 1: Write the failing test for AMF basic constants and types**

Create temporary test in `crates/platform/src/amf/abi_tests.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::super::ffi::*;

    #[test]
    fn amf_version_constants_are_valid() {
        assert_eq!(AMF_FULL_VERSION, ((1u64) << 48) | ((4u64) << 32) | ((33u64) << 16) | 0);
        assert_eq!(AMF_OK, 0);
        assert_eq!(AMF_FAIL, 1);
        assert_eq!(AMF_EOF, 11);
        assert_eq!(AMF_INPUT_FULL, 36);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p filmcraft-platform -- amf_version_constants_are_valid`
Expected: FAIL with unresolved import `ffi`.

- [ ] **Step 3: Write minimal AMF FFI definitions**

In `crates/platform/src/amf/ffi.rs`:
Define `#[repr(C)]` structs and vtables for:
- `AMF_RESULT` return codes (`AMF_OK`, `AMF_FAIL`, `AMF_EOF`, `AMF_INPUT_FULL`, etc.)
- `AMF_MEMORY_TYPE` (`HOST`, `DX11`) and `AMF_SURFACE_FORMAT` (`NV12`, `P010`, `RGBA`, `BGRA`)
- `AMFGuid`
- `AMFVariantStruct` / `AMFVariant`
- `AMFPropertyStorageVtbl` & `AMFPropertyStorage`
- `AMFDataVtbl` & `AMFData`
- `AMFBufferVtbl` & `AMFBuffer`
- `AMFSurfaceVtbl` & `AMFSurface`
- `AMFContextVtbl` & `AMFContext`
- `AMFComponentVtbl` & `AMFComponent`
- `AMFFactoryVtbl` & `AMFFactory`
- Encoder property names (`AMF_VIDEO_ENCODER_USAGE`, `AMF_VIDEO_ENCODER_TARGET_BITRATE`, `AMF_VIDEO_ENCODER_PEAK_BITRATE`, `AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD`, `AMF_VIDEO_ENCODER_PROFILE`, etc.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p filmcraft-platform -- amf_version_constants_are_valid`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/platform/src/amf/ffi.rs crates/platform/src/amf/abi_tests.rs
git commit -m "feat(platform): add AMD AMF C-ABI definitions and types"
```

---

### Task 2: AMF ABI Layout & Alignment Verification Tests

**Files:**
- Create / Modify: `crates/platform/src/amf/abi_tests.rs`

- [ ] **Step 1: Write ABI offset and size checks for AMF structures**

In `crates/platform/src/amf/abi_tests.rs`:
```rust
use super::ffi::*;
use std::mem::{align_of, size_of};

#[test]
fn amf_guid_layout() {
    assert_eq!((size_of::<AMFGuid>(), align_of::<AMFGuid>()), (16, 4));
}

#[test]
fn amf_variant_struct_layout() {
    // AMFVariantStruct is 24 bytes on x86_64 with 8-byte alignment
    assert_eq!((size_of::<AMFVariantStruct>(), align_of::<AMFVariantStruct>()), (24, 8));
}

#[test]
fn amf_rate_layout() {
    assert_eq!((size_of::<AMFRate>(), align_of::<AMFRate>()), (8, 4));
}
```

- [ ] **Step 2: Run test to verify passes**

Run: `cargo test -p filmcraft-platform -- amf_`
Expected: PASS.

- [ ] **Step 3: Add comprehensive layout tests for all vtable wrappers**

Add tests checking function pointer slot offsets for `AMFContextVtbl`, `AMFComponentVtbl`, `AMFSurfaceVtbl`, and `AMFBufferVtbl`.

- [ ] **Step 4: Run tests to verify**

Run: `cargo test -p filmcraft-platform -- amf_`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/platform/src/amf/abi_tests.rs
git commit -m "test(platform): add AMF ABI layout and structure size tests"
```

---

### Task 3: Dynamic Runtime Loader & Direct3D 11 Context

**Files:**
- Create: `crates/platform/src/amf/device.rs`

- [x] **Step 1: Write test for loading AMF runtime and probing factory**

In `crates/platform/src/amf/device.rs` (under `#[cfg(test)]`):
```rust
#[test]
fn probe_amfrt64_dll_if_present() {
    if !std::path::Path::new(r"C:\Windows\System32\amfrt64.dll").exists() {
        eprintln!("SKIPPED: amfrt64.dll not present on this system");
        return;
    }
    let runtime = AmfRuntime::load();
    assert!(runtime.is_ok(), "Failed to load amfrt64.dll: {:?}", runtime.err());
}
```

- [x] **Step 2: Run test to verify it fails before implementation**

Run: `cargo test -p filmcraft-platform -- probe_amfrt64_dll`
Expected: FAIL (`AmfRuntime` not defined).

- [x] **Step 3: Implement `AmfRuntime` and `AmfDevice`**

In `crates/platform/src/amf/device.rs`:
- Use `windows::Win32::System::LibraryLoader::LoadLibraryExW` with `LOAD_LIBRARY_SEARCH_SYSTEM32` to load `amfrt64.dll`.
- Resolve `AMFInit(AMF_FULL_VERSION, &mut factory)` using `GetProcAddress`.
- Create `AmfDevice` struct which:
  - Initializes `AMFContext` via `factory.CreateContext()`.
  - Obtains or creates a Direct3D 11 device handle for the AMD adapter.
  - Calls `context.InitDX11(d3d11_device, AMF_DX11_0)`.
  - Implements `Drop` to release COM interfaces cleanly.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p filmcraft-platform -- probe_amfrt64_dll`
Expected: PASS on Windows with AMD GPU driver.

- [x] **Step 5: Commit**

```bash
git add crates/platform/src/amf/device.rs
git commit -m "feat(platform): implement AMF runtime dynamic loading and D3D11 context initialization"
```

---

### Task 4: AMF Component Encoder Session Lifecycle

**Files:**
- Create: `crates/platform/src/amf/session.rs`

- [x] **Step 1: Write test for creating and configuring an AMF video encoder component**

In `crates/platform/src/amf/session.rs` (under `#[cfg(test)]`):
```rust
#[test]
fn test_amf_h264_session_initialization() {
    let Ok(runtime) = AmfRuntime::load() else { return; };
    let Ok(device) = AmfDevice::new(&runtime) else { return; };
    let config = AmfConfig {
        width: 1920,
        height: 1080,
        fps_num: 30,
        fps_den: 1,
        bitrate_kbps: 5000,
        max_bitrate_kbps: 7500,
        keyframe_interval: 30,
        codec: AmfCodec::H264,
        profile: AmfProfile::High,
        cbr: false,
    };
    let session = AmfSession::open(&device, config);
    assert!(session.is_ok(), "AMF session failed: {:?}", session.err());
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p filmcraft-platform -- test_amf_h264_session_initialization`
Expected: FAIL (`AmfSession` not defined).

- [x] **Step 3: Implement `AmfSession`**

In `crates/platform/src/amf/session.rs`:
- Instantiate `AMFComponent` via `factory.CreateComponent(context, L"AMFVideoEncoderVCE_AVC")` (or `L"AMFVideoEncoder_HEVC"`).
- Set encoder properties:
  - Usage: `AMF_VIDEO_ENCODER_USAGE_TRANSCODING`.
  - Profile, Bitrate (Target & Peak), Rate Control (CBR vs Peak Constrained VBR).
  - Frame rate rational and IDR period.
- Call `component.Init(format, width, height)`.
- Implement `submit_nv12(pts, dts, y_plane, uv_plane)`:
  - Allocates `AMFSurface` via `context.AllocSurface(AMF_MEMORY_HOST, AMF_SURFACE_NV12)`.
  - Copies planes into surface memory and sets PTS/DTS.
  - Calls `component.SubmitInput(surface)`.
  - Handles `AMF_INPUT_FULL` by querying outputs and retrying with bounded loop.
- Implement `query_output() -> Result<Option<AmfPacket>, String>`:
  - Calls `component.QueryOutput(&mut data)`.
  - Wraps output `AMFBuffer` memory into `AmfPacket { data, key, pts, dts }`.
- Implement `drain()`:
  - Calls `component.Drain()` and collects remaining queued packets.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p filmcraft-platform -- test_amf_h264_session_initialization`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add crates/platform/src/amf/session.rs
git commit -m "feat(platform): implement AMF encoder component session and frame submission"
```

---

### Task 5: Export VideoEncoder Adapter

**Files:**
- Create: `crates/platform/src/amf/export.rs`

- [ ] **Step 1: Write test verifying Annex B to length-prefixed conversion and parameter set extraction**

In `crates/platform/src/amf/export.rs` (under `#[cfg(test)]`):
```rust
#[test]
fn test_annexb_to_avcc_conversion() {
    let sps = [0x67, 0x64, 0x00, 0x1f, 0xac, 0xd9, 0x40, 0x78, 0x02, 0x27, 0xe5, 0x84];
    let pps = [0x68, 0xeb, 0xe3, 0xcb, 0x22, 0xc0];
    let slice = [0x65, 0x88, 0x84, 0x00, 0x10];
    let mut annexb = Vec::new();
    annexb.extend_from_slice(&[0, 0, 0, 1]); annexb.extend_from_slice(&sps);
    annexb.extend_from_slice(&[0, 0, 0, 1]); annexb.extend_from_slice(&pps);
    annexb.extend_from_slice(&[0, 0, 0, 1]); annexb.extend_from_slice(&slice);

    let parsed = parse_annexb_packets(&annexb).expect("parsed");
    assert_eq!(parsed.sps, Some(sps.to_vec()));
    assert_eq!(parsed.pps, Some(pps.to_vec()));
    assert_eq!(parsed.slices.len(), 1);
    assert_eq!(&parsed.slices[0][4..], &slice); // 4-byte length prefix
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p filmcraft-platform -- test_annexb_to_avcc_conversion`
Expected: FAIL (`parse_annexb_packets` not defined).

- [ ] **Step 3: Implement `AmfExportEncoder` and `VideoEncoder` trait**

In `crates/platform/src/amf/export.rs`:
- Implement `parse_annexb_packets` for both H.264 (extracting SPS/PPS) and HEVC (extracting VPS/SPS/PPS).
- Implement `AmfExportEncoder`:
  - Wraps `AmfSession`.
  - Implements `filmcraft_export::VideoEncoder`:
    - `sample_entry(&self) -> SampleEntry`: returns `SampleEntry::avc` or `SampleEntry::hevc`.
    - `timescale(&self) -> u32`: returns sequence timescale.
    - `encode(&mut self, frame: &EncoderFrame) -> filmcraft_export::Result<Vec<EncodedPacket>>`:
      - Converts incoming RGBA frame to NV12 using `filmcraft_export::rgba_to_yuv420_8`.
      - For HDR frames (`frame.hdr`), converts to 10-bit P010 using `filmcraft_export::rgbf_to_yuv420_10`.
      - Submits to `session` and collects output packets.
      - Converts Annex B to length-prefixed `EncodedPacket` with MP4 composition offset.
    - `flush(&mut self) -> filmcraft_export::Result<Vec<EncodedPacket>>`:
      - Drains session and packages all trailing frames.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p filmcraft-platform -- test_annexb_to_avcc_conversion`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/platform/src/amf/export.rs
git commit -m "feat(platform): implement VideoEncoder export adapter for AMF"
```

---

### Task 6: Module Assembly & Platform Pipeline Registration

**Files:**
- Create: `crates/platform/src/amf/mod.rs`
- Modify: `crates/platform/src/lib.rs`

- [ ] **Step 1: Write test for factory registration and capability probing**

In `crates/platform/src/amf/mod.rs`:
```rust
#[test]
fn test_amf_factory_declines_unsupported_settings() {
    let s = filmcraft_export::ExportSettings {
        hardware_encoding: filmcraft_export::HardwareEncoding::Off,
        ..Default::default()
    };
    let result = amf_encoder_factory(filmcraft_export::Format::H264, 1920, 1080, filmcraft_time::FrameRate::FPS_30, &s);
    assert!(result.is_none(), "Hardware encoding is off; factory should decline");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p filmcraft-platform -- test_amf_factory_declines_unsupported_settings`
Expected: FAIL (`amf_encoder_factory` not defined).

- [ ] **Step 3: Implement module exports and register in `lib.rs`**

In `crates/platform/src/amf/mod.rs`:
- Define `pub fn amf_encoder_factory(...) -> Option<filmcraft_export::Result<Box<dyn VideoEncoder>>>`.
- Define `pub fn hevc_available() -> bool`.
- Define `pub fn warm_hevc_probe()`.

In `crates/platform/src/lib.rs`:
- Add `#[cfg(target_os = "windows")] pub mod amf;`.
- Under `#[cfg(target_os = "windows")]` in `register()`:
  - Register `amf_encoder_factory` with `filmcraft_export::register_encoder`.
  - Register `amf::hevc_available` with `filmcraft_export::register_format_probe`.
  - Call `amf::warm_hevc_probe()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p filmcraft-platform -- test_amf_factory_declines_unsupported_settings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/platform/src/amf/mod.rs crates/platform/src/lib.rs
git commit -m "feat(platform): expose AMF encoder factory and register on Windows"
```

---

### Task 7: End-to-End Synthetic Export Verification & Documentation Update

**Files:**
- Modify: `docs/gaps.md`
- Test: Full cargo test run across `filmcraft-platform`

- [ ] **Step 1: Run complete platform test suite**

Run: `cargo test -p filmcraft-platform`
Expected: All tests PASS, with zero regressions.

- [ ] **Step 2: Update documentation gaps list**

In `docs/gaps.md`:
Update the Windows hardware encoding section to reflect that AMD AMF H.264 & HEVC encoding is now implemented and active.

- [ ] **Step 3: Commit**

```bash
git add docs/gaps.md
git commit -m "docs: record AMD AMF hardware encoder implementation in gaps"
```
