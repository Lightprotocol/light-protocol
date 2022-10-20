use anchor_lang::{
    prelude::*,
    solana_program::{
        log::sol_log_compute_units,
        self,
        program_pack::{IsInitialized, Pack, Sealed},
    }
};

use anchor_spl::token::{Transfer};
// use spl_token::state::GenericTokenAccount::unpack;
// use spl_token::state::Account::unpack;
use ark_std::{marker::PhantomData, vec::Vec};
use ark_ff::{
    BigInteger256,
    FpParameters,
    PrimeField,
    BigInteger,
    bytes::{
        FromBytes,
        ToBytes
    }
};

use ark_bn254::{
    FrParameters,
    Fr
};
use groth16_solana::groth16::{
    Groth16Verifier,
    Groth16Verifyingkey
};

use merkle_tree_program::{
    close_account,
    utils::create_pda::create_and_check_pda,
};

use std::ops::Neg;


use crate::errors::VerifierSdkError;
use crate::cpi_instructions::{
    initialize_nullifier_cpi,
    insert_two_leaves_cpi,
    withdraw_sol_cpi,
    withdraw_spl_cpi
};
use crate::accounts::Accounts;

fn to_be_64(bytes: &[u8]) -> Vec<u8> {
    let mut vec = Vec::new();
    for b in bytes.chunks(32) {
        for byte in b.iter().rev() {
            vec.push(*byte);
        }
    }
    vec
}

// pub const FEE_TOKEN: Pubkey = solana_program::id();
type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>;


pub trait TxConfig {
	/// Number of nullifiers to be inserted with the transaction.
	const NR_NULLIFIERS: usize;
	/// Number of output utxos.
	const NR_LEAVES: usize;
	/// Number of checked public inputs.
	const NR_CHECKED_PUBLIC_INPUTS: usize;
    const ID: [u8;32];
}
#[derive(Clone)]
pub struct LightTransaction<'info, 'a, 'c, T: TxConfig>  {
    pub merkle_root: &'a [u8; 32],
    pub public_amount: &'a [u8;32],
    pub ext_data_hash: &'a [u8;32],
    pub fee_amount: &'a [u8;32],
    pub mint_pubkey: &'a [u8;32],
    pub checked_public_inputs: Vec<[u8; 32]>,
    pub nullifiers: Vec<Vec<u8>>,
    pub leaves: Vec<[[u8; 32]; 2]>,
    pub relayer_fee: u64,
    pub proof_a: Vec<u8>,
    pub proof_b: Vec<u8>,
    pub proof_c: Vec<u8>,
    pub encrypted_utxos: Vec<u8>,
    pub merkle_root_index: usize,
    pub transferred_funds: bool,
    pub checked_tx_integrity_hash: bool,
    pub verified_proof : bool,
    pub inserted_leaves : bool,
    pub inserted_nullifier : bool,
    pub checked_root : bool,
    pub accounts: Option<&'a Accounts<'info, 'a, 'c>>,
    pub e_phantom: PhantomData<T>,
    pub verifyingkey: &'a Groth16Verifyingkey<'a>
}


