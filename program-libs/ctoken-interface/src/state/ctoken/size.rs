use crate::{
    state::{ExtensionStruct, ExtensionStructConfig},
    BASE_TOKEN_ACCOUNT_SIZE,
};
use light_zero_copy::ZeroCopyNew;

/// Calculates the size of a ctoken account based on which extensions are present.
///
/// Note: Compression info is now embedded in the base struct (CTokenZeroCopyMeta),
/// so there's no separate compressible extension parameter.
///
/// # Arguments
/// * `extensions` - Optional slice of extension configs
///
/// # Returns
/// The total account size in bytes
pub fn calculate_ctoken_account_size(extensions: Option<&[ExtensionStructConfig]>) -> usize {
    let mut size = BASE_TOKEN_ACCOUNT_SIZE as usize;

    if let Some(exts) = extensions {
        if !exts.is_empty() {
            size += 4; // Vec length prefix
            for ext in exts {
                size += ExtensionStruct::byte_len(ext).unwrap_or(0);
            }
        }
    }

    size
}
