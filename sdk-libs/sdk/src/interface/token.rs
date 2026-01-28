use light_compressed_account::compressed_account::PackedMerkleContext;
use light_token_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{
        extensions::{CompressedOnlyExtension, ExtensionStruct},
        AccountState, Token,
    },
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    instruction::{PackedAccounts, PackedStateTreeInfo},
    interface::{DecompressCtx, PackedLightAccountVariantTrait},
    AnchorDeserialize, AnchorSerialize, Pack, Unpack,
};

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct TokenDataWithSeeds<S: Pack> {
    pub seeds: S,
    pub token_data: Token,
    pub tree_info: PackedStateTreeInfo,
    pub version: u8,
}

#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TokenDataWithPackedSeeds<
    S: Unpack + AnchorSerialize + AnchorDeserialize + Clone + std::fmt::Debug,
> {
    pub seeds: S,
    pub token_data: MultiInputTokenDataWithContext,
    pub extension: Option<CompressedOnlyExtensionInstructionData>,
}

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

        let token_data = MultiInputTokenDataWithContext {
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
            version: self.version,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: self.tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: self.tree_info.queue_pubkey_index,
                leaf_index: self.tree_info.leaf_index,
                prove_by_index: self.tree_info.prove_by_index,
            },
            root_index: self.tree_info.root_index,
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
    S::Unpacked: Pack,
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
            account_type: 0,
            extensions,
        };

        let tree_info = PackedStateTreeInfo {
            root_index: self.token_data.root_index,
            prove_by_index: self.token_data.merkle_context.prove_by_index,
            merkle_tree_pubkey_index: self.token_data.merkle_context.merkle_tree_pubkey_index,
            queue_pubkey_index: self.token_data.merkle_context.queue_pubkey_index,
            leaf_index: self.token_data.merkle_context.leaf_index,
        };

        Ok(TokenDataWithSeeds {
            seeds,
            token_data,
            tree_info,
            version: self.token_data.version,
        })
    }
}

pub fn prepare_token_account_for_decompression<'info, const SEED_COUNT: usize, P>(
    packed: &P,
    token_account_info: &AccountInfo<'info>,
    ctx: &mut DecompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>
where
    P: PackedLightAccountVariantTrait<SEED_COUNT>,
{
    // return early for idempotency
    if !token_account_info.data_is_empty() && token_account_info.try_borrow_data()?[107] != 0 {
        return Ok(());
    }
    let packed_accounts = ctx.cpi_accounts.packed_accounts();

    ctx.in_token_data.push(packed.into_in_token_data()?);
    let bump = &[packed.bump()];
    let seeds = packed
        .seed_refs_with_bump(packed_accounts, bump)
        .map_err(|_| ProgramError::InvalidSeeds)?;
    ctx.token_seeds.extend(seeds.iter().map(|s| s.to_vec()));

    let in_tlv: Option<Vec<ExtensionInstructionData>> = packed.into_in_tlv()?;

    if let Some(ctx_in_tlv) = ctx.in_tlv.as_mut() {
        ctx_in_tlv.push(in_tlv.unwrap_or_default());
    } else if let Some(in_tlv) = in_tlv {
        let mut ctx_in_tlv = vec![];
        for _ in 0..ctx.in_token_data.len() - 1 {
            ctx_in_tlv.push(vec![]);
        }
        ctx_in_tlv.push(in_tlv);
        ctx.in_tlv = Some(ctx_in_tlv);
    }

    Ok(())
}
