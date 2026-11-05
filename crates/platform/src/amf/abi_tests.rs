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
