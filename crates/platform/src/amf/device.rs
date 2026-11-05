//! AMD Advanced Media Framework (AMF) dynamic runtime loader and Direct3D 11 device integration.
//!
//! FFI module (docs/adr/0001-platform-ffi.md): every `unsafe` block has a `// SAFETY:` comment,
//! public APIs are safe and return `Result<_, String>`, raw pointers are managed via RAII.

#![allow(dead_code)]

use std::ffi::c_void;
use std::sync::Arc;

use windows::Win32::Foundation::{FreeLibrary, HMODULE, RPC_E_CHANGED_MODE};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_FLAG, D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
    D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device, ID3D11Multithread,
};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::core::{Interface, PCSTR, w};

use super::ffi::{
    AMFContext, AMFFactory, AMFInitFn, AMFQueryVersionFn, AMF_ALREADY_INITIALIZED, AMF_DX11_0,
    AMF_FULL_VERSION, AMF_OK,
};

/// AMD's PCI vendor ID (ATI / AMD).
const AMD_VENDOR_ID: u32 = 0x1002;

fn ensure_com() -> Result<(), String> {
    // SAFETY: CoInitializeEx has no pointer arguments; calling it again on an already-initialized
    // COM thread (e.g., in another apartment model) returns RPC_E_CHANGED_MODE, which is allowed.
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if hr.is_ok() || hr == RPC_E_CHANGED_MODE {
        Ok(())
    } else {
        Err(format!("COM initialisation failed: {hr:?}"))
    }
}

struct AmfRuntimeInner {
    module: HMODULE,
    init_fn: AMFInitFn,
    query_version_fn: Option<AMFQueryVersionFn>,
    version: u64,
}

// SAFETY: AmfRuntimeInner holds raw module handle and function pointers that can safely be used across threads.
unsafe impl Send for AmfRuntimeInner {}
// SAFETY: Function pointers and immutable runtime DLL handle are safe to access across threads.
unsafe impl Sync for AmfRuntimeInner {}

impl Drop for AmfRuntimeInner {
    fn drop(&mut self) {
        if !self.module.is_invalid() {
            // SAFETY: self.module was obtained from LoadLibraryExW and has not yet been freed.
            unsafe {
                let _ = FreeLibrary(self.module);
            }
        }
    }
}

/// Safe handle to the dynamically loaded AMD AMF runtime DLL (`amfrt64.dll`).
#[derive(Clone)]
pub struct AmfRuntime {
    inner: Arc<AmfRuntimeInner>,
}

