use anchor_lang::{
    prelude::*,
    solana_program::{
        log::sol_log_compute_units,
        program_pack::Pack,
        msg,
        sysvar
    }
};
use anchor_spl::token::Transfer;
use ark_std::{marker::PhantomData, vec::Vec};
use ark_ff::{
    BigInteger256,
    Fp256,
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

use crate::{
    utils::{
        close_account::close_account,
        create_pda::create_and_check_pda,
        to_be_64
    },
    errors::VerifierSdkError,
    cpi_instructions::{
        insert_nullifiers_cpi,
        insert_two_leaves_cpi,
        withdraw_sol_cpi,
        withdraw_spl_cpi
    },
    accounts::Accounts
};

use std::ops::Neg;

use merkle_tree_program::{
    utils::constants::{
        POOL_CONFIG_SEED,
        POOL_SEED
    },
    program::MerkleTreeProgram
};


type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>;


pub trait Config {
	/// Number of nullifiers to be inserted with the transaction.
	const NR_NULLIFIERS: usize;
	/// Number of output utxos.
	const NR_LEAVES: usize;
	/// Number of checked public inputs.
	const NR_CHECKED_PUBLIC_INPUTS: usize;
    /// Program ID of the verifier program.
    const ID: [u8;32];
    const UTXO_SIZE: usize;
}

#[derive(Clone)]
pub struct Transaction<'info, 'a, 'c, T: Config>  {
    pub merkle_root:                Vec<u8>,
    pub public_amount:              Vec<u8>,
    pub tx_integrity_hash:          Vec<u8>,
    pub fee_amount:                 Vec<u8>,
    pub mint_pubkey:                Vec<u8>,
    pub checked_public_inputs:      Vec<Vec<u8>>,
    pub nullifiers:                 Vec<Vec<u8>>,
    pub leaves:                     Vec<Vec<Vec<u8>>>,
    pub relayer_fee:                u64,
    pub proof_a:                    Vec<u8>,
    pub proof_b:                    Vec<u8>,
    pub proof_c:                    Vec<u8>,
    pub encrypted_utxos:            Vec<u8>,
    pub pool_type:                  Vec<u8>,
    pub merkle_root_index:          usize,
    pub transferred_funds:          bool,
    pub computed_tx_integrity_hash:  bool,
    pub verified_proof:             bool,
    pub inserted_leaves:            bool,
    pub inserted_nullifier:         bool,
    pub fetched_root:               bool,
    pub fetched_mint:               bool,
    pub accounts:                   Option<&'a Accounts<'info, 'a, 'c>>,
    pub e_phantom:                  PhantomData<T>,
    pub verifyingkey:               &'a Groth16Verifyingkey<'a>
}


