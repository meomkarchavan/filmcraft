// The declarations in this file are transcribed from AMD's Advanced Media Framework (AMF)
// SDK headers (Version 1.4.33, MIT license by Advanced Micro Devices, Inc.):
//
// Copyright (c) 2016-2024 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! AMD Advanced Media Framework (AMF) runtime C-ABI types and vtables.
//!
//! FFI module (docs/adr/0001-platform-ffi.md): plain `repr(C)` data, types and function pointers,
//! no logic.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use std::ffi::c_void;

// AMF SDK version 1.4.33
pub const AMF_VERSION_MAJOR: u64 = 1;
pub const AMF_VERSION_MINOR: u64 = 4;
pub const AMF_VERSION_RELEASE: u64 = 33;
pub const AMF_VERSION_BUILD: u64 = 0;

pub const AMF_FULL_VERSION: u64 = (AMF_VERSION_MAJOR << 48)
    | (AMF_VERSION_MINOR << 32)
    | (AMF_VERSION_RELEASE << 16)
    | AMF_VERSION_BUILD;

// AMF_RESULT return codes
pub type AMF_RESULT = i32;

pub const AMF_OK: AMF_RESULT = 0;
pub const AMF_FAIL: AMF_RESULT = 1;
pub const AMF_UNEXPECTED: AMF_RESULT = 2;
pub const AMF_ACCESS_DENIED: AMF_RESULT = 3;
pub const AMF_INVALID_ARG: AMF_RESULT = 4;
pub const AMF_OUT_OF_MEMORY: AMF_RESULT = 5;
pub const AMF_NOT_INITIALIZED: AMF_RESULT = 6;
pub const AMF_NOT_SUPPORTED: AMF_RESULT = 7;
pub const AMF_NOT_IMPLEMENTED: AMF_RESULT = 8;
pub const AMF_NOT_FOUND: AMF_RESULT = 9;
pub const AMF_ALREADY_INITIALIZED: AMF_RESULT = 10;
pub const AMF_EOF: AMF_RESULT = 11;
pub const AMF_INPUT_FULL: AMF_RESULT = 36;
pub const AMF_DECODER_NO_FREE_SURFACES: AMF_RESULT = 38;
pub const AMF_REPEAT: AMF_RESULT = 40;

// AMF_MEMORY_TYPE
pub type AMF_MEMORY_TYPE = u32;

pub const AMF_MEMORY_UNKNOWN: AMF_MEMORY_TYPE = 0;
pub const AMF_MEMORY_HOST: AMF_MEMORY_TYPE = 1;
pub const AMF_MEMORY_DX9: AMF_MEMORY_TYPE = 2;
pub const AMF_MEMORY_DX11: AMF_MEMORY_TYPE = 3;
pub const AMF_MEMORY_OPENCL: AMF_MEMORY_TYPE = 4;
pub const AMF_MEMORY_OPENGL: AMF_MEMORY_TYPE = 5;
pub const AMF_MEMORY_VULKAN: AMF_MEMORY_TYPE = 6;

// AMF_SURFACE_FORMAT
pub type AMF_SURFACE_FORMAT = u32;

pub const AMF_SURFACE_UNKNOWN: AMF_SURFACE_FORMAT = 0;
pub const AMF_SURFACE_NV12: AMF_SURFACE_FORMAT = 1;
pub const AMF_SURFACE_YV12: AMF_SURFACE_FORMAT = 2;
pub const AMF_SURFACE_BGRA: AMF_SURFACE_FORMAT = 3;
pub const AMF_SURFACE_ARGB: AMF_SURFACE_FORMAT = 4;
pub const AMF_SURFACE_RGBA: AMF_SURFACE_FORMAT = 5;
pub const AMF_SURFACE_GRAY8: AMF_SURFACE_FORMAT = 6;
pub const AMF_SURFACE_YUV420P: AMF_SURFACE_FORMAT = 7;
pub const AMF_SURFACE_BGR0: AMF_SURFACE_FORMAT = 8;
pub const AMF_SURFACE_R10G10B10A2: AMF_SURFACE_FORMAT = 9;
pub const AMF_SURFACE_P010: AMF_SURFACE_FORMAT = 10;

// AMF_DX_VERSION
pub type AMF_DX_VERSION = u32;
pub const AMF_DX11_0: AMF_DX_VERSION = 0;
pub const AMF_DX11_1: AMF_DX_VERSION = 1;
pub const AMF_DX11_2: AMF_DX_VERSION = 2;

// AMF_PLANE_TYPE
pub type AMF_PLANE_TYPE = u32;
pub const AMF_PLANE_UNKNOWN: AMF_PLANE_TYPE = 0;
pub const AMF_PLANE_PACKED: AMF_PLANE_TYPE = 1;
pub const AMF_PLANE_Y: AMF_PLANE_TYPE = 2;
pub const AMF_PLANE_UV: AMF_PLANE_TYPE = 3;
pub const AMF_PLANE_U: AMF_PLANE_TYPE = 4;
pub const AMF_PLANE_V: AMF_PLANE_TYPE = 5;

// AMF_FRAME_TYPE
pub type AMF_FRAME_TYPE = u32;
pub const AMF_FRAME_UNKNOWN: AMF_FRAME_TYPE = 0;
pub const AMF_FRAME_PROGRESSIVE: AMF_FRAME_TYPE = 1;
pub const AMF_FRAME_INTERLACED_EVEN: AMF_FRAME_TYPE = 2;
pub const AMF_FRAME_INTERLACED_ODD: AMF_FRAME_TYPE = 3;

/// 128-bit GUID matching the Windows GUID layout.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AMFGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

pub const IID_AMFPropertyStorage: AMFGuid = AMFGuid {
    data1: 0xc7cec05b,
    data2: 0xcfb9,
    data3: 0x48af,
    data4: [0xac, 0xe3, 0xf6, 0x8d, 0xf8, 0x39, 0x5f, 0xe3],
};

