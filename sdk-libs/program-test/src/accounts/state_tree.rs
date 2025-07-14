use account_compression::{
    instruction::InitializeStateMerkleTreeAndNullifierQueue, NullifierQueueConfig,
    StateMerkleTreeConfig,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::rpc::{errors::RpcError, Rpc};
use light_compressed_account::instruction_data::insert_into_queues::InsertIntoQueuesInstructionDataMut;
use light_registry::protocol_config::state::ProtocolConfig;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

use crate::utils::create_account::create_account_instruction;

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
            0,
            0,
        )
    ];
    let (mut ix_data, _) = InsertIntoQueuesInstructionDataMut::new_at(
        &mut bytes,
        leaves.len() as u8,
        0,
        0,
        merkle_tree_pubkeys.len() as u8,
        0,
        0,
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
pub async fn create_state_merkle_tree_and_queue_account<R: Rpc>(
    payer: &Keypair,
    registry: bool,
    rpc: &mut R,
    merkle_tree_keypair: &Keypair,
    nullifier_queue_keypair: &Keypair,
    cpi_context_keypair: Option<&Keypair>,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    index: u64,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) -> Result<Signature, RpcError> {
    use light_registry::account_compression_cpi::sdk::create_initialize_merkle_tree_instruction as create_initialize_merkle_tree_instruction_registry;
    let size = account_compression::state::StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );

    let merkle_tree_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        rpc.get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(merkle_tree_keypair),
    );
    let size =
        account_compression::state::queue::QueueAccount::size(queue_config.capacity as usize)
            .unwrap();
    let nullifier_queue_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        rpc.get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(nullifier_queue_keypair),
    );

    let transaction = if registry {
        let cpi_context_keypair = cpi_context_keypair.unwrap();
        let rent_cpi_config = rpc
            .get_minimum_balance_for_rent_exemption(
                ProtocolConfig::default().cpi_context_size as usize,
            )
            .await
            .unwrap();
        let create_cpi_context_instruction = create_account_instruction(
            &payer.pubkey(),
            ProtocolConfig::default().cpi_context_size as usize,
            rent_cpi_config,
            &Pubkey::from(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID),
            Some(cpi_context_keypair),
        );

        let instruction = create_initialize_merkle_tree_instruction_registry(
            payer.pubkey(),
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            cpi_context_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
            program_owner,
            forester,
        );
        Transaction::new_signed_with_payer(
            &[
                create_cpi_context_instruction,
                merkle_tree_account_create_ix,
                nullifier_queue_account_create_ix,
                instruction,
            ],
            Some(&payer.pubkey()),
            &vec![
                payer,
                merkle_tree_keypair,
                nullifier_queue_keypair,
                cpi_context_keypair,
            ],
            rpc.get_latest_blockhash().await?.0,
        )
    } else {
        let instruction = create_initialize_merkle_tree_instruction(
            payer.pubkey(),
            None,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
            program_owner,
            forester,
            index,
        );
        Transaction::new_signed_with_payer(
            &[
                merkle_tree_account_create_ix,
                nullifier_queue_account_create_ix,
                instruction,
            ],
            Some(&payer.pubkey()),
            &vec![payer, merkle_tree_keypair, nullifier_queue_keypair],
            rpc.get_latest_blockhash().await?.0,
        )
    };

    rpc.process_transaction(transaction).await
}
