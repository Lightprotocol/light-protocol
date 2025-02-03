use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_system_program::{self, program::LightSystemProgram};

use crate::{program::LightCompressedToken, ProcessMintToOrCompressV2Accounts};

// TOOD: verify that source_token_account.authority == authority check is implied
#[derive(Accounts)]
pub struct CompressV2Instruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK:
    /// Authority is verified through proof since both owner and delegate
    /// are included in the token data hash, which is a public input to the
    /// validity proof.
    pub authority: Signer<'info>,
    /// CHECK: (seed constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump,)]
    pub cpi_authority_pda: UncheckedAccount<'info>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    /// CHECK: (different program) checked in account compression program
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: (account compression program) when emitting event.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: (different program) is used to cpi account compression program from light system program.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump, seeds::program = light_system_program::ID)]
    pub account_compression_authority: UncheckedAccount<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:(system program) used to derive cpi_authority_pda and check that
    /// this program is the signer of the cpi.
    pub self_program: Program<'info, LightCompressedToken>,
    #[account(mut)]
    pub token_pool_pda: Option<InterfaceAccount<'info, TokenAccount>>,
    // TODO: check if we need these constraints
    #[account(mut, constraint= if token_pool_pda.is_some() {Ok(token_pool_pda.as_ref().unwrap().key() != source_token_account.key())}else {err!(crate::ErrorCode::TokenPoolPdaUndefined)}? @crate::ErrorCode::IsTokenPoolPda)]
    pub source_token_account: Option<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
    /// CHECK: (different program) will be checked by the system program
    pub merkle_tree: UncheckedAccount<'info>,
    /// CHECK: (different program) will be checked by the system program
    pub sol_pool_pda: Option<AccountInfo<'info>>,
    #[account(
        mut,
        constraint = mint.key() == source_token_account.as_ref().unwrap().mint.key()
            @ crate::ErrorCode::InvalidSourceTokenAccountMint
    )]
    pub mint: InterfaceAccount<'info, Mint>,
}

impl<'info> ProcessMintToOrCompressV2Accounts<'info> for CompressV2Instruction<'info> {
    fn mint(&self) -> &InterfaceAccount<'info, Mint> {
        &self.mint
    }
    fn fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }
    fn sol_pool_pda(&self) -> Option<&AccountInfo<'info>> {
        self.sol_pool_pda.as_ref()
    }
    fn cpi_authority_pda(&self) -> &UncheckedAccount<'info> {
        &self.cpi_authority_pda
    }
    fn registered_program_pda(&self) -> &UncheckedAccount<'info> {
        &self.registered_program_pda
    }
    fn noop_program(&self) -> &UncheckedAccount<'info> {
        &self.noop_program
    }
    fn account_compression_authority(&self) -> &UncheckedAccount<'info> {
        &self.account_compression_authority
    }
    fn account_compression_program(&self) -> &Program<'info, AccountCompression> {
        &self.account_compression_program
    }
    fn self_program(&self) -> &Program<'info, LightCompressedToken> {
        &self.self_program
    }
    fn system_program(&self) -> &Program<'info, System> {
        &self.system_program
    }
    fn merkle_tree(&self) -> &UncheckedAccount<'info> {
        &self.merkle_tree
    }
}
