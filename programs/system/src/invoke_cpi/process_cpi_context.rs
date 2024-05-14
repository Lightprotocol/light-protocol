use crate::{errors::CompressedPdaError, sdk::accounts::InvokeCpiAccounts};

use super::{
    account::CpiContextAccount, instruction::InvokeCpiInstruction, InstructionDataInvokeCpi,
};
use anchor_lang::prelude::*;

pub fn process_cpi_context<'info>(
    mut inputs: InstructionDataInvokeCpi,
    ctx: &mut Context<'_, '_, '_, 'info, InvokeCpiInstruction<'info>>,
) -> Result<Option<InstructionDataInvokeCpi>> {
    let cpi_context = &inputs.cpi_context;
    if ctx.accounts.get_cpi_context_account().is_some() && cpi_context.is_none() {
        return err!(CompressedPdaError::CpiContextMissing);
    }

    if let Some(cpi_context) = cpi_context {
        let cpi_context_account = match ctx.accounts.get_cpi_context_account() {
            Some(cpi_context_account) => cpi_context_account,
            None => return err!(CompressedPdaError::CpiContextMissing),
        };

        if cpi_context.set_context {
            set_cpi_context(cpi_context_account, inputs);
            return Ok(None);
        } else {
            inputs.combine(&cpi_context_account.context);
        }
    }
    Ok(Some(inputs))
}

pub fn set_cpi_context(
    cpi_context_account: &mut CpiContextAccount,
    inputs: InstructionDataInvokeCpi,
) {
    // Check conditions and modify the signatures
    if cpi_context_account.context.is_empty() {
        msg!("cpi signatures are empty");
        // cpi signature account should only be used with mutiple compressed
        // accounts owned by different programs thus the first invocation
        // execute is assumed to be false
        cpi_context_account.context.push(inputs);
    } else if cpi_context_account.context[0].proof == inputs.proof {
        cpi_context_account.context.push(inputs);
    } else {
        cpi_context_account.context = vec![inputs];
    }
}
