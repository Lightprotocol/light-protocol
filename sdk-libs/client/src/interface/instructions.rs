//! Instruction builders for load/save operations.

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_sdk::{
    compressible::{compression_info::CompressedAccountData, config::LightConfig, Pack},
    instruction::{
        account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAccounts,
        SystemAccountMetaConfig, ValidityProof,
    },
};
use light_token::instruction::{
    COMPRESSIBLE_CONFIG_V1, LIGHT_TOKEN_CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR,
};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::indexer::{CompressedAccount, TreeInfo, ValidityProofWithContext};

#[inline]
fn get_output_queue(tree_info: &TreeInfo) -> Pubkey {
    tree_info
        .next_tree_info
        .as_ref()
        .map(|next| next.queue)
        .unwrap_or(tree_info.queue)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeConfigData {
    pub rent_sponsor: Pubkey,
    pub address_space: Vec<Pubkey>,
    pub config_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateConfigData {
    pub new_rent_sponsor: Option<Pubkey>,
    pub new_address_space: Option<Vec<Pubkey>>,
    pub new_update_authority: Option<Pubkey>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LoadAccountsData<T> {
    pub proof: ValidityProof,
    pub compressed_accounts: Vec<CompressedAccountData<T>>,
    pub system_accounts_offset: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SaveAccountsData {
    pub proof: ValidityProof,
    pub compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    pub system_accounts_offset: u8,
}

// Discriminators (match on-chain instruction names)
pub const INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] =
    [133, 228, 12, 169, 56, 76, 222, 61];
pub const UPDATE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] = [135, 215, 243, 81, 163, 146, 33, 70];
pub const DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR: [u8; 8] =
    [114, 67, 61, 123, 234, 31, 1, 112];
pub const COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR: [u8; 8] =
    [70, 236, 171, 120, 164, 93, 113, 181];

/// Account metas for load operations.
pub mod load {
    use super::*;

    /// With token support.
    pub fn accounts(fee_payer: Pubkey, config: Pubkey, rent_sponsor: Pubkey) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(fee_payer, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new(RENT_SPONSOR, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_CPI_AUTHORITY, false),
            AccountMeta::new_readonly(COMPRESSIBLE_CONFIG_V1, false),
        ]
    }

    /// PDAs only (no tokens).
    pub fn accounts_pda_only(
        fee_payer: Pubkey,
        config: Pubkey,
        rent_sponsor: Pubkey,
    ) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(fee_payer, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new(rent_sponsor, false), // placeholder for ctoken_rent_sponsor
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_CPI_AUTHORITY, false),
            AccountMeta::new_readonly(COMPRESSIBLE_CONFIG_V1, false),
        ]
    }
}

