use account_compression::{
    instruction::InitializeAddressMerkleTreeAndQueue, AddressMerkleTreeConfig, AddressQueueConfig,
};
use anchor_lang::InstructionData;
use light_client::rpc::{errors::RpcError, Rpc};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

use crate::utils::create_account::create_account_instruction;

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

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub async fn create_address_merkle_tree_and_queue_account<R: Rpc>(
    payer: &Keypair,
    registry: bool,
    context: &mut R,
    address_merkle_tree_keypair: &Keypair,
    address_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
    index: u64,
) -> Result<Signature, RpcError> {
    use light_registry::account_compression_cpi::sdk::create_initialize_address_merkle_tree_and_queue_instruction as create_initialize_address_merkle_tree_and_queue_instruction_registry;

    let size =
        account_compression::state::QueueAccount::size(queue_config.capacity as usize).unwrap();
    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(address_queue_keypair),
    );

    let size = account_compression::state::AddressMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        merkle_tree_config.address_changelog_size as usize,
    );
    let mt_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(address_merkle_tree_keypair),
    );
    let instruction = if registry {
        create_initialize_address_merkle_tree_and_queue_instruction_registry(
            payer.pubkey(),
            forester,
            program_owner,
            address_merkle_tree_keypair.pubkey(),
            address_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
        )
    } else {
        create_initialize_address_merkle_tree_and_queue_instruction(
            index,
            payer.pubkey(),
            None,
            program_owner,
            forester,
            address_merkle_tree_keypair.pubkey(),
            address_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
        )
    };

    let transaction = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(500_000),
            account_create_ix,
            mt_account_create_ix,
            instruction,
        ],
        Some(&payer.pubkey()),
        &vec![&payer, &address_queue_keypair, &address_merkle_tree_keypair],
        context.get_latest_blockhash().await?.0,
    );
    let result = context.process_transaction(transaction).await;
    #[allow(clippy::question_mark)]
    if let Err(e) = result {
        return Err(e);
    }
    result
}
