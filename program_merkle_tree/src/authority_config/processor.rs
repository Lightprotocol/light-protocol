
use solana_program::system_instruction;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
    bpf_loader_upgradeable::UpgradeableLoaderState,
};
use std::convert::{TryFrom, TryInto};

use crate::authority_config::state::AuthorityConfig;
use crate::utils::create_pda::create_and_check_pda;
use crate::utils::config::AUTHORITY_SEED;

// impl Deserialize for UpgradeableLoaderState {
//     fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
//         UpgradeableLoaderState::try_deserialize_unchecked(buf)
//     }

//     fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
//         bincode::deserialize(buf).map_err(|_| ProgramError::InvalidAccountData)
//     }
// }


#[allow(clippy::clone_double_ref)]
pub fn create_authority_config_pda(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    let signer_account = next_account_info(account)?;
    let authority_config_pda = next_account_info(account)?;

    let program_info = next_account_info(account)?;
    let program_data_info = next_account_info(account)?;
    let system_program_info = next_account_info(account)?;
    let rent_sysvar_info = next_account_info(account)?;
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    if *program_info.key != *program_id || program_info.executable == false {
        return Err(ProgramError::IncorrectProgramId);
    }

    let program_state: UpgradeableLoaderState = bincode::deserialize(&program_info.data.borrow()).unwrap();
    match program_state {
        UpgradeableLoaderState::Program {
            programdata_address
        } => {
            if programdata_address != *program_data_info.key {
                return Err(ProgramError::InvalidAccountData);
            }
        }
        _ => {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let program_data_state: UpgradeableLoaderState = bincode::deserialize(&mut program_data_info.data.borrow()).unwrap();
    match program_data_state {
        UpgradeableLoaderState::ProgramData {
            slot,
            upgrade_authority_address, 
        } => {
            if upgrade_authority_address != Some(*signer_account.key) {
                return Err(ProgramError::IllegalOwner);
            }
        }
        _ => {
            return Err(ProgramError::InvalidAccountData);
        }
    }


    msg!("Creating AuthorityConfig started");

    let authority_config = AuthorityConfig::new(
        *signer_account.key,
    )?;
    msg!("Creating AuthorityConfig done");

    create_and_check_pda(
        program_id,
        signer_account,
        authority_config_pda,
        system_program_info,
        rent,
        &program_id.as_ref(),
        AUTHORITY_SEED,
        AuthorityConfig::LEN.try_into().unwrap(),   //bytes
        0,                          //lamports
        true,                       //rent_exempt
    )?;
    msg!("created_pda");
    AuthorityConfig::pack(authority_config, &mut authority_config_pda.data.borrow_mut())
}

#[allow(clippy::clone_double_ref)]
pub fn update_authority_config_pda(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    let signer_account = next_account_info(account)?;
    let authority_config_pda = next_account_info(account)?;

    let derived_pubkey =
        Pubkey::find_program_address(&[program_id.as_ref(), AUTHORITY_SEED], program_id);

    if derived_pubkey.0 != *authority_config_pda.key {
        msg!("Passed-in pda pubkey != on-chain derived pda pubkey.");
        msg!("On-chain derived pda pubkey {:?}", derived_pubkey);
        msg!("Passed-in pda pubkey {:?}", *authority_config_pda.key);
        msg!("ProgramId  {:?}", program_id);
        return Err(ProgramError::InvalidInstructionData);
    }
    msg!("Loading AuthorityConfig started");

    let mut authority_config = AuthorityConfig::unpack(
        &authority_config_pda.data.borrow()
    )?;
    msg!("Loading AuthorityConfig done");

    if *signer_account.key != authority_config.authority_key {
        return Err(ProgramError::IllegalOwner);
    }

    let new_authority = Pubkey::new(&_instruction_data[0..32]);


    authority_config.authority_key = new_authority;

    msg!("updated_pda");
    AuthorityConfig::pack(authority_config, &mut authority_config_pda.data.borrow_mut())
}