pub const IID_AMFContext: AMFGuid = AMFGuid {
    data1: 0xa76a13f0,
    data2: 0xd80e,
    data3: 0x4fcc,
    data4: [0xb5, 0x08, 0x65, 0xd0, 0xb5, 0x2e, 0xd9, 0xee],
};

pub const IID_AMFData: AMFGuid = AMFGuid {
    data1: 0xa0ee3387,
    data2: 0x3cb7,
    data3: 0x47cb,
    data4: [0x8b, 0xa7, 0xfa, 0x93, 0x60, 0x3b, 0x22, 0xc1],
};

pub const IID_AMFSurface: AMFGuid = AMFGuid {
    data1: 0x3075dbe3,
    data2: 0x8718,
    data3: 0x4cfa,
    data4: [0x86, 0xfb, 0x21, 0x14, 0xc0, 0xa5, 0xa4, 0x51],
};

pub const IID_AMFBuffer: AMFGuid = AMFGuid {
    data1: 0xb04b7248,
    data2: 0xb63d,
    data3: 0x497a,
    data4: [0x8c, 0x50, 0x2b, 0x72, 0x71, 0x41, 0x79, 0xb8],
};

pub const IID_AMFComponent: AMFGuid = AMFGuid {
    data1: 0x8b776269,
    data2: 0x904a,
    data3: 0x4d59,
    data4: [0x97, 0x10, 0x6d, 0x72, 0x9b, 0x15, 0x27, 0x54],
};

/// Rational frame rate or ratio.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AMFRate {
    pub num: u32,
    pub den: u32,
}

/// 2D dimensions.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AMFSize {
    pub width: i32,
    pub height: i32,
}

/// 2D point.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AMFPoint {
    pub x: i32,
    pub y: i32,
}

/// 2D rectangle.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AMFRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// AMFVariantType
pub type AMF_VARIANT_TYPE = u32;

pub const AMF_VARIANT_EMPTY: AMF_VARIANT_TYPE = 0;
pub const AMF_VARIANT_BOOL: AMF_VARIANT_TYPE = 1;
pub const AMF_VARIANT_INT64: AMF_VARIANT_TYPE = 2;
pub const AMF_VARIANT_DOUBLE: AMF_VARIANT_TYPE = 3;
pub const AMF_VARIANT_STRING: AMF_VARIANT_TYPE = 4;
pub const AMF_VARIANT_WSTRING: AMF_VARIANT_TYPE = 5;
pub const AMF_VARIANT_INTERFACE: AMF_VARIANT_TYPE = 6;
pub const AMF_VARIANT_FLOAT: AMF_VARIANT_TYPE = 7;
pub const AMF_VARIANT_RATE: AMF_VARIANT_TYPE = 8;
pub const AMF_VARIANT_RATIO: AMF_VARIANT_TYPE = 9;
pub const AMF_VARIANT_COLOR: AMF_VARIANT_TYPE = 10;
pub const AMF_VARIANT_SIZE: AMF_VARIANT_TYPE = 11;
pub const AMF_VARIANT_RECT: AMF_VARIANT_TYPE = 12;

/// 16-byte value payload for AMFVariantStruct, 8-byte aligned.
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub union AMFVariantValue {
    pub int64_value: i64,
    pub double_value: f64,
    pub bool_value: bool,
    pub ptr_value: *mut c_void,
    pub wstring_value: *const u16,
    pub string_value: *const std::ffi::c_char,
    pub rate_value: AMFRate,
    pub size_value: AMFSize,
    pub rect_value: AMFRect,
    pub bytes: [u8; 16],
}

/// Variant structure matching AMF C-ABI (24 bytes, align 8).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AMFVariantStruct {
    pub type_: AMF_VARIANT_TYPE,
    pub padding: u32,
    pub value: AMFVariantValue,
}

impl AMFVariantStruct {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            type_: AMF_VARIANT_EMPTY,
            padding: 0,
            value: AMFVariantValue { bytes: [0u8; 16] },
        }
    }

    #[inline]
    pub const fn from_int64(v: i64) -> Self {
        Self {
            type_: AMF_VARIANT_INT64,
            padding: 0,
            value: AMFVariantValue { int64_value: v },
        }
    }

    #[inline]
    pub const fn from_bool(v: bool) -> Self {
        Self {
            type_: AMF_VARIANT_BOOL,
            padding: 0,
            value: AMFVariantValue { bool_value: v },
        }
    }

    #[inline]
    pub const fn from_rate(num: u32, den: u32) -> Self {
        Self {
            type_: AMF_VARIANT_RATE,
            padding: 0,
            value: AMFVariantValue {
                rate_value: AMFRate { num, den },
            },
        }
    }

    #[inline]
    pub const fn from_size(width: i32, height: i32) -> Self {
        Self {
            type_: AMF_VARIANT_SIZE,
            padding: 0,
            value: AMFVariantValue {
                size_value: AMFSize { width, height },
            },
        }
    }
}

// ---------------------------------------------------------------------------
// AMFPropertyStorage
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFPropertyStorage {
    pub pVtbl: *const AMFPropertyStorageVtbl,
}

#[repr(C)]
pub struct AMFPropertyStorageVtbl {
    // AMFInterface methods
    pub Acquire: unsafe extern "system" fn(pThis: *mut AMFPropertyStorage) -> i64,
    pub Release: unsafe extern "system" fn(pThis: *mut AMFPropertyStorage) -> i64,
    pub QueryInterface: unsafe extern "system" fn(
        pThis: *mut AMFPropertyStorage,
        interfaceID: *const AMFGuid,
        ppInterface: *mut *mut c_void,
    ) -> AMF_RESULT,

