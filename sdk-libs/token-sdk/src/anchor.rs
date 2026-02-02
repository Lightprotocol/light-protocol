//! Anchor integration module for Light Protocol compressed tokens.
//!
//! Provides a single import point for Anchor programs using Light Protocol.

// Re-export Light SDK core types
pub use light_account::{
    CompressAs as CompressAsTrait, CompressedInitSpace, CompressionInfo,
    HasCompressionInfo as HasCompressionInfoTrait, LightConfig, LightFinalize, LightPreInit, Space,
    Unpack,
};
#[cfg(not(target_os = "solana"))]
pub use light_account::{Pack, PackedAccounts};
pub use light_sdk::{
    account::LightAccount as LightAccountType,
    address,
    cpi::{v2::CpiAccounts, InvokeLightSystemProgram, LightCpiInstruction},
    derive_light_cpi_signer, derive_light_cpi_signer_pda,
    error::LightSdkError,
    instruction::ValidityProof,
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
    HasCompressionInfo,
    LightAccount,
    LightAccounts,
    LightDiscriminator,
    LightHasher,
    LightHasherSha,
};

// Re-export token SDK types
pub use crate::{instruction::*, CompressedProof, ValidityProof as ValidityProofAlias};
