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

    #[test]
    fn amf_data_type_constants() {
        assert_eq!(AMF_DATA_BUFFER, 0);
        assert_eq!(AMF_DATA_SURFACE, 1);
        assert_eq!(AMF_DATA_AUDIO_BUFFER, 2);
        assert_eq!(AMF_DATA_USER, 1000);
    }

    #[test]
    fn amf_variant_struct_size_and_alignment() {
        assert_eq!(std::mem::size_of::<AMFVariantStruct>(), 24);
        assert_eq!(std::mem::align_of::<AMFVariantStruct>(), 8);
    }

    #[test]
    fn amf_variant_bool_representation() {
        let var_true = AMFVariantStruct::from_bool(true);
        assert_eq!(var_true.type_, AMF_VARIANT_BOOL);
        assert_eq!(var_true.as_bool(), Some(true));

        let var_false = AMFVariantStruct::from_bool(false);
        assert_eq!(var_false.type_, AMF_VARIANT_BOOL);
        assert_eq!(var_false.as_bool(), Some(false));
    }

    #[test]
    fn amf_vtable_sizes_and_slot_counts() {
        let ptr_size = std::mem::size_of::<usize>();
        assert_eq!(std::mem::size_of::<AMFPropertyStorageVtbl>(), 13 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFPropertyStorageExVtbl>(), 17 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFDataVtbl>(), 23 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFBufferVtbl>(), 28 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFSurfaceVtbl>(), 34 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFContextVtbl>(), 43 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFComponentVtbl>(), 28 * ptr_size);
    }
}