    // AMFPropertyStorage methods
    pub SetProperty: unsafe extern "system" fn(
        pThis: *mut AMFPropertyStorage,
        name: *const u16,
        value: AMFVariantStruct,
    ) -> AMF_RESULT,
    pub GetProperty: unsafe extern "system" fn(
        pThis: *mut AMFPropertyStorage,
        name: *const u16,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub HasProperty: unsafe extern "system" fn(pThis: *mut AMFPropertyStorage, name: *const u16) -> bool,
    pub GetPropertyCount: unsafe extern "system" fn(pThis: *mut AMFPropertyStorage) -> usize,
    pub GetPropertyAt: unsafe extern "system" fn(
        pThis: *mut AMFPropertyStorage,
        index: usize,
        name: *mut u16,
        nameSize: usize,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub Clear: unsafe extern "system" fn(pThis: *mut AMFPropertyStorage) -> AMF_RESULT,
    pub AddTo: unsafe extern "system" fn(
        pThis: *mut AMFPropertyStorage,
        pDest: *mut AMFPropertyStorage,
        bOverwrite: bool,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub CopyTo: unsafe extern "system" fn(
        pThis: *mut AMFPropertyStorage,
        pDest: *mut AMFPropertyStorage,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub AddObserver: unsafe extern "system" fn(pThis: *mut AMFPropertyStorage, pObserver: *mut c_void),
    pub RemoveObserver: unsafe extern "system" fn(pThis: *mut AMFPropertyStorage, pObserver: *mut c_void),
}

// ---------------------------------------------------------------------------
// AMFData
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFData {
    pub pVtbl: *const AMFDataVtbl,
}

#[repr(C)]
pub struct AMFDataVtbl {
    // AMFInterface methods
    pub Acquire: unsafe extern "system" fn(pThis: *mut AMFData) -> i64,
    pub Release: unsafe extern "system" fn(pThis: *mut AMFData) -> i64,
    pub QueryInterface: unsafe extern "system" fn(
        pThis: *mut AMFData,
        interfaceID: *const AMFGuid,
        ppInterface: *mut *mut c_void,
    ) -> AMF_RESULT,

    // AMFPropertyStorage methods
    pub SetProperty: unsafe extern "system" fn(
        pThis: *mut AMFData,
        name: *const u16,
        value: AMFVariantStruct,
    ) -> AMF_RESULT,
    pub GetProperty: unsafe extern "system" fn(
        pThis: *mut AMFData,
        name: *const u16,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub HasProperty: unsafe extern "system" fn(pThis: *mut AMFData, name: *const u16) -> bool,
    pub GetPropertyCount: unsafe extern "system" fn(pThis: *mut AMFData) -> usize,
    pub GetPropertyAt: unsafe extern "system" fn(
        pThis: *mut AMFData,
        index: usize,
        name: *mut u16,
        nameSize: usize,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub Clear: unsafe extern "system" fn(pThis: *mut AMFData) -> AMF_RESULT,
    pub AddTo: unsafe extern "system" fn(
        pThis: *mut AMFData,
        pDest: *mut AMFPropertyStorage,
        bOverwrite: bool,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub CopyTo: unsafe extern "system" fn(
        pThis: *mut AMFData,
        pDest: *mut AMFPropertyStorage,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub AddObserver: unsafe extern "system" fn(pThis: *mut AMFData, pObserver: *mut c_void),
    pub RemoveObserver: unsafe extern "system" fn(pThis: *mut AMFData, pObserver: *mut c_void),

    // AMFData methods
    pub GetMemoryType: unsafe extern "system" fn(pThis: *mut AMFData) -> AMF_MEMORY_TYPE,
    pub Duplicate: unsafe extern "system" fn(
        pThis: *mut AMFData,
        type_: AMF_MEMORY_TYPE,
        ppData: *mut *mut AMFData,
    ) -> AMF_RESULT,
    pub Convert: unsafe extern "system" fn(pThis: *mut AMFData, type_: AMF_MEMORY_TYPE) -> AMF_RESULT,
    pub Interop: unsafe extern "system" fn(pThis: *mut AMFData, type_: AMF_MEMORY_TYPE) -> AMF_RESULT,
    pub DataType: unsafe extern "system" fn(pThis: *mut AMFData) -> u32,
    pub GetPts: unsafe extern "system" fn(pThis: *mut AMFData) -> i64,
    pub SetPts: unsafe extern "system" fn(pThis: *mut AMFData, pts: i64),
    pub GetDuration: unsafe extern "system" fn(pThis: *mut AMFData) -> i64,
    pub SetDuration: unsafe extern "system" fn(pThis: *mut AMFData, duration: i64),
}

// ---------------------------------------------------------------------------
// AMFBuffer
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFBuffer {
    pub pVtbl: *const AMFBufferVtbl,
}

#[repr(C)]
pub struct AMFBufferVtbl {
    // AMFInterface methods
    pub Acquire: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> i64,
    pub Release: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> i64,
    pub QueryInterface: unsafe extern "system" fn(
        pThis: *mut AMFBuffer,
        interfaceID: *const AMFGuid,
        ppInterface: *mut *mut c_void,
    ) -> AMF_RESULT,

    // AMFPropertyStorage methods
    pub SetProperty: unsafe extern "system" fn(
        pThis: *mut AMFBuffer,
        name: *const u16,
        value: AMFVariantStruct,
    ) -> AMF_RESULT,
    pub GetProperty: unsafe extern "system" fn(
        pThis: *mut AMFBuffer,
        name: *const u16,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub HasProperty: unsafe extern "system" fn(pThis: *mut AMFBuffer, name: *const u16) -> bool,
    pub GetPropertyCount: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> usize,
    pub GetPropertyAt: unsafe extern "system" fn(
        pThis: *mut AMFBuffer,
        index: usize,
        name: *mut u16,
        nameSize: usize,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub Clear: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> AMF_RESULT,
    pub AddTo: unsafe extern "system" fn(
        pThis: *mut AMFBuffer,
        pDest: *mut AMFPropertyStorage,
        bOverwrite: bool,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub CopyTo: unsafe extern "system" fn(
        pThis: *mut AMFBuffer,
        pDest: *mut AMFPropertyStorage,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub AddObserver: unsafe extern "system" fn(pThis: *mut AMFBuffer, pObserver: *mut c_void),
    pub RemoveObserver: unsafe extern "system" fn(pThis: *mut AMFBuffer, pObserver: *mut c_void),

    // AMFData methods
    pub GetMemoryType: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> AMF_MEMORY_TYPE,
    pub Duplicate: unsafe extern "system" fn(
        pThis: *mut AMFBuffer,
        type_: AMF_MEMORY_TYPE,
        ppData: *mut *mut AMFData,
    ) -> AMF_RESULT,
    pub Convert: unsafe extern "system" fn(pThis: *mut AMFBuffer, type_: AMF_MEMORY_TYPE) -> AMF_RESULT,
    pub Interop: unsafe extern "system" fn(pThis: *mut AMFBuffer, type_: AMF_MEMORY_TYPE) -> AMF_RESULT,
    pub DataType: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> u32,
    pub GetPts: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> i64,
    pub SetPts: unsafe extern "system" fn(pThis: *mut AMFBuffer, pts: i64),
    pub GetDuration: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> i64,
    pub SetDuration: unsafe extern "system" fn(pThis: *mut AMFBuffer, duration: i64),

    // AMFBuffer methods
    pub SetSize: unsafe extern "system" fn(pThis: *mut AMFBuffer, newSize: usize) -> AMF_RESULT,
    pub GetSize: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> usize,
    pub GetNative: unsafe extern "system" fn(pThis: *mut AMFBuffer) -> *mut c_void,
    pub AddObserver_Buffer: unsafe extern "system" fn(pThis: *mut AMFBuffer, pObserver: *mut c_void),
    pub RemoveObserver_Buffer: unsafe extern "system" fn(pThis: *mut AMFBuffer, pObserver: *mut c_void),
}

// ---------------------------------------------------------------------------
// AMFPlane
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFPlane {
    pub pVtbl: *const AMFPlaneVtbl,
}

#[repr(C)]
pub struct AMFPlaneVtbl {
    pub Acquire: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i64,
    pub Release: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i64,
    pub QueryInterface: unsafe extern "system" fn(
        pThis: *mut AMFPlane,
        interfaceID: *const AMFGuid,
        ppInterface: *mut *mut c_void,
    ) -> AMF_RESULT,

    pub GetType: unsafe extern "system" fn(pThis: *mut AMFPlane) -> AMF_PLANE_TYPE,
    pub GetNative: unsafe extern "system" fn(pThis: *mut AMFPlane) -> *mut c_void,
    pub GetPixelSizeInBytes: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i32,
    pub GetOffsetX: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i32,
    pub GetOffsetY: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i32,
    pub GetHPitch: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i32,
    pub GetVPitch: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i32,
    pub GetWidth: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i32,
    pub GetHeight: unsafe extern "system" fn(pThis: *mut AMFPlane) -> i32,
}

// ---------------------------------------------------------------------------
// AMFSurface
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFSurface {
    pub pVtbl: *const AMFSurfaceVtbl,
}

#[repr(C)]
pub struct AMFSurfaceVtbl {
    // AMFInterface methods
    pub Acquire: unsafe extern "system" fn(pThis: *mut AMFSurface) -> i64,
    pub Release: unsafe extern "system" fn(pThis: *mut AMFSurface) -> i64,
    pub QueryInterface: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        interfaceID: *const AMFGuid,
        ppInterface: *mut *mut c_void,
    ) -> AMF_RESULT,

    // AMFPropertyStorage methods
    pub SetProperty: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        name: *const u16,
        value: AMFVariantStruct,
    ) -> AMF_RESULT,
    pub GetProperty: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        name: *const u16,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub HasProperty: unsafe extern "system" fn(pThis: *mut AMFSurface, name: *const u16) -> bool,
    pub GetPropertyCount: unsafe extern "system" fn(pThis: *mut AMFSurface) -> usize,
    pub GetPropertyAt: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        index: usize,
        name: *mut u16,
        nameSize: usize,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub Clear: unsafe extern "system" fn(pThis: *mut AMFSurface) -> AMF_RESULT,
    pub AddTo: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        pDest: *mut AMFPropertyStorage,
        bOverwrite: bool,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub CopyTo: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        pDest: *mut AMFPropertyStorage,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub AddObserver: unsafe extern "system" fn(pThis: *mut AMFSurface, pObserver: *mut c_void),
    pub RemoveObserver: unsafe extern "system" fn(pThis: *mut AMFSurface, pObserver: *mut c_void),

    // AMFData methods
    pub GetMemoryType: unsafe extern "system" fn(pThis: *mut AMFSurface) -> AMF_MEMORY_TYPE,
    pub Duplicate: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        type_: AMF_MEMORY_TYPE,
        ppData: *mut *mut AMFData,
    ) -> AMF_RESULT,
    pub Convert: unsafe extern "system" fn(pThis: *mut AMFSurface, type_: AMF_MEMORY_TYPE) -> AMF_RESULT,
    pub Interop: unsafe extern "system" fn(pThis: *mut AMFSurface, type_: AMF_MEMORY_TYPE) -> AMF_RESULT,
    pub DataType: unsafe extern "system" fn(pThis: *mut AMFSurface) -> u32,
    pub GetPts: unsafe extern "system" fn(pThis: *mut AMFSurface) -> i64,
    pub SetPts: unsafe extern "system" fn(pThis: *mut AMFSurface, pts: i64),
    pub GetDuration: unsafe extern "system" fn(pThis: *mut AMFSurface) -> i64,
    pub SetDuration: unsafe extern "system" fn(pThis: *mut AMFSurface, duration: i64),

    // AMFSurface methods
    pub GetFormat: unsafe extern "system" fn(pThis: *mut AMFSurface) -> AMF_SURFACE_FORMAT,
    pub GetPlanesCount: unsafe extern "system" fn(pThis: *mut AMFSurface) -> usize,
    pub GetPlaneAt: unsafe extern "system" fn(pThis: *mut AMFSurface, index: usize) -> *mut AMFPlane,
    pub GetPlane: unsafe extern "system" fn(pThis: *mut AMFSurface, type_: AMF_PLANE_TYPE) -> *mut AMFPlane,
    pub GetFrameType: unsafe extern "system" fn(pThis: *mut AMFSurface) -> AMF_FRAME_TYPE,
    pub SetFrameType: unsafe extern "system" fn(pThis: *mut AMFSurface, type_: AMF_FRAME_TYPE),
    pub SetCrop: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> AMF_RESULT,
    pub CopySurfaceRegion: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        pDest: *mut AMFSurface,
        dstX: i32,
        dstY: i32,
        srcX: i32,
        srcY: i32,
        width: i32,
        height: i32,
    ) -> AMF_RESULT,
    pub CopySurfaceRegionScale: unsafe extern "system" fn(
        pThis: *mut AMFSurface,
        pDest: *mut AMFSurface,
        srcX: i32,
        srcY: i32,
        srcWidth: i32,
        srcHeight: i32,
        dstX: i32,
        dstY: i32,
        dstWidth: i32,
        dstHeight: i32,
    ) -> AMF_RESULT,
    pub AddObserver_Surface: unsafe extern "system" fn(pThis: *mut AMFSurface, pObserver: *mut c_void),
    pub RemoveObserver_Surface: unsafe extern "system" fn(pThis: *mut AMFSurface, pObserver: *mut c_void),
}

// ---------------------------------------------------------------------------
// AMFContext
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFContext {
    pub pVtbl: *const AMFContextVtbl,
}

#[repr(C)]
pub struct AMFContextVtbl {
    // AMFInterface methods
    pub Acquire: unsafe extern "system" fn(pThis: *mut AMFContext) -> i64,
    pub Release: unsafe extern "system" fn(pThis: *mut AMFContext) -> i64,
    pub QueryInterface: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        interfaceID: *const AMFGuid,
        ppInterface: *mut *mut c_void,
    ) -> AMF_RESULT,

    // AMFPropertyStorage methods
    pub SetProperty: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        name: *const u16,
        value: AMFVariantStruct,
    ) -> AMF_RESULT,
    pub GetProperty: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        name: *const u16,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub HasProperty: unsafe extern "system" fn(pThis: *mut AMFContext, name: *const u16) -> bool,
    pub GetPropertyCount: unsafe extern "system" fn(pThis: *mut AMFContext) -> usize,
    pub GetPropertyAt: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        index: usize,
        name: *mut u16,
        nameSize: usize,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub Clear: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,
    pub AddTo: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        pDest: *mut AMFPropertyStorage,
        bOverwrite: bool,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub CopyTo: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        pDest: *mut AMFPropertyStorage,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub AddObserver: unsafe extern "system" fn(pThis: *mut AMFContext, pObserver: *mut c_void),
    pub RemoveObserver: unsafe extern "system" fn(pThis: *mut AMFContext, pObserver: *mut c_void),

    // AMFContext methods
    pub Terminate: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,

    pub InitDX9: unsafe extern "system" fn(pThis: *mut AMFContext, pDX9Device: *mut c_void) -> AMF_RESULT,
    pub GetDX9Device: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        dxVersion: u32,
        ppDX9Device: *mut *mut c_void,
    ) -> AMF_RESULT,
    pub LockDX9: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,
    pub UnlockDX9: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,

    pub InitDX11: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        pDX11Device: *mut c_void,
        dxVersion: u32,
    ) -> AMF_RESULT,
    pub GetDX11Device: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        dxVersion: u32,
        ppDX11Device: *mut *mut c_void,
    ) -> AMF_RESULT,
    pub LockDX11: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,
    pub UnlockDX11: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,

    pub InitOpenCL: unsafe extern "system" fn(pThis: *mut AMFContext, pCommandQueue: *mut c_void) -> AMF_RESULT,
    pub GetOpenCLContext: unsafe extern "system" fn(pThis: *mut AMFContext, ppContext: *mut *mut c_void) -> AMF_RESULT,
    pub GetOpenCLCommandQueue: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        ppCommandQueue: *mut *mut c_void,
    ) -> AMF_RESULT,
    pub LockOpenCL: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,
    pub UnlockOpenCL: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,

    pub InitOpenGL: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        hOpenGLContext: *mut c_void,
        hDC: *mut c_void,
        hWnd: *mut c_void,
    ) -> AMF_RESULT,
    pub GetOpenGLContext: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        ppOpenGLContext: *mut *mut c_void,
    ) -> AMF_RESULT,
    pub GetOpenGLDrawable: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        ppOpenGLDrawable: *mut *mut c_void,
    ) -> AMF_RESULT,
    pub LockOpenGL: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,
    pub UnlockOpenGL: unsafe extern "system" fn(pThis: *mut AMFContext) -> AMF_RESULT,

    pub AllocBuffer: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        type_: AMF_MEMORY_TYPE,
        size: usize,
        ppBuffer: *mut *mut AMFBuffer,
    ) -> AMF_RESULT,
    pub AllocSurface: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        type_: AMF_MEMORY_TYPE,
        format: AMF_SURFACE_FORMAT,
        width: i32,
        height: i32,
        ppSurface: *mut *mut AMFSurface,
    ) -> AMF_RESULT,
    pub AllocAudioBuffer: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        type_: AMF_MEMORY_TYPE,
        format: u32,
        samples: i32,
        sampleRate: i32,
        channels: i32,
        ppAudioBuffer: *mut *mut c_void,
    ) -> AMF_RESULT,

    pub CreateBufferFromHostNative: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        pBuffer: *mut c_void,
        size: usize,
        ppBuffer: *mut *mut AMFBuffer,
        pObserver: *mut c_void,
    ) -> AMF_RESULT,
    pub CreateSurfaceFromHostNative: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        format: AMF_SURFACE_FORMAT,
        width: i32,
        height: i32,
        hPitch: i32,
        vPitch: i32,
        pBuffer: *mut c_void,
        ppSurface: *mut *mut AMFSurface,
        pObserver: *mut c_void,
    ) -> AMF_RESULT,
    pub CreateSurfaceFromDX9Native: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        pDX9Surface: *mut c_void,
        ppSurface: *mut *mut AMFSurface,
        pObserver: *mut c_void,
    ) -> AMF_RESULT,
    pub CreateSurfaceFromDX11Native: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        pDX11Surface: *mut c_void,
        ppSurface: *mut *mut AMFSurface,
        pObserver: *mut c_void,
    ) -> AMF_RESULT,
    pub CreateSurfaceFromOpenGLNative: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        format: AMF_SURFACE_FORMAT,
        hGLTexture: u32,
        ppSurface: *mut *mut AMFSurface,
        pObserver: *mut c_void,
    ) -> AMF_RESULT,
    pub CreateSurfaceFromOpenCLNative: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        format: AMF_SURFACE_FORMAT,
        width: i32,
        height: i32,
        pCLMem: *mut *mut c_void,
        ppSurface: *mut *mut AMFSurface,
        pObserver: *mut c_void,
    ) -> AMF_RESULT,
    pub CreateBufferFromOpenCLNative: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        pCLMem: *mut c_void,
        size: usize,
        ppBuffer: *mut *mut AMFBuffer,
        pObserver: *mut c_void,
    ) -> AMF_RESULT,
    pub GetCompute: unsafe extern "system" fn(
        pThis: *mut AMFContext,
        eComputeType: u32,
        ppCompute: *mut *mut c_void,
    ) -> AMF_RESULT,
}

