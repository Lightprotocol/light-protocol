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
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use super::pack::Unpack;
// Pack trait and PackedAccounts only available off-chain (client-side packing)
#[cfg(not(target_os = "solana"))]
use crate::{instruction::PackedAccounts, interface::Pack};
use crate::{
    interface::{
        AccountType, LightAccountVariantTrait, PackedLightAccountVariantTrait, PackedTokenSeeds,
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
    pub has_delegate: bool, // Optional delegate is set
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
}

#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TokenDataWithPackedSeeds<
    S: Unpack + AnchorSerialize + AnchorDeserialize + Clone + std::fmt::Debug,
> {
    pub seeds: S,
    pub token_data: PackedTokenData,
    pub extension: Option<CompressedOnlyExtensionInstructionData>,
}

#[cfg(not(target_os = "solana"))]
impl<S> Pack for TokenDataWithSeeds<S>
where
    S: Pack,
    S::Packed: Unpack + AnchorDeserialize + AnchorSerialize + Clone + std::fmt::Debug,
{
    type Packed = TokenDataWithPackedSeeds<S::Packed>;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError> {
        let seeds = self.seeds.pack(remaining_accounts)?;

        let owner_index = remaining_accounts
            .insert_or_get(Pubkey::new_from_array(self.token_data.owner.to_bytes()));

        let token_data = PackedTokenData {
            owner: owner_index,
            amount: self.token_data.amount,
            has_delegate: self.token_data.delegate.is_some(),
            delegate: self
                .token_data
                .delegate
                .map(|d| remaining_accounts.insert_or_get(Pubkey::new_from_array(d.to_bytes())))
                .unwrap_or(0),
            mint: remaining_accounts
                .insert_or_get(Pubkey::new_from_array(self.token_data.mint.to_bytes())),
            version: TokenDataVersion::ShaFlat as u8,
        };

        // Extract CompressedOnly extension from Token state if present.
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

impl<S> Unpack for TokenDataWithPackedSeeds<S>
where
    S: Unpack + AnchorSerialize + AnchorDeserialize + Clone + std::fmt::Debug,
{
    type Unpacked = TokenDataWithSeeds<S::Unpacked>;

    fn unpack(&self, remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError> {
        let seeds = self.seeds.unpack(remaining_accounts)?;

        let owner_key = remaining_accounts
            .get(self.token_data.owner as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;
        let mint_key = remaining_accounts
            .get(self.token_data.mint as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;
        let delegate = if self.token_data.has_delegate {
            let delegate_key = remaining_accounts
                .get(self.token_data.delegate as usize)
                .ok_or(ProgramError::InvalidAccountData)?
                .key;
            Some(light_compressed_account::Pubkey::from(
                delegate_key.to_bytes(),
            ))
        } else {
            None
        };

        // Reconstruct extensions from instruction extension data.
        let extensions = self.extension.map(|ext| {
            vec![ExtensionStruct::CompressedOnly(CompressedOnlyExtension {
                delegated_amount: ext.delegated_amount,
                withheld_transfer_fee: ext.withheld_transfer_fee,
                is_ata: ext.is_ata as u8,
            })]
        });

        let state = self.extension.map_or(AccountState::Initialized, |ext| {
            if ext.is_frozen {
                AccountState::Frozen
            } else {
                AccountState::Initialized
            }
        });

        let delegated_amount = self.extension.map_or(0, |ext| ext.delegated_amount);

        let token_data = Token {
            mint: light_compressed_account::Pubkey::from(mint_key.to_bytes()),
            owner: light_compressed_account::Pubkey::from(owner_key.to_bytes()),
            amount: self.token_data.amount,
            delegate,
            state,
            is_native: None,
            delegated_amount,
            close_authority: None,
            account_type: TokenDataVersion::ShaFlat as u8,
            extensions,
        };

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
    S::Packed: PackedTokenSeeds<N> + Unpack<Unpacked = S>,
{
    const PROGRAM_ID: Pubkey = S::PROGRAM_ID;
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
    S: PackedTokenSeeds<N>,
    S::Unpacked: UnpackedTokenSeeds<N, Packed = S>,
{
    type Unpacked = TokenDataWithSeeds<S::Unpacked>;

    const ACCOUNT_TYPE: AccountType = AccountType::Token;

    fn bump(&self) -> u8 {
        self.seeds.bump()
    }

    fn unpack(&self, accounts: &[AccountInfo]) -> anchor_lang::Result<Self::Unpacked> {
        <Self as Unpack>::unpack(self, accounts).map_err(anchor_lang::error::Error::from)
    }

    fn seed_refs_with_bump<'a>(
        &'a self,
        accounts: &'a [AccountInfo],
        bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; N], ProgramError> {
        self.seeds.seed_refs_with_bump(accounts, bump_storage)
    }

    fn into_in_token_data(
        &self,
        tree_info: &PackedStateTreeInfo,
        output_queue_index: u8,
    ) -> anchor_lang::Result<MultiInputTokenDataWithContext> {
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

    fn into_in_tlv(&self) -> anchor_lang::Result<Option<Vec<ExtensionInstructionData>>> {
        Ok(self
            .extension
            .as_ref()
            .map(|ext| vec![ExtensionInstructionData::CompressedOnly(*ext)]))
    }
}
