use light_compressed_account::Pubkey;
use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew},
    IntoBytes, Ref,
};

use super::compressed_mint::BaseMint;

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
