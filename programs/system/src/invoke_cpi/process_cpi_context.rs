use anchor_lang::prelude::*;

use super::{
    account::CpiContextAccount, instruction::InvokeCpiInstruction, InstructionDataInvokeCpi,
};
use crate::{errors::SystemProgramError, sdk::accounts::InvokeCpiAccounts};

pub fn process_cpi_context<'info>(
    mut inputs: InstructionDataInvokeCpi,
    ctx: &mut Context<'_, '_, '_, 'info, InvokeCpiInstruction<'info>>,
) -> Result<Option<InstructionDataInvokeCpi>> {
    let cpi_context = &inputs.cpi_context;
    if ctx.accounts.get_cpi_context_account().is_some() && cpi_context.is_none() {
        msg!("cpi context account is some but cpi context is none");
        return err!(SystemProgramError::CpiContextMissing);
    }
    if ctx.accounts.get_cpi_context_account().is_none() && cpi_context.is_some() {
        msg!("cpi context account is none but cpi context is some");
        return err!(SystemProgramError::CpiContextAccountUndefined);
    }

    if let Some(cpi_context) = cpi_context {
        let fee_payer = ctx.accounts.fee_payer.key();
        let cpi_context_account = match ctx.accounts.get_cpi_context_account() {
            Some(cpi_context_account) => cpi_context_account,
            None => return err!(SystemProgramError::CpiContextMissing),
        };
        if cpi_context.set_context {
            set_cpi_context(fee_payer, cpi_context_account, inputs)?;
            return Ok(None);
        } else {
            if cpi_context_account.context.is_empty() {
                msg!("cpi context account : {:?}", cpi_context_account);
                msg!("fee payer : {:?}", fee_payer);
                msg!("cpi context  : {:?}", cpi_context);
                return err!(SystemProgramError::CpiContextEmpty);
            } else if cpi_context_account.fee_payer != fee_payer || cpi_context.first_set_context {
                msg!("cpi context account : {:?}", cpi_context_account);
                msg!("fee payer : {:?}", fee_payer);
                msg!("cpi context  : {:?}", cpi_context);
                return err!(SystemProgramError::CpiContextFeePayerMismatch);
            }
            inputs.combine(&cpi_context_account.context);
            cpi_context_account.context = Vec::new();
            cpi_context_account.fee_payer = Pubkey::default();
        }
    }
    Ok(Some(inputs))
}

pub fn set_cpi_context(
    fee_payer: Pubkey,
    cpi_context_account: &mut CpiContextAccount,
    mut inputs: InstructionDataInvokeCpi,
) -> Result<()> {
    // Assumption:
    // - This is safe from someone inserting data in the cpi_context_account
    //   ahead since we require the account to be wiped in the beginning of a
    //   transaction
    // - When implemented correctly there cannot be any leftover data in the
    //   account since if the transaction fails the account doesn't changes

    // Expected usage:
    // 1. The first invocation is marked with
    // No need to store the proof (except in first invokation),
    // cpi context, compress_or_decompress_lamports,
    // relay_fee
    // 2. Subsequent invocations check the proof and fee payer
    if inputs.cpi_context.unwrap().first_set_context {
        clean_input_data(&mut inputs);
        cpi_context_account.context = vec![inputs];
        cpi_context_account.fee_payer = fee_payer;
    } else if fee_payer == cpi_context_account.fee_payer && !cpi_context_account.context.is_empty()
    {
        clean_input_data(&mut inputs);
        cpi_context_account.context.push(inputs);
    } else {
        msg!(" {} != {}", fee_payer, cpi_context_account.fee_payer);
        return err!(SystemProgramError::CpiContextFeePayerMismatch);
    }
    Ok(())
}

fn clean_input_data(inputs: &mut InstructionDataInvokeCpi) {
    inputs.cpi_context = None;
    inputs.compress_or_decompress_lamports = None;
    inputs.relay_fee = None;
    inputs.signer_seeds = Vec::new();
    inputs.proof = None;
}
