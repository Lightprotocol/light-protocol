use ark_ed_on_bn254::Fq;
use solana_program;
use anchor_lang::AnchorSerialize;
use ark_ff::PrimeField;
use crate::ErrorCode;
use anchor_lang::prelude::*;

pub fn check_tx_integrity_hash(
    recipient: Vec<u8>,
    ext_amount: Vec<u8>,
    relayer: Vec<u8>,
    fee: Vec<u8>,
    tx_integrity_hash: Vec<u8>,
    merkle_tree_index: u8,
    encrypted_utxos: Vec<u8>,
    merkle_tree_pda_pubkey: Vec<u8>,
) -> Result<()> {
    let input = [
        recipient,
        ext_amount,
        relayer,
        fee,
        merkle_tree_pda_pubkey,
        vec![merkle_tree_index],
        encrypted_utxos,
    ]
    .concat();
    msg!("integrity_hash inputs: {:?}", input);
    let hash = solana_program::keccak::hash(&input[..]).try_to_vec()?;
    msg!("hash computed {:?}", hash);

    if Fq::from_be_bytes_mod_order(&hash[..]) != Fq::from_le_bytes_mod_order(&tx_integrity_hash) {
        msg!("tx_integrity_hash verification failed.{:?} != {:?}", &hash[..] , &tx_integrity_hash);
        return err!(ErrorCode::WrongTxIntegrityHash);
    }
    Ok(())
}
