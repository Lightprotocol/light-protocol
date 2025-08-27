use light_compressed_account::Pubkey;
use light_hasher::{sha256::Sha256BE, Hasher};
use light_zero_copy::{traits::ZeroCopyAt, ZeroCopy, ZeroCopyMut};
use solana_msg::msg;

use crate::{
    instructions::mint_action::CompressedMintInstructionData, state::ExtensionStruct,
    AnchorDeserialize, AnchorSerialize, CTokenError,
};

// Order is optimized for hashing.
// freeze_authority option is skipped if None.
#[repr(C)]
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopyMut, ZeroCopy,
)]
pub struct CompressedMint {
    pub base: BaseCompressedMint,
    pub extensions: Option<Vec<ExtensionStruct>>,
}

#[repr(C)]
#[derive(
    Debug, PartialEq, Eq, Clone, AnchorSerialize, AnchorDeserialize, ZeroCopyMut, ZeroCopy,
)]
pub struct BaseCompressedMint {
    /// Version for upgradability
    pub version: u8,
    /// Pda with seed address of compressed mint
    pub spl_mint: Pubkey,
    /// Total supply of tokens.
    pub supply: u64,
    /// Number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
    /// Extension, necessary for mint to.
    pub is_decompressed: bool,
    /// Optional authority used to mint new tokens. The mint authority may only
    /// be provided during mint creation. If no mint authority is present
    /// then the mint has a fixed supply and no further tokens may be
    /// minted.
    pub mint_authority: Option<Pubkey>,
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
}

impl CompressedMint {
    pub fn hash(&self) -> Result<[u8; 32], CTokenError> {
        match self.base.version {
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
    pub fn set(
        &mut self,
        ix_data: &<CompressedMintInstructionData as ZeroCopyAt<'_>>::ZeroCopyAt,
        is_decompressed: bool,
    ) -> Result<(), CTokenError> {
        if ix_data.base.version != 3 {
            msg!(
                "Only shaflat version 3 is supported got {}",
                ix_data.base.version
            );
            return Err(CTokenError::InvalidTokenMetadataVersion);
        }
        self.base.version = ix_data.base.version;
        self.base.spl_mint = ix_data.base.spl_mint;
        self.base.supply = ix_data.base.supply;
        self.base.decimals = ix_data.base.decimals;
        self.base.is_decompressed = if is_decompressed { 1 } else { 0 };

        if let Some(self_mint_authority) = self.base.mint_authority.as_deref_mut() {
            *self_mint_authority = *ix_data
                .base
                .mint_authority
                .ok_or(CTokenError::InstructionDataExpectedMintAuthority)?;
        }

        if self.base.mint_authority.is_some() && ix_data.base.mint_authority.is_none() {
            return Err(CTokenError::ZeroCopyExpectedMintAuthority);
        }

        if let Some(self_freeze_authority) = self.base.freeze_authority.as_deref_mut() {
            *self_freeze_authority = *ix_data
                .base
                .freeze_authority
                .ok_or(CTokenError::InstructionDataExpectedFreezeAuthority)?;
        }

        if self.base.freeze_authority.is_some() && ix_data.base.freeze_authority.is_none() {
            return Err(CTokenError::ZeroCopyExpectedFreezeAuthority);
        }
        // extensions are handled separately
        Ok(())
    }
}
