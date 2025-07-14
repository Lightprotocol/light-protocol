use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    merkle_tree::get_merkle_tree_account_size,
};
use light_client::rpc::{Rpc, RpcError};
use light_registry::account_compression_cpi::sdk::create_initialize_batched_address_merkle_tree_instruction;
use solana_sdk::signature::{Keypair, Signature, Signer};

use crate::utils::create_account::create_account_instruction;

pub async fn create_batch_address_merkle_tree<R: Rpc>(
    rpc: &mut R,
    payer: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    address_tree_params: InitAddressTreeAccountsInstructionData,
) -> Result<Signature, RpcError> {
    let mt_account_size = get_merkle_tree_account_size(
        address_tree_params.input_queue_batch_size,
        address_tree_params.bloom_filter_capacity,
        address_tree_params.input_queue_zkp_batch_size,
        address_tree_params.root_history_capacity,
        address_tree_params.height,
    );
    let mt_rent = rpc
        .get_minimum_balance_for_rent_exemption(mt_account_size)
        .await
        .unwrap();
    let create_mt_account_ix = create_account_instruction(
        &payer.pubkey(),
        mt_account_size,
        mt_rent,
        &account_compression::ID,
        Some(new_address_merkle_tree_keypair),
    );

    let instruction = create_initialize_batched_address_merkle_tree_instruction(
        payer.pubkey(),
        new_address_merkle_tree_keypair.pubkey(),
        address_tree_params,
    );
    rpc.create_and_send_transaction(
        &[create_mt_account_ix, instruction],
        &payer.pubkey(),
        &[payer, new_address_merkle_tree_keypair],
    )
    .await
}
