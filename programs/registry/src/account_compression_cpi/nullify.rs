use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, StateMerkleTreeAccount,
};
use anchor_lang::prelude::*;

use crate::{epoch::register_epoch::ForesterEpochPda, errors::RegistryError};

const NULLIFY_2_PROOF_ACCOUNTS_LEN: usize = 16;

#[derive(Accounts)]
pub struct NullifyLeaves<'info> {
    /// CHECK: only eligible foresters can nullify leaves. Is checked in ix.
    #[account(mut)]
    pub registered_forester_pda: Option<Account<'info, ForesterEpochPda>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK: (account compression program) group access control.
    pub registered_program_pda: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (account compression program) when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub nullifier_queue: AccountInfo<'info>,
}

pub fn process_nullify(
    ctx: &Context<NullifyLeaves>,
    bump: u8,
    change_log_indices: Vec<u64>,
    leaves_queue_indices: Vec<u16>,
    indices: Vec<u64>,
    proofs: Vec<Vec<[u8; 32]>>,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::NullifyLeaves {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        nullifier_queue: ctx.accounts.nullifier_queue.to_account_info(),
        fee_payer: Some(ctx.accounts.authority.to_account_info()),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::nullify_leaves(
        cpi_ctx,
        change_log_indices,
        leaves_queue_indices,
        indices,
        proofs,
    )
}

pub fn process_nullify_2(
    ctx: &Context<NullifyLeaves>,
    bump: u8,
    change_log_indices: Vec<u64>,
    leaves_queue_indices: Vec<u16>,
    indices: Vec<u64>,
) -> Result<()> {
    validate_nullify_2_inputs(
        &change_log_indices,
        &leaves_queue_indices,
        &indices,
        ctx.remaining_accounts.len(),
    )?;

    let proof_nodes = extract_proof_nodes_from_remaining_accounts(ctx.remaining_accounts);

    process_nullify(
        ctx,
        bump,
        change_log_indices,
        leaves_queue_indices,
        indices,
        vec![proof_nodes],
    )
}

fn extract_proof_nodes_from_remaining_accounts(
    remaining_accounts: &[AccountInfo<'_>],
) -> Vec<[u8; 32]> {
    remaining_accounts
        .iter()
        .map(|account_info| account_info.key().to_bytes())
        .collect()
}

pub(crate) fn validate_nullify_2_inputs(
    change_log_indices: &[u64],
    leaves_queue_indices: &[u16],
    indices: &[u64],
    proof_accounts_len: usize,
) -> Result<()> {
    if change_log_indices.len() != 1
        || leaves_queue_indices.len() != 1
        || indices.len() != 1
    {
        return err!(RegistryError::InvalidCompactNullifyInputs);
    }
    if proof_accounts_len == 0 {
        return err!(RegistryError::EmptyProofAccounts);
    }
    if proof_accounts_len != NULLIFY_2_PROOF_ACCOUNTS_LEN {
        return err!(RegistryError::InvalidProofAccountsLength);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_nullify_2_inputs;
    use crate::errors::RegistryError;

    #[test]
    fn nullify_2_inputs_validate_happy_path() {
        let result = validate_nullify_2_inputs(&[1], &[1], &[42], 16);
        assert!(result.is_ok());
    }

    #[test]
    fn nullify_2_inputs_reject_empty_proof_accounts() {
        let result = validate_nullify_2_inputs(&[1], &[1], &[42], 0);
        assert_eq!(
            result.err().unwrap(),
            RegistryError::EmptyProofAccounts.into()
        );
    }

    #[test]
    fn nullify_2_inputs_reject_vector_length_mismatch() {
        let result = validate_nullify_2_inputs(&[1, 2], &[1], &[42], 16);
        assert_eq!(
            result.err().unwrap(),
            RegistryError::InvalidCompactNullifyInputs.into()
        );
    }

    #[test]
    fn nullify_2_inputs_reject_invalid_proof_accounts_length() {
        let result = validate_nullify_2_inputs(&[1], &[1], &[42], 15);
        assert_eq!(
            result.err().unwrap(),
            RegistryError::InvalidProofAccountsLength.into()
        );
    }
}
