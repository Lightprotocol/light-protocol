#![allow(deprecated)]
use anchor_lang::prelude::*;

pub mod account_traits;
pub mod constants;
pub mod errors;
pub mod instructions;
pub mod utils;
pub use instructions::*;
pub mod cpi_context_account;
use light_compressed_account::instruction_data::{
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    with_readonly::InstructionDataInvokeCpiWithReadOnly,
};

declare_id!("SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7");

#[program]
pub mod light_system_program {
    #![allow(unused_variables)]

    use super::*;

    pub fn init_cpi_context_account(ctx: Context<InitializeCpiContextAccount>) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    pub fn re_init_cpi_context_account(ctx: Context<InitializeCpiContextAccount>) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    pub fn invoke(ctx: Context<InvokeInstruction>, inputs: Vec<u8>) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    pub fn invoke_cpi(ctx: Context<InvokeCpiInstruction>, inputs: Vec<u8>) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }

    pub fn invoke_cpi_with_read_only(
        ctx: Context<InvokeCpiInstruction>,
        inputs: InstructionDataInvokeCpiWithReadOnly,
    ) -> Result<()> {
        unimplemented!("anchor wrapper not implemented")
    }
    pub fn invoke_cpi_with_account_info(
        ctx: Context<InvokeCpiInstruction>,
        inputs: InstructionDataInvokeCpiWithAccountInfo,
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

#[test]
fn test_borsh_equivalence() {
    use anchor_lang::prelude::borsh::BorshSerialize;
    let struct_a = InstructionDataInvokeCpiWithAccountInfo {
        mode: 1,
        bump: 255,
        invoking_program_id: light_compressed_account::pubkey::Pubkey::new_unique(),
        ..Default::default()
    };
    #[derive(BorshSerialize)]
    pub struct AnchorWrapped {
        inputs: InstructionDataInvokeCpiWithAccountInfo,
    }

    let struct_b = AnchorWrapped {
        inputs: struct_a.clone(),
    };

    let struct_a_bytes: Vec<u8> = struct_a.try_to_vec().unwrap();
    let struct_b_bytes: Vec<u8> = struct_b.try_to_vec().unwrap();
    assert_eq!(struct_a_bytes, struct_b_bytes);
}