impl AmfRuntime {
    /// Dynamically loads `amfrt64.dll` from the Windows system32 directory and resolves AMF symbols.
    pub fn load() -> Result<Self, String> {
        // SAFETY: Loads amfrt64.dll from the system directory only.
        let module: HMODULE = unsafe {
            LoadLibraryExW(w!("amfrt64.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
        }
        .map_err(|e| format!("AMD AMF runtime (amfrt64.dll) not found: {e}"))?;

        // SAFETY: GetProcAddress is called with valid module handle and null-terminated symbol names.
        let (init_fn, query_version_fn, version) = unsafe {
            let init_proc = match GetProcAddress(module, PCSTR(c"AMFInit".as_ptr().cast())) {
                Some(proc) => proc,
                None => {
                    let _ = FreeLibrary(module);
                    return Err("amfrt64.dll does not export AMFInit".into());
                }
            };
            let init_fn: AMFInitFn = std::mem::transmute(init_proc);

            let query_proc = GetProcAddress(module, PCSTR(c"AMFQueryVersion".as_ptr().cast()));
            let query_version_fn: Option<AMFQueryVersionFn> = query_proc.map(|p| std::mem::transmute(p));

            let mut ver = 0u64;
            if let Some(q_fn) = query_version_fn {
                let res = q_fn(&mut ver);
                if res != AMF_OK {
                    ver = 0;
                }
            }

            (init_fn, query_version_fn, ver)
        };

        Ok(Self {
            inner: Arc::new(AmfRuntimeInner {
                module,
                init_fn,
                query_version_fn,
                version,
            }),
        })
    }

    /// The runtime AMF version reported by `AMFQueryVersion`, or 0 if query was unavailable.
    pub fn version(&self) -> u64 {
        self.inner.version
    }

    /// Initializes the AMF factory singleton via `AMFInit`.
    pub fn create_factory(&self) -> Result<AmfFactory, String> {
        let mut factory: *mut AMFFactory = std::ptr::null_mut();
        // SAFETY: self.inner.init_fn is a valid AMFInitFn pointer, and factory is a valid out-pointer.
        let mut res = unsafe { (self.inner.init_fn)(AMF_FULL_VERSION, &mut factory) };
        if res != AMF_OK
            && res != AMF_ALREADY_INITIALIZED
            && self.inner.version != 0
            && self.inner.version != AMF_FULL_VERSION
        {
            // Fallback: try using the exact version returned by AMFQueryVersion
            // SAFETY: self.inner.init_fn is a valid AMFInitFn pointer, and factory is a valid out-pointer.
            res = unsafe { (self.inner.init_fn)(self.inner.version, &mut factory) };
        }
        if (res != AMF_OK && res != AMF_ALREADY_INITIALIZED) || factory.is_null() {
            return Err(format!("AMFInit failed with result code {res}"));
        }

        Ok(AmfFactory {
            raw: factory,
            _runtime: self.clone(),
        })
    }
}

/// Safe wrapper around an `AMFFactory` pointer.
///
/// Note: In the AMD AMF architecture, `AMFFactory` is a process-wide singleton managed by
/// `amfrt64.dll`. Unlike other AMF objects, it does not inherit from `AMFInterface` (it has no
/// `Release` method in its vtable) and its lifetime is tied to the loaded runtime DLL held by
/// `_runtime`.
pub struct AmfFactory {
    raw: *mut AMFFactory,
    _runtime: AmfRuntime,
}

// SAFETY: AMFFactory is thread-safe per AMF specification.
unsafe impl Send for AmfFactory {}
// SAFETY: AMFFactory is thread-safe per AMF specification.
unsafe impl Sync for AmfFactory {}

impl AmfFactory {
    /// Returns the raw pointer to the underlying `AMFFactory`.
    pub fn as_raw(&self) -> *mut AMFFactory {
        self.raw
    }

    /// Returns a reference to the loaded `AmfRuntime`.
    pub fn runtime(&self) -> &AmfRuntime {
        &self._runtime
    }

    /// Creates a new uninitialized AMF context.
    pub fn create_context(&self) -> Result<AmfContext, String> {
        if self.raw.is_null() {
            return Err("AMFFactory pointer is null".into());
        }
        let mut context: *mut AMFContext = std::ptr::null_mut();
        // SAFETY: self.raw is a valid AMFFactory pointer, and context is a valid out-pointer.
        let res = unsafe {
            let vtbl = &*(*self.raw).pVtbl;
            (vtbl.CreateContext)(self.raw, &mut context)
        };
        if res != AMF_OK || context.is_null() {
            return Err(format!("AMFFactory::CreateContext failed with result code {res}"));
        }

        Ok(AmfContext {
            raw: context,
            _runtime: self._runtime.clone(),
        })
    }
}

/// Safe RAII wrapper around an `AMFContext` pointer.
/// Terminates and releases the context when dropped.
pub struct AmfContext {
    raw: *mut AMFContext,
    _runtime: AmfRuntime,
}

// SAFETY: AMFContext is thread-safe per AMF specification.
unsafe impl Send for AmfContext {}
// SAFETY: AMFContext is thread-safe per AMF specification.
unsafe impl Sync for AmfContext {}

impl AmfContext {
    /// Returns the raw pointer to the underlying `AMFContext`.
    pub fn as_raw(&self) -> *mut AMFContext {
        self.raw
    }

    /// Returns a reference to the loaded `AmfRuntime`.
    pub fn runtime(&self) -> &AmfRuntime {
        &self._runtime
    }

    /// Initializes the AMF context with a Direct3D 11 device handle.
    ///
    /// # Safety
    /// Caller must ensure `d3d11_device` is a valid Direct3D 11 device handle.
    pub(crate) unsafe fn init_dx11(&self, d3d11_device: *mut c_void, dx_version: u32) -> Result<(), String> {
        if self.raw.is_null() {
            return Err("AMFContext pointer is null".into());
        }
        // SAFETY: self.raw is a valid AMFContext pointer; d3d11_device is a valid Direct3D 11 device handle.
        let res = unsafe {
            let vtbl = &*(*self.raw).pVtbl;
            (vtbl.InitDX11)(self.raw, d3d11_device, dx_version)
        };
        if res != AMF_OK {
            return Err(format!("AMFContext::InitDX11 failed with result code {res}"));
        }
        Ok(())
    }

    /// Locks the DX11 context for thread-safe access and returns an RAII guard that unlocks it on drop.
    pub fn lock_dx11<'a>(&'a self) -> Result<AmfContextDx11Guard<'a>, String> {
        if self.raw.is_null() {
            return Err("AMFContext pointer is null".into());
        }
        // SAFETY: self.raw is a valid AMFContext pointer.
        let res = unsafe {
            let vtbl = &*(*self.raw).pVtbl;
            (vtbl.LockDX11)(self.raw)
        };
        if res != AMF_OK {
            return Err(format!("AMFContext::LockDX11 failed with result code {res}"));
        }
        Ok(AmfContextDx11Guard { context: self })
    }

