use solana_program::{
    account_info::{next_account_info, AccountInfo},
    declare_id, entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

declare_id!("2nvp5EoMsJ4qVTdTwAmJGfCSRmzRLs16Zw7r3842t2uy");
entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // We expect the first account to be our authority, which must sign.
    msg!("accounts: {:?}", accounts);
    let account_info_iter = &mut accounts.iter();
    msg!("account_info_iter: {:?}", account_info_iter);
    let authority_info = next_account_info(account_info_iter)?;
    msg!("authority_info: {:?}", authority_info);
    // Check that the authority has signed the transaction.
    if !authority_info.is_signer {
        msg!("Error: Authority must be a signer.");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Convert the incoming instruction data (bytes) to a string.
    // If not valid UTF-8, we fallback to "Invalid utf8".
    let message = match std::str::from_utf8(instruction_data) {
        Ok(s) => s,
        Err(_) => "Invalid utf8",
    };

    // Log the authority and the parsed message.
    msg!("Authority Pubkey: {}", authority_info.key);
    msg!("Message: {}", message);

    Ok(())
}
