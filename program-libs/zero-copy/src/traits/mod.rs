#[cfg(feature = "alloc")]
mod vec_u8;
mod zero_copy_at;
mod zero_copy_at_mut;
#[cfg(feature = "alloc")]
mod zero_copy_new;

#[cfg(feature = "alloc")]
pub use vec_u8::VecU8;
pub use zero_copy_at::{borsh_vec_u8_as_slice, ZeroCopyAt, ZeroCopyStructInner};
pub use zero_copy_at_mut::{borsh_vec_u8_as_slice_mut, ZeroCopyAtMut, ZeroCopyStructInnerMut};
#[cfg(feature = "alloc")]
pub use zero_copy_new::ZeroCopyNew;
