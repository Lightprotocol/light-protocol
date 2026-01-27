//! Trait definitions for manual Light Protocol implementation.

pub mod light_account;
pub mod variant;

pub use light_account::{AccountType, LightAccount};
pub use variant::{LightAccountVariantTrait, PackedLightAccountVariantTrait};
