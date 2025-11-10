use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
#[cfg(feature = "solana")]
use solana_msg::msg;

use crate::{
    instructions::mint_action::CompressedMintInstructionData, state::ExtensionStruct,
    AnchorDeserialize, AnchorSerialize, CTokenError,
};

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize, ZeroCopyMut, ZeroCopy)]
pub struct CompressedMint {
    pub base: BaseMint,
    pub metadata: CompressedMintMetadata,
    pub extensions: Option<Vec<ExtensionStruct>>,
}

// and subsequent deserialization for remaining data (compression metadata + extensions)
/// SPL-compatible base mint structure with padding for COption alignment
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BaseMint {
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: Option<Pubkey>,
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Is initialized - for SPL compatibility
    pub is_initialized: bool,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
}

/// Light Protocol-specific metadata for compressed mints
#[repr(C)]
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorDeserialize, AnchorSerialize, ZeroCopyMut, ZeroCopy,
)]
pub struct CompressedMintMetadata {
    /// Version for upgradability
    pub version: u8,
    /// Extension, necessary for mint to.
    pub spl_mint_initialized: bool,
    /// Pda with seed address of compressed mint
    pub mint: Pubkey,
}

impl CompressedMint {
    pub fn hash(&self) -> Result<[u8; 32], CTokenError> {
        match self.metadata.version {
            3 => Ok(Sha256BE::hash(
                self.try_to_vec()
                    .map_err(|_| CTokenError::BorshFailed)?
                    .as_slice(),
            )?),
            _ => Err(CTokenError::InvalidTokenDataVersion),
        }
    }
}

// Implementation for zero-copy mutable CompressedMint
impl ZCompressedMintMut<'_> {
    /// Set all fields of the CompressedMint struct at once
    #[inline]
    #[profile]
    pub fn set(
        &mut self,
        ix_data: &<CompressedMintInstructionData as light_zero_copy::traits::ZeroCopyAt<'_>>::ZeroCopyAt,
        spl_mint_initialized: bool,
    ) -> Result<(), CTokenError> {
        if ix_data.metadata.version != 3 {
            #[cfg(feature = "solana")]
            msg!(
                "Only shaflat version 3 is supported got {}",
                ix_data.metadata.version
            );
            return Err(CTokenError::InvalidTokenMetadataVersion);
        }
        // Set metadata fields from instruction data
        self.metadata.version = ix_data.metadata.version;
        self.metadata.mint = ix_data.metadata.mint;
        self.metadata.spl_mint_initialized = if spl_mint_initialized { 1 } else { 0 };

        // Set base fields
        *self.base.supply = ix_data.supply;
        *self.base.decimals = ix_data.decimals;
        *self.base.is_initialized = 1; // Always initialized for compressed mints

        if let Some(mint_authority) = ix_data.mint_authority.as_deref() {
            self.base.set_mint_authority(Some(*mint_authority));
        }
        // Set freeze authority using COption format
        if let Some(freeze_authority) = ix_data.freeze_authority.as_deref() {
            self.base.set_freeze_authority(Some(*freeze_authority));
        }

        // extensions are handled separately
        Ok(())
    }
}
