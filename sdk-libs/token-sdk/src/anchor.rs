//! Anchor integration module for Light Protocol compressed tokens.
//!
//! Provides a single import point for Anchor programs using Light Protocol.

// Re-export anchor_lang prelude
pub use anchor_lang::prelude::*;
// Re-export anchor_spl (includes memo and idl-build features from git dependency)
pub use anchor_spl;
// Re-export Light SDK core types
pub use light_sdk::{
    account::LightAccount as LightAccountType,
    address,
    cpi::{v2::CpiAccounts, InvokeLightSystemProgram, LightCpiInstruction},
    derive_light_cpi_signer, derive_light_cpi_signer_pda,
    error::LightSdkError,
    instruction::{PackedAccounts, ValidityProof},
    interface::{
        CompressAs as CompressAsTrait, CompressedInitSpace, CompressionInfo,
        HasCompressionInfo as HasCompressionInfoTrait, LightConfig, LightFinalize, LightPreInit,
        Pack, Space, Unpack,
    },
    CpiSigner, LightDiscriminator as LightDiscriminatorTrait,
};
// Re-export Light SDK macros
pub use light_sdk_macros::{
    // Proc macros
    derive_light_rent_sponsor,
    derive_light_rent_sponsor_pda,
    // Attribute macros
    light_program,
    // Derive macros
    CompressAs,
    Compressible,
    CompressiblePack,
    HasCompressionInfo,
    LightAccount,
    LightAccounts,
    LightDiscriminator,
    LightHasher,
    LightHasherSha,
};

// Re-export token SDK types
pub use crate::{token::*, CompressedProof, ValidityProof as ValidityProofAlias};
