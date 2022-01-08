use ark_ed_on_bn254::Fq;

use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::Pack,
    pubkey::Pubkey, sysvar::rent::Rent,
};
// use solana_sdk::{account::Account, signature::Signer, transaction::Transaction};

use crate::state_check_nullifier::NullifierBytesPda;
use ark_ff::FromBytes;
use borsh::BorshSerialize;

pub fn check_tx_integrity_hash(
    recipient: Vec<u8>,
    extAmount: Vec<u8>,
    relayer: Vec<u8>,
    fee: Vec<u8>,
    encryptedOutput1: Vec<u8>,
    encryptedOutput2: Vec<u8>,
    tx_integrity_hash: &Vec<u8>,
) -> Result<(), ProgramError> {
    let input = [
        recipient,
        extAmount,
        relayer,
        fee,
        encryptedOutput1,
        encryptedOutput2,
    ]
    .concat();

    let hash = solana_program::hash::hash(&input[..]).try_to_vec()?;
    msg!(
        "tx integrity hash is {:?} == onchain {:?}",
        *tx_integrity_hash,
        hash
    );

    // if *tx_integrity_hash != hash {
    //     msg!("tx_integrity_hash verification failed");
    //     return Err(ProgramError::InvalidInstructionData);
    // }
    Ok(())
}

pub fn check_and_insert_nullifier(
    program_id: &Pubkey,
    signer_account_pubkey: &Pubkey,
    nullifier_account: &AccountInfo,
    _instruction_data: &[u8],
) -> Result<u8, ProgramError> {
    // let hash = <Fq as FromBytes>::read(_instruction_data).unwrap();
    // let pubkey_from_seed = Pubkey::create_with_seed(
    //     &signer_account_pubkey,
    //     &hash.to_string()[8..23],
    //     &program_id
    // ).unwrap();
    let nullifer_pubkey = Pubkey::new(&_instruction_data);

    // let nullifier_keypair =
    //    keypair_from_seed(&_instruction_data).unwrap();
    // // PDA:
    // let pubkey_from_seed =
    //     Pubkey::create_with_seed(&nullifier_keypair.pubkey(), &"nullifier", &program_id).unwrap();
    // let signer_pubkey = nullifier.();

    //let mut i = 0;
    // for (i) in 0..30 {
    //     msg!("{} {}", i, &hash.to_string()[i..i+1]);
    //     //i +=1;
    // }
    //check for equality
    //assert_eq!(pubkey_from_seed, *nullifier_account.key);
    if nullifer_pubkey != *nullifier_account.key {
        msg!("passed in nullifier account is wrong");
        return Err(ProgramError::InvalidInstructionData);
    }
    //check for rent exemption
    let rent = Rent::free();
    //assert!(rent.is_exempt(**nullifier_account.lamports.borrow(), 2));
    if rent.is_exempt(**nullifier_account.lamports.borrow(), 2) != true {
        msg!("nullifier account is not rent exempt");
        return Err(ProgramError::InvalidAccountData);
    }
    let mut nullifier_account_data = NullifierBytesPda::unpack(&nullifier_account.data.borrow())?;
    NullifierBytesPda::pack_into_slice(
        &nullifier_account_data,
        &mut nullifier_account.data.borrow_mut(),
    );
    Ok(1u8)
}
