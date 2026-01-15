use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{address::derive_address, Pubkey};
use light_compressible::compression_info::CompressionInfo;
use light_hasher::{sha256::Sha256BE, Hasher};
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use pinocchio::account_info::AccountInfo;
#[cfg(feature = "solana")]
use solana_msg::msg;

use crate::{
    state::ExtensionStruct, AnchorDeserialize, AnchorSerialize, TokenError, CMINT_ADDRESS_TREE,
    LIGHT_TOKEN_PROGRAM_ID,
};

/// AccountType::Mint discriminator value
pub const ACCOUNT_TYPE_MINT: u8 = 1;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub struct CompressedMint {
    pub base: BaseMint,
    pub metadata: CompressedMintMetadata,
    /// Reserved bytes (16 bytes) for T22 layout compatibility.
    /// Positions `account_type` at offset 165: 82 (BaseMint) + 67 (metadata) + 16 (reserved) = 165.
    pub reserved: [u8; 16],
    /// Account type discriminator at byte offset 165 (1 = Mint, 2 = Account)
    pub account_type: u8,
    /// Compression info embedded directly in the mint
    pub compression: CompressionInfo,
    pub extensions: Option<Vec<ExtensionStruct>>,
}

impl Default for CompressedMint {
    fn default() -> Self {
        Self {
            base: BaseMint::default(),
            metadata: CompressedMintMetadata::default(),
            reserved: [0u8; 16],
            account_type: ACCOUNT_TYPE_MINT,
            compression: CompressionInfo::default(),
            extensions: None,
        }
    }
}

// and subsequent deserialization for remaining data (compression metadata + extensions)
/// SPL-compatible base mint structure with padding for COption alignment
#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq, Clone)]
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

/// Light Protocol-specific metadata for compressed mints.
///
/// Total size: 67 bytes
/// - version: 1 byte
/// - cmint_decompressed: 1 byte
/// - mint: 32 bytes
/// - mint_signer: 32 bytes
/// - bump: 1 byte
#[repr(C)]
#[derive(
    Debug, Default, PartialEq, Eq, Clone, AnchorDeserialize, AnchorSerialize, ZeroCopyMut, ZeroCopy,
)]
pub struct CompressedMintMetadata {
    /// Version for upgradability
    pub version: u8,
    /// Whether the compressed mint has been decompressed to a CMint Solana account.
    /// When true, the CMint account is the source of truth.
    pub cmint_decompressed: bool,
    /// PDA derived from mint_signer, used as seed for the compressed address
    pub mint: Pubkey,
    /// Signer pubkey used to derive the mint PDA
    pub mint_signer: [u8; 32],
    /// Bump seed from mint PDA derivation
    pub bump: u8,
}

impl CompressedMintMetadata {
    /// Derives the compressed address from mint PDA, CMINT_ADDRESS_TREE and LIGHT_TOKEN_PROGRAM_ID
    pub fn compressed_address(&self) -> [u8; 32] {
        derive_address(
            self.mint.array_ref(),
            &CMINT_ADDRESS_TREE,
            &LIGHT_TOKEN_PROGRAM_ID,
        )
    }
}

impl ZCompressedMintMetadata<'_> {
    /// Derives the compressed address from mint PDA, CMINT_ADDRESS_TREE and LIGHT_TOKEN_PROGRAM_ID
    pub fn compressed_address(&self) -> [u8; 32] {
        derive_address(
            self.mint.array_ref(),
            &CMINT_ADDRESS_TREE,
            &LIGHT_TOKEN_PROGRAM_ID,
        )
    }
}

impl CompressedMint {
    pub fn hash(&self) -> Result<[u8; 32], TokenError> {
        match self.metadata.version {
            3 => Ok(Sha256BE::hash(
                self.try_to_vec()
                    .map_err(|_| TokenError::BorshFailed)?
                    .as_slice(),
            )?),
            _ => Err(TokenError::InvalidTokenDataVersion),
        }
    }

    /// Deserialize a CompressedMint from a CMint Solana account with validation.
    ///
    /// Checks:
    /// 1. Account is owned by the specified program
    /// 2. Account is initialized (BaseMint.is_initialized == true)
    ///
    /// Note: CMint accounts follow SPL token mint pattern (no discriminator).
    /// Validation is done via owner check + PDA derivation (caller responsibility).
    pub fn from_account_info_checked(account_info: &AccountInfo) -> Result<Self, TokenError> {
        // 1. Check program ownership
        if !account_info.is_owned_by(&LIGHT_TOKEN_PROGRAM_ID) {
            #[cfg(feature = "solana")]
            msg!("CMint account has invalid owner");
            return Err(TokenError::InvalidCMintOwner);
        }

        // 2. Borrow and deserialize account data
        let data = account_info
            .try_borrow_data()
            .map_err(|_| TokenError::CMintBorrowFailed)?;

        let mint =
            Self::try_from_slice(&data).map_err(|_| TokenError::CMintDeserializationFailed)?;

        // 3. Check is_initialized
        if !mint.base.is_initialized {
            #[cfg(feature = "solana")]
            msg!("CMint account is not initialized");
            return Err(TokenError::CMintNotInitialized);
        }

        if !mint.is_cmint_account() {
            #[cfg(feature = "solana")]
            msg!("CMint account is not a CMint account");
            return Err(TokenError::MintMismatch);
        }

        Ok(mint)
    }

    /// Checks if account_type matches CMint discriminator value
    #[inline(always)]
    pub fn is_cmint_account(&self) -> bool {
        self.account_type == ACCOUNT_TYPE_MINT
    }
}
