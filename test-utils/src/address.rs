use account_compression::instruction::InsertAddresses;
use anchor_lang::{prelude::AccountMeta, system_program, InstructionData, ToAccountMetas};
use light_client::rpc::{RpcConnection, RpcError};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    transaction::Transaction,
};

pub async fn insert_addresses<R: RpcConnection>(
    context: &mut R,
    address_queue_pubkey: Pubkey,
    address_merkle_tree_pubkey: Pubkey,
    addresses: Vec<[u8; 32]>,
) -> Result<Signature, RpcError> {
    let num_addresses = addresses.len();
    let instruction_data = InsertAddresses { addresses };
    let accounts = account_compression::accounts::InsertIntoQueues {
        fee_payer: context.get_payer().pubkey(),
        authority: context.get_payer().pubkey(),
        registered_program_pda: None,
        system_program: system_program::ID,
    };
    let insert_ix = Instruction {
        program_id: account_compression::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            vec![
                vec![
                    AccountMeta::new(address_queue_pubkey, false),
                    AccountMeta::new(address_merkle_tree_pubkey, false)
                ];
                num_addresses
            ]
            .iter()
            .flat_map(|x| x.to_vec())
            .collect::<Vec<AccountMeta>>(),
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let latest_blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[insert_ix],
        Some(&context.get_payer().pubkey()),
        &[&context.get_payer()],
        latest_blockhash,
    );
    context.process_transaction(transaction).await
}
