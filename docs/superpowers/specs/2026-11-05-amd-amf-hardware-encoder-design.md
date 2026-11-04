# Design Document: AMD AMF Hardware Video Encoder in FilmCraft

**Date:** 2026-11-05  
**Topic:** AMD Advanced Media Framework (AMF) H.264 & HEVC Hardware Encoding on Windows  
**Status:** Approved for Implementation Planning  
**Target Hardware:** AMD Radeon GPUs (RDNA2 / RDNA3 / VCN 2+, e.g. AMD Radeon RX 6700 XT)

---

## 1. Executive Summary & Problem Statement

Currently, FilmCraft on Windows only provides hardware-accelerated video encoding for NVIDIA GPUs via NVENC (`crates/platform/src/nvenc/`). When running on systems with AMD Radeon GPUs (such as the AMD Radeon RX 6700 XT), FilmCraft:
1. Rejects hardware encoding because no NVENC device is found.
2. Falls back to single-threaded or CPU-bound software encoding (`filmcraft-h264enc` / `openh264`) for H.264 exports, causing high CPU usage and long export times.
3. Cannot export to HEVC (H.265) at all, because FilmCraft lacks a software HEVC encoder.

This project implements native hardware video encoding for AMD GPUs on Windows using the **AMD Advanced Media Framework (AMF)** runtime (`amfrt64.dll`). It enables accelerated H.264 and HEVC (8-bit and 10-bit HDR) exports, matching the feature set and reliability of NVENC and VideoToolbox.

---

## 2. Architectural Design

The implementation resides entirely within `crates/platform`, which is the only crate in the workspace permitted to use platform FFI and `unsafe` blocks (`unsafe_code = "deny"` with strict FFI scoping per `AGENTS.md` and `docs/adr/0001-platform-ffi.md`).

### 2.1 Module Layout

```text
crates/platform/src/
├── amf/
│   ├── mod.rs          # Safe public module exports, probe caching, and encoder types
│   ├── ffi.rs          # Unsafe: AMF C-ABI definitions, vtables, GUIDs, and error codes
│   ├── device.rs       # Unsafe: amfrt64.dll dynamic loader & D3D11 context binding
│   ├── session.rs      # Unsafe: AMFComponent lifecycle, properties, submit & query loops
│   ├── export.rs       # Safe: implements filmcraft_export::VideoEncoder (MP4 packaging, avcC/hvcC)
│   └── abi_tests.rs    # Verification: struct sizes, alignments, and vtable offsets
├── amf.rs              # Module entry point
└── lib.rs              # Registration in platform export pipeline
```

---

## 3. Component Details & FFI Specifications

### 3.1 AMF Runtime Dynamic Loading (`ffi.rs`, `device.rs`)
* **DLL Target**: `amfrt64.dll` located in `C:\Windows\System32\amfrt64.dll` (provided by AMD Radeon Adrenalin drivers; no external SDK installation required).
* **Entry Point**:
  ```rust
  type AmfInitFn = unsafe extern "system" fn(
      version: u64,
      factory: *mut *mut c_void,
  ) -> i32;
  ```
  `AMF_FULL_VERSION` is computed as `(AMF_VERSION_MAJOR << 48) | (AMF_VERSION_MINOR << 32) | (AMF_VERSION_RELEASE << 16) | AMF_VERSION_BUILD`.
* **COM-style Interfaces**:
  * `AMFFactory`: Creates `AMFContext` and `AMFComponent`.
  * `AMFContext`: Manages D3D11 device binding (`InitDX11`), surface allocation (`AllocSurface`), and host memory buffers.
  * `AMFPropertyStorage`: Sets key-value configurations (bitrate, profile, GOP, quality preset).
  * `AMFComponent`: Encapsulates the encoder pipeline (`SubmitInput`, `QueryOutput`, `Drain`, `Flush`).
  * `AMFSurface`: Encapsulates video frame buffers (NV12 / P010).
  * `AMFBuffer`: Encapsulates compressed bitstream packets.

### 3.2 Encoder Components & Profiles (`session.rs`)
* **H.264 (AVC)**:
  * Component ID: `L"AMFVideoEncoderVCE_AVC"`.
  * Profiles: Baseline (66), Main (77), High (100).
  * Surface Format: `AMF_SURFACE_NV12` (8-bit 4:2:0).
* **HEVC (H.265)**:
  * Component ID: `L"AMFVideoEncoder_HEVC"`.
  * Profiles: Main (8-bit 4:2:0, `AMF_SURFACE_NV12`), Main 10 (10-bit 4:2:0, `AMF_SURFACE_P010`).
  * HDR Support: Handles BT.2020 color primaries and PQ / HLG transfer characteristics with static HDR10 SEI metadata.

