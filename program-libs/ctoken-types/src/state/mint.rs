use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;
use light_hasher::{sha256::Sha256BE, Hasher};
use light_profiler::profile;
use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew},
    IntoBytes, Ref, ZeroCopy, ZeroCopyMut,
};
use solana_msg::msg;

use crate::{
    instructions::mint_action::CompressedMintInstructionData, state::ExtensionStruct,
    AnchorDeserialize, AnchorSerialize, CTokenError,
};
// Order is optimized for hashing.
// freeze_authority option is skipped if None.
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize, ZeroCopyMut, ZeroCopy)]
pub struct CompressedMint {
    pub base: BaseMint,
    pub metadata: CompressedMintMetadata,
    pub extensions: Option<Vec<ExtensionStruct>>,
}

/// SPL-compatible base mint structure with padding for COption alignment
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BaseMint {
    // /// Padding to align with SPL's COption discriminator (3 bytes padding + 1 byte Option)
    // pub _padding_mint_auth: [u8; 3],
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
    // /// Padding to align with SPL's COption discriminator (3 bytes padding + 1 byte Option)
    // pub _padding_freeze_auth: [u8; 3],
    /// Optional authority to freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
}

// Manual implementation of BorshSerialize for SPL compatibility
impl BorshSerialize for BaseMint {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write mint_authority as COption (4 bytes + 32 bytes)
        if let Some(authority) = self.mint_authority {
            writer.write_all(&[1, 0, 0, 0])?; // COption Some discriminator
            writer.write_all(&authority.to_bytes())?;
        } else {
            writer.write_all(&[0; 36])?; // COption None (4 bytes) + empty pubkey (32 bytes)
        }

        // Write supply (8 bytes)
        writer.write_all(&self.supply.to_le_bytes())?;

        // Write decimals (1 byte)
        writer.write_all(&[self.decimals])?;

        // Write is_initialized (1 byte)
        writer.write_all(&[if self.is_initialized { 1 } else { 0 }])?;

        // Write freeze_authority as COption (4 bytes + 32 bytes)
        if let Some(authority) = self.freeze_authority {
            writer.write_all(&[1, 0, 0, 0])?; // COption Some discriminator
            writer.write_all(&authority.to_bytes())?;
        } else {
            writer.write_all(&[0; 36])?; // COption None (4 bytes) + empty pubkey (32 bytes)
        }

        Ok(())
    }
}

// Manual implementation of BorshDeserialize for SPL compatibility
impl BorshDeserialize for BaseMint {
    fn deserialize_reader<R: std::io::Read>(buf: &mut R) -> std::io::Result<Self> {
        // Read mint_authority COption
        let mut discriminator = [0u8; 4];
        buf.read_exact(&mut discriminator)?;
        let mut pubkey_bytes = [0u8; 32];
        buf.read_exact(&mut pubkey_bytes)?;
        let mint_authority = if u32::from_le_bytes(discriminator) == 1 {
            Some(Pubkey::from(pubkey_bytes))
        } else {
            None
        };

        // Read supply
        let mut supply_bytes = [0u8; 8];
        buf.read_exact(&mut supply_bytes)?;
        let supply = u64::from_le_bytes(supply_bytes);

        // Read decimals
        let mut decimals = [0u8; 1];
        buf.read_exact(&mut decimals)?;
        let decimals = decimals[0];

        // Read is_initialized
        let mut is_initialized = [0u8; 1];
        buf.read_exact(&mut is_initialized)?;
        let is_initialized = is_initialized[0] != 0;

        // Read freeze_authority COption
        let mut discriminator = [0u8; 4];
        buf.read_exact(&mut discriminator)?;
        let mut pubkey_bytes = [0u8; 32];
        buf.read_exact(&mut pubkey_bytes)?;
        let freeze_authority = if u32::from_le_bytes(discriminator) == 1 {
            Some(Pubkey::from(pubkey_bytes))
        } else {
            None
        };

        Ok(Self {
            mint_authority,
            supply,
            decimals,
            is_initialized,
            freeze_authority,
        })
    }

    // fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
    //     let mut buf = Vec::new();
    //     reader.read_to_end(&mut buf)?;
    //     let mut slice = buf.as_slice();
    //     Self::deserialize(&mut slice)
    // }
}

// Manual implementation of ZeroCopyAt for BaseMint with SPL COption compatibility
impl<'a> ZeroCopyAt<'a> for BaseMint {
    type ZeroCopyAt = ZBaseMint<'a>;

    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        if bytes.len() < 82 {
            return Err(ZeroCopyError::Size);
        }

        // Parse mint_authority COption (4 bytes + 32 bytes)
        let (mint_auth_disc, bytes) = bytes.split_at(4);
        let (mint_auth_pubkey, bytes) = Ref::<&[u8], Pubkey>::from_prefix(bytes)?;

        let mint_auth_pubkey = if mint_auth_disc[0] == 1 {
            Some(mint_auth_pubkey)
        } else {
            None
        };

        // Parse supply, decimals, is_initialized
        let (supply, bytes) =
            Ref::<&[u8], light_zero_copy::little_endian::U64>::from_prefix(bytes)?;
        let (decimals, bytes) = u8::zero_copy_at(bytes)?;
        let (is_initialized, bytes) = u8::zero_copy_at(bytes)?;

        // Parse freeze_authority COption (4 bytes + 32 bytes)
        let (freeze_auth_disc, bytes) = bytes.split_at(4);
        let (freeze_auth_pubkey, bytes) = Ref::<&[u8], Pubkey>::from_prefix(bytes)?;
        let freeze_auth_pubkey = if freeze_auth_disc[0] == 1 {
            Some(freeze_auth_pubkey)
        } else {
            None
        };
        Ok((
            ZBaseMint {
                mint_authority: mint_auth_pubkey,
                supply,
                decimals,
                is_initialized,
                freeze_authority: freeze_auth_pubkey,
            },
            bytes,
        ))
    }
}

