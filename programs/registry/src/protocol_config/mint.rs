use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint as SplMint, Token, TokenAccount};
use light_compressed_token::program::LightCompressedToken;
use light_macros::pubkey;
use light_system_program::program::LightSystemProgram;

use crate::{
    delegate::traits::{
        CompressedTokenProgramAccounts, MintToAccounts, SignerAccounts, SystemProgramAccounts,
    },
    AUTHORITY_PDA_SEED,
};

use super::state::ProtocolConfigPda;

pub const MINT: Pubkey = pubkey!("2bpg7jkqKDUSxB8dGh3SB4BC2b7JhbgY9cvYzpLP1PcZ");

#[derive(Accounts)]
pub struct Mint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(mut, seeds = [AUTHORITY_PDA_SEED], bump, has_one = authority)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    #[account(mut)]
    pub mint: Account<'info, SplMint>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    /// CHECK:
    pub token_cpi_authority_pda: AccountInfo<'info>,
    pub compressed_token_program: Program<'info, LightCompressedToken>,
    /// CHECK: this account is checked implictly since a mint to from a mint
    /// account to a token account of a different mint will fail
    #[account(mut)]
    pub token_pool_pda: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    /// CHECK: (different program) checked in account compression program
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (different program) checked in system and account compression
    /// programs
    pub noop_program: AccountInfo<'info>,
    /// CHECK: this account in account compression program
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in account compression program
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK: (different program) will be checked by the system program
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> SystemProgramAccounts<'info> for Mint<'info> {
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
        self.light_system_program.to_account_info()
    }
}

impl<'info> SignerAccounts<'info> for Mint<'info> {
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

impl<'info> CompressedTokenProgramAccounts<'info> for Mint<'info> {
    fn get_token_cpi_authority_pda(&self) -> AccountInfo<'info> {
        self.token_cpi_authority_pda.to_account_info()
    }
    fn get_compressed_token_program(&self) -> AccountInfo<'info> {
        self.compressed_token_program.to_account_info()
    }
    fn get_escrow_authority_pda(&self) -> AccountInfo<'info> {
        unimplemented!("escrow authority not implemented");
        // self.cpi_authority.to_account_info()
    }
    fn get_spl_token_program(&self) -> AccountInfo<'info> {
        self.token_program.to_account_info()
    }
    fn get_token_pool_pda(&self) -> AccountInfo<'info> {
        self.token_pool_pda.to_account_info()
    }
    fn get_compress_or_decompress_token_account(&self) -> Option<AccountInfo<'info>> {
        None
    }
}

impl<'info> MintToAccounts<'info> for Mint<'info> {
    fn get_mint(&self) -> AccountInfo<'info> {
        self.mint.to_account_info()
    }
}
