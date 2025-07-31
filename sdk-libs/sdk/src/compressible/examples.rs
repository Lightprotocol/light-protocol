//! Examples of how to use compressible accounts with both regular and zero-copy approaches

#[cfg(feature = "anchor")]
use anchor_lang::prelude::*;
use crate::compressible::compression_info::{CompressionInfo, CompressionState, HasCompressionInfo, ZeroCopyCompressionInfo, HasZeroCopyCompressionInfo};

/// Regular compressible account using Option<CompressionInfo>
#[cfg(feature = "anchor")]
#[derive(Default, AnchorSerialize, AnchorDeserialize)]
pub struct RegularCompressibleAccount {
    pub owner: Pubkey,
    pub data: u64,
    pub compression_info: Option<CompressionInfo>,
}

#[cfg(feature = "anchor")]
impl HasCompressionInfo for RegularCompressibleAccount {
    fn compression_info(&self) -> &CompressionInfo {
        static DEFAULT: CompressionInfo = CompressionInfo {
            last_written_slot: 0,
            state: CompressionState::Uninitialized,
        };
        self.compression_info.as_ref().unwrap_or(&DEFAULT)
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info.get_or_insert_with(CompressionInfo::default)
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

/// Zero-copy compressible account using ZeroCopyCompressionInfo
/// This can be used with Anchor's zero-copy derive
#[cfg(feature = "anchor")]
#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    AnchorSerialize,
    AnchorDeserialize,
    // Add zero-copy derives when available:
    // light_zero_copy_derive::ZeroCopy,
    // zerocopy::FromBytes,
    // zerocopy::IntoBytes,
    // zerocopy::KnownLayout,
    // zerocopy::Immutable,
    // zerocopy::Unaligned,
)]
pub struct ZeroCopyCompressibleAccount {
    pub owner: Pubkey,
    pub data: u64,
    pub compression_info: ZeroCopyCompressionInfo,
    pub _padding: [u8; 0], // Adjust padding as needed for alignment
}

#[cfg(feature = "anchor")]
impl Default for ZeroCopyCompressibleAccount {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            data: 0,
            compression_info: ZeroCopyCompressionInfo::none(),
            _padding: [],
        }
    }
}

#[cfg(feature = "anchor")]
impl HasZeroCopyCompressionInfo for ZeroCopyCompressibleAccount {
    fn zero_copy_compression_info(&self) -> &ZeroCopyCompressionInfo {
        &self.compression_info
    }

    fn zero_copy_compression_info_mut(&mut self) -> &mut ZeroCopyCompressionInfo {
        &mut self.compression_info
    }

    fn set_zero_copy_compression_info_none(&mut self) {
        self.compression_info.set_none();
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for ZeroCopyCompressibleAccount {
    const INIT_SPACE: usize = 32 + 8 + ZeroCopyCompressionInfo::INIT_SPACE + 0; // owner + data + compression_info + padding
}

/// Example usage showing how both types can work
#[cfg(feature = "anchor")]
pub fn example_usage() {
    // Regular account
    let mut regular_account = RegularCompressibleAccount::default();
    
    // Initialize compression info
    *regular_account.compression_info_mut_opt() = Some(CompressionInfo::new_decompressed().unwrap());
    
    // Set to none for compression
    regular_account.set_compression_info_none();
    
    // Zero-copy account
    let mut zero_copy_account = ZeroCopyCompressibleAccount::default();
    
    // Initialize compression info (secure way)
    zero_copy_account.zero_copy_compression_info_mut()
        .set_some_decompressed().unwrap();
    
    // Set to none for compression (secure way)
    zero_copy_account.set_zero_copy_compression_info_none();
    
    // Both can be used with the same compression functions by implementing appropriate traits
    println!("Regular account compression info present: {}", 
             regular_account.compression_info_mut_opt().is_some());
    
    // Use safe method to check presence
    println!("Zero-copy account compression info present: {}", 
             zero_copy_account.zero_copy_compression_info().is_some_unchecked());
    
    // Or use validated method for critical operations
    match zero_copy_account.zero_copy_compression_info().is_some() {
        Ok(present) => println!("Zero-copy account validated presence: {}", present),
        Err(err) => println!("Validation failed: {:?}", err),
    }
}

/// Example showing how to convert between the two formats safely
#[cfg(feature = "anchor")]
pub fn conversion_example() {
    let regular_info = Some(CompressionInfo::new_decompressed().unwrap());
    
    // Safe conversion to zero-copy format
    let zero_copy_info = ZeroCopyCompressionInfo::from_option(regular_info);
    
    // Validate the conversion result
    assert!(zero_copy_info.validate().is_ok());
    
    // Safe conversion back to regular format
    let back_to_regular = zero_copy_info.to_option().unwrap();
    
    println!("Conversion successful: {}", back_to_regular.is_some());
}

/// Example showing security validation
#[cfg(feature = "anchor")]
pub fn security_example() {
    // Create a valid zero-copy compression info
    let mut info = ZeroCopyCompressionInfo::from_option(Some(CompressionInfo {
        last_written_slot: 12345,
        state: CompressionState::Decompressed,
    }));
    
    // Validate it's secure
    match info.validate() {
        Ok(()) => println!("Info is valid and secure"),
        Err(err) => println!("Security validation failed: {:?}", err),
    }
    
    // Safe operations
    match info.last_written_slot() {
        Ok(slot) => println!("Last written slot: {}", slot),
        Err(err) => println!("Failed to get slot: {:?}", err),
    }
    
    // Set to compressed state safely
    match info.set_compressed() {
        Ok(()) => println!("Successfully set to compressed"),
        Err(err) => println!("Failed to set compressed: {:?}", err),
    }
    
    // For performance-critical paths, use unchecked methods when you're certain the state is valid
    if info.is_compressed_unchecked() {
        println!("Account is compressed (unchecked)");
    }
}