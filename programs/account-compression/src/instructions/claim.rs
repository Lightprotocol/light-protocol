use anchor_lang::prelude::*;

use crate::{initialize_nullifier_queue::NullifierQueueAccount, StateMerkleTreeAccount};

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    pub nullifier_queue: AccountLoader<'info, NullifierQueueAccount>,
    /// CHECK: recipient is unchecked
    pub recipient: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: cpi_signer is a pda of account-compression program
    #[account(mut)]
    pub cpi_signer: AccountInfo<'info>,
}

pub fn transfer_lamports_cpi_signed<'info>(
    authority: &AccountInfo<'info>,
    signer: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
    bump: u8,
) -> Result<()> {
    // let bump = Pubkey::find_program_address(&[authority.key().to_bytes().as_slice()], &ID).1;
    let seeds = [&authority.key().to_bytes()[..32], &[bump]];
    let instruction =
        anchor_lang::solana_program::system_instruction::transfer(from.key, to.key, lamports);
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        &[signer.clone(), from.clone(), to.clone()],
        &[seeds.as_slice()],
    )?;
    Ok(())
}
