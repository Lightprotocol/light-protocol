//! # light-indexed-array
//!
//! Indexed array for indexed Merkle trees. Stores elements as
//! a sorted linked list with index, value, and next-index pointers.
//!
//! | Type | Description |
//! |------|-------------|
//! | [`array::IndexedElement`] | Element with index, BigUint value, and next-index |
//! | [`array::IndexedArray`] | Array of indexed elements with insert and lookup |
//! | [`changelog`] | Raw indexed element and changelog entry types |
//! | [`errors`] | `IndexedArrayError` variants |

pub mod array;
pub mod changelog;
pub mod errors;

pub const HIGHEST_ADDRESS_PLUS_ONE: &str =
    "452312848583266388373324160190187140051835877600158453279131187530910662655";
