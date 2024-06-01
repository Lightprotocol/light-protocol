use crate::{errors::CompressedPdaError, sdk::accounts::InvokeCpiContextAccountMut};

use super::{
    account::CpiContextAccount, instruction::InvokeCpiInstruction, InstructionDataInvokeCpi,
};
use anchor_lang::prelude::*;

pub fn process_cpi_context<'info>(
    mut inputs: InstructionDataInvokeCpi,
    ctx: &mut Context<'_, '_, '_, 'info, InvokeCpiInstruction<'info>>,
) -> Result<Option<InstructionDataInvokeCpi>> {
    let cpi_context = &inputs.cpi_context;
    if ctx.accounts.get_cpi_context_account_mut().is_some() && cpi_context.is_none() {
        msg!("cpi context account is some but cpi context is none");
        return err!(CompressedPdaError::CpiContextMissing);
    }
    if ctx.accounts.get_cpi_context_account_mut().is_none() && cpi_context.is_some() {
        msg!("cpi context account is none but cpi context is some");
        return err!(CompressedPdaError::CpiContextAccountUndefined);
    }

    if let Some(cpi_context) = cpi_context {
        let cpi_context_account = match ctx.accounts.get_cpi_context_account_mut() {
            Some(cpi_context_account) => cpi_context_account,
            None => return err!(CompressedPdaError::CpiContextMissing),
        };
        if cpi_context.set_context {
            set_cpi_context(cpi_context_account, inputs);
            return Ok(None);
        } else {
            if cpi_context_account.context[0].proof != inputs.proof {
                return err!(CompressedPdaError::CpiContextProofMismatch);
            } else if cpi_context_account.context.is_empty() {
                return err!(CompressedPdaError::CpiContextEmpty);
            }
            inputs.combine(&cpi_context_account.context);
        }
    }
    Ok(Some(inputs))
}

pub fn set_cpi_context(
    cpi_context_account: &mut CpiContextAccount,
    inputs: InstructionDataInvokeCpi,
) {
    // Check conditions and modify the context.
    // The proof is used as a unique identifier to ensure that the context
    // is only used within the correct transaction.
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