// Zero-copy representation of BaseMint
#[derive(Debug, Clone, PartialEq)]
pub struct ZBaseMint<'a> {
    pub mint_authority: <Option<Pubkey> as ZeroCopyAt<'a>>::ZeroCopyAt,
    pub supply: Ref<&'a [u8], light_zero_copy::little_endian::U64>,
    pub decimals: u8,
    pub is_initialized: u8,
    pub freeze_authority: <Option<Pubkey> as ZeroCopyAt<'a>>::ZeroCopyAt,
}

// Manual implementation of ZeroCopyAtMut for BaseMint
impl<'a> ZeroCopyAtMut<'a> for BaseMint {
    type ZeroCopyAtMut = ZBaseMintMut<'a>;

    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < 82 {
            return Err(ZeroCopyError::Size);
        }

        // Parse mint_authority COption (4 bytes + 32 bytes)
        let (mint_auth_disc, bytes) = Ref::<&mut [u8], [u8; 4]>::from_prefix(bytes)?;
        let (mint_auth_pubkey, bytes) = Ref::<&mut [u8], Pubkey>::from_prefix(bytes)?;

        // Parse supply, decimals, is_initialized
        let (supply, bytes) =
            Ref::<&mut [u8], light_zero_copy::little_endian::U64>::from_prefix(bytes)?;
        let (decimals, bytes) = Ref::<&mut [u8], u8>::from_prefix(bytes)?;
        let (is_initialized, bytes) = Ref::<&mut [u8], u8>::from_prefix(bytes)?;

        // Parse freeze_authority COption (4 bytes + 32 bytes)
        let (freeze_auth_disc, bytes) = Ref::<&mut [u8], [u8; 4]>::from_prefix(bytes)?;
        let (freeze_auth_pubkey, bytes) = Ref::<&mut [u8], Pubkey>::from_prefix(bytes)?;

        Ok((
            ZBaseMintMut {
                mint_authority_discriminator: mint_auth_disc,
                mint_authority: mint_auth_pubkey,
                supply,
                decimals,
                is_initialized,
                freeze_authority_discriminator: freeze_auth_disc,
                freeze_authority: freeze_auth_pubkey,
            },
            bytes,
        ))
    }
}

// Mutable zero-copy representation of BaseMint
#[derive(Debug)]
pub struct ZBaseMintMut<'a> {
    mint_authority_discriminator: Ref<&'a mut [u8], [u8; 4]>,
    mint_authority: Ref<&'a mut [u8], Pubkey>,
    pub supply: Ref<&'a mut [u8], light_zero_copy::little_endian::U64>,
    pub decimals: Ref<&'a mut [u8], u8>,
    pub is_initialized: Ref<&'a mut [u8], u8>,
    freeze_authority_discriminator: Ref<&'a mut [u8], [u8; 4]>,
    freeze_authority: Ref<&'a mut [u8], Pubkey>,
}

impl ZBaseMintMut<'_> {
    pub fn mint_authority(&self) -> Option<&Pubkey> {
        if self.mint_authority_discriminator[0] == 1 {
            Some(&*self.mint_authority)
        } else {
            None
        }
    }

    pub fn set_mint_authority(&mut self, pubkey: Option<Pubkey>) {
        if let Some(pubkey) = pubkey {
            if self.mint_authority_discriminator[0] == 0 {
                self.mint_authority_discriminator[0] = 1;
            }
            *self.mint_authority = pubkey;
        } else {
            if self.mint_authority_discriminator[0] == 1 {
                self.mint_authority_discriminator[0] = 0;
            }
            self.mint_authority.as_mut_bytes().fill(0);
        }
    }
    pub fn freeze_authority(&self) -> Option<&Pubkey> {
        if self.freeze_authority_discriminator[0] == 1 {
            Some(&*self.freeze_authority)
        } else {
            None
        }
    }

    pub fn set_freeze_authority(&mut self, pubkey: Option<Pubkey>) {
        if let Some(pubkey) = pubkey {
            if self.freeze_authority_discriminator[0] == 0 {
                self.freeze_authority_discriminator[0] = 1;
            }
            *self.freeze_authority = pubkey;
        } else {
            if self.freeze_authority_discriminator[0] == 1 {
                self.freeze_authority_discriminator[0] = 0;
            }
            self.freeze_authority.as_mut_bytes().fill(0);
        }
    }
}

// Manual implementation of ZeroCopyNew for BaseMint
impl<'a> ZeroCopyNew<'a> for BaseMint {
    type ZeroCopyConfig = ();
    type Output = ZBaseMintMut<'a>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(82) // SPL Mint size
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < 82 {
            return Err(ZeroCopyError::Size);
        }

        // is_initialized
        bytes[45] = 1;

        // Now parse as mutable zero-copy
        Self::zero_copy_at_mut(bytes)
    }
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
    pub is_decompressed: bool,
    /// Pda with seed address of compressed mint
    pub spl_mint: Pubkey,
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
        ix_data: &<CompressedMintInstructionData as ZeroCopyAt<'_>>::ZeroCopyAt,
        is_decompressed: bool,
    ) -> Result<(), CTokenError> {
        if ix_data.metadata.version != 3 {
            msg!(
                "Only shaflat version 3 is supported got {}",
                ix_data.metadata.version
            );
            return Err(CTokenError::InvalidTokenMetadataVersion);
        }
        // Set metadata fields from instruction data
        self.metadata.version = ix_data.metadata.version;
        self.metadata.spl_mint = ix_data.metadata.spl_mint;
        self.metadata.is_decompressed = if is_decompressed { 1 } else { 0 };

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
