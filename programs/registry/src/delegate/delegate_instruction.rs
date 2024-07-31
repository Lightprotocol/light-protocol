use crate::{protocol_config::state::ProtocolConfigPda, ForesterAccount};

use super::traits::{CompressedCpiContextTrait, SignerAccounts, SystemProgramAccounts};
use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use light_system_program::program::LightSystemProgram;

#[derive(Accounts)]
pub struct DelegatetOrUndelegateInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub protocol_config: Account<'info, ProtocolConfigPda>,
    /// CHECK:
    #[account(
        seeds = [CPI_AUTHORITY_PDA_SEED], bump
        )]
    pub cpi_authority: AccountInfo<'info>,
    /// Forester pda which is being delegated or undelegated to or from.
    #[account(mut)]
    pub forester_pda: Account<'info, ForesterAccount>,
    /// CHECK: (account compression program) as part of light system program invocation.
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: checked in emit_event.rs.
    pub noop_program: AccountInfo<'info>,
    /// CHECK: (account compression program) as part of light system program invocation.
    /// Cpi authority of light system program to invoke account
    /// compression program.
    pub account_compression_authority: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (account compression program) as part of light system program invocation.
    pub system_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::LightRegistry>,
    pub light_system_program: Program<'info, LightSystemProgram>,
}

impl<'info> SystemProgramAccounts<'info> for DelegatetOrUndelegateInstruction<'info> {
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
        unimplemented!()
    }
}

impl<'info> SignerAccounts<'info> for DelegatetOrUndelegateInstruction<'info> {
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

impl<'info> CompressedCpiContextTrait<'info> for DelegatetOrUndelegateInstruction<'info> {
    fn get_cpi_context(&self) -> Option<AccountInfo<'info>> {
        None
    }
}
