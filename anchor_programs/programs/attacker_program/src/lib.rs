
use anchor_lang::prelude::*;
use anchor_lang::prelude::Pubkey;
use solana_program;
use merkle_tree_program::{
    self,
    program::MerkleTreeProgram,
    utils::config::STORAGE_SEED,
    wrapped_state:: {MerkleTree},
};
declare_id!("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL");
use merkle_tree_program::utils::config::NF_SEED;

#[program]
pub mod attacker_program {
    use super::*;

    pub fn test_nullifier_insert(ctx: Context<TestNullifierInsert>, nullifer: [u8;32]) -> Result<()> {
        let merkle_tree_program_id = ctx.accounts.program_merkle_tree.to_account_info();

        let (address, bump) = solana_program::pubkey::Pubkey::find_program_address(&[merkle_tree_program_id.key().to_bytes().as_ref()], ctx.program_id);
        msg!("find_program_address: {:?}" ,address);
        msg!("ctx.accounts.authority: {:?}" ,ctx.accounts.authority.key());

        let bump = &[bump][..];
        let seed = &merkle_tree_program_id.key().to_bytes()[..];
        let seeds = &[&[seed, bump][..]];
        msg!("seeds: {:?}", seeds);
        // authority.is_signer = true;
        // msg!("authority1 {:?}", authority);
        let accounts = merkle_tree_program::cpi::accounts::InitializeNullifier {
            authority: ctx.accounts.authority.to_account_info(),
            nullifier_pda: ctx.accounts.nullifier0_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(merkle_tree_program_id.clone(), accounts, seeds);
        merkle_tree_program::cpi::initialize_nullifier(
            cpi_ctx,
            nullifer
        ).unwrap();
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(nullifier: [u8;32])]
pub struct TestNullifierInsert<'info> {
    #[account(
        mut,
        seeds = [nullifier.as_ref(), NF_SEED.as_ref()],
        bump,
        seeds::program = MerkleTreeProgram::id(),
    )]
    /// CHECK:` doc comment explaining why no checks through types are necessary
    pub nullifier0_pda: UncheckedAccount<'info>,
    pub program_merkle_tree: Program<'info, MerkleTreeProgram>,
    /// CHECK:` should be a pda
    // #[account(init, payer = signing_address, space=0)]
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK:` doc comment explaining why no checks through types are necessary.
    // #[account(mut)]
    pub signing_address: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>
}

#[error_code]
pub enum ErrorCode {
    #[msg("Incompatible Verifying Key")]
    IncompatibleVerifyingKey,
    #[msg("WrongPubAmount")]
    WrongPubAmount,
    #[msg("PrepareInputsDidNotFinish")]
    PrepareInputsDidNotFinish,
    #[msg("NotLastTransactionState")]
    NotLastTransactionState,
    #[msg("Tx is not a deposit")]
    NotDeposit,
    #[msg("WrongTxIntegrityHash")]
    WrongTxIntegrityHash
}
