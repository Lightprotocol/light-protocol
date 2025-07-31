#[cfg(test)]
mod tests {
    use crate::compressible::compression_info::{CompressionInfo, CompressionState, ZeroCopyCompressionInfo};

    #[test]
    fn test_zero_copy_compression_info_none() {
        let info = ZeroCopyCompressionInfo::none();
        
        // Test validation passes
        assert!(info.validate().is_ok());
        
        // Test state queries
        assert!(info.is_none_unchecked());
        assert!(!info.is_some_unchecked());
        assert!(!info.is_compressed_unchecked());
        assert_eq!(info.last_written_slot_unchecked(), 0);
        assert_eq!(info.to_option_unchecked(), None);
        
        // Test safe queries
        assert!(info.is_none().unwrap());
        assert!(!info.is_some().unwrap());
        assert!(!info.is_compressed().unwrap());
        assert_eq!(info.last_written_slot().unwrap(), 0);
        assert_eq!(info.to_option().unwrap(), None);
    }

    #[test]
    fn test_zero_copy_compression_info_some() {
        // Create valid Some state using safe constructor
        let info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 12345,
            state: CompressionState::Decompressed,
        }));
        
        // Test validation passes
        assert!(info.validate().is_ok());
        
        // Test state queries
        assert!(info.is_some_unchecked());
        assert!(!info.is_none_unchecked());
        assert!(!info.is_compressed_unchecked());
        assert_eq!(info.last_written_slot_unchecked(), 12345);
        
        // Test safe queries
        assert!(info.is_some().unwrap());
        assert!(!info.is_none().unwrap());
        assert!(!info.is_compressed().unwrap());
        assert_eq!(info.last_written_slot().unwrap(), 12345);
        
        // Convert to Option should be Some
        let opt = info.to_option().unwrap();
        assert!(opt.is_some());
        assert_eq!(opt.unwrap().state, CompressionState::Decompressed);
    }

    #[test]
    fn test_zero_copy_compression_info_compressed() {
        // Create valid decompressed state first
        let mut info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 12345,
            state: CompressionState::Decompressed,
        }));
        
        assert!(!info.is_compressed_unchecked());
        
        // Then set to compressed using safe method
        info.set_compressed().unwrap();
        
        // Test validation passes
        assert!(info.validate().is_ok());
        
        // Test state
        assert!(info.is_compressed().unwrap());
        assert!(info.is_some().unwrap());
        
        // Convert to Option should be Some(Compressed)
        let opt = info.to_option().unwrap();
        assert!(opt.is_some());
        assert_eq!(opt.unwrap().state, CompressionState::Compressed);
    }

    #[test]
    fn test_zero_copy_compression_info_set_none() {
        // Create valid Some state first
        let mut info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 12345,
            state: CompressionState::Decompressed,
        }));
        
        assert!(info.is_some_unchecked());
        
        // Set back to none using safe method
        info.set_none();
        
        // Test validation passes
        assert!(info.validate().is_ok());
        
        // Test state
        assert!(!info.is_some().unwrap());
        assert!(info.is_none().unwrap());
        assert_eq!(info.last_written_slot().unwrap(), 0);
    }

    #[test]
    fn test_conversion_from_option_none() {
        let opt: Option<CompressionInfo> = None;
        let zero_copy = ZeroCopyCompressionInfo::from_option(opt);
        
        // Test validation passes
        assert!(zero_copy.validate().is_ok());
        
        assert!(zero_copy.is_none_unchecked());
        assert_eq!(zero_copy.to_option_unchecked(), None);
        
        // Test safe methods
        assert!(zero_copy.is_none().unwrap());
        assert_eq!(zero_copy.to_option().unwrap(), None);
    }

    #[test]
    fn test_conversion_from_option_some() {
        let regular = CompressionInfo {
            last_written_slot: 12345,
            state: CompressionState::Decompressed,
        };
        let opt = Some(regular.clone());
        let zero_copy = ZeroCopyCompressionInfo::from_option(opt);
        
        // Test validation passes
        assert!(zero_copy.validate().is_ok());
        
        assert!(zero_copy.is_some_unchecked());
        assert!(!zero_copy.is_compressed_unchecked());
        assert_eq!(zero_copy.last_written_slot_unchecked(), 12345);
        
        // Test safe methods
        assert!(zero_copy.is_some().unwrap());
        assert!(!zero_copy.is_compressed().unwrap());
        assert_eq!(zero_copy.last_written_slot().unwrap(), 12345);
        
        // Convert back
        let back_to_opt = zero_copy.to_option().unwrap();
        assert_eq!(back_to_opt, Some(regular));
    }

    #[test]
    fn test_conversion_roundtrip() {
        let original = CompressionInfo {
            last_written_slot: 67890,
            state: CompressionState::Compressed,
        };
        
        // Regular -> ZeroCopy -> Regular
        let zero_copy = ZeroCopyCompressionInfo::from_option(Some(original.clone()));
        
        // Test validation passes
        assert!(zero_copy.validate().is_ok());
        
        let back = zero_copy.to_option().unwrap().unwrap();
        
        assert_eq!(original.last_written_slot, back.last_written_slot);
        assert_eq!(original.state, back.state);
    }

    #[test]
    fn test_zero_copy_size() {
        use std::mem::size_of;
        
        // Should be exactly 16 bytes
        assert_eq!(size_of::<ZeroCopyCompressionInfo>(), 16);
        
        // Should be aligned properly
        assert_eq!(std::mem::align_of::<ZeroCopyCompressionInfo>(), 8);
    }

    #[test]
    fn test_zero_copy_default() {
        let info = ZeroCopyCompressionInfo::default();
        
        // Test validation passes
        assert!(info.validate().is_ok());
        
        assert!(info.is_none_unchecked());
        assert_eq!(info.last_written_slot_unchecked(), 0);
        
        // Test safe methods
        assert!(info.is_none().unwrap());
        assert_eq!(info.last_written_slot().unwrap(), 0);
    }

    #[test]
    fn test_compressed_state_only_when_present() {
        let mut info = ZeroCopyCompressionInfo::none();
        
        // Try to set compressed when None - should return error
        assert!(info.set_compressed().is_err());
        assert!(info.is_none_unchecked());
        assert!(!info.is_compressed_unchecked());
        
        // Create valid Some state first, then compressed should work
        info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 12345,
            state: CompressionState::Decompressed,
        }));
        
        assert!(info.set_compressed().is_ok());
        assert!(info.is_compressed().unwrap());
    }
    
    #[test]
    fn test_security_validation() {
        // Test invalid discriminant
        let mut invalid_discriminant = ZeroCopyCompressionInfo::none();
        unsafe {
            let ptr = &mut invalid_discriminant as *mut _ as *mut u8;
            // Set is_present to invalid value (2)
            *ptr.add(9) = 2;
        }
        assert!(invalid_discriminant.validate().is_err());
        
        // Test invalid state
        let mut invalid_state = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 123,
            state: CompressionState::Decompressed,
        }));
        unsafe {
            let ptr = &mut invalid_state as *mut _ as *mut u8;
            // Set state to invalid value (3)
            *ptr.add(8) = 3;
        }
        assert!(invalid_state.validate().is_err());
        
        // Test padding not zero
        let mut invalid_padding = ZeroCopyCompressionInfo::none();
        unsafe {
            let ptr = &mut invalid_padding as *mut _ as *mut u8;
            // Set padding byte to non-zero
            *ptr.add(10) = 0xFF;
        }
        assert!(invalid_padding.validate().is_err());
        
        // Test inconsistent state
        let mut inconsistent = ZeroCopyCompressionInfo::none();
        unsafe {
            let ptr = &mut inconsistent as *mut _ as *mut u64;
            // Set last_written_slot to non-zero while is_present is 0
            *ptr = 12345;
        }
        assert!(inconsistent.validate().is_err());
    }
    
    #[test]
    fn test_safe_deserialization() {
        // Test valid bytes
        let valid_info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 12345,
            state: CompressionState::Decompressed,
        }));
        
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &valid_info as *const _ as *const u8,
                std::mem::size_of::<ZeroCopyCompressionInfo>()
            )
        };
        
        let deserialized = ZeroCopyCompressionInfo::from_bytes(bytes).unwrap();
        assert_eq!(deserialized.last_written_slot().unwrap(), 12345);
        
        // Test invalid bytes (wrong size)
        let short_bytes = &bytes[..8];
        assert!(ZeroCopyCompressionInfo::from_bytes(short_bytes).is_err());
        
        // Test corrupted bytes
        let mut corrupted_bytes = bytes.to_vec();
        corrupted_bytes[9] = 3; // Invalid is_present value
        assert!(ZeroCopyCompressionInfo::from_bytes(&corrupted_bytes).is_err());
    }
    
    #[test]
    fn test_compression_state_getter() {
        // Test None state
        let none_info = ZeroCopyCompressionInfo::none();
        assert_eq!(none_info.compression_state().unwrap(), None);
        
        // Test Decompressed state
        let decompressed_info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 123,
            state: CompressionState::Decompressed,
        }));
        assert_eq!(decompressed_info.compression_state().unwrap(), Some(CompressionState::Decompressed));
        
        // Test Compressed state
        let mut compressed_info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
            last_written_slot: 123,
            state: CompressionState::Decompressed,
        }));
        compressed_info.set_compressed().unwrap();
        assert_eq!(compressed_info.compression_state().unwrap(), Some(CompressionState::Compressed));
    }
}