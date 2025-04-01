use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

pub mod account_traits;
pub mod constants;
pub mod errors;
pub mod instructions;
pub mod utils;
pub use instructions::*;
pub mod cpi_context_account;

declare_id!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light_system_program",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}

#[program]
pub mod light_system_program {
    #![allow(unused_variables)]

    use super::*;

    pub fn init_cpi_context_account(ctx: Context<InitializeCpiContextAccount>) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    pub fn invoke<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    pub fn invoke_cpi<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    #[allow(unused_variables)]
    pub fn invoke_cpi_with_read_only<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InvokeCpiInstruction<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    // /// This function is a stub to allow Anchor to include the input types in
    // /// the IDL. It should not be included in production builds nor be called in
    // /// practice.
    // #[cfg(feature = "idl-build")]
    // pub fn stub_idl_build<'info>(
    //     _ctx: Context<'_, '_, '_, 'info, InvokeInstruction<'info>>,
    //     _inputs1: InstructionDataInvoke,
    //     _inputs2: InstructionDataInvokeCpi,
    //     _inputs3: PublicTransactionEvent,
    // ) -> Result<()> {
    //     Err(SystemProgramError::InstructionNotCallable.into())
    // }
}
