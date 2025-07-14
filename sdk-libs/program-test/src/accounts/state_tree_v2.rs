use anchor_lang::{AnchorSerialize, InstructionData, ToAccountMetas};
use light_batched_merkle_tree::{
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::get_merkle_tree_account_size, queue::get_output_queue_account_size,
};
use light_client::rpc::{Rpc, RpcError};
use light_registry::{
    account_compression_cpi::sdk::create_initialize_batched_merkle_tree_instruction,
    protocol_config::state::ProtocolConfig,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature, Signer};

use crate::utils::create_account::create_account_instruction;

pub async fn create_batched_state_merkle_tree<R: Rpc>(
    payer: &Keypair,
    registry: bool,
    rpc: &mut R,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    cpi_context_keypair: &Keypair,
    params: InitStateTreeAccountsInstructionData,
) -> Result<Signature, RpcError> {
    let queue_account_size = get_output_queue_account_size(
        params.output_queue_batch_size,
        params.output_queue_zkp_batch_size,
    );
    let mt_account_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
    );
    let queue_rent = rpc
        .get_minimum_balance_for_rent_exemption(queue_account_size)
        .await?;
    let create_queue_account_ix = create_account_instruction(
        &payer.pubkey(),
        queue_account_size,
        queue_rent,
        &account_compression::ID,
        Some(queue_keypair),
    );
    let mt_rent = rpc
        .get_minimum_balance_for_rent_exemption(mt_account_size)
        .await?;
    let create_mt_account_ix = create_account_instruction(
        &payer.pubkey(),
        mt_account_size,
        mt_rent,
        &account_compression::ID,
        Some(merkle_tree_keypair),
    );
    let rent_cpi_config = rpc
        .get_minimum_balance_for_rent_exemption(ProtocolConfig::default().cpi_context_size as usize)
        .await?;
    let create_cpi_context_instruction = create_account_instruction(
        &payer.pubkey(),
        ProtocolConfig::default().cpi_context_size as usize,
        rent_cpi_config,
        &Pubkey::from(light_sdk::constants::LIGHT_SYSTEM_PROGRAM_ID),
        Some(cpi_context_keypair),
    );
    let instruction = if registry {
        create_initialize_batched_merkle_tree_instruction(
            payer.pubkey(),
            merkle_tree_keypair.pubkey(),
            queue_keypair.pubkey(),
            cpi_context_keypair.pubkey(),
            params,
        )
    } else {
        let instruction = account_compression::instruction::InitializeBatchedStateMerkleTree {
            bytes: params.try_to_vec().unwrap(),
        };
        let accounts = account_compression::accounts::InitializeBatchedStateMerkleTreeAndQueue {
            authority: payer.pubkey(),
            merkle_tree: merkle_tree_keypair.pubkey(),
            queue: queue_keypair.pubkey(),
            registered_program_pda: None,
        };

        Instruction {
            program_id: account_compression::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction.data(),
        }
    };

    rpc.create_and_send_transaction(
        &[
            create_mt_account_ix,
            create_queue_account_ix,
            create_cpi_context_instruction,
            instruction,
        ],
        &payer.pubkey(),
        &[
            payer,
            merkle_tree_keypair,
            queue_keypair,
            cpi_context_keypair,
        ],
    )
    .await
}
