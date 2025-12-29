use light_zero_copy::{errors::ZeroCopyError, ZeroCopyNew};

use crate::{
    state::{ExtensionStruct, ExtensionStructConfig},
    BASE_TOKEN_ACCOUNT_SIZE,
};

/// Calculates the size of a ctoken account based on which extensions are present.
///
/// Note: Compression info is now embedded in the base struct (CTokenZeroCopyMeta),
/// so there's no separate compressible extension parameter.
///
/// # Arguments
/// * `extensions` - Optional slice of extension configs
///
/// # Returns
/// * `Ok(usize)` - The total account size in bytes
/// * `Err(ZeroCopyError)` - If extension size calculation fails
pub fn calculate_ctoken_account_size(
    extensions: Option<&[ExtensionStructConfig]>,
) -> Result<usize, ZeroCopyError> {
    let mut size = BASE_TOKEN_ACCOUNT_SIZE as usize;

    if let Some(exts) = extensions {
        if !exts.is_empty() {
            size += 1; // account_type byte at position 165
            size += 4; // Vec length prefix
            for ext in exts {
                size += ExtensionStruct::byte_len(ext)?;
            }
        }
    }

    Ok(size)
}
