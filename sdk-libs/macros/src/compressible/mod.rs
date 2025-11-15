//! Compressible account macro generation.
//!
//! This module contains all macro generation logic for compressible accounts:
//! - Core trait implementations (HasCompressionInfo, CompressAs, Size, etc.)
//! - Pack/Unpack implementations for Pubkey compression
//! - Variant enum generation (CompressedAccountVariant)
//! - Decompress context trait implementation
//! - Complete instruction generation (compress/decompress)
//!- Seed provider implementations (PdaSeedProvider, CTokenSeedProvider)

pub mod decompress_context;
pub mod instructions;
pub mod pack_unpack;
pub mod seed_providers;
pub mod traits;
pub mod variant_enum;