    /// Unlocks the DX11 context.
    pub fn unlock_dx11(&self) -> Result<(), String> {
        if self.raw.is_null() {
            return Err("AMFContext pointer is null".into());
        }
        // SAFETY: self.raw is a valid AMFContext pointer.
        let res = unsafe {
            let vtbl = &*(*self.raw).pVtbl;
            (vtbl.UnlockDX11)(self.raw)
        };
        if res != AMF_OK {
            return Err(format!("AMFContext::UnlockDX11 failed with result code {res}"));
        }
        Ok(())
    }
}

/// RAII guard for a locked AMF Direct3D 11 context.
/// Unlocks the DX11 context when dropped.
#[must_use = "if unused the AmfContext will immediately unlock"]
pub struct AmfContextDx11Guard<'a> {
    context: &'a AmfContext,
}

impl<'a> Drop for AmfContextDx11Guard<'a> {
    fn drop(&mut self) {
        let _ = self.context.unlock_dx11();
    }
}

impl<'a> std::ops::Deref for AmfContextDx11Guard<'a> {
    type Target = AmfContext;

    fn deref(&self) -> &Self::Target {
        self.context
    }
}

impl Drop for AmfContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: self.raw is a valid AMFContext pointer. Calling Terminate and Release frees the native context.
            unsafe {
                let vtbl = &*(*self.raw).pVtbl;
                let _ = (vtbl.Terminate)(self.raw);
                let _ = (vtbl.Release)(self.raw);
            }
            self.raw = std::ptr::null_mut();
        }
    }
}

/// Direct3D 11 helper: enumerates DXGI adapters and creates a device on the first AMD GPU.
fn create_amd_adapter_device() -> Result<ID3D11Device, String> {
    // SAFETY: CreateDXGIFactory1 creates a valid DXGI 1.1 factory COM interface.
    let factory: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1() }.map_err(|e| format!("no DXGI factory: {e}"))?;

    let mut i = 0;
    // SAFETY: factory is a valid DXGI factory COM interface; EnumAdapters1 enumerates adapters.
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(i) } {
        i += 1;
        // SAFETY: adapter is a valid DXGI adapter COM interface; GetDesc1 retrieves adapter description.
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        let name = String::from_utf16_lossy(&desc.Description);
        println!("Checking adapter: {name}, VendorId=0x{:x}", desc.VendorId);
        if desc.VendorId != AMD_VENDOR_ID {
            continue;
        }
        let adapter: IDXGIAdapter = adapter.cast().map_err(|e| e.to_string())?;
        let mut device = None;
        let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT;
        // SAFETY: D3D11CreateDevice creates a D3D11 device on the specified AMD adapter.
        let mut hr = unsafe {
            D3D11CreateDevice(
                &adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                flags,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        };
        if hr.is_err() {
            // SAFETY: Fallback: retry creating device without video support flag if driver rejected it.
            hr = unsafe {
                D3D11CreateDevice(
                    &adapter,
                    D3D_DRIVER_TYPE_UNKNOWN,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    None,
                    None,
                )
            };
        }
        if let Some(dev) = device {
            let mt: ID3D11Multithread = dev
                .cast()
                .map_err(|e| format!("Direct3D 11 device does not support ID3D11Multithread: {e}"))?;
            // SAFETY: COM call on live ID3D11Multithread interface to enable multithread protection.
            unsafe {
                let _ = mt.SetMultithreadProtected(true);
            }
            return Ok(dev);
        }
    }
    Err("No AMD adapter found".into())
}

