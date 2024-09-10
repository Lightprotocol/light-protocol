#![cfg(not(target_os = "solana"))]

use account_compression::{
    utils::constants::CPI_AUTHORITY_PDA_SEED, AddressMerkleTreeConfig, AddressQueueConfig,
    NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_token::get_token_pool_pda;
use light_sdk::{
    address::{pack_new_address_params, NewAddressParams},
    compressed_account::PackedCompressedAccountWithMerkleContext,
    merkle_context::RemainingAccounts,
    proof::CompressedProof,
};
use light_system_program::utils::get_registered_program_pda;
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
}

pub fn create_pda_instruction(input_params: CreateCompressedPdaInstructionInputs) -> Instruction {
    let (cpi_signer, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &crate::id());
    let mut remaining_accounts = RemainingAccounts::default();
    remaining_accounts.insert_or_get(*input_params.output_compressed_account_merkle_tree_pubkey);
    let new_address_params =
        pack_new_address_params(input_params.new_address_params, &mut remaining_accounts);

    // TODO(vadorovsky): Instead of doing these conversions, do the instruction
    // call without relying on the Anchor types.
    let new_address_params = light_system_program::invoke::instruction::NewAddressParamsPacked {
        seed: new_address_params.seed,
        address_queue_account_index: new_address_params.address_queue_account_index,
        address_merkle_tree_account_index: new_address_params.address_merkle_tree_account_index,
        address_merkle_tree_root_index: new_address_params.address_merkle_tree_root_index,
    };
    let proof = &input_params.proof;
    let proof = light_system_program::invoke::processor::CompressedProof {
        a: proof.a,
        b: proof.b,
        c: proof.c,
    };

    let instruction_data = crate::instruction::CreateCompressedPda {
        data: input_params.data,
        proof: Some(proof),
        new_address_parameters: new_address_params,
        owner_program: *input_params.owner_program,
        bump,
        signer_is_program: input_params.signer_is_program,
        cpi_context: None,
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
    let remaining_accounts = remaining_accounts.to_account_metas();

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

    let mut remaining_accounts = RemainingAccounts::default();
    remaining_accounts.insert_or_get(*input_params.input_merkle_tree_pubkey);
    remaining_accounts.insert_or_get(*input_params.input_nullifier_pubkey);
    remaining_accounts.insert_or_get(*input_params.cpi_context_account);
    remaining_accounts.insert_or_get(*input_params.invalid_fee_payer);

    // TODO(vadorovsky): Instead of doing these conversions, do the instruction
    // call without relying on the Anchor types.
    let proof = &input_params.proof;
    let proof = light_system_program::invoke::processor::CompressedProof {
        a: proof.a,
        b: proof.b,
        c: proof.c,
    };
    let compressed_account =
        light_system_program::sdk::compressed_account::PackedCompressedAccountWithMerkleContext {
            compressed_account: light_system_program::sdk::compressed_account::CompressedAccount {
                owner: input_params.compressed_account.compressed_account.owner,
                lamports: input_params.compressed_account.compressed_account.lamports,
                address: input_params.compressed_account.compressed_account.address,
                data: input_params
                    .compressed_account
                    .compressed_account
                    .data
                    .as_ref()
                    .map(|data| {
                        light_system_program::sdk::compressed_account::CompressedAccountData {
                            discriminator: data.discriminator,
                            data: data.data.clone(),
                            data_hash: data.data_hash,
                        }
                    }),
            },
            merkle_context: light_system_program::sdk::compressed_account::PackedMerkleContext {
                merkle_tree_pubkey_index: input_params
                    .compressed_account
                    .merkle_context
                    .merkle_tree_pubkey_index,
                nullifier_queue_pubkey_index: input_params
                    .compressed_account
                    .merkle_context
                    .nullifier_queue_pubkey_index,
                leaf_index: input_params.compressed_account.merkle_context.leaf_index,
                queue_index: input_params
                    .compressed_account
                    .merkle_context
                    .queue_index
                    .map(
                        |queue_index| light_system_program::sdk::compressed_account::QueueIndex {
                            queue_id: queue_index.queue_id,
                            index: queue_index.index,
                        },
                    ),
            },
            root_index: input_params.compressed_account.root_index,
            read_only: input_params.compressed_account.read_only,
        };

    let instruction_data = crate::instruction::WithInputAccounts {
        proof: Some(proof),
        compressed_account,
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
    let remaining_accounts = remaining_accounts.to_account_metas();

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
