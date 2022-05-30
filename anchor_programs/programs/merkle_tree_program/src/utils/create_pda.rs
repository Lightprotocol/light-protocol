use anchor_lang::solana_program::{
    system_instruction,
    program::invoke_signed,
    account_info::{AccountInfo},
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::rent::Rent,
};
use std::convert::TryInto;

pub fn create_and_check_pda<'a, 'b>(
  program_id: &Pubkey,
  signer_account: &'a AccountInfo<'b>,
  passed_in_pda: &'a AccountInfo<'b>,
  system_program: &'a AccountInfo<'b>,
  rent: &Rent,
  _instruction_data: &[u8],
  domain_separation_seed: &[u8],
  number_storage_bytes: u64,
  lamports: u64,
  rent_exempt: bool,
) -> Result<(), ProgramError> {
    msg!("trying to derive pda");
  let derived_pubkey =
      Pubkey::find_program_address(&[_instruction_data, domain_separation_seed], program_id);

  if derived_pubkey.0 != *passed_in_pda.key {
      msg!("Passed-in pda pubkey != on-chain derived pda pubkey.");
      msg!("On-chain derived pda pubkey {:?}", derived_pubkey);
      msg!("Passed-in pda pubkey {:?}", *passed_in_pda.key);
      msg!("Instruction data seed  {:?}", _instruction_data);
      return Err(ProgramError::InvalidInstructionData);
  }

  let mut account_lamports = lamports;
  if rent_exempt {
      account_lamports += rent.minimum_balance(number_storage_bytes.try_into().unwrap());
  } else {
      account_lamports += rent.minimum_balance(number_storage_bytes.try_into().unwrap()) / 365;
  }
  msg!("account_lamports: {}", account_lamports);
  invoke_signed(
      &system_instruction::create_account(
          signer_account.key,   // from_pubkey
          passed_in_pda.key,    // to_pubkey
          account_lamports,     // lamports
          number_storage_bytes, // space
          program_id,           // owner
      ),
      &[
          signer_account.clone(),
          passed_in_pda.clone(),
          system_program.clone(),
      ],
      &[&[
          _instruction_data,
          domain_separation_seed,
          &[derived_pubkey.1],
      ]],
  )?;

  // Check for rent exemption
  if rent_exempt
      && !rent.is_exempt(
          **passed_in_pda.lamports.borrow(),
          number_storage_bytes.try_into().unwrap(),
      )
  {
      msg!("Account is not rent exempt.");
      return Err(ProgramError::AccountNotRentExempt);
  }
  Ok(())
}
