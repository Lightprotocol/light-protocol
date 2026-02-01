//! Token seed types for packed/unpacked token account variants.
//!
//! Provides `TokenDataWithSeeds<S>`, `PackedTokenData`, and `TokenDataWithPackedSeeds<S>`
//! along with Pack/Unpack impls and blanket impls for variant traits.

use light_compressed_account::compressed_account::PackedMerkleContext;
use light_sdk_types::instruction::PackedStateTreeInfo;
pub use light_token_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{
        extensions::{CompressedOnlyExtension, ExtensionStruct},
        AccountState, Token, TokenDataVersion,
    },
};

use light_account_checks::AccountInfoTrait;

use super::pack::Unpack;
#[cfg(not(target_os = "solana"))]
use light_account_checks::AccountMetaTrait;
#[cfg(not(target_os = "solana"))]
use crate::{account::pack::Pack, instruction::PackedAccounts};
use crate::{
    account::light_account::AccountType,
    error::LightPdaError,
    program::variant::{
        LightAccountVariantTrait, PackedLightAccountVariantTrait, PackedTokenSeeds,
        UnpackedTokenSeeds,
    },
    AnchorDeserialize, AnchorSerialize,
};

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct TokenDataWithSeeds<S> {
    pub seeds: S,
    pub token_data: Token,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct PackedTokenData {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool,
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
}

#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TokenDataWithPackedSeeds<
    S: AnchorSerialize + AnchorDeserialize + Clone + core::fmt::Debug,
> {
    pub seeds: S,
    pub token_data: PackedTokenData,
    pub extension: Option<CompressedOnlyExtensionInstructionData>,
}

// =============================================================================
// Helper: unpack token data from packed indices
// =============================================================================

fn unpack_token_data_from_packed<AI: AccountInfoTrait>(
    packed: &PackedTokenData,
    extension: &Option<CompressedOnlyExtensionInstructionData>,
    accounts: &[AI],
) -> Result<Token, LightPdaError> {
    let owner_key = accounts
        .get(packed.owner as usize)
        .ok_or(LightPdaError::InvalidInstructionData)?
        .key();
    let mint_key = accounts
        .get(packed.mint as usize)
        .ok_or(LightPdaError::InvalidInstructionData)?
        .key();
    let delegate = if packed.has_delegate {
        let delegate_key = accounts
            .get(packed.delegate as usize)
            .ok_or(LightPdaError::InvalidInstructionData)?
            .key();
        Some(light_compressed_account::Pubkey::from(delegate_key))
    } else {
        None
    };

    let extensions = extension.map(|ext| {
        vec![ExtensionStruct::CompressedOnly(CompressedOnlyExtension {
            delegated_amount: ext.delegated_amount,
            withheld_transfer_fee: ext.withheld_transfer_fee,
            is_ata: ext.is_ata as u8,
        })]
    });

    let state = extension.map_or(AccountState::Initialized, |ext| {
        if ext.is_frozen {
            AccountState::Frozen
        } else {
            AccountState::Initialized
        }
    });

    let delegated_amount = extension.map_or(0, |ext| ext.delegated_amount);

    Ok(Token {
        mint: light_compressed_account::Pubkey::from(mint_key),
        owner: light_compressed_account::Pubkey::from(owner_key),
        amount: packed.amount,
        delegate,
        state,
        is_native: None,
        delegated_amount,
        close_authority: None,
        account_type: TokenDataVersion::ShaFlat as u8,
        extensions,
    })
}

// =============================================================================
// Pack impl (client-side only)
// =============================================================================

#[cfg(not(target_os = "solana"))]
impl<S, AM: AccountMetaTrait> Pack<AM> for TokenDataWithSeeds<S>
where
    S: Pack<AM>,
    S::Packed: AnchorDeserialize + AnchorSerialize + Clone + core::fmt::Debug,
{
    type Packed = TokenDataWithPackedSeeds<S::Packed>;

    fn pack(
        &self,
        remaining_accounts: &mut PackedAccounts<AM>,
    ) -> Result<Self::Packed, LightPdaError> {
        let seeds = self.seeds.pack(remaining_accounts)?;

        let owner_index = remaining_accounts.insert_or_get(self.token_data.owner.to_bytes());

        let token_data = PackedTokenData {
            owner: owner_index,
            amount: self.token_data.amount,
            has_delegate: self.token_data.delegate.is_some(),
            delegate: self
                .token_data
                .delegate
                .map(|d| remaining_accounts.insert_or_get(d.to_bytes()))
                .unwrap_or(0),
            mint: remaining_accounts.insert_or_get(self.token_data.mint.to_bytes()),
            version: TokenDataVersion::ShaFlat as u8,
        };

        let extension = self.token_data.extensions.as_ref().and_then(|exts| {
            exts.iter().find_map(|ext| {
                if let ExtensionStruct::CompressedOnly(co) = ext {
                    Some(CompressedOnlyExtensionInstructionData {
                        delegated_amount: co.delegated_amount,
                        withheld_transfer_fee: co.withheld_transfer_fee,
                        is_frozen: self.token_data.state == AccountState::Frozen,
                        compression_index: 0,
                        is_ata: co.is_ata != 0,
                        bump: 0,
                        owner_index,
                    })
                } else {
                    None
                }
            })
        });

        Ok(TokenDataWithPackedSeeds {
            seeds,
            token_data,
            extension,
        })
    }
}

// =============================================================================
// Unpack impl
// =============================================================================

impl<S, AI: AccountInfoTrait> Unpack<AI> for TokenDataWithPackedSeeds<S>
where
    S: Unpack<AI> + AnchorSerialize + AnchorDeserialize + Clone + core::fmt::Debug,
{
    type Unpacked = TokenDataWithSeeds<<S as Unpack<AI>>::Unpacked>;

    fn unpack(&self, remaining_accounts: &[AI]) -> Result<Self::Unpacked, LightPdaError> {
        let seeds = self.seeds.unpack(remaining_accounts)?;
        let token_data =
            unpack_token_data_from_packed(&self.token_data, &self.extension, remaining_accounts)?;
        Ok(TokenDataWithSeeds { seeds, token_data })
    }
}

// =============================================================================
// Blanket impls: LightAccountVariantTrait / PackedLightAccountVariantTrait
// for TokenDataWithSeeds<S> / TokenDataWithPackedSeeds<S>
// where S implements the seed-specific helper traits.
// =============================================================================

impl<const N: usize, S> LightAccountVariantTrait<N> for TokenDataWithSeeds<S>
where
    S: UnpackedTokenSeeds<N>,
    S::Packed: PackedTokenSeeds<N, Unpacked = S>,
{
    const PROGRAM_ID: [u8; 32] = S::PROGRAM_ID;
    type Seeds = S;
    type Data = Token;
    type Packed = TokenDataWithPackedSeeds<S::Packed>;

    fn data(&self) -> &Self::Data {
        &self.token_data
    }

    fn seed_vec(&self) -> Vec<Vec<u8>> {
        self.seeds.seed_vec()
    }

    fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; N] {
        self.seeds.seed_refs_with_bump(bump_storage)
    }
}

