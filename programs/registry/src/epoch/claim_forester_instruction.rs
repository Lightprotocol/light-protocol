use crate::constants::{FORESTER_EPOCH_SEED, FORESTER_TOKEN_POOL_SEED};
use crate::delegate::traits::MintToAccounts;
use crate::delegate::traits::{
    CompressedCpiContextTrait, CompressedTokenProgramAccounts, SignerAccounts,
    SystemProgramAccounts,
};
use crate::errors::RegistryError;
use crate::{EpochPda, ForesterAccount, ForesterEpochPda};
use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use light_compressed_token::program::LightCompressedToken;
use light_compressed_token::POOL_SEED;
use light_system_program::program::LightSystemProgram;

#[derive(Accounts)]
pub struct ClaimForesterInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK: (seed constraint).
    #[account(
        seeds = [CPI_AUTHORITY_PDA_SEED], bump
        )]
    pub cpi_authority: AccountInfo<'info>,
    // START LIGHT ACCOUNTS
    /// CHECK: (light system program).
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (light system program) in emit_event.rs.
    pub noop_program: AccountInfo<'info>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: checked in cpi_signer_check.
    pub invoking_program: AccountInfo<'info>,
    /// CHECK:
    pub system_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::LightRegistry>,
    /// CHECK:
    pub token_cpi_authority_pda: AccountInfo<'info>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    pub compressed_token_program: Program<'info, LightCompressedToken>,
    pub spl_token_program: Program<'info, Token>,
    // END LIGHT ACCOUNTS
    /// CHECK: (seed constraint).
    /// Pool account for epoch rewards excluding forester fee.
    #[account(mut, seeds = [FORESTER_TOKEN_POOL_SEED, forester_pda.key().as_ref()],bump,)]
    pub forester_token_pool: Account<'info, TokenAccount>,
    #[account(mut, has_one = authority)]
    pub forester_pda: Account<'info, ForesterAccount>,
    /// CHECK: (seed constraint) derived from epoch_pda.epoch and forester_pda.
    #[account(mut, seeds=[FORESTER_EPOCH_SEED, forester_pda.key().as_ref(), epoch_pda.epoch.to_le_bytes().as_ref()], bump ,close=fee_payer)]
    pub forester_epoch_pda: Account<'info, ForesterEpochPda>,
    #[account(mut)]
    pub epoch_pda: Account<'info, EpochPda>,
    #[account(mut, constraint= epoch_pda.protocol_config.mint == mint.key() @RegistryError::InvalidMint)]
    pub mint: Account<'info, Mint>,
    /// CHECK: (checked in different program)
    #[account(mut)]
    pub output_merkle_tree: AccountInfo<'info>,
    #[account(mut, seeds= [POOL_SEED, mint.key().as_ref()], bump, seeds::program= compressed_token_program)]
    pub compression_token_pool: Account<'info, TokenAccount>,
}

impl<'info> SystemProgramAccounts<'info> for ClaimForesterInstruction<'info> {
    fn get_registered_program_pda(&self) -> AccountInfo<'info> {
        self.registered_program_pda.to_account_info()
    }
    fn get_noop_program(&self) -> AccountInfo<'info> {
        self.noop_program.to_account_info()
    }
    fn get_account_compression_authority(&self) -> AccountInfo<'info> {
        self.account_compression_authority.to_account_info()
    }
    fn get_account_compression_program(&self) -> AccountInfo<'info> {
        self.account_compression_program.to_account_info()
    }
    fn get_system_program(&self) -> AccountInfo<'info> {
        self.system_program.to_account_info()
    }
    fn get_sol_pool_pda(&self) -> Option<AccountInfo<'info>> {
        None
    }
    fn get_decompression_recipient(&self) -> Option<AccountInfo<'info>> {
        None
    }
    fn get_light_system_program(&self) -> AccountInfo<'info> {
        self.light_system_program.to_account_info()
    }
    fn get_self_program(&self) -> AccountInfo<'info> {
        self.invoking_program.to_account_info()
    }
}

impl<'info> SignerAccounts<'info> for ClaimForesterInstruction<'info> {
    fn get_fee_payer(&self) -> AccountInfo<'info> {
        self.fee_payer.to_account_info()
    }
    fn get_authority(&self) -> AccountInfo<'info> {
        self.authority.to_account_info()
    }
    fn get_cpi_authority_pda(&self) -> AccountInfo<'info> {
        self.cpi_authority.to_account_info()
    }
}

impl<'info> CompressedTokenProgramAccounts<'info> for ClaimForesterInstruction<'info> {
    fn get_token_cpi_authority_pda(&self) -> AccountInfo<'info> {
        self.token_cpi_authority_pda.to_account_info()
    }
    fn get_compressed_token_program(&self) -> AccountInfo<'info> {
        self.compressed_token_program.to_account_info()
    }
    fn get_escrow_authority_pda(&self) -> AccountInfo<'info> {
        unimplemented!("escrow authority not implemented for ClaimForesterInstruction");
    }
    fn get_spl_token_program(&self) -> AccountInfo<'info> {
        self.spl_token_program.to_account_info()
    }
    fn get_token_pool_pda(&self) -> AccountInfo<'info> {
        self.compression_token_pool.to_account_info()
    }
    fn get_compress_or_decompress_token_account(&self) -> Option<AccountInfo<'info>> {
        None
    }
}
impl<'info> CompressedCpiContextTrait<'info> for ClaimForesterInstruction<'info> {
    fn get_cpi_context(&self) -> Option<AccountInfo<'info>> {
        None
    }
}
impl<'info> MintToAccounts<'info> for ClaimForesterInstruction<'info> {
    fn get_mint(&self) -> AccountInfo<'info> {
        self.mint.to_account_info()
    }
}
