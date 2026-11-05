//! AMD Advanced Media Framework (AMF) hardware encoding support.

#[allow(unsafe_code)]
pub(crate) mod device;
#[allow(unsafe_code)]
pub(crate) mod ffi;

#[cfg(test)]
mod abi_tests;
