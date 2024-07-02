#![cfg(not(target_os = "solana"))]

use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::{
    instruction::{
        InitializeAddressMerkleTreeAndQueue, InitializeStateMerkleTreeAndNullifierQueue,
    },
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};

pub fn create_initialize_merkle_tree_instruction(
    payer: Pubkey,
    registered_program_pda: Option<Pubkey>,
    merkle_tree_pubkey: Pubkey,
    nullifier_queue_pubkey: Pubkey,
    state_merkle_tree_config: StateMerkleTreeConfig,
    nullifier_queue_config: NullifierQueueConfig,
    program_owner: Option<Pubkey>,
    index: u64,
    additional_rent: u64,
) -> Instruction {
    let instruction_data = InitializeStateMerkleTreeAndNullifierQueue {
        index,
        program_owner,
        state_merkle_tree_config,
        nullifier_queue_config,
        additional_rent,
    };
    let registered_program = match registered_program_pda {
        Some(registered_program_pda) => AccountMeta::new(registered_program_pda, false),
        None => AccountMeta::new(crate::ID, false),
    };
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(merkle_tree_pubkey, false),
            AccountMeta::new(nullifier_queue_pubkey, false),
            registered_program,
        ],
        data: instruction_data.data(),
    }
}

pub fn create_insert_leaves_instruction(
    leaves: Vec<(u8, [u8; 32])>,
    fee_payer: Pubkey,
    authority: Pubkey,
    merkle_tree_pubkeys: Vec<Pubkey>,
) -> Instruction {
    let instruction_data = crate::instruction::AppendLeavesToMerkleTrees { leaves };

    let accounts = crate::accounts::AppendLeaves {
        fee_payer,
        authority,
        registered_program_pda: None,
        system_program: system_program::ID,
    };
    let merkle_tree_account_metas = merkle_tree_pubkeys
        .iter()
        .map(|pubkey| AccountMeta::new(*pubkey, false))
        .collect::<Vec<AccountMeta>>();

    Instruction {
        program_id: crate::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            merkle_tree_account_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    }
}

pub fn create_initialize_address_merkle_tree_and_queue_instruction(
    index: u64,
    payer: Pubkey,
    registered_program_pda: Option<Pubkey>,
    program_owner: Option<Pubkey>,
    merkle_tree_pubkey: Pubkey,
    queue_pubkey: Pubkey,
    address_merkle_tree_config: AddressMerkleTreeConfig,
    address_queue_config: AddressQueueConfig,
) -> Instruction {
    let instruction_data = InitializeAddressMerkleTreeAndQueue {
        index,
        program_owner,
        address_merkle_tree_config,
        address_queue_config,
    };
    let registered_program = match registered_program_pda {
        Some(registered_program_pda) => AccountMeta::new(registered_program_pda, false),
        None => AccountMeta::new(crate::ID, false),
    };
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(merkle_tree_pubkey, false),
            AccountMeta::new(queue_pubkey, false),
            registered_program,
        ],
        data: instruction_data.data(),
    }
}