// ---------------------------------------------------------------------------
// AMFComponent
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFComponent {
    pub pVtbl: *const AMFComponentVtbl,
}

#[repr(C)]
pub struct AMFComponentVtbl {
    // AMFInterface methods
    pub Acquire: unsafe extern "system" fn(pThis: *mut AMFComponent) -> i64,
    pub Release: unsafe extern "system" fn(pThis: *mut AMFComponent) -> i64,
    pub QueryInterface: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        interfaceID: *const AMFGuid,
        ppInterface: *mut *mut c_void,
    ) -> AMF_RESULT,

    // AMFPropertyStorage methods
    pub SetProperty: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        name: *const u16,
        value: AMFVariantStruct,
    ) -> AMF_RESULT,
    pub GetProperty: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        name: *const u16,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub HasProperty: unsafe extern "system" fn(pThis: *mut AMFComponent, name: *const u16) -> bool,
    pub GetPropertyCount: unsafe extern "system" fn(pThis: *mut AMFComponent) -> usize,
    pub GetPropertyAt: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        index: usize,
        name: *mut u16,
        nameSize: usize,
        pValue: *mut AMFVariantStruct,
    ) -> AMF_RESULT,
    pub Clear: unsafe extern "system" fn(pThis: *mut AMFComponent) -> AMF_RESULT,
    pub AddTo: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        pDest: *mut AMFPropertyStorage,
        bOverwrite: bool,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub CopyTo: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        pDest: *mut AMFPropertyStorage,
        bDeep: bool,
    ) -> AMF_RESULT,
    pub AddObserver: unsafe extern "system" fn(pThis: *mut AMFComponent, pObserver: *mut c_void),
    pub RemoveObserver: unsafe extern "system" fn(pThis: *mut AMFComponent, pObserver: *mut c_void),

    // AMFComponent methods
    pub Init: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        format: AMF_SURFACE_FORMAT,
        width: i32,
        height: i32,
    ) -> AMF_RESULT,
    pub ReInit: unsafe extern "system" fn(pThis: *mut AMFComponent, width: i32, height: i32) -> AMF_RESULT,
    pub Terminate: unsafe extern "system" fn(pThis: *mut AMFComponent) -> AMF_RESULT,
    pub Drain: unsafe extern "system" fn(pThis: *mut AMFComponent) -> AMF_RESULT,
    pub Flush: unsafe extern "system" fn(pThis: *mut AMFComponent) -> AMF_RESULT,
    pub SubmitInput: unsafe extern "system" fn(pThis: *mut AMFComponent, pData: *mut AMFData) -> AMF_RESULT,
    pub QueryOutput: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        ppData: *mut *mut AMFData,
    ) -> AMF_RESULT,
    pub GetContext: unsafe extern "system" fn(pThis: *mut AMFComponent) -> *mut AMFContext,
    pub SetOutputDataAllocatorCB: unsafe extern "system" fn(
        pThis: *mut AMFComponent,
        callback: *mut c_void,
    ) -> AMF_RESULT,
    pub GetCaps: unsafe extern "system" fn(pThis: *mut AMFComponent, ppCaps: *mut *mut c_void) -> AMF_RESULT,
    pub Optimize: unsafe extern "system" fn(pThis: *mut AMFComponent, pCallback: *mut c_void) -> AMF_RESULT,
}

