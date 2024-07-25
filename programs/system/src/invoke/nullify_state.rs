use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Bumps, InstructionData};
use light_macros::heap_neutral;

use crate::{
    constants::CPI_AUTHORITY_PDA_BUMP,
    invoke::InstructionDataInvoke,
    invoke_cpi::verify_signer::check_program_owner_state_merkle_tree,
    sdk::accounts::{InvokeAccounts, SignerAccounts},
};

/// 1. Checks that if nullifier queue has program_owner it invoking_program is
///    program_owner.
/// 2. Inserts nullifiers into the queue.
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
        AccountMeta::new_readonly(account_infos[1].key(), true),
        AccountMeta::new_readonly(account_infos[2].key(), false),
        AccountMeta::new_readonly(account_infos[3].key(), false),
    ];
    // If the transaction contains at least one input compressed account a
    // network fee is paid. This network fee is paid in addition to the address
    // network fee. The network fee is paid once per transaction, defined in the
    // state Merkle tree and transferred to the nullifier queue because the
    // nullifier queue is mutable. The network fee field in the queue is not
    // used.
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
        let account_info =
            &ctx.remaining_accounts[account.merkle_context.merkle_tree_pubkey_index as usize];
        accounts.push(AccountMeta {
            pubkey: account_info.key(),
            is_signer: false,
            is_writable: false,
        });
        account_infos.push(account_info.clone());
    }

    light_heap::bench_sbf_end!("cpda_insert_nullifiers_prep_accs");
    light_heap::bench_sbf_start!("cpda_instruction_data");

    let instruction_data = account_compression::instruction::InsertIntoNullifierQueues {
        nullifiers: nullifiers.to_vec(),
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
