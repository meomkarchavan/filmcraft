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

pub use export::{amf_encoder_factory, AmfVideoEncoder};