/// Direct3D 11 helper: creates a device on the default hardware adapter.
fn create_default_d3d11_device() -> Result<ID3D11Device, String> {
    let mut device = None;
    // SAFETY: Uses default hardware adapter with valid out-pointers.
    let mut hr = unsafe {
        D3D11CreateDevice(
            None::<&IDXGIAdapter>,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
    };
    if hr.is_err() {
        // SAFETY: Fallback: retry creating device without video support flag if driver rejected it.
        hr = unsafe {
            D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        };
    }
    let dev = device.ok_or_else(|| format!("Failed to create Direct3D 11 device: {hr:?}"))?;
    let mt: ID3D11Multithread = dev
        .cast()
        .map_err(|e| format!("Direct3D 11 device does not support ID3D11Multithread: {e}"))?;
    // SAFETY: COM call on live ID3D11Multithread interface to enable multithread protection.
    unsafe {
        let _ = mt.SetMultithreadProtected(true);
    }
    Ok(dev)
}

fn create_d3d11_device() -> Result<ID3D11Device, String> {
    ensure_com()?;
    if let Ok(dev) = create_amd_adapter_device() {
        return Ok(dev);
    }
    create_default_d3d11_device()
}

/// AMF Device wrapping an initialized `AmfContext`, `AmfFactory`, and Direct3D 11 device.
pub struct AmfDevice {
    factory: AmfFactory,
    context: AmfContext,
    d3d11_device: ID3D11Device,
}

// SAFETY: AmfDevice wraps thread-safe AMF objects and a multithread-protected Direct3D 11 device.
unsafe impl Send for AmfDevice {}
// SAFETY: AmfDevice wraps thread-safe AMF objects and a multithread-protected Direct3D 11 device.
unsafe impl Sync for AmfDevice {}

impl AmfDevice {
    /// Creates a new `AmfDevice` by initializing a Direct3D 11 device on an AMD GPU adapter
    /// (or default hardware adapter) and binding it to a new AMF context.
    pub fn new(runtime: &AmfRuntime) -> Result<Self, String> {
        let d3d11_device = create_d3d11_device()?;
        Self::with_d3d11_device(runtime, d3d11_device)
    }

    /// Creates a new `AmfDevice` with a caller-provided Direct3D 11 device.
    pub fn with_d3d11_device(runtime: &AmfRuntime, d3d11_device: ID3D11Device) -> Result<Self, String> {
        let multithread: ID3D11Multithread = d3d11_device
            .cast()
            .map_err(|e| format!("Direct3D 11 device does not support ID3D11Multithread: {e}"))?;
        // SAFETY: COM call on live ID3D11Multithread interface to enable multithread protection.
        unsafe {
            let _ = multithread.SetMultithreadProtected(true);
        }

        let factory = runtime.create_factory()?;
        let context = factory.create_context()?;
        // SAFETY: d3d11_device is a valid live ID3D11Device.
        unsafe { context.init_dx11(d3d11_device.as_raw(), AMF_DX11_0)? };
        Ok(Self {
            factory,
            context,
            d3d11_device,
        })
    }

    /// Returns a reference to the `AmfFactory`.
    pub fn factory(&self) -> &AmfFactory {
        &self.factory
    }

    /// Returns a reference to the initialized `AmfContext`.
    pub fn context(&self) -> &AmfContext {
        &self.context
    }

    /// Returns a reference to the `ID3D11Device`.
    pub fn d3d11_device(&self) -> &ID3D11Device {
        &self.d3d11_device
    }

    /// Returns the raw pointer to the `ID3D11Device`.
    pub fn d3d11_device_raw(&self) -> *mut c_void {
        self.d3d11_device.as_raw()
    }

    /// Returns a reference to the loaded `AmfRuntime`.
    pub fn runtime(&self) -> &AmfRuntime {
        &self.factory._runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amf_runtime_probe_and_device() {
        let runtime = match AmfRuntime::load() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("AMF runtime not present on this system: {e}");
                return;
            }
        };

        let ver = runtime.version();
        println!("AMF runtime detected, version: {ver:#x}");

        let factory = runtime.create_factory();
        assert!(factory.is_ok(), "create_factory failed: {:?}", factory.err());
        let factory = factory.unwrap();
        assert!(!factory.as_raw().is_null());

        // Test calling create_factory again in the same process (AMF_ALREADY_INITIALIZED handling)
        let factory2 = runtime.create_factory();
        assert!(
            factory2.is_ok(),
            "second create_factory failed (AMF_ALREADY_INITIALIZED not handled): {:?}",
            factory2.err()
        );

        let context = factory.create_context();
        assert!(context.is_ok(), "create_context failed: {:?}", context.err());
        let context = context.unwrap();
        assert!(!context.as_raw().is_null());

        let device = AmfDevice::new(&runtime);
        assert!(device.is_ok(), "AmfDevice::new failed: {:?}", device.err());
        let device = device.unwrap();
        assert!(!device.context().as_raw().is_null());
        assert!(!device.factory().as_raw().is_null());
        assert!(!device.d3d11_device_raw().is_null());

        // Test RAII DX11 lock guard
        let guard = device.context().lock_dx11();
        assert!(guard.is_ok(), "lock_dx11 failed: {:?}", guard.err());
        drop(guard);
    }
}
