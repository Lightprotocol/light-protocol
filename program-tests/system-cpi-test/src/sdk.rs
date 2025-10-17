#![cfg(not(target_os = "solana"))]

use std::collections::HashMap;

use account_compression::{
    utils::constants::CPI_AUTHORITY_PDA_SEED, AddressMerkleTreeConfig, AddressQueueConfig,
    NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_account::{
    compressed_account::{
        CompressedAccountWithMerkleContext, PackedCompressedAccountWithMerkleContext,
        ReadOnlyCompressedAccount,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{NewAddressParams, ReadOnlyAddress},
    },
};
use light_compressed_token::{
    get_token_pool_pda, process_transfer::transfer_sdk::to_account_metas,
};
use light_system_program::utils::get_registered_program_pda;
use light_test_utils::{
    compressed_account_pack::{
        pack_compressed_account, pack_new_address_params, pack_read_only_accounts,
        pack_read_only_address_params,
    },
    e2e_test_env::to_account_metas_light,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::CreatePdaMode;

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaInstructionInputs<'a> {
    pub data: [u8; 31],
    pub signer: &'a Pubkey,
    pub output_compressed_account_merkle_tree_pubkey: &'a Pubkey,
    pub proof: &'a CompressedProof,
    pub new_address_params: NewAddressParams,
    pub cpi_context_account: &'a Pubkey,
    pub owner_program: &'a Pubkey,
    pub signer_is_program: CreatePdaMode,
    pub registered_program_pda: &'a Pubkey,
    pub readonly_adresses: Option<Vec<ReadOnlyAddress>>,
    pub read_only_accounts: Option<Vec<ReadOnlyCompressedAccount>>,
    pub input_compressed_accounts_with_merkle_context:
        Option<Vec<CompressedAccountWithMerkleContext>>,
    pub state_roots: Option<Vec<Option<u16>>>,
}

pub fn create_pda_instruction(input_params: CreateCompressedPdaInstructionInputs) -> Instruction {
    let (cpi_signer, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &crate::id());
    let mut remaining_accounts = HashMap::<light_compressed_account::Pubkey, usize>::new();
    remaining_accounts.insert(
        input_params
            .output_compressed_account_merkle_tree_pubkey
            .into(),
        0,
    );
    let new_address_params =
        pack_new_address_params(&[input_params.new_address_params], &mut remaining_accounts);
    let read_only_address = input_params
        .readonly_adresses
        .as_ref()
        .map(|read_only_adresses| {
            pack_read_only_address_params(read_only_adresses, &mut remaining_accounts)
        });
    let read_only_accounts = input_params
        .read_only_accounts
        .as_ref()
        .map(|read_only_accounts| {
            pack_read_only_accounts(read_only_accounts, &mut remaining_accounts)
        });
    let input_accounts = input_params
        .input_compressed_accounts_with_merkle_context
        .as_ref()
        .map(|input_accounts| {
            input_accounts
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    pack_compressed_account(
                        x,
                        input_params.state_roots.as_ref().unwrap()[i],
                        &mut remaining_accounts,
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>()
        });
    let instruction_data = crate::instruction::CreateCompressedPda {
        data: input_params.data,
        proof: Some(*input_params.proof),
        new_address_parameters: new_address_params[0],
        owner_program: *input_params.owner_program,
        bump,
        signer_is_program: input_params.signer_is_program,
        cpi_context: None,
        read_only_address,
        read_only_accounts,
        input_accounts,
    };

    let compressed_token_cpi_authority_pda = get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);

    let accounts = crate::accounts::CreateCompressedPda {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda: *input_params.registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };
    let remaining_accounts = to_account_metas_light(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}

#[derive(Debug, Clone)]
pub struct InvalidateNotOwnedCompressedAccountInstructionInputs<'a> {
    pub signer: &'a Pubkey,
    pub proof: &'a CompressedProof,
    pub input_merkle_tree_pubkey: &'a Pubkey,
    pub input_nullifier_pubkey: &'a Pubkey,
    pub cpi_context_account: &'a Pubkey,
    pub compressed_account: &'a PackedCompressedAccountWithMerkleContext,
    pub token_transfer_data: Option<crate::TokenTransferData>,
    pub cpi_context: Option<crate::CompressedCpiContext>,
    pub invalid_fee_payer: &'a Pubkey,
}

pub fn create_invalidate_not_owned_account_instruction(
    input_params: InvalidateNotOwnedCompressedAccountInstructionInputs,
    mode: crate::WithInputAccountsMode,
) -> Instruction {
    let (cpi_signer, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &crate::id());
    let cpi_context = input_params.cpi_context;

    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(*input_params.input_merkle_tree_pubkey, 0);
    remaining_accounts.insert(*input_params.input_nullifier_pubkey, 1);
    remaining_accounts.insert(*input_params.cpi_context_account, 2);
    remaining_accounts.insert(*input_params.invalid_fee_payer, 3);

    let instruction_data = crate::instruction::WithInputAccounts {
        proof: Some(*input_params.proof),
        compressed_account: input_params.compressed_account.clone(),
        bump,
        mode,
        cpi_context,
        token_transfer_data: input_params.token_transfer_data.clone(),
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda =
        light_compressed_token::process_transfer::get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let mint = match input_params.token_transfer_data.as_ref() {
        Some(data) => data.mint,
        None => Pubkey::new_unique(),
    };
    let token_pool_account = get_token_pool_pda(&mint);
    let accounts = crate::accounts::InvalidateNotOwnedCompressedAccount {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
        compressed_token_program: light_compressed_token::ID,
        invalid_fee_payer: *input_params.invalid_fee_payer,
        token_pool_account,
        mint,
        token_program: anchor_spl::token::ID,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}
pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &crate::ID)
}

pub fn create_initialize_address_merkle_tree_and_queue_instruction(
    index: u64,
    payer: Pubkey,
    program_owner: Option<Pubkey>,
    merkle_tree_pubkey: Pubkey,
    queue_pubkey: Pubkey,
    address_merkle_tree_config: AddressMerkleTreeConfig,
    address_queue_config: AddressQueueConfig,
    invalid_group: bool,
) -> Instruction {
    let register_program_pda = if invalid_group {
        get_registered_program_pda(&light_registry::ID)
    } else {
        get_registered_program_pda(&crate::ID)
    };
    let (cpi_authority, bump) = crate::sdk::get_cpi_authority_pda();

    let instruction_data = crate::instruction::InitializeAddressMerkleTree {
        bump,
        index,
        program_owner,
        merkle_tree_config: address_merkle_tree_config,
        queue_config: address_queue_config,
    };
    let accounts = crate::accounts::InitializeAddressMerkleTreeAndQueue {
        authority: payer,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        queue: queue_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

pub fn create_initialize_merkle_tree_instruction(
    payer: Pubkey,
    merkle_tree_pubkey: Pubkey,
    nullifier_queue_pubkey: Pubkey,
    state_merkle_tree_config: StateMerkleTreeConfig,
    nullifier_queue_config: NullifierQueueConfig,
    program_owner: Option<Pubkey>,
    index: u64,
    additional_bytes: u64,
    invalid_group: bool,
) -> Instruction {
    let register_program_pda = if invalid_group {
        get_registered_program_pda(&light_registry::ID)
    } else {
        get_registered_program_pda(&crate::ID)
    };
    let (cpi_authority, bump) = crate::sdk::get_cpi_authority_pda();

    let instruction_data = crate::instruction::InitializeStateMerkleTree {
        bump,
        index,
        program_owner,
        merkle_tree_config: state_merkle_tree_config,
        queue_config: nullifier_queue_config,
        additional_bytes,
    };
    let accounts = crate::accounts::InitializeAddressMerkleTreeAndQueue {
        authority: payer,
        registered_program_pda: register_program_pda,
        merkle_tree: merkle_tree_pubkey,
        queue: nullifier_queue_pubkey,
        cpi_authority,
        account_compression_program: account_compression::ID,
    };
    Instruction {
        program_id: crate::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}