impl <T: TxConfig>LightTransaction<'_, '_, '_, T> {

    pub fn new<'info, 'a, 'c> (
        proof: &'a [u8;256],
        merkle_root: &'a [u8; 32],
        public_amount: &'a [u8;32],
        ext_data_hash: &'a [u8;32],
        fee_amount: &'a [u8;32],
        mint_pubkey: &'a [u8;32],
        checked_public_inputs: Vec<[u8; 32]>,
        nullifiers: Vec<Vec<u8>>,
        leaves: Vec<[[u8; 32]; 2]>,
        encrypted_utxos: Vec<u8>,
        relayer_fee: u64,
        merkle_root_index: usize,
        accounts: Option<&'a Accounts<'info, 'a, 'c>>,//Context<'info, LightInstructionTrait<'info>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>
    ) -> LightTransaction<'info, 'a, 'c, T> {
        // assert_eq!(nullifiers.len(), NR_NULLIFIERS);
        // assert_eq!(leaves.len(), NR_LEAVES  / 2);
        // assert_eq!(0, NR_LEAVES  % 2);
        assert_eq!(T::NR_NULLIFIERS, nullifiers.len());
        let proof_a: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&proof[0..64])[..], &[0u8][..]].concat()).unwrap();

        let mut proof_a_neg = [0u8;65];
        <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]).unwrap();

        return LightTransaction {
            merkle_root,
            public_amount,
            ext_data_hash,
            fee_amount,
            mint_pubkey,
            checked_public_inputs,
            nullifiers,
            leaves,
            relayer_fee: relayer_fee,
            proof_a: to_be_64(&proof_a_neg[..64]).to_vec(),
            proof_b: proof[64..64 + 128].to_vec(),
            proof_c: proof[64 + 128..256].to_vec(),
            encrypted_utxos: encrypted_utxos,
            merkle_root_index,
            transferred_funds: false,
            checked_tx_integrity_hash: false,
            verified_proof : false,
            inserted_leaves : false,
            inserted_nullifier : false,
            checked_root : false,
            e_phantom: PhantomData,
            verifyingkey,
            accounts
        }
    }



    pub fn verify(&mut self) -> Result<()> {
        let mut public_inputs = vec![
            self.merkle_root[..].to_vec(),
            self.public_amount[..].to_vec(),
            self.ext_data_hash[..].to_vec(),
            self.fee_amount[..].to_vec(),
            self.mint_pubkey[..].to_vec(),
        ];

        for input in self.nullifiers.iter() {
            public_inputs.push(input.to_vec());
        }

        for input in self.leaves.iter() {
            public_inputs.push(input[0].to_vec());
            public_inputs.push(input[1].to_vec());
        }

        for input in self.checked_public_inputs.iter() {
            public_inputs.push(input.to_vec());
        }

        let mut verifier = Groth16Verifier::new(
            self.proof_a.to_vec(),
            self.proof_b.to_vec(),
            self.proof_c.to_vec(),
            public_inputs,
            &self.verifyingkey
        ).unwrap();
        verifier.verify().unwrap();

        self.verified_proof = true;
        Ok(())
    }

    pub fn check_tx_integrity_hash(&mut self) -> Result<()> {
        msg!("removed merkle tree index");
        let input = [
            self.accounts.unwrap().recipient.key().to_bytes().to_vec(),
            self.accounts.unwrap().recipient_fee.key().to_bytes().to_vec(),
            self.accounts.unwrap().signing_address.key().to_bytes().to_vec(),
            self.relayer_fee.to_le_bytes().to_vec(),
            self.encrypted_utxos.clone()
        ]
        .concat();
        // msg!("recipient: {:?}", self.accounts.unwrap().recipient.key().to_bytes().to_vec());
        // msg!("recipient_fee: {:?}", self.accounts.unwrap().recipient_fee.key().to_bytes().to_vec());
        // msg!("signing_address: {:?}", self.accounts.unwrap().signing_address.key().to_bytes().to_vec());
        // msg!("relayer_fee: {:?}", self.relayer_fee.to_le_bytes().to_vec());
        // msg!("integrity_hash inputs: {:?}", input);
        // msg!("integrity_hash inputs.len(): {}", input.len());
        let hash = anchor_lang::solana_program::keccak::hash(&input[..]).try_to_vec()?;
        msg!("Fq::from_be_bytes_mod_order(&hash[..]) : {}", Fr::from_be_bytes_mod_order(&hash[..]) );
        if Fr::from_be_bytes_mod_order(&hash[..]) != Fr::from_be_bytes_mod_order(self.ext_data_hash) {
            msg!(
                "tx_integrity_hash verification failed.{:?} != {:?}",
                &hash[..],
                &self.ext_data_hash
            );
            return err!(VerifierSdkError::WrongTxIntegrityHash);
        }
        self.checked_tx_integrity_hash = true;
        Ok(())
    }

    pub fn insert_leaves(&mut self) -> Result<()> {

        if T::NR_NULLIFIERS != self.nullifiers.len() {
            msg!("NR_NULLIFIERS  {} != self.nullifiers.len() {}", T::NR_NULLIFIERS, self.nullifiers.len());
            return err!(VerifierSdkError::InvalidNrNullifieraccounts);
        }

        if T::NR_NULLIFIERS + (T::NR_LEAVES / 2) != self.accounts.unwrap().remaining_accounts.len() {
            msg!("NR_NULLIFIERS  {} != self.nullifiers.len() {}", T::NR_NULLIFIERS, self.nullifiers.len());
            return err!(VerifierSdkError::InvalidNrLeavesaccounts);
        }

        // check merkle tree
        for (i, leaves) in self.leaves.iter().enumerate()/*.zip(self.accounts.unwrap().remaining_accounts).skip(NR_NULLIFIERS)*/ {
            // check account integrities

            insert_two_leaves_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().remaining_accounts[T::NR_NULLIFIERS + i].to_account_info(),
                &self.accounts.unwrap().pre_inserted_leaves_index.to_account_info(),
                &self.accounts.unwrap().system_program.to_account_info(),
                &self.accounts.unwrap().rent.to_account_info(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                to_be_64(&leaves[0]).try_into().unwrap(),
                to_be_64(&leaves[0]).try_into().unwrap(),
                to_be_64(&leaves[1]).try_into().unwrap(),
                self.accounts.unwrap().merkle_tree.key(),
                self.encrypted_utxos.clone().try_into().unwrap()
            )?;
        }

        self.inserted_leaves = true;
        return Ok(());
    }

    pub fn check_root(&mut self) -> Result<()> {
        // check account integrities
        msg!("implement rootcheck with index");
        let merkle_tree = self.accounts.unwrap().merkle_tree.load()?;
        if merkle_tree.roots[self.merkle_root_index].to_vec() != to_be_64(self.merkle_root) {
            msg!("self.merkle_root_index: {}", self.merkle_root_index);
            msg!("merkle_tree.roots[self.merkle_root_index].to_vec() {:?}", merkle_tree.roots[self.merkle_root_index].to_vec());
            msg!("to_be_64(self.merkle_root) {:?}", to_be_64(self.merkle_root));
            return err!(VerifierSdkError::InvalidMerkleTreeRoot);
        }
        self.checked_root = true;
        Ok(())
    }

    pub fn insert_nullifiers(&mut self) -> Result<()> {

        if T::NR_NULLIFIERS != self.nullifiers.len() {
            msg!("NR_NULLIFIERS  {} != self.nullifiers.len() {}", T::NR_NULLIFIERS, self.nullifiers.len());
            return err!(VerifierSdkError::InvalidNrNullifieraccounts);
        }

        if T::NR_NULLIFIERS + (T::NR_LEAVES / 2) != self.accounts.unwrap().remaining_accounts.len() {
            msg!("NR_LEAVES / 2  {} != self.leaves.len() {}", T::NR_LEAVES / 2, self.leaves.len());
            return err!(VerifierSdkError::InvalidNrLeavesaccounts);
        }

        for (nullifier, nullifier_pda) in self.nullifiers.iter().zip(self.accounts.unwrap().remaining_accounts) {

            initialize_nullifier_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &nullifier_pda.to_account_info(),
                &self.accounts.unwrap().system_program.to_account_info().clone(),
                &self.accounts.unwrap().rent.to_account_info().clone(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                (nullifier.clone()).try_into().unwrap()
            )?;
        }
        self.inserted_nullifier = true;
        Ok(())
    }

    pub fn transfer_user_funds(&mut self) -> Result<()> {
        // msg!("self.public_amount {:?}", self.public_amount);
        // msg!("self.relayer_fe {:?}", self.relayer_fee);
        msg!("transferring user funds");
        sol_log_compute_units();
        // check mintPubkey
        let (pub_amount_checked, relayer_fee) = self.check_external_amount(
            0,
            0,
            to_be_64(self.public_amount).try_into().unwrap(),
            false
        )?;

        //check accounts
        if self.is_deposit() {
            // sender is user no check
            // recipient is merkle tree pda, check correct derivation

        } else {
            // sender is merkle tree pda check correct derivation

            // recipient is the same as in integrity_hash

        }

        if *self.mint_pubkey == [0u8;32] {
            // either sol withdrawal or normal withdrawal
            // deposit


            if self.is_deposit() {
                // sender is user no check
                // recipient is merkle tree pda, check correct derivation

                let rent = &Rent::from_account_info(&self.accounts.unwrap().rent.to_account_info())?;

                create_and_check_pda(
                    &self.accounts.unwrap().program_id,
                    &self.accounts.unwrap().signing_address.to_account_info(),
                    &self.accounts.unwrap().escrow.to_account_info(),
                    &self.accounts.unwrap().system_program.to_account_info(),
                    &rent,
                    &b"escrow"[..],
                    &Vec::new(),
                    0,                  //bytes
                    pub_amount_checked, //lamports
                    false,               //rent_exempt
                )?;
                close_account(&self.accounts.unwrap().escrow.to_account_info(), &self.accounts.unwrap().recipient.to_account_info())?;

            } else {
                // sender is merkle tree pda check correct derivation
                // recipient is the same as in integrity_hash
                withdraw_sol_cpi(
                    &self.accounts.unwrap().program_id,
                    &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                    &self.accounts.unwrap().authority.to_account_info(),
                    &self.accounts.unwrap().sender.to_account_info(),
                    &self.accounts.unwrap().recipient.to_account_info(),
                    &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                    pub_amount_checked,
                )?;
            }
        } else {
            let recipient_mint = spl_token::state::Account::unpack(&self.accounts.unwrap().recipient.data.borrow())?;
            let sender_mint = spl_token::state::Account::unpack(&self.accounts.unwrap().sender.data.borrow())?;

            // check mint
            if self.mint_pubkey[1..] != recipient_mint.mint.to_bytes()[..31] || self.mint_pubkey[1..] != sender_mint.mint.to_bytes()[..31] {
                msg!("*self.mint_pubkey[..31] {:?}, {:?}, {:?}", self.mint_pubkey[1..].to_vec(), recipient_mint.mint.to_bytes()[..31].to_vec(), sender_mint.mint.to_bytes()[..31].to_vec() );
                return err!(VerifierSdkError::InconsistentMintProofSenderOrRecipient);
            }
            // is a token deposit or withdrawal
            // do I need another token pda check here?

            if self.is_deposit() {

                let seed = merkle_tree_program::ID.to_bytes();
                let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
                    &[seed.as_ref()],
                    self.accounts.unwrap().program_id,
                );
                let bump = &[bump];
                let seeds = &[&[seed.as_slice(), bump][..]];

                let accounts = Transfer {
                    from:       self.accounts.unwrap().sender.to_account_info().clone(),
                    to:         self.accounts.unwrap().recipient.to_account_info().clone(),
                    authority:  self.accounts.unwrap().authority.to_account_info().clone()
                };

                let cpi_ctx = CpiContext::new_with_signer(self.accounts.unwrap().token_program.to_account_info().clone(), accounts, seeds);
                anchor_spl::token::transfer(cpi_ctx, pub_amount_checked)?;
            } else {
                msg!("token authority might be wrong");
                // withdraw_spl_cpi
                withdraw_spl_cpi(
                    &self.accounts.unwrap().program_id,
                    &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                    &self.accounts.unwrap().authority.to_account_info(),
                    &self.accounts.unwrap().sender.to_account_info(),
                    &self.accounts.unwrap().recipient.to_account_info(),
                    &self.accounts.unwrap().token_authority.to_account_info(), // token authority
                    &self.accounts.unwrap().token_program.to_account_info(),
                    &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                    pub_amount_checked
                )?;
            }
        }
        self.transferred_funds = true;
        msg!("transferred");
        sol_log_compute_units();

        Ok(())
    }

    fn is_deposit(&self) -> bool {
        if self.public_amount[24..] != [0u8;8] && self.public_amount[..24] == [0u8;24] {
            return true;
        }
        return false;
    }

    pub fn transfer_fee(&self) -> Result<()> {
        // TODO: check that it is a native account

        // check that it is the native token pool
        msg!("self.relayer_fee: {}", self.relayer_fee);
        msg!("self.fee_amount; {:?}", self.fee_amount);
        let (fee_amount_checked, relayer_fee) = self.check_external_amount(
            0,
            self.relayer_fee,
            to_be_64(self.fee_amount).try_into().unwrap(),
            true
        )?;
        msg!("fee_amount_checked: {}", fee_amount_checked);
        if self.is_deposit() {
            msg!("is deposit");
            let rent = &Rent::from_account_info(&self.accounts.unwrap().rent.to_account_info())?;

            create_and_check_pda(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().signing_address.to_account_info(),
                &self.accounts.unwrap().escrow.to_account_info(),
                &self.accounts.unwrap().system_program.to_account_info(),
                &rent,
                &b"escrow"[..],
                &Vec::new(),
                0,                  //bytes
                fee_amount_checked, //lamports
                false,               //rent_exempt
            )?;
            close_account(&self.accounts.unwrap().escrow.to_account_info(), &self.accounts.unwrap().recipient_fee.to_account_info())?;

        } else {
            // withdraws sol for the user
            withdraw_sol_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().sender_fee.to_account_info(),
                &self.accounts.unwrap().recipient_fee.to_account_info(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                fee_amount_checked,
            )?;

            // pays the relayer fee
            withdraw_sol_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().sender_fee.to_account_info(),
                &self.accounts.unwrap().relayer_recipient.to_account_info(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                relayer_fee,
            )?;

        }

        Ok(())
    }

    pub fn check_completion(&self) -> Result<()>{
        if self.transferred_funds &&
            self.checked_tx_integrity_hash &&
            self.verified_proof &&
            self.inserted_leaves &&
            self.inserted_nullifier &&
            self.checked_root
        {
            return Ok(());
        }
        err!(VerifierSdkError::TransactionIncomplete)
    }

    #[allow(clippy::comparison_chain)]
    pub fn check_external_amount(
            &self,
            ext_amount: i64,
            relayer_fee: u64,
            amount: [u8;32],
            is_fee_token: bool
        ) -> Result<(u64, u64)> {
        // let ext_amount = i64::from_le_bytes(ext_amount);
        // ext_amount includes relayer_fee
        // pub_amount is the public amount included in public inputs for proof verification
        let pub_amount = <BigInteger256 as FromBytes>::read(&amount[..]).unwrap();
        msg!("pub_amount: {:?}", pub_amount);
        if pub_amount.0[0] > 0 && pub_amount.0[1] == 0 && pub_amount.0[2] == 0 && pub_amount.0[3] == 0 {
            if pub_amount.0[1] != 0 || pub_amount.0[2] != 0 || pub_amount.0[3] != 0 {
                msg!("Public amount is larger than u64.");
                return Err(VerifierSdkError::WrongPubAmount.into());
            }
            msg!("entered deposit");
            let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);

            if pub_amount_fits_i64.is_err() {
                msg!("Public amount is larger than i64.");
                return Err(VerifierSdkError::WrongPubAmount.into());
            }

            //check amount
            if pub_amount.0[0].checked_add(relayer_fee).unwrap() != ext_amount as u64 && ext_amount > 0 && relayer_fee > 0 {
                msg!(
                    "Deposit invalid external amount (relayer_fee) {} != {}",
                    pub_amount.0[0] + relayer_fee,
                    ext_amount
                );
                return Err(VerifierSdkError::WrongPubAmount.into());
            }
            msg!("returning public amount");
            Ok((pub_amount.0[0], relayer_fee))
        } else if pub_amount.0[1] > 0 && pub_amount.0[2] > 0 && pub_amount.0[3] > 0 {
            // calculate ext_amount from pubAmount:
            let mut field = FrParameters::MODULUS;
            field.sub_noborrow(&pub_amount);

            // field.0[0] is the positive value
            if field.0[1] != 0 || field.0[2] != 0 || field.0[3] != 0 {
                msg!("Public amount is larger than u64.");
                return Err(VerifierSdkError::WrongPubAmount.into());
            }
            let pub_amount_fits_i64 = i64::try_from(pub_amount.0[0]);
            if pub_amount_fits_i64.is_err() {
                msg!("Public amount is larger than i64.");
                return Err(VerifierSdkError::WrongPubAmount.into());
            }

            if field.0[0]
                < relayer_fee && is_fee_token
            {
                msg!(
                    "Withdrawal invalid relayer_fee: {} < {}",
                    pub_amount.0[0],
                    relayer_fee
                );
                return Err(VerifierSdkError::WrongPubAmount.into());
            }

            // if field.0[0]
            //     != u64::try_from(-ext_amount)
            //         .unwrap()
            //         .checked_add(relayer_fee)
            //         .unwrap() && is_fee_token
            // {
            //     msg!(
            //         "Withdrawal invalid external amount: {} != {}",
            //         pub_amount.0[0],
            //         relayer_fee + u64::try_from(-ext_amount).unwrap()
            //     );
            //     return Err(VerifierSdkError::WrongPubAmount.into());
            // }
            let pub_amount = field.0[0] - relayer_fee;
            msg!("pub amount: {}, relayer fee {}",pub_amount, relayer_fee);
            Ok((pub_amount, relayer_fee))
        }
        //  else if ext_amount == 0 {
        //     Ok((ext_amount.try_into().unwrap(), relayer_fee))
        // }
        else {
            msg!("Invalid state checking external amount.");
            Err(VerifierSdkError::WrongPubAmount.into())
        }
    }

}

