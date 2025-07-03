use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::{
    error::LightSdkError,
    instruction::{account_meta::CompressedAccountMeta, ValidityProof},
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, pubkey::Pubkey, sysvar::Sysvar,
};

use crate::{
    create_pda::MyCompressedAccount, decompress_to_pda::DecompressedPdaAccount,
    sdk::compress_pda::compress_pda,
};

/// Compresses a PDA back into a compressed account
/// Anyone can call this after the timeout period has elapsed
/// pda check missing yet.
pub fn compress_from_pda<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    msg!("Compressing PDA back to compressed account");

    let mut instruction_data = instruction_data;
    let instruction_data = CompressFromPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let fee_payer = &accounts[0];
    let pda_account = &accounts[1];
    let rent_recipient = &accounts[2]; // can be hardcoded by caller program

    // Verify the PDA account is owned by our program
    if pda_account.owner != &crate::ID {
        msg!("PDA account not owned by this program");
        return Err(LightSdkError::ConstraintViolation);
    }

    compress_pda::<MyCompressedAccount>(
        pda_account,
        &instruction_data.compressed_account_meta,
        Some(instruction_data.proof),
        accounts,
        instruction_data.system_accounts_offset,
        fee_payer,
        crate::LIGHT_CPI_SIGNER,
        &crate::ID,
        rent_recipient,
    )?;

    msg!("Successfully compressed PDA back to compressed account");
    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct CompressFromPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account_meta: CompressedAccountMeta,
    pub system_accounts_offset: u8,
}