impl <T: Config>Transaction<'_, '_, '_, T> {

    pub fn new<'info, 'a, 'c> (
        proof: Vec<u8>,
        public_amount: Vec<u8>,
        fee_amount: Vec<u8>,
        checked_public_inputs: Vec<Vec<u8>>,
        nullifiers: Vec<Vec<u8>>,
        leaves: Vec<Vec<Vec<u8>>>,
        encrypted_utxos: Vec<u8>,
        relayer_fee: u64,
        merkle_root_index: usize,
        pool_type: Vec<u8>,
        accounts: Option<&'a Accounts<'info, 'a, 'c>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>
    ) -> Transaction<'info, 'a, 'c, T> {

        assert_eq!(T::NR_NULLIFIERS, nullifiers.len());
        assert_eq!(T::NR_LEAVES / 2, leaves.len());

        let proof_a: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&proof[0..64])[..], &[0u8][..]].concat()).unwrap();
        let mut proof_a_neg = [0u8;65];
        <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]).unwrap();

        return Transaction {
            merkle_root: vec![0u8;32],
            public_amount,
            tx_integrity_hash: vec![0u8;32],
            fee_amount,
            mint_pubkey: vec![0u8;32],
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
            computed_tx_integrity_hash: false,
            verified_proof : false,
            inserted_leaves : false,
            inserted_nullifier : false,
            fetched_root : false,
            fetched_mint: false,
            e_phantom: PhantomData,
            verifyingkey,
            accounts,
            pool_type
        }
    }

    /// Transact is a wrapper function which computes the integrity hash, checks the root,
    /// verifies the zero knowledge proof, inserts leaves, inserts nullifiers, transfers funds and fees.
    pub fn transact(&mut self) -> Result<()> {
        self.compute_tx_integrity_hash()?;
        self.fetch_root()?;
        self.fetch_mint()?;
        self.verify()?;
        self.insert_leaves()?;
        self.insert_nullifiers()?;
        self.transfer_user_funds()?;
        self.transfer_fee()?;
        self.check_completion()
    }

    /// Verifies a Goth16 zero knowledge proof over the bn254 curve.
    pub fn verify(&mut self) -> Result<()> {
        if !self.computed_tx_integrity_hash {
            msg!("Tried to verify proof without computing integrity hash.");
        }

        if !self.fetched_mint {
            msg!("Tried to verify proof without fetching mind.");
        }

        if !self.fetched_root {
            msg!("Tried to verify proof without fetching root.");
        }

        let mut public_inputs = vec![
            self.merkle_root[..].to_vec(),
            self.public_amount[..].to_vec(),
            self.tx_integrity_hash[..].to_vec(),
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
            self.proof_a.clone(),
            self.proof_b.clone(),
            self.proof_c.clone(),
            public_inputs,
            &self.verifyingkey
        ).unwrap();

        match verifier.verify() {
            Ok(_) => {
                self.verified_proof = true;
                Ok(())
            },
            Err(e) => {
                msg!("Public Inputs:");
                msg!("merkle tree root {:?}", self.merkle_root);
                msg!("public_amount {:?}", self.public_amount);
                msg!("tx_integrity_hash {:?}", self.tx_integrity_hash);
                msg!("fee_amount {:?}", self.fee_amount);
                msg!("nullifiers {:?}", self.nullifiers);
                msg!("leaves {:?}", self.leaves);
                msg!("checked_public_inputs {:?}", self.checked_public_inputs);
                msg!("error {:?}", e);
                err!(VerifierSdkError::ProofVerificationFailed)
            }
        }

    }

    /// Computes the integrity hash of the transaction. This hash is an input to the ZKP, and
    /// ensures that the relayer cannot change parameters of the internal or unshield transaction.
    /// H(recipient||recipient_fee||signer||relayer_fee||encrypted_utxos).
    pub fn compute_tx_integrity_hash(&mut self) -> Result<()> {
        let input = [
            self.accounts.unwrap().recipient.as_ref().unwrap().key().to_bytes().to_vec(),
            self.accounts.unwrap().recipient_fee.as_ref().unwrap().key().to_bytes().to_vec(),
            self.accounts.unwrap().signing_address.key().to_bytes().to_vec(),
            self.relayer_fee.to_le_bytes().to_vec(),
            self.encrypted_utxos.clone()
        ]
        .concat();
        // msg!("recipient: {:?}", self.accounts.unwrap().recipient.as_ref().unwrap().key().to_bytes().to_vec());
        // msg!("recipient_fee: {:?}", self.accounts.unwrap().recipient_fee.as_ref().unwrap().key().to_bytes().to_vec());
        // msg!("signing_address: {:?}", self.accounts.unwrap().signing_address.key().to_bytes().to_vec());
        // msg!("relayer_fee: {:?}", self.relayer_fee.to_le_bytes().to_vec());
        // msg!("relayer_fee {}", self.relayer_fee);
        // msg!("integrity_hash inputs: {:?}", input);
        // msg!("integrity_hash inputs.len(): {}", input.len());
        let hash = Fr::from_be_bytes_mod_order(&anchor_lang::solana_program::keccak::hash(&input[..]).try_to_vec()?[..]);
        let mut bytes = Vec::<u8>::new();
        <Fp256::<FrParameters> as ToBytes>::write(&hash, &mut bytes).unwrap();
        self.tx_integrity_hash = to_be_64(&bytes[..32]);
        // msg!("tx_integrity_hash be: {:?}", self.tx_integrity_hash);
        // msg!("Fq::from_be_bytes_mod_order(&hash[..]) : {}", hash);
        self.computed_tx_integrity_hash = true;
        Ok(())
    }

    /// Fetches the root according to an index from the passed-in Merkle tree.
    pub fn fetch_root(&mut self) -> Result<()> {
        let merkle_tree = self.accounts.unwrap().merkle_tree.load()?;
        self.merkle_root = to_be_64(&merkle_tree.roots[self.merkle_root_index].to_vec());
        self.fetched_root = true;
        Ok(())
    }

    /// Fetches the token mint from passed in sender account. If the sender account is not a
    /// token account, native mint is assumed.
    pub fn fetch_mint(&mut self) -> Result<()> {
         match spl_token::state::Account::unpack(&self.accounts.unwrap().sender.as_ref().unwrap().data.borrow()) {
             Ok(sender_mint) => {
                 // Omits the last byte for the mint pubkey bytes to fit into the bn254 field.
                 self.mint_pubkey = [vec![0u8], sender_mint.mint.to_bytes()[..31].to_vec()].concat();
                 self.fetched_mint = true;
                 Ok(())
             },
             Err(_) => {
                 self.mint_pubkey = vec![0u8;32];
                 self.fetched_mint = true;
                 Ok(())
             }
         }
    }

    /// Calls merkle tree via cpi to insert leaves.
    pub fn insert_leaves(&mut self) -> Result<()> {

        if !self.verified_proof {
            msg!("Tried to insert leaves without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        if T::NR_NULLIFIERS != self.nullifiers.len() {
            msg!("NR_NULLIFIERS  {} != self.nullifiers.len() {}", T::NR_NULLIFIERS, self.nullifiers.len());
            return err!(VerifierSdkError::InvalidNrNullifieraccounts);
        }

        if T::NR_NULLIFIERS + (T::NR_LEAVES / 2) != self.accounts.unwrap().remaining_accounts.len() {
            msg!("NR_NULLIFIERS  {} != self.nullifiers.len() {}", T::NR_NULLIFIERS, self.nullifiers.len());
            return err!(VerifierSdkError::InvalidNrLeavesaccounts);
        }

        // check merkle tree
        for (i, leaves) in self.leaves.iter().enumerate() {
            // check account integrities
            insert_two_leaves_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().remaining_accounts[T::NR_NULLIFIERS + i].to_account_info(),
                &self.accounts.unwrap().pre_inserted_leaves_index.to_account_info(),
                &self.accounts.unwrap().system_program.to_account_info(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                to_be_64(&leaves[0]).try_into().unwrap(),
                to_be_64(&leaves[1]).try_into().unwrap(),
                self.accounts.unwrap().merkle_tree.key(),
                self.encrypted_utxos.clone()
            )?;
        }

        self.inserted_leaves = true;
        Ok(())
    }

    /// Calls merkle tree via cpi to insert nullifiers.
    pub fn insert_nullifiers(&mut self) -> Result<()> {

        if !self.verified_proof {
            msg!("Tried to insert nullifiers without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        if T::NR_NULLIFIERS != self.nullifiers.len() {
            msg!("NR_NULLIFIERS  {} != self.nullifiers.len() {}", T::NR_NULLIFIERS, self.nullifiers.len());
            return err!(VerifierSdkError::InvalidNrNullifieraccounts);
        }

        if T::NR_NULLIFIERS + (T::NR_LEAVES / 2) != self.accounts.unwrap().remaining_accounts.len() {
            msg!("NR_LEAVES / 2  {} != self.leaves.len() {}", T::NR_LEAVES / 2, self.leaves.len());
            return err!(VerifierSdkError::InvalidNrLeavesaccounts);
        }

        insert_nullifiers_cpi(
            &self.accounts.unwrap().program_id,
            &self.accounts.unwrap().program_merkle_tree.to_account_info(),
            &self.accounts.unwrap().authority.to_account_info(),
            &self.accounts.unwrap().system_program.to_account_info().clone(),
            &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
            self.nullifiers.to_vec(),
            self.accounts.unwrap().remaining_accounts.to_vec()
        )?;

        self.inserted_nullifier = true;
        Ok(())
    }

    /// Transfers user funds either to or from a merkle tree liquidity pool.
    pub fn transfer_user_funds(&mut self) -> Result<()> {

        if !self.verified_proof {
            msg!("Tried to transfer funds without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        msg!("transferring user funds");
        // check mintPubkey
        let (pub_amount_checked, _) = self.check_amount(
            0,
            to_be_64(&self.public_amount).try_into().unwrap(),
        )?;


        let recipient_mint = spl_token::state::Account::unpack(&self.accounts.unwrap().recipient.as_ref().unwrap().data.borrow())?;
        let sender_mint = spl_token::state::Account::unpack(&self.accounts.unwrap().sender.as_ref().unwrap().data.borrow())?;

        // check mint
        if self.mint_pubkey[1..] != recipient_mint.mint.to_bytes()[..31] || self.mint_pubkey[1..] != sender_mint.mint.to_bytes()[..31] {
            msg!("*self.mint_pubkey[..31] {:?}, {:?}, {:?}", self.mint_pubkey[1..].to_vec(), recipient_mint.mint.to_bytes()[..31].to_vec(), sender_mint.mint.to_bytes()[..31].to_vec() );
            return err!(VerifierSdkError::InconsistentMintProofSenderOrRecipient);
        }
        // is a token deposit or withdrawal

        if self.is_deposit() {
            self.check_spl_pool_account_derivation(&self.accounts.unwrap().recipient.as_ref().unwrap().key(), &recipient_mint.mint)?;

            let seed = merkle_tree_program::ID.to_bytes();
            let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
                &[seed.as_ref()],
                self.accounts.unwrap().program_id,
            );
            let bump = &[bump];
            let seeds = &[&[seed.as_slice(), bump][..]];

            let accounts = Transfer {
                from:       self.accounts.unwrap().sender.as_ref().unwrap().to_account_info().clone(),
                to:         self.accounts.unwrap().recipient.as_ref().unwrap().to_account_info().clone(),
                authority:  self.accounts.unwrap().authority.to_account_info().clone()
            };

            let cpi_ctx = CpiContext::new_with_signer(self.accounts.unwrap().token_program.unwrap().to_account_info().clone(), accounts, seeds);
            anchor_spl::token::transfer(cpi_ctx, pub_amount_checked)?;
        } else {
            self.check_spl_pool_account_derivation(&self.accounts.unwrap().sender.as_ref().unwrap().key(), &sender_mint.mint)?;

            // withdraw_spl_cpi
            withdraw_spl_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().sender.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().recipient.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().token_authority.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().token_program.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                pub_amount_checked
            )?;
        }

        self.transferred_funds = true;
        msg!("transferred");
        sol_log_compute_units();

        Ok(())
    }

    /// Transfers the relayer fee  to or from a merkle tree liquidity pool.
    pub fn transfer_fee(&self) -> Result<()> {

        if !self.verified_proof {
            msg!("Tried to transfer fees without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        // check that it is the native token pool
        let (fee_amount_checked, relayer_fee) = self.check_amount(
            self.relayer_fee,
            to_be_64(&self.fee_amount).try_into().unwrap()
        )?;

        if self.is_deposit() {

            self.deposit_sol(fee_amount_checked, &self.accounts.unwrap().recipient_fee.as_ref().unwrap().to_account_info())?;

        } else {

            self.check_sol_pool_account_derivation(&self.accounts.unwrap().sender_fee.as_ref().unwrap().key())?;

            // withdraws sol for the user
            withdraw_sol_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().sender_fee.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().recipient_fee.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                fee_amount_checked,
            )?;

            // pays the relayer fee
            withdraw_sol_cpi(
                &self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().sender_fee.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().relayer_recipient.as_ref().unwrap().to_account_info(),
                &self.accounts.unwrap().registered_verifier_pda.to_account_info(),
                relayer_fee,
            )?;

        }

        Ok(())
    }

    /// Creates and closes an account such that deposited sol is part of the transaction fees.
    fn deposit_sol(&self, amount_checked: u64, recipient: &AccountInfo) -> Result<()> {
        self.check_sol_pool_account_derivation(&recipient.key())?;

        msg!("is deposit");
        let rent = <Rent as sysvar::Sysvar>::get()?;

        create_and_check_pda(
            &self.accounts.unwrap().program_id,
            &self.accounts.unwrap().signing_address.to_account_info(),
            &self.accounts.unwrap().escrow.as_ref().unwrap().to_account_info(),
            &self.accounts.unwrap().system_program.to_account_info(),
            &rent,
            &b"escrow"[..],
            &Vec::new(),
            0,                  //bytes
            amount_checked,     //lamports
            false,              //rent_exempt
        )?;
        close_account(&self.accounts.unwrap().escrow.as_ref().unwrap().to_account_info(), &recipient)
    }

    /// Checks whether a transaction is a deposit by inspecting the public amount.
    pub fn is_deposit(&self) -> bool {
        if self.public_amount[24..] != [0u8;8] && self.public_amount[..24] == [0u8;24] {
            return true;
        }
        return false;
    }


    pub fn check_sol_pool_account_derivation(&self, pubkey: &Pubkey) -> Result<()> {
        let derived_pubkey =
            Pubkey::find_program_address(&[&[0u8;32], &self.pool_type, &POOL_CONFIG_SEED[..]], &MerkleTreeProgram::id());

        if derived_pubkey.0 != *pubkey {
            return err!(VerifierSdkError::InvalidSenderorRecipient);
        }
        Ok(())
    }


    pub fn check_spl_pool_account_derivation(&self, pubkey: &Pubkey, mint: &Pubkey) -> Result<()> {
        let derived_pubkey =
            Pubkey::find_program_address(&[&mint.to_bytes(), &self.pool_type, &POOL_SEED[..]], &MerkleTreeProgram::id());

        if derived_pubkey.0 != *pubkey {
            return err!(VerifierSdkError::InvalidSenderorRecipient);
        }
        Ok(())
    }


    pub fn check_completion(&self) -> Result<()>{
        if self.transferred_funds &&
            self.verified_proof &&
            self.inserted_leaves &&
            self.inserted_nullifier
        {
            return Ok(());
        }
        msg!("verified_proof {}", self.verified_proof);
        msg!("inserted_leaves {}", self.inserted_leaves);
        msg!("transferred_funds {}", self.transferred_funds);
        err!(VerifierSdkError::TransactionIncomplete)
    }


    #[allow(clippy::comparison_chain)]
    pub fn check_amount(
            &self,
            relayer_fee: u64,
            amount: [u8;32],
        ) -> Result<(u64, u64)> {

        // pub_amount is the public amount included in public inputs for proof verification
        let pub_amount = <BigInteger256 as FromBytes>::read(&amount[..]).unwrap();

        // Big integers are stored in 4 u64 limbs, if the number is <= U64::max() and encoded in little endian,
        // only the first limb is greater than 0.
        // Amounts in shielded accounts are limited to 64bit therefore a withdrawal will always be greater
        // than one U64::max().
        if pub_amount.0[0] > 0 && pub_amount.0[1] == 0 && pub_amount.0[2] == 0 && pub_amount.0[3] == 0 {
            if relayer_fee != 0 {
                msg!("relayer_fee {}", relayer_fee);
                return Err(VerifierSdkError::WrongPubAmount.into());
            }
            Ok((pub_amount.0[0], 0))

        } else if pub_amount.0[0] != 0 {
            // calculate ext_amount from pubAmount:
            let mut field = FrParameters::MODULUS;
            field.sub_noborrow(&pub_amount);

            // field.0[0] is the positive value
            if field.0[1] != 0 || field.0[2] != 0 || field.0[3] != 0 {
                msg!("Public amount is larger than u64.");
                return Err(VerifierSdkError::WrongPubAmount.into());
            }

            if field.0[0]
                < relayer_fee
            {
                msg!(
                    "Withdrawal invalid relayer_fee: {} < {}",
                    pub_amount.0[0],
                    relayer_fee
                );
                return Err(VerifierSdkError::WrongPubAmount.into());
            }

            let pub_amount = field.0[0].saturating_sub(relayer_fee);
            Ok((pub_amount, relayer_fee))
        } else if pub_amount.0[0] == 0 && pub_amount.0[1] == 0 && pub_amount.0[2] == 0 && pub_amount.0[3] == 0 {
            Ok((0, 0))

        } else {
            Err(VerifierSdkError::WrongPubAmount.into())
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use groth16_solana::{groth16::{Groth16Verifyingkey, Groth16Verifier}, errors::Groth16Error};
    use ark_ff::bytes::{ToBytes, FromBytes};
    use ark_ec;
    use ark_bn254;
    use std::ops::Neg;
    use crate::light_transaction::Transaction;
    use anchor_lang::solana_program::account_info::AccountInfo;
    use ark_ff::biginteger::BigInteger;
    use ark_ff::biginteger::BigInteger256;
    use ark_bn254::{
        FrParameters,
        Fr
    };
    use ark_ff::fields::PrimeField;

    pub const VERIFYING_KEY: Groth16Verifyingkey =  Groth16Verifyingkey {
        nr_pubinputs: 10,

    	vk_alpha_g1: [
    	45,77,154,167,227,2,217,223,65,116,157,85,7,148,157,5,219,234,51,251,177,108,100,59,34,245,153,162,190,109,242,226,
    	20,190,221,80,60,55,206,176,97,216,236,96,32,159,227,69,206,137,131,10,25,35,3,1,240,118,202,255,0,77,25,38,
    	],

    	vk_beta_g2: [
    	9,103,3,47,203,247,118,209,175,201,133,248,136,119,241,130,211,132,128,166,83,242,222,202,169,121,76,188,59,243,6,12,
    	14,24,120,71,173,76,121,131,116,208,214,115,43,245,1,132,125,214,139,192,224,113,36,30,2,19,188,127,193,61,183,171,
    	48,76,251,209,224,138,112,74,153,245,232,71,217,63,140,60,170,253,222,196,107,122,13,55,157,166,154,77,17,35,70,167,
    	23,57,193,177,164,87,168,199,49,49,35,210,77,47,145,146,248,150,183,198,62,234,5,169,213,127,6,84,122,208,206,200,
    	],

    	vk_gamme_g2: [
    	25,142,147,147,146,13,72,58,114,96,191,183,49,251,93,37,241,170,73,51,53,169,231,18,151,228,133,183,174,243,18,194,
    	24,0,222,239,18,31,30,118,66,106,0,102,94,92,68,121,103,67,34,212,247,94,218,221,70,222,189,92,217,146,246,237,
    	9,6,137,208,88,95,240,117,236,158,153,173,105,12,51,149,188,75,49,51,112,179,142,243,85,172,218,220,209,34,151,91,
    	18,200,94,165,219,140,109,235,74,171,113,128,141,203,64,143,227,209,231,105,12,67,211,123,76,230,204,1,102,250,125,170,
    	],

    	vk_delta_g2: [
    	25,142,147,147,146,13,72,58,114,96,191,183,49,251,93,37,241,170,73,51,53,169,231,18,151,228,133,183,174,243,18,194,
    	24,0,222,239,18,31,30,118,66,106,0,102,94,92,68,121,103,67,34,212,247,94,218,221,70,222,189,92,217,146,246,237,
    	9,6,137,208,88,95,240,117,236,158,153,173,105,12,51,149,188,75,49,51,112,179,142,243,85,172,218,220,209,34,151,91,
    	18,200,94,165,219,140,109,235,74,171,113,128,141,203,64,143,227,209,231,105,12,67,211,123,76,230,204,1,102,250,125,170,
    	],

    	vk_ic: &[
    	[
    	3,183,175,189,219,73,183,28,132,200,83,8,65,22,184,81,82,36,181,186,25,216,234,25,151,2,235,194,13,223,32,145,
    	15,37,113,122,93,59,91,25,236,104,227,238,58,154,67,250,186,91,93,141,18,241,150,59,202,48,179,1,53,207,155,199,
    	],
    	[
    	46,253,85,84,166,240,71,175,111,174,244,62,87,96,235,196,208,85,186,47,163,237,53,204,176,190,62,201,189,216,132,71,
    	6,91,228,97,74,5,0,255,147,113,161,152,238,177,78,81,111,13,142,220,24,133,27,149,66,115,34,87,224,237,44,162,
    	],
    	[
    	29,157,232,254,238,178,82,15,152,205,175,129,90,108,114,60,82,162,37,234,115,69,191,125,212,85,176,176,113,41,23,84,
    	8,229,196,41,191,243,112,105,166,75,113,160,140,34,139,179,53,180,245,195,5,24,42,18,82,60,173,192,67,149,211,250,
    	],
    	[
    	18,4,92,105,55,33,222,133,144,185,99,131,167,143,52,120,44,79,164,63,119,223,199,154,26,86,22,208,50,53,159,65,
    	14,171,53,159,255,133,91,30,162,209,152,18,251,112,105,90,65,234,44,4,42,173,31,230,229,137,177,112,241,142,62,176,
    	],
    	[
    	13,117,56,250,131,38,119,205,221,228,32,185,236,82,102,29,198,53,117,151,19,10,255,211,41,210,72,221,79,107,251,150,
    	35,187,30,32,198,17,220,4,68,10,71,51,31,169,4,174,10,38,227,229,193,129,150,76,94,224,182,13,166,65,175,89,
    	],
    	[
    	21,167,160,214,213,132,208,197,115,195,129,111,129,38,56,52,41,57,72,249,50,187,184,49,240,228,142,147,187,96,96,102,
    	34,163,43,218,199,187,250,245,119,151,237,67,231,70,236,67,157,181,216,174,25,82,120,255,191,89,230,165,179,241,188,218,
    	],
    	[
    	4,136,219,130,55,89,21,224,41,30,53,234,66,160,129,174,154,139,151,33,163,221,150,192,171,102,241,161,48,130,31,175,
    	6,47,176,127,13,8,36,228,239,219,6,158,22,31,22,162,91,196,132,188,156,228,30,1,178,246,197,186,236,249,236,147,
    	],
    	[
    	9,41,120,80,67,24,240,221,136,156,137,182,168,17,176,118,119,72,170,188,227,31,15,22,252,37,198,154,195,163,64,125,
    	37,211,235,67,249,133,45,90,162,9,173,19,80,154,208,173,221,203,206,254,81,197,104,26,177,78,86,210,51,116,60,87,
    	],
    	[
    	3,41,86,208,125,147,53,187,213,220,195,141,216,40,92,137,70,210,168,103,105,236,85,37,165,209,246,75,122,251,75,93,
    	28,108,154,181,15,16,35,88,65,211,8,11,123,84,185,187,184,1,83,141,67,46,241,222,232,135,59,44,152,217,237,106,
    	],
    	[
    	34,98,189,118,119,197,102,193,36,150,200,143,226,60,0,239,21,40,5,156,73,7,247,14,249,157,2,241,181,208,144,0,
    	34,45,86,133,116,53,235,160,107,36,195,125,122,10,206,88,85,166,62,150,65,159,130,7,255,224,227,229,206,138,68,71,
    	],
    	]

    };
    type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>;

    fn to_be_64(bytes: &[u8]) -> Vec<u8> {
        let mut vec = Vec::new();
        for b in bytes.chunks(32) {
            for byte in b.iter().rev() {
                vec.push(*byte);
            }
        }
        vec
    }

    pub const PUBLIC_INPUTS: [u8; 9 * 32] = [34,238,251,182,234,248,214,189,46,67,42,25,71,58,145,58,61,28,116,110,60,17,82,149,178,187,160,211,37,226,174,231,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,51,152,17,147,4,247,199,87,230,85,103,90,28,183,95,100,200,46,3,158,247,196,173,146,207,167,108,33,199,18,13,204,198,101,223,186,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,7,49,65,41,7,130,55,65,197,232,175,217,44,151,149,225,75,86,158,105,43,229,65,87,51,150,168,243,176,175,11,203,180,149,72,103,46,93,177,62,42,66,223,153,51,193,146,49,154,41,69,198,224,13,87,80,222,171,37,141,0,1,50,172,18,28,213,213,40,141,45,3,180,200,250,112,108,94,35,143,82,63,125,9,147,37,191,75,62,221,138,20,166,151,219,237,254,58,230,189,33,100,143,241,11,251,73,141,229,57,129,168,83,23,235,147,138,225,177,250,13,97,226,162,6,232,52,95,128,84,90,202,25,178,1,208,219,169,222,123,113,202,165,77,183,98,103,237,187,93,178,95,169,156,38,100,125,218,104,94,104,119,13,21];

    pub const PROOF: [u8; 256] = [45,206,255,166,152,55,128,138,79,217,145,164,25,74,120,234,234,217,68,149,162,44,133,120,184,205,12,44,175,98,168,172,20,24,216,15,209,175,106,75,147,236,90,101,123,219,245,151,209,202,218,104,148,8,32,254,243,191,218,122,42,81,193,84,40,57,233,205,180,46,35,111,215,5,23,93,12,71,118,225,7,46,247,147,47,130,106,189,184,80,146,103,141,52,242,25,0,203,124,176,110,34,151,212,66,180,238,151,236,189,133,209,17,137,205,183,168,196,92,159,75,174,81,168,18,86,176,56,16,26,210,20,18,81,122,142,104,62,251,169,98,141,21,253,50,130,182,15,33,109,228,31,79,183,88,147,174,108,4,22,14,129,168,6,80,246,254,100,218,131,94,49,247,211,3,245,22,200,177,91,60,144,147,174,90,17,19,189,62,147,152,18,41,139,183,208,246,198,118,127,89,160,9,27,61,26,123,180,221,108,17,166,47,115,82,48,132,139,253,65,152,92,209,53,37,25,83,61,252,42,181,243,16,21,2,199,123,96,218,151,253,86,69,181,202,109,64,129,124,254,192,25,177,199,26,50];


    #[derive(Clone)]
    pub struct TransactionConfig;
    impl Config for TransactionConfig {
        /// Number of nullifiers to be inserted with the transaction.
        const NR_NULLIFIERS: usize = 10;
        /// Number of output utxos.
        const NR_LEAVES: usize = 2;
        /// Number of checked public inputs.
        const NR_CHECKED_PUBLIC_INPUTS: usize = 0;
        /// ProgramId in bytes.
        const ID: [u8; 32] = [
            34, 112, 33, 68, 178, 147, 230, 193, 113, 82, 213, 107, 154, 193, 174, 159, 246, 190, 23,
            138, 211, 16, 120, 183, 7, 91, 10, 173, 20, 245, 75, 167,
        ];
        const UTXO_SIZE: usize = 256;
    }

    #[test]
    fn proof_verification_should_succeed() {
        let mut public_inputs_vec = Vec::new();
        for input in PUBLIC_INPUTS.chunks(32) {
            public_inputs_vec.push(input.to_vec());
        }

        let proof_a: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&PROOF[0..64])[..], &[0u8][..]].concat()).unwrap();
        let mut proof_a_neg = [0u8;65];
        <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]).unwrap();


        let mut tx: Transaction<'_, '_, '_, TransactionConfig> = Transaction {
            merkle_root: public_inputs_vec[0].clone(),
            public_amount: public_inputs_vec[1].clone(),
            tx_integrity_hash: public_inputs_vec[2].clone(),
            fee_amount: public_inputs_vec[3].clone(),
            mint_pubkey: public_inputs_vec[4].clone(),
            checked_public_inputs: Vec::<Vec<u8>>::new(),
            nullifiers: public_inputs_vec[5..7].to_vec(),
            leaves: vec![vec![public_inputs_vec[7].to_vec(), public_inputs_vec[8].to_vec()]],
            relayer_fee: 0,
            proof_a: to_be_64(&proof_a_neg[..64]).to_vec(),
            proof_b: PROOF[64..64 + 128].to_vec(),
            proof_c: PROOF[64 + 128..256].to_vec(),
            encrypted_utxos: Vec::<u8>::new(),
            merkle_root_index: 0,
            transferred_funds: false,
            computed_tx_integrity_hash: false,
            verified_proof : false,
            inserted_leaves : false,
            inserted_nullifier : false,
            fetched_root : false,
            fetched_mint: false,
            e_phantom: PhantomData,
            verifyingkey:&VERIFYING_KEY,
            accounts: None,
            pool_type: Vec::<u8>::new()
        };

        assert!(tx.verify().is_ok());

        tx.proof_a = PROOF[..64].to_vec();
        assert!(tx.verify().is_err());

    }

    #[test]
    fn check_amount_and_is_deposit_test() {
        let mut public_inputs_vec = Vec::new();
        for input in PUBLIC_INPUTS.chunks(32) {
            public_inputs_vec.push(input.to_vec());
        }

        let proof_a: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&PROOF[0..64])[..], &[0u8][..]].concat()).unwrap();
        let mut proof_a_neg = [0u8;65];
        <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]).unwrap();


        let mut tx: Transaction<'_, '_, '_, TransactionConfig> = Transaction {
            merkle_root: public_inputs_vec[0].clone(),
            public_amount: public_inputs_vec[1].clone(),
            tx_integrity_hash: public_inputs_vec[2].clone(),
            fee_amount: public_inputs_vec[3].clone(),
            mint_pubkey: public_inputs_vec[4].clone(),
            checked_public_inputs: Vec::<Vec<u8>>::new(),
            nullifiers: public_inputs_vec[5..7].to_vec(),
            leaves: vec![vec![public_inputs_vec[7].to_vec(), public_inputs_vec[8].to_vec()]],
            relayer_fee: 0,
            proof_a: to_be_64(&proof_a_neg[..64]).to_vec(),
            proof_b: PROOF[64..64 + 128].to_vec(),
            proof_c: PROOF[64 + 128..256].to_vec(),
            encrypted_utxos: Vec::<u8>::new(),
            merkle_root_index: 0,
            transferred_funds: false,
            computed_tx_integrity_hash: false,
            verified_proof : false,
            inserted_leaves : false,
            inserted_nullifier : false,
            fetched_root : false,
            fetched_mint: false,
            e_phantom: PhantomData,
            verifyingkey:&VERIFYING_KEY,
            accounts: None,
            pool_type: Vec::<u8>::new()
        };
        tx.verify().unwrap();


        // deposit
        let (amount, relayer_fee) = tx.check_amount(tx.relayer_fee, to_be_64(&tx.public_amount).clone().try_into().unwrap()).unwrap();
        let new_bn = BigInteger256::new([123u64, 0, 0, 0]);
        let bytes = <BigInteger256 as BigInteger>::to_bytes_be(&new_bn);
        assert_eq!( <BigInteger256 as BigInteger>::to_bytes_le(&new_bn), to_be_64(&bytes));
        tx.public_amount = bytes.clone();
        assert!(tx.is_deposit());
        let (amount, relayer_fee) = tx.check_amount(1u64, to_be_64(&bytes).clone().try_into().unwrap()).unwrap();
        assert_eq!(amount, new_bn.0[0]);
        assert_eq!(relayer_fee, 0);

        // withdrawal
        let mut field = FrParameters::MODULUS;
        field.sub_noborrow(&new_bn);
        let bytes = <BigInteger256 as BigInteger>::to_bytes_be(&field);
        let x: Fp256<FrParameters> = ark_ff::fields::PrimeField::from_be_bytes_mod_order(&bytes);
        let bytes =<BigInteger256 as BigInteger>::to_bytes_be(&x.into_repr());
        tx.public_amount = bytes.clone();
        assert!(!tx.is_deposit());

        let (amount, relayer_fee) =tx.check_amount(0u64, to_be_64(&bytes).clone().try_into().unwrap()).unwrap();
        assert_eq!(amount, new_bn.0[0]);
        assert_eq!(relayer_fee, 0);

        let (amount, relayer_fee) = tx.check_amount(1u64, to_be_64(&bytes).clone().try_into().unwrap()).unwrap();
        assert_eq!(amount, new_bn.0[0] - 1);

        let (amount, relayer_fee) = tx.check_amount(1u64, to_be_64(&bytes).clone().try_into().unwrap()).unwrap();
        assert!(amount != new_bn.0[0]);
        assert_eq!(relayer_fee, 1);

        // amount larger than u64
        let mut field = FrParameters::MODULUS;
        field.add_nocarry(&new_bn);
        let bytes = <BigInteger256 as BigInteger>::to_bytes_be(&field);
        tx.public_amount = bytes.clone();
        assert!(!tx.is_deposit());
        assert!(tx.check_amount(0u64, to_be_64(&bytes).clone().try_into().unwrap()).is_err());
        // amount larger than u64
        let new_bn = BigInteger256::new([123u64, 23u64, 0, 0]);
        let mut field = FrParameters::MODULUS;
        field.sub_noborrow(&new_bn);
        let bytes = <BigInteger256 as BigInteger>::to_bytes_be(&field);
        tx.public_amount = bytes.clone();
        assert!(!tx.is_deposit());
        assert!(tx.check_amount(0u64, to_be_64(&bytes).clone().try_into().unwrap()).is_err());
    }

    #[test]
    fn test_derivation_checks() {
        let mut public_inputs_vec = Vec::new();
        for input in PUBLIC_INPUTS.chunks(32) {
            public_inputs_vec.push(input.to_vec());
        }

        let proof_a: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&PROOF[0..64])[..], &[0u8][..]].concat()).unwrap();
        let mut proof_a_neg = [0u8;65];
        <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]).unwrap();


        let mut tx: Transaction<'_, '_, '_, TransactionConfig> = Transaction {
            merkle_root: public_inputs_vec[0].clone(),
            public_amount: vec![1u8;32], //public_inputs_vec[1].clone(),
            tx_integrity_hash: public_inputs_vec[2].clone(),
            fee_amount: vec![1u8;32], //public_inputs_vec[3].clone(),
            mint_pubkey: public_inputs_vec[4].clone(),
            checked_public_inputs: Vec::<Vec<u8>>::new(),
            nullifiers: public_inputs_vec[5..7].to_vec(),
            leaves: vec![vec![public_inputs_vec[7].to_vec(), public_inputs_vec[8].to_vec()]],
            relayer_fee: 0,
            proof_a: to_be_64(&proof_a_neg[..64]).to_vec(),
            proof_b: PROOF[64..64 + 128].to_vec(),
            proof_c: PROOF[64 + 128..256].to_vec(),
            encrypted_utxos: Vec::<u8>::new(),
            merkle_root_index: 0,
            transferred_funds: false,
            computed_tx_integrity_hash: false,
            verified_proof : true,
            inserted_leaves : false,
            inserted_nullifier : false,
            fetched_root : false,
            fetched_mint: false,
            e_phantom: PhantomData,
            verifyingkey:&VERIFYING_KEY,
            accounts: None,
            pool_type: vec![0u8;32]
        };

        let program_id = Pubkey::new(&[0u8;32]);
        let (mint, _authority_bump_seed) =
        Pubkey::find_program_address(&[&b"mint"[..]], &program_id);

        assert!(tx.check_spl_pool_account_derivation(&mint, &mint).is_err());

        assert!(tx.check_sol_pool_account_derivation(&mint).is_err());

        let derived_pubkey =
            Pubkey::find_program_address(&[&[0u8;32], &tx.pool_type, &POOL_CONFIG_SEED[..]], &MerkleTreeProgram::id()).0;

        let derived_pubkey_spl =
            Pubkey::find_program_address(&[&mint.to_bytes(), &tx.pool_type, &POOL_SEED[..]], &MerkleTreeProgram::id()).0;


        assert!(tx.check_spl_pool_account_derivation(&derived_pubkey_spl, &mint).is_ok());

        assert!(tx.check_sol_pool_account_derivation(&derived_pubkey).is_ok());
        // let generic = AccountInfo::new(&program_id, false, false,&mut 1u64, [1u8;32].as_mut_slice(), &program_id, false, 0u64);


        // let accounts = Accounts::new(
        //     program_id,
        //     generic,
        //     &accounts.system_program,
        //     &accounts.program_merkle_tree,
        //     &accounts.rent,
        //     &accounts.merkle_tree,
        //     &accounts.pre_inserted_leaves_index,
        //     ctx.accounts.authority.to_account_info(),
        //     Some(&ctx.accounts.token_program),
        //     Some(ctx.accounts.sender.to_account_info()),
        //     Some(ctx.accounts.recipient.to_account_info()),
        //     Some(ctx.accounts.sender_fee.to_account_info()),
        //     Some(ctx.accounts.recipient_fee.to_account_info()),
        //     Some(ctx.accounts.relayer_recipient.to_account_info()),
        //     Some(ctx.accounts.escrow.to_account_info()),
        //     Some(ctx.accounts.token_authority.to_account_info()),
        //     &ctx.accounts.registered_verifier_pda,
        //     ctx.remaining_accounts,
        // )?;



    }

}