/*
#[cfg(test)]
mod test {
    use super::*;
    use ark_ff::{
        BigInteger,
        bytes::{FromBytes, ToBytes},
        Fp256
    };
    use ark_ec::AffineCurve;
    use ark_ec::ProjectiveCurve;
    use std::ops::AddAssign;
    use ark_ff::FpParameters;
    use std::ops::MulAssign;
    use ark_ff::BigInteger256;
    use ark_std::{Zero, One};


    #[test]
    fn test_multiplication() {

        let public_inputs = [231,174,226,37,211,160,187,178,149,82,17,60,110,116,28,61,58,145,58,71,25,42,67,46,189,214,248,234,182,251,238,34,0,202,154,59,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,225,157,11,252,221,230,8,141,243,173,43,5,181,92,233,158,1,49,222,73,181,162,6,187,38,215,115,133,129,28,41,33,64,66,15,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,31,11,137,87,252,84,250,28,95,106,202,3,89,36,51,65,87,13,68,84,74,168,117,74,173,9,245,77,76,208,8,43,9,104,56,69,0,210,158,191,124,224,87,221,60,245,64,77,144,7,188,85,172,210,50,118,177,19,152,107,59,12,91,18,91,254,46,62,123,95,171,253,40,21,64,207,111,160,248,60,12,79,137,212,36,211,220,186,107,150,211,98,38,138,17,11,6,157,54,154,53,7,47,129,189,27,245,196,6,142,80,113,42,122,200,199,126,246,182,237,223,200,251,91,92,40,239,9];
        let input_mul_bytes = [to_be_64(&vk_ic[1]).to_vec(), vec![0u8;32]].concat();

        let mul_res_syscall = alt_bn128_multiplication(&input_mul_bytes[..]).unwrap();
        let input_addition_bytes= [to_be_64(&vk_ic[0]).to_vec(), mul_res_syscall.clone().to_vec()].concat();

        let addition_res_syscall = alt_bn128_addition(&input_addition_bytes[..]).unwrap();

        let mut g_ic = <G1 as FromBytes>::read(&*[&vk_ic[0][..], &[0u8][..]].concat()).unwrap();

        let mut g_ic_1 = <G1 as FromBytes>::read(&*[&vk_ic[2][..], &[0u8][..]].concat()).unwrap().into_projective();
        // BigInteger256::new([0,0,0,0]).into()
        g_ic_1.mul_assign(Fp256::<ark_bn254::FrParameters>::zero());
        let mut mul_res_ark_bytes = [0u8;64];
        <G1 as ToBytes>::write(&g_ic_1.into(),&mut mul_res_ark_bytes[..]);
        // BigInteger256::zero();
        // g_ic.add_assign(&g_ic_1);
        println!("p ark {:?}", g_ic);
        println!("q ark {:?}", g_ic_1.into_affine());
        let res = g_ic + g_ic_1.into_affine();
        let mut addition_res_ark_bytes = [0u8;64];
        <G1 as ToBytes>::write(&res.into(),&mut addition_res_ark_bytes[..]);
        println!("mul_res_syscall{:?}", mul_res_syscall);
        println!("to_be_64(&mul_res_ark_bytes[..]) {:?}",to_be_64(&mul_res_ark_bytes[..]) );
        assert_eq!(mul_res_syscall, to_be_64(&mul_res_ark_bytes[..]));
        assert_eq!(addition_res_syscall, to_be_64(&addition_res_ark_bytes[..]));
        println!("g1 zero{:?}",G1::zero() );

        // g_ic.add_assign(&b.mul(scalar.into_repr()));


    }
    type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>;
    type G1p = ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>;
    type G2 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g2::Parameters>;

    #[test]
    fn test_groth16_verification() {

        // original public inputs the all 0 element throws a group error
        // let public_inputs = [34,238,251,182,234,248,214,189,46,67,42,25,71,58,145,58,61,28,116,110,60,17,82,149,178,187,160,211,37,226,174,231,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,59,154,202,0,43,223,170,106,86,191,3,134,169,166,97,179,10,139,71,201,124,116,122,168,7,166,16,82,87,87,55,138,100,65,144,63,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,15,66,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,42,193,192,156,15,46,99,214,68,44,64,245,153,95,88,47,59,97,174,9,81,73,224,59,175,90,81,176,130,35,75,65,29,25,86,66,122,132,239,36,216,86,2,150,23,205,25,62,124,65,157,152,212,7,0,36,58,27,199,147,203,0,75,247,17,165,151,106,130,197,203,27,237,151,250,137,37,238,192,5,166,225,6,33,133,86,177,4,157,118,125,201,22,195,106,9,41,29,214,42,35,223,191,115,24,160,192,52,55,2,154,201,186,194,34,3,155,134,210,36,91,144,30,243,80,76,197,199];
        println!("{:?}",Fq::zero().into_repr().to_bytes_be() );
        let public_inputs = [34,238,251,182,234,248,214,189,46,67,42,25,71,58,145,58,61,28,116,110,60,17,82,149,178,187,160,211,37,226,174,231,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,59,154,202,0,17,5,192,204,2,243,79,210,29,182,212,226,240,137,53,73,145,50,226,160,164,78,236,246,92,34,161,201,84,83,101,246,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,15,66,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,20,66,157,100,204,79,6,203,25,53,193,48,66,197,84,169,97,31,70,54,150,204,162,133,78,192,152,90,179,50,27,61,35,225,126,79,110,121,27,239,65,55,42,135,141,226,196,86,76,197,43,108,83,141,218,92,206,197,180,6,35,146,190,217,32,237,108,29,147,0,45,108,178,182,216,135,120,162,105,59,219,237,211,2,150,14,241,15,161,182,178,46,42,230,246,12,31,136,211,135,126,239,49,29,239,109,125,103,216,179,48,173,197,154,212,243,94,253,188,114,83,16,116,158,66,237,98,253];
        let proof = [32,81,3,142,46,160,165,147,183,128,61,106,49,182,204,176,237,55,160,156,173,44,137,54,51,179,116,55,108,64,62,211,0,16,68,248,207,185,88,210,7,214,155,69,15,254,237,64,101,106,40,44,28,210,14,180,10,238,244,108,159,7,131,183,30,41,7,90,120,134,3,249,13,230,173,46,54,98,96,130,108,78,152,13,166,145,215,118,148,186,82,129,145,194,209,24,13,151,119,20,241,30,150,215,26,211,45,149,73,211,138,90,44,191,70,100,58,1,35,71,158,163,33,66,211,44,179,36,4,217,46,128,69,35,39,220,36,131,96,225,190,122,27,8,151,241,171,144,75,233,13,0,190,37,25,52,65,90,245,79,13,221,252,140,182,101,208,225,172,188,237,80,101,85,148,218,67,247,20,194,253,56,0,192,230,170,15,58,178,240,105,81,43,133,107,239,178,29,180,149,177,37,6,73,162,30,96,33,96,235,249,198,168,51,204,89,94,184,81,198,175,67,173,93,47,116,232,166,155,67,104,125,214,53,75,190,249,119,138,16,134,81,226,217,118,130,81,166,50,31,255,28,96,124,139,10];
        // let mut g_ic = <G1 as FromBytes>::read(&*[&*to_be_64(&vk_ic[0][..]), &[0u8][..]].concat()).unwrap().into_projective();
        // for (i, input) in public_inputs.chunks(32).enumerate() {
        //     if i != 0{
        //         let scalar = <Fq as FromBytes>::read(&*to_be_64(&input[..])).unwrap();
        //         let b = <G1 as FromBytes>::read(&*[&*to_be_64(&vk_ic[i][..]), &[0u8][..]].concat()).unwrap().into_projective();
        //         g_ic.add_assign(&b.mul(scalar.into_repr()));
        //     }
        // }
        // let mut public_inputs_ark = Vec::new();
        // for (i, input) in public_inputs.chunks(32).enumerate() {
        //     let scalar = <Fp256<ark_bn254::FqParameters> as FromBytes> ::read(&input[..]).unwrap();
        //     public_inputs_ark.push(scalar);
        // }
        // let prepared_inputs = prepare_inputs(&pvk, &public_inputs_ark[..]).unwrap();
        let mut g_ic = <G1 as FromBytes>::read(&*[&to_be_64(&vk_ic[0])[..], &[0u8][..]].concat()).unwrap().into_projective();
        // for (i, input) in public_inputs.chunks(32).enumerate() {
        //     // let scalar = <Fr as FromBytes>::read(&input[..]).unwrap();
        //     println!("g_ic{}", g_ic);
        //
        //     let scalar = <Fp256<ark_bn254::FqParameters> as FromBytes> ::read(&input[..]).unwrap();
        //     // let scalar = <Fq as FromBytes>::read(&*to_be_64(&input[..])).unwrap();
        //     let b = <G1 as FromBytes>::read(&*[&vk_ic[i+1][..], &[0u8][..]].concat()).unwrap().into_projective();
        //     println!("b {}", b.into_affine());
        //     println!("scalar{}", scalar);
        //     g_ic.add_assign(&b.mul(scalar.into_repr()));
        //
        //
        // }
        let mut g_ic_affine_bytes = [0u8;64];
        <G1 as ToBytes>::write(&g_ic.into_affine(),&mut g_ic_affine_bytes[..]);

        // let mut g_ic_bytes :[u8;96] = [0u8;96];
        // <G1p as ToBytes>::write(&g_ic,&mut g_ic_bytes[..]);
        // assert_eq!(snarkjs_public_inputs, g_ic_bytes);

        // let mut g_ic = pvk.vk.gamma_abc_g1[0].into_projective();
        // for (i, b) in public_inputs.iter().zip(pvk.vk.gamma_abc_g1.iter().skip(1)) {
        //     g_ic.add_assign(&b.mul(i.into_repr()));
        // }
        // let snarkjs_public_inputs_be = to_be_64(&snarkjs_public_inputs[..]);
        let mut public_inputs_res_bytes = vk_ic[0];
        for (i, input) in public_inputs.chunks(32).enumerate() {
            let scalar = <Fp256<ark_bn254::FqParameters> as FromBytes> ::read(&*to_be_64(&input[..])).unwrap();
            let b = <G1 as FromBytes>::read(&*[&to_be_64(&vk_ic[i+1])[..], &[0u8][..]].concat()).unwrap().into_projective();
            println!("b {:?}", b.into_affine());
            println!("scalar {:?}", scalar);
            println!("p ark {:?}", g_ic);
            let mul_res_ark = b.mul(scalar.into_repr());
            println!("mul_res_ark {:?}", mul_res_ark.into_affine());
            g_ic.add_assign(&mul_res_ark);


            let input_mul_bytes = [vk_ic[i+1].to_vec(), input.to_vec()].concat();
            let mul_res = alt_bn128_multiplication(&input_mul_bytes[..]).unwrap();
            println!("mul_res {:?}",<G1 as FromBytes>::read(&*to_be_64(&mul_res[..])) );
            let input_addition_bytes= [mul_res, public_inputs_res_bytes.to_vec()].concat();
            // .add_assign(&b.mul(i.into_repr()));
            public_inputs_res_bytes = alt_bn128_addition(&input_addition_bytes[..]).unwrap().try_into().unwrap();
            println!("public_inputs_res_bytes {:?}",<G1 as FromBytes>::read(&*[&*to_be_64(&public_inputs_res_bytes[..]).to_vec(), &vec![0]].concat()).unwrap() );
            println!("iteration {}",i);
            assert_eq!(<G1 as FromBytes>::read(&*[&*to_be_64(&public_inputs_res_bytes[..]).to_vec(), &vec![0]].concat()).unwrap(), g_ic);
        }
        // assert_eq!(public_inputs_res_bytes, to_be_64(&g_ic_affine_bytes));
        // assert_eq!(snarkjs_public_inputs_be, public_inputs_res_bytes);
        println!("public_inputs_res_bytes: {:?}", public_inputs_res_bytes);
        // let mut affine_public_inputs_snarkjs_rs_bytes = [0u8;64];
        // let affine_public_inputs_snarkjs_rs = <G1p as FromBytes>::read(&*[snarkjs_public_inputs.to_vec(), vec![0u8]].concat()).unwrap().into_affine();
        // println!("affine_public_inputs_snarkjs_rs {:?}", affine_public_inputs_snarkjs_rs);
        // assert!(affine_public_inputs_snarkjs_rs.is_on_curve(), "not on curve");
        // <G1 as ToBytes>::write(&affine_public_inputs_snarkjs_rs, &mut affine_public_inputs_snarkjs_rs_bytes[..]);
        println!("public_inputs_res_bytes {:?}", public_inputs_res_bytes);
        // println!("affine_public_inputs_snarkjs_rs_bytes {:?}", affine_public_inputs_snarkjs_rs_bytes);


       // // assert!(pairing_res == 1);
       // println!("{:?}",pairing_res);
       let proof_a: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&proof[0..64])[..], &[0u8][..]].concat()).unwrap();
       let proof_b: G2 =  <G2 as FromBytes>::read(&*[&to_be_128(&proof[64..192])[..], &[0u8][..]].concat()).unwrap();

       let g_ic: G1 = g_ic.into();
       let gamma_g2_neg_pc: G2 =  <G2 as FromBytes>::read(&*[&to_be_128(&vk_gamme_g2)[..], &[0u8][..]].concat()).unwrap();

       let delta_g2_neg_pc: G2 =  <G2 as FromBytes>::read(&*[&to_be_128(&vk_delta_g2)[..], &[0u8][..]].concat()).unwrap();
       let proof_c: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&proof[192..256])[..], &[0u8][..]].concat()).unwrap();

       let alpha_g1: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&vk_alpha_g1)[..], &[0u8][..]].concat()).unwrap();
       let beta_g2 : G2 =  <G2 as FromBytes>::read(&*[&to_be_128(&vk_beta_g2)[..], &[0u8][..]].concat()).unwrap();


       let miller_output_ref =
       <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
           [
               (proof_a.neg().into(), proof_b.into()),
               (
                   g_ic.into(),
                   gamma_g2_neg_pc.clone().into(),
               ),
               (proof_c.into(), delta_g2_neg_pc.clone().into()),
               (alpha_g1.into(), beta_g2.into())
           ]
           .iter(),
       );
       let fe_output_ref = <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&miller_output_ref);
       println!("fe_output_ref {:?}", fe_output_ref);
       type GT = <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk;

       assert_eq!(fe_output_ref.unwrap(),GT::one());

       let mut proof_a_neg = [0u8;64];
       <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]);

       let pairing_input = [
           to_be_64(&proof_a_neg).to_vec(), // proof_a
           proof[64..64 + 128].to_vec(), // proof_b
           public_inputs_res_bytes.to_vec(),
           vk_gamme_g2.to_vec(),
           proof[64 + 128..256].to_vec(), // proof_c
           vk_delta_g2.to_vec(),
           vk_alpha_g1.to_vec(),
           vk_beta_g2.to_vec(),
       ].concat();
       let pairing_res = alt_bn128_pairing(&pairing_input[..]).unwrap();
       assert_eq!(pairing_res[31], 1);
    }

    #[test]
    fn test_groth16_struct_verification() {
        let public_inputs = [34,238,251,182,234,248,214,189,46,67,42,25,71,58,145,58,61,28,116,110,60,17,82,149,178,187,160,211,37,226,174,231,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,59,154,202,0,17,5,192,204,2,243,79,210,29,182,212,226,240,137,53,73,145,50,226,160,164,78,236,246,92,34,161,201,84,83,101,246,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,15,66,64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,20,66,157,100,204,79,6,203,25,53,193,48,66,197,84,169,97,31,70,54,150,204,162,133,78,192,152,90,179,50,27,61,35,225,126,79,110,121,27,239,65,55,42,135,141,226,196,86,76,197,43,108,83,141,218,92,206,197,180,6,35,146,190,217,32,237,108,29,147,0,45,108,178,182,216,135,120,162,105,59,219,237,211,2,150,14,241,15,161,182,178,46,42,230,246,12,31,136,211,135,126,239,49,29,239,109,125,103,216,179,48,173,197,154,212,243,94,253,188,114,83,16,116,158,66,237,98,253];
        let proof = [32,81,3,142,46,160,165,147,183,128,61,106,49,182,204,176,237,55,160,156,173,44,137,54,51,179,116,55,108,64,62,211,0,16,68,248,207,185,88,210,7,214,155,69,15,254,237,64,101,106,40,44,28,210,14,180,10,238,244,108,159,7,131,183,30,41,7,90,120,134,3,249,13,230,173,46,54,98,96,130,108,78,152,13,166,145,215,118,148,186,82,129,145,194,209,24,13,151,119,20,241,30,150,215,26,211,45,149,73,211,138,90,44,191,70,100,58,1,35,71,158,163,33,66,211,44,179,36,4,217,46,128,69,35,39,220,36,131,96,225,190,122,27,8,151,241,171,144,75,233,13,0,190,37,25,52,65,90,245,79,13,221,252,140,182,101,208,225,172,188,237,80,101,85,148,218,67,247,20,194,253,56,0,192,230,170,15,58,178,240,105,81,43,133,107,239,178,29,180,149,177,37,6,73,162,30,96,33,96,235,249,198,168,51,204,89,94,184,81,198,175,67,173,93,47,116,232,166,155,67,104,125,214,53,75,190,249,119,138,16,134,81,226,217,118,130,81,166,50,31,255,28,96,124,139,10];
        // order of publicInputs
        // let mut public_inputs = vec![
        //     self.merkle_root,
        //     self.public_amount,
        //     self.ext_data_hash,
        //     self.fee_amount,
        //     self.mint_pubkey,
        //     self.nullifier0,
        //     self.nullifier1,
        //     self.leaf_left,
        //     self.leaf_right
        // ];
        let pub_amount = <BigInteger256 as FromBytes>::read(&to_be_64(&public_inputs[32..64])[..]).unwrap().0[0];
        println!("pub amount {}", pub_amount);
        /*
        let mut tx = LightTransaction::new(
                proof,
                public_inputs[0..32].try_into().unwrap(),// merkle_root,
                public_inputs[32..64].try_into().unwrap(),// merkle_root,public_amount: [u8;32],
                public_inputs[64..96].try_into().unwrap(),// ext_data_hash: [u8;32],
                public_inputs[96..128].try_into().unwrap(),// fee_amount,public_amount: [u8;32],
                public_inputs[128..160].try_into().unwrap(),// mint_pubkey,public_amount: [u8;32],
                Vec::<Vec<u8>>::new(),//checked_public_inputs: Vec<Vec<u8>>,
                vec![public_inputs[160..192].to_vec(), public_inputs[192..224].to_vec()], // nullifiers:
                vec![(public_inputs[224..256].to_vec(), public_inputs[256..288].to_vec())], // leaves: Vec<(Vec<u8>, Vec<u8>)>,
                [0u8;256],
                0u64,
                pub_amount as i64,
                0u64,
                // Context
                Context::<LightInstructionTrait>::new(
                    &Pubkey::new(b"Sxg7dBh5VLT8S1o6BqncZCPq9nhHHukjfVd6ohQJeAk"),//program_id: &'a Pubkey,
                    &mut create_verifier_state::LightInstructionTrait{
                        signing_address: Signer<'info>,
                        system_program: Program<'info, System>,
                        program_merkle_tree: Program<'info, MerkleTreeProgram>,
                        rent: Sysvar<'info, Rent>,
                        /// CHECK: Is the same as in integrity hash.
                        //#[account(mut, address = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[usize::try_from(self.load()?.merkle_tree_index).unwrap()].0))]
                        merkle_tree: AccountInfo<'info>,
                        #[account(
                            mut,
                            address = anchor_lang::prelude::Pubkey::find_program_address(&[merkle_tree.key().to_bytes().as_ref()], &MerkleTreeProgram::id()).0
                        )]
                        pre_inserted_leaves_index: Account<'info, PreInsertedLeavesIndex>,
                        /// CHECK: This is the cpi authority and will be enforced in the Merkle tree program.
                        #[account(mut, seeds= [MerkleTreeProgram::id().to_bytes().as_ref()], bump)]
                        authority: UncheckedAccount<'info>,
                        token_program: Program<'info, Token>,
                        /// CHECK:` Is checked depending on deposit or withdrawal.
                        sender: UncheckedAccount<'info>,
                        /// CHECK:` Is checked depending on deposit or withdrawal.
                        recipient: UncheckedAccount<'info>,
                        /// CHECK:` Is not checked the relayer has complete freedom.
                        relayer_recipient: AccountInfo<'info>,

                    },//accounts: &'b mut T,
                    &[Pubkey::new(b"Sxg7dBh5VLT8S1o6BqncZCPq9nhHHukjfVd6ohQJeAk")]//remaining_accounts: &'c [AccountInfo<'info>],
                    //bumps: BTreeMap<String, u8>,
                )
        );
        tx.verify().unwrap();
        tx.transfer_user_funds().unwrap();
        */
    }
    #[test]
    fn test_hash() {
        let input = vec![1u8;32];
        let hash = anchor_lang::solana_program::hash::hash(&input[..]);
        println!("hash {:?}", hash.to_bytes());
    }
    use std::ops::Neg;

    fn to_be_64(bytes: &[u8]) -> Vec<u8> {
        let mut vec = Vec::new();
        for b in bytes.chunks(32) {
            for byte in b.iter().rev() {
                vec.push(*byte);
            }
        }
        vec
    }

    fn to_be_128(bytes: &[u8]) -> Vec<u8> {
        let mut vec = Vec::new();
        for b in bytes.chunks(64) {
            for byte in b.iter().rev() {
                vec.push(*byte);
            }
        }
        vec
    }


    const TEST_DATA: [[([u8; 64], [u8; 128]); 3]; 3] = [
        [
            (
                [
                    169, 188, 126, 23, 234, 181, 49, 44, 76, 155, 186, 163, 180, 151, 19, 153, 6, 220,
                    171, 29, 119, 54, 44, 34, 82, 130, 81, 172, 144, 32, 252, 41, 51, 218, 77, 129,
                    230, 75, 37, 139, 138, 25, 61, 229, 38, 121, 209, 134, 47, 83, 24, 40, 105, 229,
                    156, 143, 191, 172, 172, 88, 204, 23, 187, 29,
                ],
                [
                    133, 52, 151, 123, 19, 114, 157, 14, 21, 62, 189, 188, 4, 178, 35, 99, 225, 132,
                    32, 193, 205, 86, 200, 15, 25, 57, 244, 156, 6, 174, 131, 16, 112, 192, 162, 11,
                    208, 105, 38, 25, 207, 152, 137, 184, 141, 148, 183, 25, 137, 165, 117, 9, 241,
                    106, 140, 254, 1, 125, 113, 17, 96, 189, 169, 2, 253, 248, 3, 180, 29, 86, 110, 90,
                    49, 229, 224, 58, 22, 188, 76, 132, 220, 16, 176, 51, 132, 26, 126, 45, 224, 132,
                    17, 56, 248, 37, 12, 7, 23, 2, 42, 116, 42, 173, 235, 102, 244, 191, 177, 1, 93,
                    177, 63, 151, 44, 150, 232, 54, 181, 66, 207, 138, 144, 211, 104, 119, 163, 198, 6,
                    17,
                ],
            ),
            (
                [
                    220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89,
                    0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187,
                    136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133,
                    250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15,
                ],
                [
                    237, 246, 146, 217, 92, 189, 222, 70, 221, 218, 94, 247, 212, 34, 67, 103, 121, 68,
                    92, 94, 102, 0, 106, 66, 118, 30, 31, 18, 239, 222, 0, 24, 194, 18, 243, 174, 183,
                    133, 228, 151, 18, 231, 169, 53, 51, 73, 170, 241, 37, 93, 251, 49, 183, 191, 96,
                    114, 58, 72, 13, 146, 147, 147, 142, 25, 157, 127, 130, 113, 21, 192, 57, 239, 17,
                    247, 45, 92, 40, 131, 175, 179, 205, 23, 182, 243, 53, 212, 164, 109, 62, 50, 165,
                    5, 205, 239, 155, 29, 236, 101, 90, 7, 58, 177, 115, 230, 153, 59, 190, 247, 93,
                    57, 54, 219, 199, 36, 117, 24, 9, 172, 177, 203, 179, 175, 209, 136, 162, 196, 93,
                    39,
                ],
            ),
            (
                [
                    181, 129, 186, 7, 53, 61, 26, 93, 210, 29, 170, 46, 100, 150, 94, 3, 69, 237, 166,
                    21, 152, 146, 211, 52, 142, 103, 21, 166, 133, 176, 141, 24, 57, 122, 149, 35, 146,
                    161, 222, 19, 116, 168, 229, 88, 0, 246, 241, 65, 134, 237, 213, 24, 65, 254, 219,
                    138, 55, 223, 50, 68, 107, 147, 187, 32,
                ],
                [
                    83, 221, 254, 184, 55, 148, 227, 43, 133, 7, 18, 158, 114, 71, 125, 201, 138, 190,
                    192, 0, 56, 234, 29, 190, 13, 53, 55, 124, 65, 213, 82, 16, 190, 225, 85, 93, 216,
                    143, 253, 91, 162, 249, 28, 124, 77, 137, 187, 191, 41, 63, 204, 124, 190, 22, 134,
                    112, 142, 91, 162, 209, 153, 210, 182, 31, 36, 167, 184, 235, 213, 41, 254, 96, 37,
                    227, 187, 127, 87, 12, 115, 172, 212, 196, 214, 182, 240, 132, 194, 165, 181, 15,
                    200, 254, 250, 69, 45, 32, 97, 149, 114, 77, 166, 31, 30, 137, 84, 29, 211, 14,
                    204, 3, 70, 171, 70, 14, 213, 156, 243, 16, 201, 200, 211, 247, 42, 95, 196, 13,
                    58, 48,
                ],
            ),
        ],
        [
            (
                [
                    143, 15, 147, 99, 79, 60, 78, 50, 8, 203, 226, 62, 60, 109, 217, 225, 121, 35, 63,
                    247, 36, 118, 48, 28, 46, 227, 216, 210, 143, 152, 178, 32, 196, 95, 169, 192, 62,
                    112, 118, 209, 62, 38, 48, 221, 92, 177, 39, 6, 209, 164, 125, 146, 25, 41, 79, 58,
                    75, 8, 43, 65, 211, 110, 225, 30,
                ],
                [
                    133, 52, 151, 123, 19, 114, 157, 14, 21, 62, 189, 188, 4, 178, 35, 99, 225, 132,
                    32, 193, 205, 86, 200, 15, 25, 57, 244, 156, 6, 174, 131, 16, 112, 192, 162, 11,
                    208, 105, 38, 25, 207, 152, 137, 184, 141, 148, 183, 25, 137, 165, 117, 9, 241,
                    106, 140, 254, 1, 125, 113, 17, 96, 189, 169, 2, 253, 248, 3, 180, 29, 86, 110, 90,
                    49, 229, 224, 58, 22, 188, 76, 132, 220, 16, 176, 51, 132, 26, 126, 45, 224, 132,
                    17, 56, 248, 37, 12, 7, 23, 2, 42, 116, 42, 173, 235, 102, 244, 191, 177, 1, 93,
                    177, 63, 151, 44, 150, 232, 54, 181, 66, 207, 138, 144, 211, 104, 119, 163, 198, 6,
                    17,
                ],
            ),
            (
                [
                    220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89,
                    0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187,
                    136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133,
                    250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15,
                ],
                [
                    173, 107, 171, 22, 221, 71, 45, 8, 196, 71, 21, 41, 91, 194, 234, 150, 169, 187,
                    191, 168, 232, 15, 151, 135, 154, 78, 26, 82, 238, 227, 241, 40, 226, 243, 148, 20,
                    235, 209, 68, 253, 43, 11, 170, 29, 250, 120, 231, 225, 205, 97, 222, 24, 170, 83,
                    144, 237, 88, 237, 120, 135, 51, 94, 186, 31, 225, 243, 95, 76, 78, 195, 89, 183,
                    200, 17, 179, 211, 10, 171, 25, 250, 102, 190, 107, 2, 80, 178, 187, 180, 75, 67,
                    5, 167, 39, 0, 171, 13, 198, 43, 144, 117, 20, 112, 3, 248, 251, 68, 197, 76, 168,
                    116, 200, 43, 119, 58, 222, 243, 112, 199, 3, 134, 49, 71, 184, 111, 92, 200, 89,
                    4,
                ],
            ),
            (
                [
                    43, 199, 220, 200, 152, 163, 210, 104, 247, 237, 3, 10, 42, 146, 151, 211, 32, 128,
                    69, 115, 173, 153, 226, 245, 198, 70, 127, 50, 105, 103, 69, 5, 225, 143, 168, 217,
                    93, 12, 51, 233, 218, 140, 240, 72, 95, 27, 69, 243, 32, 194, 245, 194, 132, 60,
                    63, 203, 107, 244, 113, 109, 83, 157, 100, 21,
                ],
                [
                    83, 221, 254, 184, 55, 148, 227, 43, 133, 7, 18, 158, 114, 71, 125, 201, 138, 190,
                    192, 0, 56, 234, 29, 190, 13, 53, 55, 124, 65, 213, 82, 16, 190, 225, 85, 93, 216,
                    143, 253, 91, 162, 249, 28, 124, 77, 137, 187, 191, 41, 63, 204, 124, 190, 22, 134,
                    112, 142, 91, 162, 209, 153, 210, 182, 31, 36, 167, 184, 235, 213, 41, 254, 96, 37,
                    227, 187, 127, 87, 12, 115, 172, 212, 196, 214, 182, 240, 132, 194, 165, 181, 15,
                    200, 254, 250, 69, 45, 32, 97, 149, 114, 77, 166, 31, 30, 137, 84, 29, 211, 14,
                    204, 3, 70, 171, 70, 14, 213, 156, 243, 16, 201, 200, 211, 247, 42, 95, 196, 13,
                    58, 48,
                ],
            ),
        ],
        [
            (
                [
                    34, 122, 253, 204, 243, 16, 201, 133, 161, 151, 13, 130, 78, 126, 94, 163, 224, 32,
                    110, 105, 60, 173, 80, 225, 5, 251, 211, 85, 42, 227, 225, 17, 66, 75, 107, 118,
                    161, 223, 82, 148, 65, 172, 88, 173, 9, 109, 108, 229, 250, 87, 112, 159, 113, 219,
                    102, 31, 149, 48, 83, 81, 141, 139, 169, 17,
                ],
                [
                    133, 52, 151, 123, 19, 114, 157, 14, 21, 62, 189, 188, 4, 178, 35, 99, 225, 132,
                    32, 193, 205, 86, 200, 15, 25, 57, 244, 156, 6, 174, 131, 16, 112, 192, 162, 11,
                    208, 105, 38, 25, 207, 152, 137, 184, 141, 148, 183, 25, 137, 165, 117, 9, 241,
                    106, 140, 254, 1, 125, 113, 17, 96, 189, 169, 2, 253, 248, 3, 180, 29, 86, 110, 90,
                    49, 229, 224, 58, 22, 188, 76, 132, 220, 16, 176, 51, 132, 26, 126, 45, 224, 132,
                    17, 56, 248, 37, 12, 7, 23, 2, 42, 116, 42, 173, 235, 102, 244, 191, 177, 1, 93,
                    177, 63, 151, 44, 150, 232, 54, 181, 66, 207, 138, 144, 211, 104, 119, 163, 198, 6,
                    17,
                ],
            ),
            (
                [
                    220, 210, 225, 96, 65, 152, 212, 86, 43, 63, 222, 140, 149, 68, 69, 209, 141, 89,
                    0, 170, 89, 149, 222, 17, 80, 181, 170, 29, 142, 207, 12, 12, 195, 251, 228, 187,
                    136, 200, 161, 205, 225, 188, 70, 173, 169, 183, 19, 63, 115, 136, 119, 101, 133,
                    250, 123, 233, 146, 120, 213, 224, 177, 91, 158, 15,
                ],
                [
                    27, 204, 124, 11, 165, 70, 231, 141, 30, 176, 235, 127, 5, 147, 187, 136, 179, 176,
                    39, 54, 240, 245, 69, 79, 225, 2, 29, 28, 30, 92, 220, 14, 154, 121, 195, 133, 58,
                    138, 48, 178, 244, 161, 30, 12, 144, 147, 201, 94, 26, 26, 180, 238, 105, 53, 232,
                    123, 16, 26, 111, 42, 131, 150, 17, 32, 184, 189, 171, 1, 21, 45, 85, 39, 172, 64,
                    214, 75, 179, 42, 172, 248, 41, 111, 116, 204, 218, 37, 202, 100, 74, 134, 56, 35,
                    193, 179, 194, 47, 24, 25, 165, 85, 203, 222, 32, 43, 140, 89, 155, 150, 92, 130,
                    129, 161, 37, 230, 36, 249, 77, 180, 149, 50, 16, 212, 248, 81, 4, 241, 71, 46,
                ],
            ),
            (
                [
                    208, 81, 69, 193, 208, 184, 9, 149, 1, 84, 164, 160, 88, 157, 70, 224, 244, 253,
                    90, 181, 20, 25, 183, 146, 153, 228, 241, 189, 117, 142, 186, 30, 161, 103, 48, 84,
                    73, 70, 218, 115, 168, 176, 143, 92, 214, 13, 203, 2, 34, 146, 69, 99, 20, 32, 206,
                    167, 153, 85, 92, 14, 242, 134, 25, 5,
                ],
                [
                    83, 221, 254, 184, 55, 148, 227, 43, 133, 7, 18, 158, 114, 71, 125, 201, 138, 190,
                    192, 0, 56, 234, 29, 190, 13, 53, 55, 124, 65, 213, 82, 16, 190, 225, 85, 93, 216,
                    143, 253, 91, 162, 249, 28, 124, 77, 137, 187, 191, 41, 63, 204, 124, 190, 22, 134,
                    112, 142, 91, 162, 209, 153, 210, 182, 31, 36, 167, 184, 235, 213, 41, 254, 96, 37,
                    227, 187, 127, 87, 12, 115, 172, 212, 196, 214, 182, 240, 132, 194, 165, 181, 15,
                    200, 254, 250, 69, 45, 32, 97, 149, 114, 77, 166, 31, 30, 137, 84, 29, 211, 14,
                    204, 3, 70, 171, 70, 14, 213, 156, 243, 16, 201, 200, 211, 247, 42, 95, 196, 13,
                    58, 48,
                ],
            ),
        ],
    ];
}
*/
