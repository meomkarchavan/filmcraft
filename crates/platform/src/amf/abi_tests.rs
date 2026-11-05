#[cfg(test)]
mod amf_abi_tests {
    use super::super::ffi::*;

    #[test]
    fn amf_version_constants_are_valid() {
        assert_eq!(AMF_FULL_VERSION, ((1u64) << 48) | ((4u64) << 32) | ((33u64) << 16) | 0);
        assert_eq!(AMF_OK, 0);
        assert_eq!(AMF_FAIL, 1);
        assert_eq!(AMF_EOF, 23);
        assert_eq!(AMF_REPEAT, 24);
        assert_eq!(AMF_INPUT_FULL, 25);
    }

    #[test]
    fn amf_data_type_constants() {
        assert_eq!(AMF_DATA_BUFFER, 0);
        assert_eq!(AMF_DATA_SURFACE, 1);
        assert_eq!(AMF_DATA_AUDIO_BUFFER, 2);
        assert_eq!(AMF_DATA_USER, 1000);
    }

    #[test]
    fn amf_structure_sizes_and_alignments() {
        assert_eq!(std::mem::size_of::<AMFGuid>(), 16);
        assert_eq!(std::mem::align_of::<AMFGuid>(), 4);

        assert_eq!(std::mem::size_of::<AMFRate>(), 8);
        assert_eq!(std::mem::align_of::<AMFRate>(), 4);

        assert_eq!(std::mem::size_of::<AMFSize>(), 8);
        assert_eq!(std::mem::align_of::<AMFSize>(), 4);

        assert_eq!(std::mem::size_of::<AMFPoint>(), 8);
        assert_eq!(std::mem::align_of::<AMFPoint>(), 4);

        assert_eq!(std::mem::size_of::<AMFRect>(), 16);
        assert_eq!(std::mem::align_of::<AMFRect>(), 4);

        assert_eq!(std::mem::size_of::<AMFVariantStruct>(), 24);
        assert_eq!(std::mem::align_of::<AMFVariantStruct>(), 8);

        assert_eq!(std::mem::size_of::<AMFVariantValue>(), 16);
        assert_eq!(std::mem::align_of::<AMFVariantValue>(), 8);
    }

    #[test]
    fn amf_variant_bool_representation() {
        let var_true = AMFVariantStruct::from_bool(true);
        assert_eq!(var_true.type_, AMF_VARIANT_BOOL);
        assert_eq!(var_true.as_bool(), Some(true));

        let var_false = AMFVariantStruct::from_bool(false);
        assert_eq!(var_false.type_, AMF_VARIANT_BOOL);
        assert_eq!(var_false.as_bool(), Some(false));

        let empty = AMFVariantStruct::empty();
        assert_eq!(empty.type_, AMF_VARIANT_EMPTY);
        assert_eq!(empty.as_bool(), None);
        assert_eq!(empty.as_int64(), None);

        let int_val = AMFVariantStruct::from_int64(42);
        assert_eq!(int_val.type_, AMF_VARIANT_INT64);
        assert_eq!(int_val.as_int64(), Some(42));
        assert_eq!(int_val.as_bool(), None);
    }

    #[test]
    fn amf_guid_constants_and_layout() {
        let guid = AMFGuid {
            data1: 0x12345678,
            data2: 0x9ABC,
            data3: 0xDEF0,
            data4: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        assert_eq!(guid.data1, 0x12345678);
        assert_eq!(guid.data2, 0x9ABC);
        assert_eq!(guid.data3, 0xDEF0);
        assert_eq!(guid.data4, [1, 2, 3, 4, 5, 6, 7, 8]);

        assert_eq!(IID_AMFPropertyStorage.data1, 0xc7cec05b);
        assert_eq!(IID_AMFPropertyStorageEx.data1, 0x524b07cf);
        assert_eq!(IID_AMFContext.data1, 0xa76a13f0);
        assert_eq!(IID_AMFData.data1, 0xa0ee3387);
        assert_eq!(IID_AMFSurface.data1, 0x3075dbe3);
        assert_eq!(IID_AMFBuffer.data1, 0xb04b7248);
        assert_eq!(IID_AMFComponent.data1, 0x8b776269);
    }

    #[test]
    fn amf_vtable_sizes_and_slot_offsets() {
        let ptr_size = std::mem::size_of::<usize>();
        assert_eq!(std::mem::size_of::<AMFPropertyStorageVtbl>(), 13 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFPropertyStorageExVtbl>(), 17 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFDataVtbl>(), 23 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFBufferVtbl>(), 28 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFPlaneVtbl>(), 13 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFSurfaceVtbl>(), 33 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFContextVtbl>(), 52 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFComponentVtbl>(), 28 * ptr_size);
        assert_eq!(std::mem::size_of::<AMFFactoryVtbl>(), 7 * ptr_size);

        // Verify critical method slot offsets to prevent reordering bugs
        assert_eq!(std::mem::offset_of!(AMFVariantStruct, value), 8);
        assert_eq!(std::mem::offset_of!(AMFContextVtbl, InitDX11), 18 * ptr_size);
        assert_eq!(std::mem::offset_of!(AMFContextVtbl, AllocSurface), 44 * ptr_size);
        assert_eq!(std::mem::offset_of!(AMFContextVtbl, CreateSurfaceFromDX11Native), 47 * ptr_size);
        assert_eq!(std::mem::offset_of!(AMFComponentVtbl, Init), 17 * ptr_size);
        assert_eq!(std::mem::offset_of!(AMFComponentVtbl, SubmitInput), 22 * ptr_size);
        assert_eq!(std::mem::offset_of!(AMFComponentVtbl, QueryOutput), 23 * ptr_size);
        assert_eq!(std::mem::offset_of!(AMFSurfaceVtbl, GetPlaneAt), 25 * ptr_size);
    }

    #[test]
    fn amf_encoder_property_constants() {
        assert_eq!(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_IDR, 0);
        assert_eq!(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_I, 1);
        assert_eq!(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_P, 2);
        assert_eq!(AMF_VIDEO_ENCODER_OUTPUT_DATA_TYPE_B, 3);

        assert_eq!(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_IDR, 0);
        assert_eq!(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_I, 1);
        assert_eq!(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_P, 2);
        assert_eq!(AMF_VIDEO_ENCODER_HEVC_OUTPUT_DATA_TYPE_B, 3);
    }
}
