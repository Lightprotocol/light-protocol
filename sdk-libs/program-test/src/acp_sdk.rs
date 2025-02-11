use account_compression::{
    instruction::{
        InitializeAddressMerkleTreeAndQueue, InitializeStateMerkleTreeAndNullifierQueue,
    },
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_account::insert_into_queues::InsertIntoQueuesInstructionDataMut;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

#[allow(clippy::too_many_arguments)]
pub fn create_initialize_merkle_tree_instruction(
    payer: Pubkey,
    registered_program_pda: Option<Pubkey>,
    merkle_tree_pubkey: Pubkey,
    nullifier_queue_pubkey: Pubkey,
    state_merkle_tree_config: StateMerkleTreeConfig,
    nullifier_queue_config: NullifierQueueConfig,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    index: u64,
) -> Instruction {
    let instruction_data = InitializeStateMerkleTreeAndNullifierQueue {
        index,
        program_owner,
        forester,
        state_merkle_tree_config,
        nullifier_queue_config,
        additional_bytes: 0,
    };
    let registered_program = match registered_program_pda {
        Some(registered_program_pda) => AccountMeta::new(registered_program_pda, false),
        None => AccountMeta::new(account_compression::ID, false),
    };
    Instruction {
        program_id: account_compression::ID,
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
    _fee_payer: Pubkey,
    authority: Pubkey,
    merkle_tree_pubkeys: Vec<Pubkey>,
) -> Instruction {
    let mut bytes = vec![
        0u8;
        InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
            leaves.len() as u8,
            0,
            0,
            merkle_tree_pubkeys.len() as u8,
        )
    ];
    let mut ix_data = InsertIntoQueuesInstructionDataMut::new(
        &mut bytes,
        leaves.len() as u8,
        0,
        0,
        merkle_tree_pubkeys.len() as u8,
    )
    .unwrap();
    ix_data.num_output_queues = merkle_tree_pubkeys.len() as u8;
    for (i, (index, leaf)) in leaves.iter().enumerate() {
        ix_data.leaves[i].leaf = *leaf;
        ix_data.leaves[i].account_index = *index;
    }

    let instruction_data = account_compression::instruction::InsertIntoQueues { bytes };

    let accounts = account_compression::accounts::GenericInstruction { authority };
    let merkle_tree_account_metas = merkle_tree_pubkeys
        .iter()
        .map(|pubkey| AccountMeta::new(*pubkey, false))
        .collect::<Vec<AccountMeta>>();

    Instruction {
        program_id: account_compression::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            merkle_tree_account_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_initialize_address_merkle_tree_and_queue_instruction(
    index: u64,
    payer: Pubkey,
    registered_program_pda: Option<Pubkey>,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_pubkey: Pubkey,
    queue_pubkey: Pubkey,
    address_merkle_tree_config: AddressMerkleTreeConfig,
    address_queue_config: AddressQueueConfig,
) -> Instruction {
    let instruction_data = InitializeAddressMerkleTreeAndQueue {
        index,
        program_owner,
        forester,
        address_merkle_tree_config,
        address_queue_config,
    };
    let registered_program = match registered_program_pda {
        Some(registered_program_pda) => AccountMeta::new(registered_program_pda, false),
        None => AccountMeta::new(account_compression::ID, false),
    };
    Instruction {
        program_id: account_compression::ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(merkle_tree_pubkey, false),
            AccountMeta::new(queue_pubkey, false),
            registered_program,
        ],
        data: instruction_data.data(),
    }
}