#[allow(clippy::too_many_arguments)]
pub fn initialize_config(
    program_id: &Pubkey,
    discriminator: &[u8],
    payer: &Pubkey,
    authority: &Pubkey,
    rent_sponsor: Pubkey,
    address_space: Vec<Pubkey>,
    config_bump: Option<u8>,
) -> Instruction {
    let config_bump = config_bump.unwrap_or(0);
    let (config_pda, _) = LightConfig::derive_pda(program_id, config_bump);

    let bpf_loader = solana_pubkey::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
    let (program_data_pda, _) = Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader);

    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111");
    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(program_data_pda, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(system_program, false),
    ];

    let ix_data = InitializeConfigData {
        rent_sponsor,
        address_space,
        config_bump,
    };

    let serialized = ix_data.try_to_vec().expect("serialize");
    let mut data = Vec::with_capacity(discriminator.len() + serialized.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(&serialized);

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

pub fn update_config(
    program_id: &Pubkey,
    discriminator: &[u8],
    authority: &Pubkey,
    new_rent_sponsor: Option<Pubkey>,
    new_address_space: Option<Vec<Pubkey>>,
    new_update_authority: Option<Pubkey>,
) -> Instruction {
    let (config_pda, _) = LightConfig::derive_pda(program_id, 0);

    let accounts = vec![
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    let ix_data = UpdateConfigData {
        new_rent_sponsor,
        new_address_space,
        new_update_authority,
    };

    let serialized = ix_data.try_to_vec().expect("serialize");
    let mut data = Vec::with_capacity(discriminator.len() + serialized.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(&serialized);

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Build load (decompress) instruction.
#[allow(clippy::too_many_arguments)]
pub fn create_decompress_accounts_idempotent_instruction<T>(
    program_id: &Pubkey,
    discriminator: &[u8],
    hot_addresses: &[Pubkey],
    cold_accounts: &[(CompressedAccount, T)],
    program_account_metas: &[AccountMeta],
    proof: ValidityProofWithContext,
) -> Result<Instruction, Box<dyn std::error::Error>>
where
    T: Pack + Clone + std::fmt::Debug,
{
    if cold_accounts.is_empty() {
        return Err("cold_accounts cannot be empty".into());
    }
    if hot_addresses.len() != cold_accounts.len() {
        return Err("hot_addresses and cold_accounts must have same length".into());
    }

    let mut remaining_accounts = PackedAccounts::default();

    let mut has_tokens = false;
    let mut has_pdas = false;
    for (acc, _) in cold_accounts.iter() {
        if acc.owner == LIGHT_TOKEN_PROGRAM_ID {
            has_tokens = true;
        } else {
            has_pdas = true;
        }
        if has_tokens && has_pdas {
            break;
        }
    }
    if !has_tokens && !has_pdas {
        return Err("No tokens or PDAs found".into());
    }

    // When mixing PDAs + tokens, use first token's CPI context
    if has_pdas && has_tokens {
        let first_token_acc = cold_accounts
            .iter()
            .find(|(acc, _)| acc.owner == LIGHT_TOKEN_PROGRAM_ID)
            .ok_or("expected at least one token account when has_tokens is true")?;
        let first_token_cpi = first_token_acc
            .0
            .tree_info
            .cpi_context
            .ok_or("missing cpi_context on token account")?;
        let config = SystemAccountMetaConfig::new_with_cpi_context(*program_id, first_token_cpi);
        remaining_accounts.add_system_accounts_v2(config)?;
    } else {
        remaining_accounts.add_system_accounts_v2(SystemAccountMetaConfig::new(*program_id))?;
    }

    let output_queue = get_output_queue(&cold_accounts[0].0.tree_info);
    let output_state_tree_index = remaining_accounts.insert_or_get(output_queue);

    let packed_tree_infos = proof.pack_tree_infos(&mut remaining_accounts);
    let tree_infos = &packed_tree_infos
        .state_trees
        .as_ref()
        .ok_or("missing state_trees in packed_tree_infos")?
        .packed_tree_infos;

    let mut accounts = program_account_metas.to_vec();
    let mut typed_accounts = Vec::with_capacity(cold_accounts.len());

    for (i, (acc, data)) in cold_accounts.iter().enumerate() {
        let _queue_index = remaining_accounts.insert_or_get(acc.tree_info.queue);
        let tree_info = tree_infos
            .get(i)
            .copied()
            .ok_or("tree info index out of bounds")?;

        let packed_data = data.pack(&mut remaining_accounts)?;
        typed_accounts.push(CompressedAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress {
                tree_info,
                output_state_tree_index,
            },
            data: packed_data,
        });
    }

    let (system_accounts, system_accounts_offset, _) = remaining_accounts.to_account_metas();
    accounts.extend(system_accounts);

    for addr in hot_addresses {
        accounts.push(AccountMeta::new(*addr, false));
    }

    let ix_data = LoadAccountsData {
        proof: proof.proof,
        compressed_accounts: typed_accounts,
        system_accounts_offset: system_accounts_offset as u8,
    };

    let serialized = ix_data.try_to_vec()?;
    let mut data = Vec::with_capacity(discriminator.len() + serialized.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(&serialized);

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Build compress instruction.
#[allow(clippy::too_many_arguments)]
pub fn build_compress_accounts_idempotent(
    program_id: &Pubkey,
    discriminator: &[u8],
    account_pubkeys: &[Pubkey],
    _accounts_to_save: &[Account],
    program_account_metas: &[AccountMeta],
    proof: ValidityProofWithContext,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    if proof.accounts.is_empty() {
        return Err("proof.accounts cannot be empty".into());
    }

    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_system_accounts_v2(SystemAccountMetaConfig::new(*program_id))?;

    let output_queue = get_output_queue(&proof.accounts[0].tree_info);
    let output_state_tree_index = remaining_accounts.insert_or_get(output_queue);

    let packed_tree_infos = proof.pack_tree_infos(&mut remaining_accounts);
    let tree_infos = packed_tree_infos
        .state_trees
        .as_ref()
        .ok_or("missing state_trees in packed_tree_infos")?;

    let cold_metas: Vec<_> = tree_infos
        .packed_tree_infos
        .iter()
        .map(|tree_info| CompressedAccountMetaNoLamportsNoAddress {
            tree_info: *tree_info,
            output_state_tree_index,
        })
        .collect();

    let mut accounts = program_account_metas.to_vec();
    let (system_accounts, system_accounts_offset, _) = remaining_accounts.to_account_metas();
    accounts.extend(system_accounts);

    for pubkey in account_pubkeys {
        accounts.push(AccountMeta::new(*pubkey, false));
    }

    let ix_data = SaveAccountsData {
        proof: proof.proof,
        compressed_accounts: cold_metas,
        system_accounts_offset: system_accounts_offset as u8,
    };

    let serialized = ix_data.try_to_vec()?;
    let mut data = Vec::with_capacity(discriminator.len() + serialized.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(&serialized);

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