### 3.3 Rate Control & Session Parameters
* **Bitrate Modes**:
  * CBR: `AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR`. Target bitrate equals peak bitrate.
  * VBR: `AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_PEAK_CONSTRAINED_VBR`. Target bitrate from settings, max bitrate constrained.
* **GOP / Keyframe Interval**: Configures `AMF_VIDEO_ENCODER_IDR_PERIOD_SIZE` or `AMF_VIDEO_ENCODER_GOP_SIZE` to match `ExportSettings::keyframe_distance`.
* **Frame Rate**: Rational fraction (`AMFRate { num, den }`).

---

## 4. Data Flow & Bitstream Packaging (`export.rs`)

```text
[FilmCraft Export Pipeline]
       │
       ▼ (EncoderFrame: RGBA or 10-bit HDR Floats)
[Color Conversion & Plane Assembly]
       │ ──► 8-bit SDR: rgba_to_yuv420_8 -> NV12 plane layout
       │ ──► 10-bit HDR: rgbf_to_yuv420_10 -> P010 plane layout
       ▼
[AMF Surface Submission]
       │ ──► AMFContext::AllocSurface(Host / DX11)
       │ ──► Copies NV12 / P010 planes into AMFSurface memory
       │ ──► Sets PTS / DTS (frame_index × duration)
       ▼
[AMFComponent Pipeline]
       │ ──► SubmitInput(surface)
       │     (if AMF_INPUT_FULL: drains QueryOutput and retries)
       ▼ (Hardware VCN Encode)
[AMFComponent::QueryOutput]
       │ ──► Receives AMFBuffer containing Annex B NAL stream
       ▼
[Stream Parsing & Packaging]
       │ ──► Scans Annex B NAL start codes (00 00 00 01 / 00 00 01)
       │ ──► Splits out parameter sets:
       │       - H.264: SPS (type 7) & PPS (type 8) -> AvcConfig (avcC)
       │       - HEVC: VPS (32), SPS (33), PPS (34) -> HevcConfig (hvcC)
       │ ──► Formats slice NALs with 4-byte big-endian length headers
       │ ──► Calculates MP4 composition offset: (pts - dts) × timescale
       ▼
[filmcraft_export::EncodedPacket] ──► MP4 Container Muxer
```

---

## 5. End-of-Stream & Draining

1. When the timeline completes, `VideoEncoder::flush()` calls `AMFComponent::Drain()`.
2. A bounded loop calls `QueryOutput()` repeatedly until `AMF_EOF` is returned or `QueryOutput()` produces no further data.
3. All buffered packets are converted to length-prefixed `EncodedPacket`s and returned, ensuring zero lost trailing frames.

---

## 6. Safety, Error Handling & Fallback Invariants

In compliance with `AGENTS.md`:

1. **Rule 0 (Never Crash)**:
   * Every public function returns `Result<T, String>` or `filmcraft_export::Result<T>`.
   * Zero panicking shortcuts: no `unwrap()`, `expect()`, `panic!`, `unimplemented!`, or `todo!`.
   * Hostile input protection: Picture dimensions must be even (`width.is_multiple_of(2) && height.is_multiple_of(2)`), frame rate numerator and denominator must be non-zero, and integer arithmetic uses `checked_*` / `saturating_*`.
   * Bounded loops: `SubmitInput` and `QueryOutput` retry loops have a hard limit of 1,000 iterations to prevent hung driver states from hanging the host process.
2. **Rule 3 (Platform FFI Boundaries)**:
   * `unsafe` code is strictly prohibited in `export.rs` and `mod.rs`.
   * `unsafe` is isolated to `ffi.rs`, `device.rs`, and `session.rs`.
   * Every single `unsafe` block must include a descriptive `// SAFETY:` comment justifying pointer provenance and COM lifetime.
3. **Graceful Fallback**:
   * For H.264: If AMF fails to initialize (no AMD driver, incompatible dimensions, or initialization error), the factory returns `None`, allowing FilmCraft to fall back to the built-in software H.264 encoder transparently.
   * For HEVC: Since FilmCraft has no software fallback for HEVC, a failure reports an explicit, helpful error (`ExportError::Unsupported`).

---

## 7. Verification & Testing Strategy

1. **ABI Layout Tests (`abi_tests.rs`)**:
   * Compile-time and runtime checks on struct sizes, field offsets, and alignments for AMF COM vtables against AMD AMF specifications.
2. **Synthetic Pipeline Test**:
   * Unit tests passing synthetic gradient test frames through `AMFEncoder` in software/mock or live hardware environment.
3. **Integration Verification**:
   * Verification of exported `.mp4` file playability in VLC and Adobe Premiere Pro.
   * Parameter set inspection (`avcC` / `hvcC` validation via `filmcraft_isobmff`).