// ---------------------------------------------------------------------------
// AMFFactory
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct AMFFactory {
    pub pVtbl: *const AMFFactoryVtbl,
}

#[repr(C)]
pub struct AMFFactoryVtbl {
    pub CreateContext: unsafe extern "system" fn(
        pThis: *mut AMFFactory,
        ppContext: *mut *mut AMFContext,
    ) -> AMF_RESULT,
    pub CreateComponent: unsafe extern "system" fn(
        pThis: *mut AMFFactory,
        pContext: *mut AMFContext,
        id: *const u16,
        ppComponent: *mut *mut AMFComponent,
    ) -> AMF_RESULT,
    pub SetCacheFolder: unsafe extern "system" fn(pThis: *mut AMFFactory, path: *const u16) -> AMF_RESULT,
    pub GetCacheFolder: unsafe extern "system" fn(pThis: *mut AMFFactory) -> *const u16,
    pub GetDebug: unsafe extern "system" fn(pThis: *mut AMFFactory, ppDebug: *mut *mut c_void) -> AMF_RESULT,
    pub GetTrace: unsafe extern "system" fn(pThis: *mut AMFFactory, ppTrace: *mut *mut c_void) -> AMF_RESULT,
    pub GetPrograms: unsafe extern "system" fn(
        pThis: *mut AMFFactory,
        ppPrograms: *mut *mut c_void,
    ) -> AMF_RESULT,
}

