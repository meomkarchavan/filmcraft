//! AMD Advanced Media Framework (AMF) hardware encoding support.

#[allow(unsafe_code)]
pub mod device;
#[allow(unsafe_code)]
pub mod ffi;
#[allow(unsafe_code)]
pub mod session;

#[cfg(test)]
mod abi_tests;
