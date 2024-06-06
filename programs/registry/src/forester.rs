use anchor_lang::prelude::*;

use crate::{LightGovernanceAuthority, RegistryError};
use aligned_sized::aligned_sized;

pub const FORESTER_EPOCH_SEED: &[u8] = b"forester_epoch";

#[aligned_sized(anchor)]
#[account]
#[derive(PartialEq, Debug)]
pub struct ForesterEpoch {
    pub authority: Pubkey,
    pub counter: u64,
    pub epoch_start: u64,
    pub epoch_end: u64,
}

#[derive(Accounts)]
#[instruction(bump: u8, authority: Pubkey)]
pub struct RegisterForester<'info> {
    /// CHECK:
    #[account(init, seeds = [FORESTER_EPOCH_SEED, authority.to_bytes().as_slice()], bump, space =ForesterEpoch::LEN , payer = signer)]
    pub forester_epoch_pda: Account<'info, ForesterEpoch>,
    #[account(mut, address = authority_pda.authority)]
    pub signer: Signer<'info>,
    pub authority_pda: Account<'info, LightGovernanceAuthority>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateForesterEpochPda<'info> {
    #[account(address = forester_epoch_pda.authority)]
    pub signer: Signer<'info>,
    /// CHECK:
    #[account(mut)]
    pub forester_epoch_pda: Account<'info, ForesterEpoch>,
}

pub fn check_forester(forester_epoch_pda: &mut ForesterEpoch, authority: &Pubkey) -> Result<()> {
    if forester_epoch_pda.authority != *authority {
        msg!(
            "Invalid forester: forester_epoch_pda authority {} != provided {}",
            forester_epoch_pda.authority,
            authority
        );
        return err!(RegistryError::InvalidForester);
    }
    forester_epoch_pda.counter += 1;
    Ok(())
}

pub fn get_forester_epoch_pda_address(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[FORESTER_EPOCH_SEED, authority.to_bytes().as_slice()],
        &crate::ID,
    )
}
