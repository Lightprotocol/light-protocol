use anchor_lang::prelude::*;
use anchor_lang::solana_program::log::sol_log_compute_units;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_compressed_account::instruction_data::cpi_context::CompressedCpiContext;
use light_compressed_token_sdk::{
    account::CTokenAccount,
    instructions::transfer::instruction::{TransferConfig, TransferInputs},
    TokenAccountMeta,
};
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiInputs},
    instruction::{account_meta::CompressedAccountMeta, PackedStateTreeInfo, ValidityProof},
    light_account_checks::AccountInfoTrait,
    LightDiscriminator, LightHasher,
};

#[event]
#[derive(Clone, Debug, Default, LightHasher, LightDiscriminator)]
pub struct MyTokenCompressedAccount {
    pub amount: u64,
    #[hash]
    pub owner: Pubkey,
}

pub fn process_update_escrow_pda(
    cpi_accounts: CpiAccounts,
    account_meta: CompressedAccountMeta,
    proof: ValidityProof,
    existing_amount: u64,
    deposit_amount: u64,
) -> Result<()> {
    let mut my_compressed_account = LightAccount::<'_, MyTokenCompressedAccount>::new_mut(
        &crate::ID,
        &account_meta,
        MyTokenCompressedAccount {
            owner: *cpi_accounts.fee_payer().key,
            amount: existing_amount,
        },
    )
    .unwrap();

    my_compressed_account.amount += deposit_amount;

    let cpi_inputs = CpiInputs {
        proof,
        account_infos: Some(vec![my_compressed_account
            .to_account_info()
            .map_err(ProgramError::from)?]),
        new_addresses: None,
        cpi_context: Some(CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            // change to bool works well.
            cpi_context_account_index: 0, // seems to be useless. Seems to be unused.
                                          // TODO: unify the account meta generation on and offchain.
        }),
        ..Default::default()
    };
    msg!("invoke");
    sol_log_compute_units();
    cpi_inputs
        .invoke_light_system_program(cpi_accounts)
        .map_err(ProgramError::from)?;
    sol_log_compute_units();

    Ok(())
}

