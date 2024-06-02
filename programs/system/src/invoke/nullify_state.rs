use crate::{
    constants::CPI_AUTHORITY_PDA_BUMP,
    invoke::InstructionDataInvoke,
    invoke_cpi::verify_signer::check_program_owner_state_merkle_tree,
    sdk::accounts::{InvokeAccounts, SignerAccounts},
};
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Bumps, InstructionData};
use light_macros::heap_neutral;

/// 1. Checks that the nullifier queue account is associated with a state Merkle tree account.
/// 2. Checks that if nullifier queue has program_owner it invoking_program is program_owner.
/// 3. Inserts nullifiers into the queue.
#[heap_neutral]
pub fn insert_nullifiers<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    inputs: &'a InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    nullifiers: &'a [[u8; 32]],
    invoking_program: &Option<Pubkey>,
) -> Result<Option<(u8, u64)>> {
    light_heap::bench_sbf_start!("cpda_insert_nullifiers_prep_accs");
    let mut account_infos = vec![
        ctx.accounts.get_fee_payer().to_account_info(),
        ctx.accounts
            .get_account_compression_authority()
            .to_account_info(),
        ctx.accounts.get_registered_program_pda().to_account_info(),
        ctx.accounts.get_system_program().to_account_info(),
    ];
    let mut accounts = vec![
        AccountMeta {
            pubkey: account_infos[0].key(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: account_infos[1].key(),
            is_signer: true,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[2].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta::new_readonly(account_infos[3].key(), false),
    ];
    let mut network_fee_bundle = None;
    for account in inputs.input_compressed_accounts_with_merkle_context.iter() {
        let account_info =
            &ctx.remaining_accounts[account.merkle_context.nullifier_queue_pubkey_index as usize];
        accounts.push(AccountMeta {
            pubkey: account_info.key(),
            is_signer: false,
            is_writable: true,
        });
        account_infos.push(account_info.clone());
        let (_, network_fee, _) = check_program_owner_state_merkle_tree(
            &ctx.remaining_accounts[account.merkle_context.merkle_tree_pubkey_index as usize],
            invoking_program,
        )?;
        if network_fee_bundle.is_none() && network_fee.is_some() {
            network_fee_bundle = Some((
                account.merkle_context.nullifier_queue_pubkey_index,
                network_fee.unwrap(),
            ));
        }
        accounts.push(AccountMeta {
            pubkey: ctx.remaining_accounts
                [account.merkle_context.merkle_tree_pubkey_index as usize]
                .key(),
            is_signer: false,
            is_writable: false,
        });
        account_infos.push(
            ctx.remaining_accounts[account.merkle_context.merkle_tree_pubkey_index as usize]
                .clone(),
        );
    }

    light_heap::bench_sbf_end!("cpda_insert_nullifiers_prep_accs");
    light_heap::bench_sbf_start!("cpda_instruction_data");

    let instruction_data = account_compression::instruction::InsertIntoNullifierQueues {
        elements: nullifiers.to_vec(),
    };

    let data = instruction_data.data();
    light_heap::bench_sbf_end!("cpda_instruction_data");
    let bump = &[CPI_AUTHORITY_PDA_BUMP];
    let seeds = &[&[CPI_AUTHORITY_PDA_SEED, bump][..]];
    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: account_compression::ID,
        accounts,
        data,
    };
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        account_infos.as_slice(),
        seeds,
    )?;
    Ok(network_fee_bundle)
}
