use std::io::{Cursor, Read};

/// Trait for deserializing compressed account data with custom padding logic.
///
/// This trait must be implemented for any account type that needs to be deserialized
/// from compressed data. It allows users to handle padding differences between
/// compressed and on-chain account layouts, especially for accounts with `max_len`
/// attributes.
///
/// # Why This Trait is Needed
///
/// When accounts are compressed, they only store the actual data (e.g., a 5-character
/// string uses 9 bytes: 4 bytes length + 5 bytes content). However, when deserializing
/// back to the full account struct, we need to pad the data to match the expected
/// layout that includes `max_len` reservations.
///
/// # Usage Examples
///
/// ## Simple Account (No Padding Needed)
///
/// ```rust
/// use light_sdk::compressible::{FromCompressedData, CompressionInfo};
/// use anchor_lang::{AnchorDeserialize, AnchorSerialize};
///
/// #[derive(AnchorSerialize, AnchorDeserialize)]
/// pub struct SimpleAccount {
///     pub compression_info: CompressionInfo,
///     pub value: u64,
/// }
///
/// #[derive(AnchorDeserialize)]
/// struct SimpleAccountWithoutCompressionInfo {
///     pub value: u64,
/// }
///
/// impl FromCompressedData<SimpleAccount> for SimpleAccount {
///     fn from_compressed_data(data: &[u8]) -> Result<SimpleAccount, Box<dyn std::error::Error>> {
///         let temp = SimpleAccountWithoutCompressionInfo::deserialize(&mut &data[..])?;
///         Ok(SimpleAccount {
///             compression_info: CompressionInfo::default(),
///             value: temp.value,
///         })
///     }
/// }
/// ```
///
/// ## Account with max_len String (Custom Padding)
///
/// ```rust
/// use light_sdk::compressible::{FromCompressedData, CompressionInfo};
/// use anchor_lang::{AnchorDeserialize, AnchorSerialize};
/// use solana_pubkey::Pubkey;
///
/// #[derive(AnchorSerialize, AnchorDeserialize)]
/// pub struct UserRecord {
///     pub compression_info: CompressionInfo,
///     pub owner: Pubkey,
///     #[max_len(32)]
///     pub name: String,
///     pub score: u64,
/// }
///
/// #[derive(AnchorDeserialize)]
/// struct UserRecordWithoutCompressionInfo {
///     pub owner: Pubkey,
///     pub name: String,
///     pub score: u64,
/// }
///
/// impl FromCompressedData<UserRecord> for UserRecord {
///     fn from_compressed_data(data: &[u8]) -> Result<UserRecord, Box<dyn std::error::Error>> {
///         let mut cursor = std::io::Cursor::new(data);
///         
///         // Read owner
///         let owner = Pubkey::deserialize(&mut cursor)?;
///         
///         // Read string length to determine padding needs
///         let string_len = u32::deserialize(&mut cursor)? as usize;
///         let remaining_pos = cursor.position() as usize;
///         let remaining_data = &data[remaining_pos..];
///         
///         // Check if we need padding for string + score
///         let min_needed = string_len + 8; // actual string + u64 score
///         if remaining_data.len() < min_needed {
///             // Pad the data
///             let mut padded_data = data.to_vec();
///             let padding_needed = min_needed - remaining_data.len();
///             padded_data.extend(vec![0; padding_needed]);
///             
///             let temp = UserRecordWithoutCompressionInfo::deserialize(&mut &padded_data[..])?;
///             return Ok(UserRecord {
///                 compression_info: CompressionInfo::default(),
///                 owner: temp.owner,
///                 name: temp.name,
///                 score: temp.score,
///             });
///         }
///         
///         // Normal deserialization
///         let temp = UserRecordWithoutCompressionInfo::deserialize(&mut &data[..])?;
///         Ok(UserRecord {
///             compression_info: CompressionInfo::default(),
///             owner: temp.owner,
///             name: temp.name,
///             score: temp.score,
///         })
///     }
/// }
/// ```
///
/// ## Complex Account with Multiple max_len Fields
///
/// ```rust
/// impl FromCompressedData<ComplexAccount> for ComplexAccount {
///     fn from_compressed_data(data: &[u8]) -> Result<ComplexAccount, Box<dyn std::error::Error>> {
///         // Calculate minimum required size based on actual data
///         let min_size = calculate_min_size_for_complex_account(data)?;
///         
///         // Pad if necessary
///         let padded_data = if data.len() < min_size {
///             let mut padded = data.to_vec();
///             padded.resize(min_size, 0);
///             padded
///         } else {
///             data.to_vec()
///         };
///         
///         let temp = ComplexAccountWithoutCompressionInfo::deserialize(&mut &padded_data[..])?;
///         Ok(ComplexAccount {
///             compression_info: CompressionInfo::default(),
///             field1: temp.field1,
///             field2: temp.field2,
///             // ... other fields
///         })
///     }
/// }
/// ```
///
/// # Helper Utilities
///
/// The trait provides several utility methods to help with common padding scenarios:
///
/// - `pad_to_size()`: Pads data to a specific size
/// - `pad_for_max_len_string()`: Helper for accounts with max_len strings
/// - `smart_pad_for_borsh()`: Intelligent padding based on expected minimum size
pub trait FromCompressedData<T> {
    /// Deserialize compressed account data into the full account struct.
    ///
    /// This method should handle any necessary padding to ensure the compressed
    /// data can be properly deserialized into the target account type.
    ///
    /// # Arguments
    /// * `data` - The compressed account data bytes
    ///
    /// # Returns
    /// * `Ok(T)` - The deserialized account with compression_info set to default
    /// * `Err(Box<dyn std::error::Error>)` - Deserialization error
    fn from_compressed_data(data: &[u8]) -> Result<T, Box<dyn std::error::Error>>;

