//! Size trait for compressed accounts.

use crate::error::LightSdkTypesError;

/// Trait to get the serialized size of a compressed account.
pub trait Size {
    fn size(&self) -> Result<usize, LightSdkTypesError>;
}
