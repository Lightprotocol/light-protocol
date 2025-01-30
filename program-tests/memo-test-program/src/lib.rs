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

// TODO: to check ergonomics, add handler with unpack/pack account state.
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;

    if !authority_info.is_signer {
        msg!("Error: Authority must be a signer.");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let message = match std::str::from_utf8(instruction_data) {
        Ok(s) => s,
        Err(_) => "Invalid utf8",
    };

    msg!("Authority Pubkey: {}", authority_info.key);
    msg!("Message: {}", message);

    Ok(())
}
