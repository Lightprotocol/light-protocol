use crate::constants::FORESTER_TOKEN_POOL_SEED;
use crate::delegate::process_deposit::DelegateAccountWithPackedContext;
use crate::delegate::traits::{CompressedCpiContextTrait, CompressedTokenProgramAccounts};
use crate::delegate::{
    traits::{SignerAccounts, SystemProgramAccounts},
    ESCROW_TOKEN_ACCOUNT_SEED,
};
use crate::protocol_config::state::ProtocolConfigPda;
use crate::ForesterAccount;
use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use light_compressed_token::program::LightCompressedToken;
use light_compressed_token::POOL_SEED;
use light_system_program::program::LightSystemProgram;

#[derive(Accounts)]
#[instruction(salt: u64,delegate_account: DelegateAccountWithPackedContext)]
pub struct SyncDelegateInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(
        seeds = [ESCROW_TOKEN_ACCOUNT_SEED,authority.key().as_ref(), salt.to_le_bytes().as_slice()], bump
        )]
    pub escrow_token_authority: Option<AccountInfo<'info>>,
    /// CHECK:
    #[account(
        seeds = [CPI_AUTHORITY_PDA_SEED], bump
        )]
    pub cpi_authority: AccountInfo<'info>,
    pub protocol_config: Account<'info, ProtocolConfigPda>,
    // START LIGHT ACCOUNTS
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: checked in emit_event.rs.
    pub noop_program: AccountInfo<'info>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:
    pub system_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::LightRegistry>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    /// CHECK:
    pub cpi_context_account: Option<AccountInfo<'info>>,
    pub compressed_token_program: Option<Program<'info, LightCompressedToken>>,
    /// CHECK:
    pub token_cpi_authority_pda: Option<AccountInfo<'info>>,
    pub spl_token_program: Option<Program<'info, Token>>,
    #[account(mut, seeds= [POOL_SEED, protocol_config.config.mint.as_ref()], bump, seeds::program= compressed_token_program.as_ref().unwrap())]
    pub spl_token_pool: Option<Account<'info, TokenAccount>>,
    // END LIGHT ACCOUNTS
    #[account(constraint = forester_pda.key() == delegate_account.delegate_account.delegate_forester_delegate_account.unwrap())]
    pub forester_pda: Account<'info, ForesterAccount>,
    #[account(mut, seeds = [FORESTER_TOKEN_POOL_SEED, forester_pda.key().as_ref()],bump,)]
    pub forester_token_pool: Option<Account<'info, TokenAccount>>,
}

impl<'info> SystemProgramAccounts<'info> for SyncDelegateInstruction<'info> {
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
        self.self_program.to_account_info()
    }
}

impl<'info> SignerAccounts<'info> for SyncDelegateInstruction<'info> {
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
impl<'info> CompressedCpiContextTrait<'info> for SyncDelegateInstruction<'info> {
    fn get_cpi_context(&self) -> Option<AccountInfo<'info>> {
        Some(self.cpi_context_account.as_ref().unwrap().to_account_info())
    }
}

impl<'info> CompressedTokenProgramAccounts<'info> for SyncDelegateInstruction<'info> {
    fn get_token_cpi_authority_pda(&self) -> AccountInfo<'info> {
        self.token_cpi_authority_pda
            .as_ref()
            .unwrap()
            .to_account_info()
    }
    fn get_compressed_token_program(&self) -> AccountInfo<'info> {
        self.compressed_token_program
            .as_ref()
            .unwrap()
            .to_account_info()
    }
    fn get_escrow_authority_pda(&self) -> AccountInfo<'info> {
        self.escrow_token_authority
            .as_ref()
            .unwrap()
            .to_account_info()
    }
    fn get_token_pool_pda(&self) -> AccountInfo<'info> {
        self.spl_token_pool.as_ref().unwrap().to_account_info()
    }
    fn get_spl_token_program(&self) -> AccountInfo<'info> {
        self.spl_token_program.as_ref().unwrap().to_account_info()
    }
    fn get_compress_or_decompress_token_account(&self) -> Option<AccountInfo<'info>> {
        self.forester_token_pool
            .as_ref()
            .map(|account| account.to_account_info())
    }
}
