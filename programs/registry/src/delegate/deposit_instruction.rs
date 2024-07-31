use crate::protocol_config::state::ProtocolConfigPda;

use super::{
    traits::{
        CompressedCpiContextTrait, CompressedTokenProgramAccounts, SignerAccounts,
        SystemProgramAccounts,
    },
    ESCROW_TOKEN_ACCOUNT_SEED,
};
use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use light_compressed_token::program::LightCompressedToken;
use light_system_program::program::LightSystemProgram;

#[derive(Accounts)]
#[instruction(salt: u64)]
pub struct DepositOrWithdrawInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK: (seed constraint).
    /// Authority derived from delegate authority and salt.
    #[account(
        seeds = [ESCROW_TOKEN_ACCOUNT_SEED, authority.key().as_ref(), salt.to_le_bytes().as_slice()], bump
        )]
    pub escrow_token_authority: AccountInfo<'info>,
    /// CHECK:
    #[account(
        seeds = [CPI_AUTHORITY_PDA_SEED], bump
        )]
    pub cpi_authority: AccountInfo<'info>,
    pub protocol_config: Account<'info, ProtocolConfigPda>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: checked in emit_event.rs.
    pub noop_program: AccountInfo<'info>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: checked in cpi_signer_check.
    pub invoking_program: AccountInfo<'info>,
    /// CHECK:
    pub system_program: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub cpi_context_account: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::LightRegistry>,
    /// CHECK:
    pub token_cpi_authority_pda: AccountInfo<'info>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    pub compressed_token_program: Program<'info, LightCompressedToken>,
}

impl<'info> SystemProgramAccounts<'info> for DepositOrWithdrawInstruction<'info> {
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

impl<'info> SignerAccounts<'info> for DepositOrWithdrawInstruction<'info> {
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

impl<'info> CompressedTokenProgramAccounts<'info> for DepositOrWithdrawInstruction<'info> {
    fn get_token_cpi_authority_pda(&self) -> AccountInfo<'info> {
        self.token_cpi_authority_pda.to_account_info()
    }
    fn get_compressed_token_program(&self) -> AccountInfo<'info> {
        self.compressed_token_program.to_account_info()
    }
    fn get_escrow_authority_pda(&self) -> AccountInfo<'info> {
        self.escrow_token_authority.to_account_info()
    }
    fn get_token_pool_pda(&self) -> AccountInfo<'info> {
        unimplemented!("escrow authority not implemented for DepositOrWithdrawInstruction");
    }
    fn get_spl_token_program(&self) -> AccountInfo<'info> {
        unimplemented!("escrow authority not implemented for DepositOrWithdrawInstruction");
    }
    fn get_compress_or_decompress_token_account(&self) -> Option<AccountInfo<'info>> {
        None
    }
}
impl<'info> CompressedCpiContextTrait<'info> for DepositOrWithdrawInstruction<'info> {
    fn get_cpi_context(&self) -> Option<AccountInfo<'info>> {
        Some(self.cpi_context_account.to_account_info())
    }
}