// AMF runtime DLL entry points
pub type AMFInitFn = unsafe extern "system" fn(
    version: u64,
    factory: *mut *mut AMFFactory,
) -> AMF_RESULT;

pub type AMFQueryVersionFn = unsafe extern "system" fn(
    version: *mut u64,
) -> AMF_RESULT;

// ---------------------------------------------------------------------------
// AMF Component IDs (UTF-16 null-terminated)
// ---------------------------------------------------------------------------

pub const AMFVideoEncoderVCE_AVC: &[u16] = &[
    b'A' as u16, b'M' as u16, b'F' as u16, b'V' as u16, b'i' as u16, b'd' as u16, b'e' as u16, b'o' as u16,
    b'E' as u16, b'n' as u16, b'c' as u16, b'o' as u16, b'd' as u16, b'e' as u16, b'r' as u16, b'V' as u16,
    b'C' as u16, b'E' as u16, b'_' as u16, b'A' as u16, b'V' as u16, b'C' as u16, 0,
];

pub const AMFVideoEncoder_HEVC: &[u16] = &[
    b'A' as u16, b'M' as u16, b'F' as u16, b'V' as u16, b'i' as u16, b'd' as u16, b'e' as u16, b'o' as u16,
    b'E' as u16, b'n' as u16, b'c' as u16, b'o' as u16, b'd' as u16, b'e' as u16, b'r' as u16, b'_' as u16,
    b'H' as u16, b'E' as u16, b'V' as u16, b'C' as u16, 0,
];

