use crate::{
    invoke::InstructionDataInvoke,
    invoke_cpi::verify_signer::check_program_owner_state_merkle_tree,
    sdk::accounts::{InvokeAccounts, SignerAccounts},
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Bumps};
use light_macros::heap_neutral;

/// 1. Checks that the nullifier queue account is associated with a state Merkle tree account.
/// 2. Checks that if nullifier queue has delegate it invoking_program is delegate.
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
) -> Result<()> {
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
    for account in inputs.input_compressed_accounts_with_merkle_context.iter() {
        let account_info =
            &ctx.remaining_accounts[account.merkle_context.nullifier_queue_pubkey_index as usize];
        accounts.push(AccountMeta {
            pubkey: account_info.key(),
            is_signer: false,
            is_writable: true,
        });
        account_infos.push(account_info.clone());
    }
    inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .try_for_each(|account| -> Result<()> {
            check_program_owner_state_merkle_tree(
                &ctx.remaining_accounts[account.merkle_context.merkle_tree_pubkey_index as usize],
                invoking_program,
            )?;
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
            Ok(())
        })?;
    light_heap::bench_sbf_end!("cpda_insert_nullifiers_prep_accs");
    light_heap::bench_sbf_start!("cpda_instruction_data");

    use anchor_lang::InstructionData;
    let instruction_data = account_compression::instruction::InsertIntoNullifierQueues {
        elements: nullifiers.to_vec(),
    };

    let data = instruction_data.data();
    light_heap::bench_sbf_end!("cpda_instruction_data");

    // [91, 101, 183, 28, 35, 25, 67, 221]
    let bump = &[254];
    let seeds = &[&[b"cpi_authority".as_slice(), bump][..]];
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
    Ok(())
}