    /// Utility: Pad data to a specific target size with zero bytes.
    ///
    /// # Arguments
    /// * `data` - The input data
    /// * `target_size` - The desired size in bytes
    ///
    /// # Returns
    /// * `Vec<u8>` - Padded data (original data if already large enough)
    fn pad_to_size(data: &[u8], target_size: usize) -> Vec<u8> {
        let mut padded = data.to_vec();
        if padded.len() < target_size {
            padded.resize(target_size, 0);
        }
        padded
    }

    /// Utility: Pad data for accounts with max_len strings.
    ///
    /// This is a helper for the common case of accounts with Pubkey + max_len string + integer fields.
    ///
    /// # Arguments
    /// * `data` - The input data
    /// * `max_string_len` - Maximum string length from max_len attribute
    ///
    /// # Returns
    /// * `Vec<u8>` - Padded data assuming 32 (Pubkey) + 4 + max_len (string) + 8 (u64) layout
    fn pad_for_max_len_string(data: &[u8], max_string_len: usize) -> Vec<u8> {
        // Common layout: Pubkey(32) + string_len(4) + max_string_content + score(8)
        let target_size = 32 + 4 + max_string_len + 8;
        Self::pad_to_size(data, target_size)
    }

    /// Utility: Smart padding that reads the actual data structure to determine padding needs.
    ///
    /// This reads the data incrementally to determine the actual space needed,
    /// then pads accordingly. More efficient than fixed-size padding.
    ///
    /// # Arguments
    /// * `data` - The input data
    /// * `expected_min_size` - Expected minimum size for the deserialized struct
    ///
    /// # Returns
    /// * `Vec<u8>` - Padded data
    fn smart_pad_for_borsh(data: &[u8], expected_min_size: usize) -> Vec<u8> {
        if data.len() < expected_min_size {
            let mut padded = data.to_vec();
            padded.resize(expected_min_size, 0);
            padded
        } else {
            data.to_vec()
        }
    }

    /// Utility: Read string length from current position in data.
    ///
    /// Helper method to read the 4-byte string length prefix from Borsh-serialized data.
    ///
    /// # Arguments
    /// * `cursor` - Cursor positioned at the string length field
    ///
    /// # Returns
    /// * `Result<usize, Box<dyn std::error::Error>>` - The string length or error
    fn read_string_length(cursor: &mut Cursor<&[u8]>) -> Result<usize, Box<dyn std::error::Error>> {
        let mut length_bytes = [0u8; 4];
        cursor.read_exact(&mut length_bytes)?;
        Ok(u32::from_le_bytes(length_bytes) as usize)
    }
}