// ---------------------------------------------------------------------------
// Encoder Property Names (UTF-16 null-terminated)
// ---------------------------------------------------------------------------

// Common / H.264 properties
pub const AMF_VIDEO_ENCODER_USAGE: &[u16] = &[
    b'U' as u16, b's' as u16, b'a' as u16, b'g' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_PROFILE: &[u16] = &[
    b'P' as u16, b'r' as u16, b'o' as u16, b'f' as u16, b'i' as u16, b'l' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_PROFILE_LEVEL: &[u16] = &[
    b'P' as u16, b'r' as u16, b'o' as u16, b'f' as u16, b'i' as u16, b'l' as u16, b'e' as u16,
    b'L' as u16, b'e' as u16, b'v' as u16, b'e' as u16, b'l' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD: &[u16] = &[
    b'R' as u16, b'a' as u16, b't' as u16, b'e' as u16, b'C' as u16, b'o' as u16, b'n' as u16, b't' as u16,
    b'r' as u16, b'o' as u16, b'l' as u16, b'M' as u16, b'e' as u16, b't' as u16, b'h' as u16, b'o' as u16,
    b'd' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_TARGET_BITRATE: &[u16] = &[
    b'T' as u16, b'a' as u16, b'r' as u16, b'g' as u16, b'e' as u16, b't' as u16, b'B' as u16, b'i' as u16,
    b't' as u16, b'r' as u16, b'a' as u16, b't' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_PEAK_BITRATE: &[u16] = &[
    b'P' as u16, b'e' as u16, b'a' as u16, b'k' as u16, b'B' as u16, b'i' as u16, b't' as u16, b'r' as u16,
    b'a' as u16, b't' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_FRAMESIZE: &[u16] = &[
    b'F' as u16, b'r' as u16, b'a' as u16, b'm' as u16, b'e' as u16, b'S' as u16, b'i' as u16, b'z' as u16,
    b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_FRAMERATE: &[u16] = &[
    b'F' as u16, b'r' as u16, b'a' as u16, b'm' as u16, b'e' as u16, b'R' as u16, b'a' as u16, b't' as u16,
    b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_IDR_PERIOD_SIZE: &[u16] = &[
    b'I' as u16, b'D' as u16, b'R' as u16, b'P' as u16, b'e' as u16, b'r' as u16, b'i' as u16, b'o' as u16,
    b'd' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_GOP_SIZE: &[u16] = &[
    b'G' as u16, b'O' as u16, b'P' as u16, b'S' as u16, b'i' as u16, b'z' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_B_PIC_PATTERN: &[u16] = &[
    b'B' as u16, b'P' as u16, b'i' as u16, b'c' as u16, b't' as u16, b'u' as u16, b'r' as u16, b'e' as u16,
    b's' as u16, b'P' as u16, b'a' as u16, b't' as u16, b't' as u16, b'e' as u16, b'r' as u16, b'n' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_QUALITY_PRESET: &[u16] = &[
    b'Q' as u16, b'u' as u16, b'a' as u16, b'l' as u16, b'i' as u16, b't' as u16, b'y' as u16,
    b'P' as u16, b'r' as u16, b'e' as u16, b's' as u16, b'e' as u16, b't' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEADER_INSERTION_SPACING: &[u16] = &[
    b'H' as u16, b'e' as u16, b'a' as u16, b'd' as u16, b'e' as u16, b'r' as u16,
    b'I' as u16, b'n' as u16, b's' as u16, b'e' as u16, b'r' as u16, b't' as u16, b'i' as u16, b'o' as u16,
    b'n' as u16, b'S' as u16, b'p' as u16, b'a' as u16, b'c' as u16, b'i' as u16, b'n' as u16, b'g' as u16, 0,
];

// HEVC properties
pub const AMF_VIDEO_ENCODER_HEVC_USAGE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'U' as u16, b's' as u16, b'a' as u16, b'g' as u16,
    b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_PROFILE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'P' as u16, b'r' as u16, b'o' as u16, b'f' as u16,
    b'i' as u16, b'l' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_TIER: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'T' as u16, b'i' as u16, b'e' as u16, b'r' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_PROFILE_LEVEL: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'P' as u16, b'r' as u16, b'o' as u16, b'f' as u16,
    b'i' as u16, b'l' as u16, b'e' as u16, b'L' as u16, b'e' as u16, b'v' as u16, b'e' as u16, b'l' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_RATE_CONTROL_METHOD: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'R' as u16, b'a' as u16, b't' as u16, b'e' as u16,
    b'C' as u16, b'o' as u16, b'n' as u16, b't' as u16, b'r' as u16, b'o' as u16, b'l' as u16, b'M' as u16,
    b'e' as u16, b't' as u16, b'h' as u16, b'o' as u16, b'd' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_TARGET_BITRATE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'T' as u16, b'a' as u16, b'r' as u16, b'g' as u16,
    b'e' as u16, b't' as u16, b'B' as u16, b'i' as u16, b't' as u16, b'r' as u16, b'a' as u16, b't' as u16,
    b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_PEAK_BITRATE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'P' as u16, b'e' as u16, b'a' as u16, b'k' as u16,
    b'B' as u16, b'i' as u16, b't' as u16, b'r' as u16, b'a' as u16, b't' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_FRAMESIZE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'F' as u16, b'r' as u16, b'a' as u16, b'm' as u16,
    b'e' as u16, b'S' as u16, b'i' as u16, b'z' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_FRAMERATE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'F' as u16, b'r' as u16, b'a' as u16, b'm' as u16,
    b'e' as u16, b'R' as u16, b'a' as u16, b't' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_GOP_SIZE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'G' as u16, b'O' as u16, b'P' as u16, b'S' as u16,
    b'i' as u16, b'z' as u16, b'e' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_NUM_GOPS_PER_IDR: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'N' as u16, b'u' as u16, b'm' as u16, b'G' as u16,
    b'O' as u16, b'P' as u16, b's' as u16, b'P' as u16, b'e' as u16, b'r' as u16, b'I' as u16, b'D' as u16,
    b'R' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_QUALITY_PRESET: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'Q' as u16, b'u' as u16, b'a' as u16, b'l' as u16,
    b'i' as u16, b't' as u16, b'y' as u16, b'P' as u16, b'r' as u16, b'e' as u16, b's' as u16, b'e' as u16,
    b't' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_COLOR_BIT_DEPTH: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'C' as u16, b'o' as u16, b'l' as u16, b'o' as u16,
    b'r' as u16, b'B' as u16, b'i' as u16, b't' as u16, b'D' as u16, b'e' as u16, b'p' as u16, b't' as u16,
    b'h' as u16, 0,
];
pub const AMF_VIDEO_ENCODER_HEVC_NOMINAL_RANGE: &[u16] = &[
    b'H' as u16, b'e' as u16, b'v' as u16, b'c' as u16, b'N' as u16, b'o' as u16, b'm' as u16, b'i' as u16,
    b'n' as u16, b'a' as u16, b'l' as u16, b'R' as u16, b'a' as u16, b'n' as u16, b'g' as u16, b'e' as u16, 0,
];

// ---------------------------------------------------------------------------
// Rate Control, Presets & Profiles
// ---------------------------------------------------------------------------

pub const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_UNKNOWN: i64 = 0;
pub const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CONSTRAINED_QP: i64 = 1;
pub const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_CBR: i64 = 2;
pub const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_PEAK_CONSTRAINED_VBR: i64 = 3;
pub const AMF_VIDEO_ENCODER_RATE_CONTROL_METHOD_LATENCY_CONSTRAINED_VBR: i64 = 4;

pub const AMF_VIDEO_ENCODER_USAGE_TRANSCODING: i64 = 0;
pub const AMF_VIDEO_ENCODER_USAGE_ULTRA_LOW_LATENCY: i64 = 1;
pub const AMF_VIDEO_ENCODER_USAGE_LOW_LATENCY: i64 = 2;
pub const AMF_VIDEO_ENCODER_USAGE_WEBCAM: i64 = 3;

pub const AMF_VIDEO_ENCODER_QUALITY_PRESET_BALANCED: i64 = 0;
pub const AMF_VIDEO_ENCODER_QUALITY_PRESET_SPEED: i64 = 1;
pub const AMF_VIDEO_ENCODER_QUALITY_PRESET_QUALITY: i64 = 2;

pub const AMF_VIDEO_ENCODER_PROFILE_BASELINE: i64 = 66;
pub const AMF_VIDEO_ENCODER_PROFILE_MAIN: i64 = 77;
pub const AMF_VIDEO_ENCODER_PROFILE_HIGH: i64 = 100;

pub const AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN: i64 = 1;
pub const AMF_VIDEO_ENCODER_HEVC_PROFILE_MAIN_10: i64 = 2;

pub const AMF_COLOR_BIT_DEPTH_8: i64 = 8;
pub const AMF_COLOR_BIT_DEPTH_10: i64 = 10;
