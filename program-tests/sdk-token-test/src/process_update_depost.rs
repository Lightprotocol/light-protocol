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
    cpi_accounts: &CpiAccounts<'_, 'info>,
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
) -> Result<()> {
    let tree_pubkeys = cpi_accounts.tree_pubkeys().unwrap();
    msg!("tree_pubkeys: {:?}", tree_pubkeys);
    msg!("output_tree_queue_index {:?}", output_tree_queue_index);
    msg!("output_tree_index {:?}", output_tree_index);
    // We want to keep only one escrow compressed token account
    // But ctoken transfers can only have one signer -> we cannot from 2 signers at the same time
    // 1. transfer depositing token to recipient pda -> escrow token account 2
    // 2. merge escrow token account 2 into escrow token account
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
        let leaf_index = output_queue.batch_metadata.next_index as u32;

        let new_input = TokenAccountMeta {
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
        msg!("tree_pubkeys {:?}", tree_pubkeys);
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
        new_input
    };

    {
        msg!("recipient {}", recipient);
        msg!("escrowed_token_meta {:?}", escrowed_token_meta);
        let escrow_account = CTokenAccount::new(
            mint,
            recipient,
            vec![escrowed_token_meta, escrow_token_account_meta_2],
            output_tree_queue_index,
        );
        let total_escrowed_amount = escrow_account.amount;

        let tree_account_infos = cpi_accounts.tree_accounts().unwrap();
        let tree_account_infos = &tree_account_infos[1..];
        let tree_pubkeys = tree_account_infos
            .iter()
            .map(|x| x.pubkey())
            .collect::<Vec<Pubkey>>();
        let cpi_context_pubkey = *cpi_accounts.cpi_context().unwrap().key;
        let transfer_inputs = TransferInputs {
            fee_payer: *cpi_accounts.fee_payer().key,
            sender_account: escrow_account,
            // No validity proof necessary we are just storing state in the cpi context.
            validity_proof: None.into(),
            recipient,
            tree_pubkeys,
            config: Some(TransferConfig {
                cpi_context: Some(CompressedCpiContext {
                    set_context: true,
                    first_set_context: true,
                    cpi_context_account_index: 0, // TODO: replace with Pubkey (maybe not because it is in tree pubkeys 1 in this case)
                }),
                cpi_context_pubkey: Some(cpi_context_pubkey),
                ..Default::default()
            }),
            amount: total_escrowed_amount,
        };
        let instruction =
            light_compressed_token_sdk::instructions::transfer::instruction::transfer(
                transfer_inputs,
            )
            .unwrap();

        let account_infos = [
            &[cpi_accounts.fee_payer().clone(), authority][..],
            remaining_accounts,
        ]
        .concat();
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
    }
    Ok(())
}