pub fn deposit_additional_tokens<'info>(
    cpi_accounts: CpiAccounts<'_, 'info>,
    depositing_token_metas: Vec<TokenAccountMeta>,
    escrowed_token_meta: TokenAccountMeta,
    output_tree_index: u8,
    output_tree_queue_index: u8,
    mint: Pubkey,
    recipient: Pubkey,
    recipient_bump: u8,
    amount: u64,
    address: [u8; 32],
    remaining_accounts: &[AccountInfo<'info>],
    authority: AccountInfo<'info>,
    existing_amount: u64,
    account_meta: CompressedAccountMeta,
    proof: ValidityProof,
) -> Result<()> {
    // We want to keep only one escrow compressed token account
    // But ctoken transfers can only have one signer -> we cannot from 2 signers at the same time
    // 1. transfer depositing token to recipient pda -> escrow token account 2
    // 2. update escrow pda balance
    // 3. merge escrow token account 2 into escrow token account
    // Note:
    // - if the escrow pda only stores the amount and the owner we can omit the escrow pda.
    // - the escrowed token accounts are owned by a pda derived from the owner
    //      that is sufficient to verify ownership.
    // - no escrow pda will simplify the transaction, for no cpi context account is required

    // 1.transfer depositing token to recipient pda -> escrow token account 2
    let escrow_token_account_meta_2 = {
        let sender_account = CTokenAccount::new(
            mint,
            *cpi_accounts.fee_payer().key,
            depositing_token_metas,
            output_tree_queue_index,
        );
        // leaf index is the next index in the output queue,
        let output_queue = BatchedQueueAccount::output_from_account_info(
            cpi_accounts
                .get_tree_account_info(output_tree_queue_index as usize)
                .unwrap(),
        )
        .unwrap();
        // SAFETY: state trees are height 32
        let leaf_index = output_queue.batch_metadata.next_index as u32 + 1;

        let escrow_token_account_meta_2 = TokenAccountMeta {
            amount,
            delegate_index: None,
            lamports: None,
            tlv: None,
            packed_tree_info: PackedStateTreeInfo {
                root_index: 0, // not used proof by index
                prove_by_index: true,
                merkle_tree_pubkey_index: output_tree_index,
                queue_pubkey_index: output_tree_queue_index,
                leaf_index,
            },
        };

        let tree_account_infos = cpi_accounts.tree_accounts().unwrap();
        let tree_account_infos = &tree_account_infos[1..];
        let tree_pubkeys = tree_account_infos
            .iter()
            .map(|x| x.pubkey())
            .collect::<Vec<Pubkey>>();
        let cpi_context_pubkey = *cpi_accounts.cpi_context().unwrap().key;
        let transfer_inputs = TransferInputs {
            fee_payer: *cpi_accounts.fee_payer().key,
            sender_account,
            // No validity proof necessary we are just storing state in the cpi context.
            validity_proof: None.into(),
            recipient,
            tree_pubkeys,
            config: Some(TransferConfig {
                cpi_context: Some(CompressedCpiContext {
                    set_context: true,
                    first_set_context: true,
                    // TODO: change to bool and add sanity check that if true account in index 0 is a cpi context pubkey
                    cpi_context_account_index: 0, // TODO: replace with Pubkey (maybe not because it is in tree pubkeys 1 in this case)
                }),
                cpi_context_pubkey: Some(cpi_context_pubkey), // cpi context pubkey is in index 0.
                ..Default::default()
            }),
            amount,
        };
        let instruction =
            light_compressed_token_sdk::instructions::transfer::instruction::transfer(
                transfer_inputs,
            )
            .unwrap();

        let account_infos = [&[cpi_accounts.fee_payer().clone()][..], &remaining_accounts].concat();
        sol_log_compute_units();

        sol_log_compute_units();
        msg!("invoke");
        sol_log_compute_units();
        let seeds = [&b"escrow"[..], &address, &[recipient_bump]];
        anchor_lang::solana_program::program::invoke_signed(
            &instruction,
            account_infos.as_slice(),
            &[&seeds],
        )?;
        sol_log_compute_units();
        escrow_token_account_meta_2
    };
    let tree_account_infos = cpi_accounts.tree_accounts().unwrap()[1..].to_vec();
    let fee_payer = cpi_accounts.fee_payer().clone();

    // 2. Update escrow pda balance
    // - settle tx 1 in the same instruction with the cpi context account
    process_update_escrow_pda(cpi_accounts, account_meta, proof, existing_amount, amount)?;

    // 3. Merge the newly escrowed tokens into the existing escrow account.
    {
        // We remove the cpi context account -> we decrement all packed account indices by 1.
        let mut output_tree_queue_index = output_tree_queue_index;
        output_tree_queue_index -= 1;
        let mut escrowed_token_meta = escrowed_token_meta;
        escrowed_token_meta
            .packed_tree_info
            .merkle_tree_pubkey_index -= 1;
        escrowed_token_meta.packed_tree_info.queue_pubkey_index -= 1;
        let mut escrow_token_account_meta_2 = escrow_token_account_meta_2;
        escrow_token_account_meta_2
            .packed_tree_info
            .merkle_tree_pubkey_index -= 1;
        escrow_token_account_meta_2
            .packed_tree_info
            .queue_pubkey_index -= 1;
        let escrow_account = CTokenAccount::new(
            mint,
            recipient,
            vec![escrowed_token_meta, escrow_token_account_meta_2],
            output_tree_queue_index,
        );

        let total_escrowed_amount = escrow_account.amount;

        let tree_pubkeys = tree_account_infos
            .iter()
            .map(|x| x.pubkey())
            .collect::<Vec<Pubkey>>();
        let transfer_inputs = TransferInputs {
            fee_payer: *fee_payer.key,
            sender_account: escrow_account,
            // No validity proof necessary we are just storing state in the cpi context.
            validity_proof: None.into(),
            recipient,
            tree_pubkeys,
            config: Some(TransferConfig {
                cpi_context: None,
                cpi_context_pubkey: None,
                ..Default::default()
            }),
            amount: total_escrowed_amount,
        };
        let instruction =
            light_compressed_token_sdk::instructions::transfer::instruction::transfer(
                transfer_inputs,
            )
            .unwrap();

        let account_infos = [&[fee_payer, authority][..], remaining_accounts].concat();

        let seeds = [&b"escrow"[..], &address, &[recipient_bump]];
        anchor_lang::solana_program::program::invoke_signed(
            &instruction,
            account_infos.as_slice(),
            &[&seeds],
        )?;
    }
    Ok(())
}