impl<const N: usize, S> PackedLightAccountVariantTrait<N> for TokenDataWithPackedSeeds<S>
where
    S: PackedTokenSeeds<N> + AnchorSerialize + AnchorDeserialize + Clone + core::fmt::Debug,
    S::Unpacked: UnpackedTokenSeeds<N, Packed = S>,
{
    type Unpacked = TokenDataWithSeeds<S::Unpacked>;

    const ACCOUNT_TYPE: AccountType = AccountType::Token;

    fn bump(&self) -> u8 {
        self.seeds.bump()
    }

    fn unpack<AI: AccountInfoTrait>(
        &self,
        accounts: &[AI],
    ) -> Result<Self::Unpacked, LightPdaError> {
        let seeds = self.seeds.unpack_seeds::<AI>(accounts)?;
        let token_data =
            unpack_token_data_from_packed(&self.token_data, &self.extension, accounts)?;
        Ok(TokenDataWithSeeds { seeds, token_data })
    }

    fn seed_refs_with_bump<'a, AI: AccountInfoTrait>(
        &'a self,
        accounts: &'a [AI],
        bump_storage: &'a [u8; 1],
    ) -> Result<[&'a [u8]; N], LightPdaError> {
        self.seeds.seed_refs_with_bump(accounts, bump_storage)
    }

    fn into_in_token_data(
        &self,
        tree_info: &PackedStateTreeInfo,
        output_queue_index: u8,
    ) -> Result<MultiInputTokenDataWithContext, LightPdaError> {
        Ok(MultiInputTokenDataWithContext {
            amount: self.token_data.amount,
            mint: self.token_data.mint,
            owner: self.token_data.owner,
            version: self.token_data.version,
            has_delegate: self.token_data.has_delegate,
            delegate: self.token_data.delegate,
            root_index: tree_info.root_index,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: output_queue_index,
                leaf_index: tree_info.leaf_index,
                prove_by_index: tree_info.prove_by_index,
            },
        })
    }

    fn into_in_tlv(&self) -> Result<Option<Vec<ExtensionInstructionData>>, LightPdaError> {
        Ok(self
            .extension
            .as_ref()
            .map(|ext| vec![ExtensionInstructionData::CompressedOnly(*ext)]))
    }

    fn derive_owner(&self) -> [u8; 32] {
        self.seeds.derive_owner()
    }
}
