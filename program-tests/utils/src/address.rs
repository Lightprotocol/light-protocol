use account_compression::instruction::InsertIntoQueues;
use anchor_lang::{prelude::AccountMeta, InstructionData, ToAccountMetas};
use light_client::rpc::{Rpc, RpcError};
use light_compressed_account::instruction_data::insert_into_queues::InsertIntoQueuesInstructionDataMut;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    transaction::Transaction,
};

pub async fn insert_addresses<R: Rpc>(
    context: &mut R,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    addresses: Vec<[u8; 32]>,
) -> Result<Signature, RpcError> {
    let num_addresses = addresses.len() as u8;
    let mut bytes = vec![
        0u8;
        InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
            0,
            0,
            num_addresses,
            0,
            0,
            1
        )
    ];
    let (ix_data, _) =
        &mut InsertIntoQueuesInstructionDataMut::new_at(&mut bytes, 0, 0, num_addresses, 0, 0, 1)
            .unwrap();
    ix_data.num_address_queues = 1;
    let is_batched = address_queue_pubkey == address_merkle_tree_pubkey;

    for (a_ix, address) in ix_data.addresses.iter_mut().zip(addresses.iter()) {
        a_ix.address = *address;
        a_ix.queue_index = 0;
        a_ix.tree_index = if is_batched { 0 } else { 1 };
    }
    let instruction_data = InsertIntoQueues { bytes };
    let accounts = account_compression::accounts::GenericInstruction {
        authority: context.get_payer().pubkey(),
    };
    let insert_ix = Instruction {
        program_id: account_compression::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            vec![
                AccountMeta::new(address_queue_pubkey, false),
                AccountMeta::new(address_merkle_tree_pubkey, false),
            ],
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let latest_blockhash = context.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[insert_ix],
        Some(&context.get_payer().pubkey()),
        &[&context.get_payer()],
        latest_blockhash.0,
    );
    context.process_transaction(transaction).await
}
