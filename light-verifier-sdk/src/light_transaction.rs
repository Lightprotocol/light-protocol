use anchor_lang::{
    prelude::*,
    solana_program::{keccak::hash, msg, program_pack::Pack, sysvar},
};
use anchor_spl::token::Transfer;
use ark_ff::{
    bytes::{FromBytes, ToBytes},
    BigInteger, BigInteger256, Fp256, FpParameters, PrimeField,
};
use ark_std::{marker::PhantomData, vec::Vec};

use ark_bn254::{Fr, FrParameters};

use groth16_solana::groth16::{Groth16Verifier, Groth16Verifyingkey};
use light_merkle_tree::HashFunction;

use crate::{
    accounts::Accounts,
    cpi_instructions::{
        insert_nullifiers_cpi, insert_two_leaves_cpi, insert_two_leaves_messsage_cpi,
        invoke_indexer_transaction_event, withdraw_sol_cpi, withdraw_spl_cpi,
    },
    errors::VerifierSdkError,
    state::TransactionIndexerEvent,
    utils::{change_endianness, close_account::close_account},
};

use merkle_tree_program::{
    program::MerkleTreeProgram,
    utils::{
        constants::{POOL_CONFIG_SEED, POOL_SEED},
        create_pda::create_and_check_pda,
    },
};
pub const VERIFIER_STATE_SEED: &[u8] = b"VERIFIER_STATE";

pub trait Config {
    /// Number of nullifiers to be inserted with the transaction.
    const NR_NULLIFIERS: usize;
    /// Number of output utxos.
    const NR_LEAVES: usize;
    /// Program ID of the verifier program.
    const ID: Pubkey;
}

#[derive(Clone)]
pub struct Transaction<
    'info,
    'a,
    'c,
    const NR_CHECKED_INPUTS: usize,
    const NR_LEAVES: usize,
    const NR_NULLIFIERS: usize,
    const NR_PUBLIC_INPUTS: usize,
    T: Config,
> {
    pub merkle_root: [u8; 32],
    pub public_amount_spl: &'a [u8; 32],
    pub tx_integrity_hash: [u8; 32],
    pub public_amount_sol: &'a [u8; 32],
    pub mint_pubkey: [u8; 32],
    pub checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
    /// Hash of the optional message included in the transaction.
    pub message_hash: Option<&'a [u8; 32]>,
    pub message: Option<&'a Vec<u8>>,
    pub nullifiers: &'a [[u8; 32]; NR_NULLIFIERS],
    pub leaves: &'a [[[u8; 32]; 2]; NR_LEAVES],
    pub relayer_fee: u64,
    pub proof_a: &'a [u8; 32],
    pub proof_b: &'a [u8; 64],
    pub proof_c: &'a [u8; 32],
    pub encrypted_utxos: &'a Vec<u8>,
    pub pool_type: &'a [u8; 32],
    pub merkle_root_index: usize,
    pub transferred_funds: bool,
    pub computed_tx_integrity_hash: bool,
    pub verified_proof: bool,
    pub inserted_leaves: bool,
    pub inserted_nullifier: bool,
    pub fetched_root: bool,
    pub fetched_mint: bool,
    pub accounts: Option<&'a Accounts<'info, 'a, 'c>>,
    pub e_phantom: PhantomData<T>,
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
}

impl<
        T: Config,
        const NR_CHECKED_INPUTS: usize,
        const NR_LEAVES: usize,
        const NR_NULLIFIERS: usize,
        const NR_PUBLIC_INPUTS: usize,
    > Transaction<'_, '_, '_, NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, NR_PUBLIC_INPUTS, T>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new<'info, 'a, 'c>(
        message_hash: Option<&'a [u8; 32]>,
        message: Option<&'a Vec<u8>>,
        proof_a: &'a [u8; 32],
        proof_b: &'a [u8; 64],
        proof_c: &'a [u8; 32],
        public_amount_spl: &'a [u8; 32],
        public_amount_sol: &'a [u8; 32],
        checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
        nullifiers: &'a [[u8; 32]; NR_NULLIFIERS],
        leaves: &'a [[[u8; 32]; 2]; NR_LEAVES],
        encrypted_utxos: &'a Vec<u8>,
        relayer_fee: u64,
        merkle_root_index: usize,
        pool_type: &'a [u8; 32],
        accounts: Option<&'a Accounts<'info, 'a, 'c>>,
        verifyingkey: &'a Groth16Verifyingkey<'a>,
    ) -> Transaction<'info, 'a, 'c, NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, NR_PUBLIC_INPUTS, T>
    {
        assert_eq!(T::NR_NULLIFIERS, nullifiers.len());
        assert_eq!(T::NR_LEAVES / 2, leaves.len());

        Transaction {
            merkle_root: [0u8; 32],
            public_amount_spl,
            tx_integrity_hash: [0u8; 32],
            public_amount_sol,
            mint_pubkey: [0u8; 32],
            checked_public_inputs,
            message_hash,
            nullifiers,
            leaves,
            relayer_fee,
            proof_a,
            proof_b,
            proof_c,
            encrypted_utxos,
            merkle_root_index,
            transferred_funds: false,
            computed_tx_integrity_hash: false,
            verified_proof: false,
            inserted_leaves: false,
            inserted_nullifier: false,
            fetched_root: false,
            fetched_mint: false,
            e_phantom: PhantomData,
            verifyingkey,
            accounts,
            pool_type,
            message,
        }
    }

    /// Transact is a wrapper function which computes the integrity hash, checks the root,
    /// verifies the zero knowledge proof, inserts leaves, inserts nullifiers, transfers funds and fees.
    pub fn transact(&mut self) -> Result<()> {
        self.insert_message_leaves()?;
        self.compute_tx_integrity_hash()?;
        self.fetch_root()?;
        self.fetch_mint()?;
        self.verify()?;
        self.insert_leaves()?;
        self.insert_nullifiers()?;
        self.emit_indexer_transaction_event()?;
        self.transfer_user_funds()?;
        self.transfer_fee()?;
        self.check_completion()
    }

    pub fn emit_indexer_transaction_event(&mut self) -> Result<()> {
        // Initialize the vector of leaves
        let mut leaves_vec: Vec<[u8; 32]> = Vec::new();

        let merkle_tree = self.accounts.unwrap().transaction_merkle_tree.load_mut()?;

        let mut first_leaf_index = merkle_tree.next_queued_index;

        for (_i, leaves) in self.leaves.iter().enumerate() {
            let leaf_left = change_endianness(&leaves[0]).try_into().unwrap();
            let leaf_right = change_endianness(&leaves[1]).try_into().unwrap();
            leaves_vec.push(leaf_left);
            leaves_vec.push(leaf_right);
            first_leaf_index = first_leaf_index - 2
        }

        let transaction_data_event = TransactionIndexerEvent {
            leaves: leaves_vec.clone(),
            public_amount_sol: self.public_amount_sol.clone(),
            public_amount_spl: self.public_amount_spl.clone(),
            relayer_fee: self.relayer_fee.clone(),
            encrypted_utxos: self.encrypted_utxos.clone(),
            nullifiers: self.nullifiers.to_vec(),
            first_leaf_index: first_leaf_index.clone(),
            message: self.message.as_ref().unwrap_or(&&Vec::<u8>::new()).to_vec(),
        };

        invoke_indexer_transaction_event(
            &transaction_data_event,
            &self.accounts.unwrap().log_wrapper.to_account_info(),
            &self
                .accounts
                .unwrap()
                .transaction_merkle_tree
                .to_account_info(),
        )?;

        Ok(())
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

        assert_eq!(
            NR_PUBLIC_INPUTS,
            5 + NR_NULLIFIERS + NR_LEAVES * 2 + NR_CHECKED_INPUTS,
        );

        let mut public_inputs: [[u8; 32]; NR_PUBLIC_INPUTS] = [[0u8; 32]; NR_PUBLIC_INPUTS];

        public_inputs[0] = self.merkle_root;
        public_inputs[1] = *self.public_amount_spl;
        public_inputs[2] = self.tx_integrity_hash;
        public_inputs[3] = *self.public_amount_sol;
        public_inputs[4] = self.mint_pubkey;

        for (i, input) in self.nullifiers.iter().enumerate() {
            public_inputs[5 + i] = *input;
        }

        for (i, input) in self.leaves.iter().enumerate() {
            public_inputs[5 + NR_NULLIFIERS + i * 2] = input[0];
            public_inputs[5 + NR_NULLIFIERS + i * 2 + 1] = input[1];
        }

        for (i, input) in self.checked_public_inputs.iter().enumerate() {
            public_inputs[5 + NR_NULLIFIERS + NR_LEAVES * 2 + i] = *input;
        }
        msg!("decompressing");

        let proof_a: [u8; 64] = change_endianness(
            &groth16_solana::decompression::decompress_g1(&change_endianness(self.proof_a), true)
                .unwrap(),
        )
        .try_into()
        .unwrap();
        msg!("proof_a decompressed");
        let proof_b: [u8; 128] = change_endianness(
            &groth16_solana::decompression::decompress_g2(&change_endianness(self.proof_b))
                .unwrap(),
        )
        .try_into()
        .unwrap();
        msg!("proof_b decompressed");

        let proof_c: [u8; 64] = change_endianness(
            &groth16_solana::decompression::decompress_g1(&change_endianness(self.proof_c), false)
                .unwrap(),
        )
        .try_into()
        .unwrap();
        msg!("proof_c decompressed");

        // println!("proof_a {:?}", proof_a);
        // println!("proof_b {:?}", proof_b);
        // println!("proof_c {:?}", proof_c);
        let mut verifier = Groth16Verifier::new(
            &proof_a,
            &proof_b,
            &proof_c,
            &public_inputs,
            self.verifyingkey,
        )
        .unwrap();

        match verifier.verify() {
            Ok(_) => {
                self.verified_proof = true;
                Ok(())
            }
            Err(e) => {
                msg!("Public Inputs:");
                msg!("merkle tree root {:?}", self.merkle_root);
                msg!("public_amount_spl {:?}", self.public_amount_spl);
                msg!("tx_integrity_hash {:?}", self.tx_integrity_hash);
                msg!("public_amount_sol {:?}", self.public_amount_sol);
                msg!("mint_pubkey {:?}", self.mint_pubkey);
                msg!("nullifiers {:?}", self.nullifiers);
                msg!("leaves {:?}", self.leaves);
                msg!("checked_public_inputs {:?}", self.checked_public_inputs);
                msg!("error {:?}", e);
                err!(VerifierSdkError::ProofVerificationFailed)
            }
        }
    }

    /// Calls the Merkle tree program via CPI to insert message leaves.
    pub fn insert_message_leaves(&mut self) -> Result<()> {
        let (message_hash, message_merkle_tree) = match self.message_hash {
            Some(message_hash) => match self.accounts.unwrap().message_merkle_tree {
                Some(message_merkle_tree) => (message_hash, message_merkle_tree),
                None => return err!(VerifierSdkError::MessageNoMerkleTreeAccount),
            },
            None => return Ok(()),
        };
        if message_merkle_tree.load()?.merkle_tree.hash_function != HashFunction::Sha256 {
            return err!(VerifierSdkError::MessageMerkleTreeInvalidHashFunction);
        }
        insert_two_leaves_messsage_cpi(
            self.accounts.unwrap().program_id,
            &self.accounts.unwrap().program_merkle_tree.to_account_info(),
            &message_merkle_tree.to_account_info(),
            &self.accounts.unwrap().system_program.to_account_info(),
            message_hash,
            &[0; 32],
        )?;

        Ok(())
    }

    /// Computes the integrity hash of the transaction. This hash is an input to the ZKP, and
    /// ensures that the relayer cannot change parameters of the internal or unshield transaction.
    /// H(recipient_spl||recipient_sol||signer||relayer_fee||encrypted_utxos).
    pub fn compute_tx_integrity_hash(&mut self) -> Result<()> {
        let message_hash = self.message_hash.unwrap_or(&[0u8; 32]);
        let recipient_spl = match self.accounts.unwrap().recipient_spl.as_ref() {
            Some(recipient_spl) => recipient_spl.key().to_bytes(),
            None => [0u8; 32],
        };
        let input = [
            message_hash.to_vec(),
            recipient_spl.to_vec(),
            self.accounts
                .unwrap()
                .recipient_sol
                .as_ref()
                .unwrap()
                .key()
                .to_bytes()
                .to_vec(),
            self.accounts
                .unwrap()
                .signing_address
                .key()
                .to_bytes()
                .to_vec(),
            self.relayer_fee.to_le_bytes().to_vec(),
            self.encrypted_utxos.clone(),
        ]
        .concat();
        // msg!("message_hash: {:?}", message_hash.to_vec());
        // msg!("recipient_spl: {:?}", recipient_spl.to_vec());
        // msg!(
        //     "recipient_sol: {:?}",
        //     self.accounts
        //         .unwrap()
        //         .recipient_sol
        //         .as_ref()
        //         .unwrap()
        //         .key()
        //         .to_bytes()
        //         .to_vec()
        // );
        // msg!(
        //     "signing_address: {:?}",
        //     self.accounts
        //         .unwrap()
        //         .signing_address
        //         .key()
        //         .to_bytes()
        //         .to_vec()
        // );
        // msg!("relayer_fee: {:?}", self.relayer_fee.to_le_bytes().to_vec());
        // msg!("relayer_fee {}", self.relayer_fee);
        // msg!("integrity_hash inputs.len(): {}", input.len());
        // msg!("encrypted_utxos: {:?}", self.encrypted_utxos);

        let hash = Fr::from_be_bytes_mod_order(&hash(&input[..]).try_to_vec()?[..]);
        let mut bytes = Vec::<u8>::new();
        <Fp256<FrParameters> as ToBytes>::write(&hash, &mut bytes).unwrap();
        self.tx_integrity_hash = change_endianness(&bytes[..32]).try_into().unwrap();
        // msg!("tx_integrity_hash be: {:?}", self.tx_integrity_hash);
        // msg!("Fq::from_be_bytes_mod_order(&hash[..]) : {}", hash);
        self.computed_tx_integrity_hash = true;
        Ok(())
    }

    /// Fetches the root according to an index from the passed-in Merkle tree.
    pub fn fetch_root(&mut self) -> Result<()> {
        let merkle_tree = self.accounts.unwrap().transaction_merkle_tree.load()?;
        self.merkle_root =
            change_endianness(merkle_tree.roots[self.merkle_root_index].to_vec().as_ref())
                .try_into()
                .unwrap();
        self.fetched_root = true;
        Ok(())
    }

    /// Fetches the token mint from passed in sender_spl account. If the sender_spl account is not a
    /// token account, native mint is assumed.
    pub fn fetch_mint(&mut self) -> Result<()> {
        match &self.accounts.unwrap().sender_spl {
            Some(sender_spl) => {
                match spl_token::state::Account::unpack(sender_spl.data.borrow().as_ref()) {
                    Ok(sender_mint) => {
                        // Omits the last byte for the mint pubkey bytes to fit into the bn254 field.
                        // msg!(
                        //     "{:?}",
                        //     [vec![0u8], sender_mint.mint.to_bytes()[..31].to_vec()].concat()
                        // );
                        if self.public_amount_spl[24..32] == vec![0u8; 8] {
                            self.mint_pubkey = [0u8; 32];
                        } else {
                            self.mint_pubkey = [
                                vec![0u8],
                                hash(&sender_mint.mint.to_bytes()).try_to_vec()?[1..].to_vec(),
                            ]
                            .concat()
                            .try_into()
                            .unwrap();
                        }

                        self.fetched_mint = true;
                        Ok(())
                    }
                    Err(_) => {
                        self.mint_pubkey = [0u8; 32];
                        self.fetched_mint = true;
                        Ok(())
                    }
                }
            }
            None => {
                self.mint_pubkey = [0u8; 32];
                self.fetched_mint = true;
                Ok(())
            }
        }
    }

    /// Calls the Merkle tree program via cpi to insert transaction leaves.
    pub fn insert_leaves(&mut self) -> Result<()> {
        if !self.verified_proof {
            msg!("Tried to insert leaves without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        if T::NR_NULLIFIERS != self.nullifiers.len() {
            msg!(
                "NR_NULLIFIERS  {} != self.nullifiers.len() {}",
                T::NR_NULLIFIERS,
                self.nullifiers.len()
            );
            return err!(VerifierSdkError::InvalidNrNullifieraccounts);
        }

        if T::NR_NULLIFIERS + (T::NR_LEAVES / 2) != self.accounts.unwrap().remaining_accounts.len()
        {
            msg!(
                "NR_LEAVES / 2
                {} != self.leaves.len() {}",
                T::NR_NULLIFIERS + (T::NR_LEAVES / 2),
                self.leaves.len()
            );
            return err!(VerifierSdkError::InvalidNrLeavesaccounts);
        }

        // check merkle tree
        for (i, leaves) in self.leaves.iter().enumerate() {
            let mut msg = Vec::new();

            if self.encrypted_utxos.len() > i * 256 {
                msg.append(&mut self.encrypted_utxos[i * 256..(i + 1) * 256].to_vec());
            }

            // check account integrities
            insert_two_leaves_cpi(
                self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self.accounts.unwrap().remaining_accounts[T::NR_NULLIFIERS + i].to_account_info(),
                &self
                    .accounts
                    .unwrap()
                    .transaction_merkle_tree
                    .to_account_info(),
                &self.accounts.unwrap().system_program.to_account_info(),
                &self
                    .accounts
                    .unwrap()
                    .registered_verifier_pda
                    .to_account_info(),
                change_endianness(&leaves[0]).try_into().unwrap(),
                change_endianness(&leaves[1]).try_into().unwrap(),
                msg,
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
            msg!(
                "NR_NULLIFIERS  {} != self.nullifiers.len() {}",
                T::NR_NULLIFIERS,
                self.nullifiers.len()
            );
            return err!(VerifierSdkError::InvalidNrNullifieraccounts);
        }

        if T::NR_NULLIFIERS + (T::NR_LEAVES / 2) != self.accounts.unwrap().remaining_accounts.len()
        {
            msg!(
                "NR_LEAVES / 2  {} != self.leaves.len() {}",
                T::NR_LEAVES / 2,
                self.leaves.len()
            );
            return err!(VerifierSdkError::InvalidNrLeavesaccounts);
        }

        insert_nullifiers_cpi(
            self.accounts.unwrap().program_id,
            &self.accounts.unwrap().program_merkle_tree.to_account_info(),
            &self.accounts.unwrap().authority.to_account_info(),
            &self
                .accounts
                .unwrap()
                .system_program
                .to_account_info()
                .clone(),
            &self
                .accounts
                .unwrap()
                .registered_verifier_pda
                .to_account_info(),
            self.nullifiers.to_vec(),
            self.accounts.unwrap().remaining_accounts.to_vec(),
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
            change_endianness(self.public_amount_spl.as_slice())
                .try_into()
                .unwrap(),
        )?;

        // Only transfer if pub amount is greater than zero otherwise recipient_spl and sender_spl accounts are not checked
        if pub_amount_checked > 0 {
            let recipient_mint = spl_token::state::Account::unpack(
                &self
                    .accounts
                    .unwrap()
                    .recipient_spl
                    .as_ref()
                    .unwrap()
                    .data
                    .borrow(),
            )?;
            let sender_mint = spl_token::state::Account::unpack(
                &self
                    .accounts
                    .unwrap()
                    .sender_spl
                    .as_ref()
                    .unwrap()
                    .data
                    .borrow(),
            )?;

            // check mint
            if self.mint_pubkey[1..] != hash(&recipient_mint.mint.to_bytes()).try_to_vec()?[1..] {
                msg!(
                    "*self.mint_pubkey[..31] {:?}, {:?}, recipient_spl mint",
                    self.mint_pubkey[1..].to_vec(),
                    hash(&recipient_mint.mint.to_bytes()).try_to_vec()?[1..].to_vec()
                );
                return err!(VerifierSdkError::InconsistentMintProofSenderOrRecipient);
            }
            if self.mint_pubkey[1..] != hash(&sender_mint.mint.to_bytes()).try_to_vec()?[1..] {
                msg!(
                    "*self.mint_pubkey[..31] {:?}, {:?}, sender_spl mint",
                    self.mint_pubkey[1..].to_vec(),
                    hash(&sender_mint.mint.to_bytes()).try_to_vec()?[1..].to_vec()
                );
                return err!(VerifierSdkError::InconsistentMintProofSenderOrRecipient);
            }

            // is a token deposit or withdrawal
            if self.is_deposit() {
                self.check_spl_pool_account_derivation(
                    &self.accounts.unwrap().recipient_spl.as_ref().unwrap().key(),
                    &recipient_mint.mint,
                )?;

                let seed = merkle_tree_program::ID.to_bytes();
                let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
                    &[seed.as_ref()],
                    self.accounts.unwrap().program_id,
                );
                let bump = &[bump];
                let seeds = &[&[seed.as_slice(), bump][..]];

                let accounts = Transfer {
                    from: self
                        .accounts
                        .unwrap()
                        .sender_spl
                        .as_ref()
                        .unwrap()
                        .to_account_info()
                        .clone(),
                    to: self
                        .accounts
                        .unwrap()
                        .recipient_spl
                        .as_ref()
                        .unwrap()
                        .to_account_info()
                        .clone(),
                    authority: self.accounts.unwrap().authority.to_account_info().clone(),
                };

                let cpi_ctx = CpiContext::new_with_signer(
                    self.accounts
                        .unwrap()
                        .token_program
                        .unwrap()
                        .to_account_info()
                        .clone(),
                    accounts,
                    seeds,
                );
                anchor_spl::token::transfer(cpi_ctx, pub_amount_checked)?;
            } else {
                self.check_spl_pool_account_derivation(
                    &self.accounts.unwrap().sender_spl.as_ref().unwrap().key(),
                    &sender_mint.mint,
                )?;

                // withdraw_spl_cpi
                withdraw_spl_cpi(
                    self.accounts.unwrap().program_id,
                    &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                    &self.accounts.unwrap().authority.to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .sender_spl
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .recipient_spl
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .token_authority
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .token_program
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .registered_verifier_pda
                        .to_account_info(),
                    pub_amount_checked,
                )?;
            }
            msg!("transferred");
        }

        self.transferred_funds = true;
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
            change_endianness(self.public_amount_sol.as_slice())
                .try_into()
                .unwrap(),
        )?;
        msg!("fee amount {} ", fee_amount_checked);
        if fee_amount_checked > 0 {
            if self.is_deposit_fee() {
                msg!("is deposit");
                self.deposit_sol(
                    fee_amount_checked,
                    &self
                        .accounts
                        .unwrap()
                        .recipient_sol
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                )?;
            } else {
                msg!("is withdrawal");

                self.check_sol_pool_account_derivation(
                    &self.accounts.unwrap().sender_sol.as_ref().unwrap().key(),
                    &*self
                        .accounts
                        .unwrap()
                        .sender_sol
                        .as_ref()
                        .unwrap()
                        .to_account_info()
                        .data
                        .try_borrow()
                        .unwrap(),
                )?;
                // withdraws sol for the user
                withdraw_sol_cpi(
                    self.accounts.unwrap().program_id,
                    &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                    &self.accounts.unwrap().authority.to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .sender_sol
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .recipient_sol
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .accounts
                        .unwrap()
                        .registered_verifier_pda
                        .to_account_info(),
                    fee_amount_checked,
                )?;
                msg!("withdrew sol for the user");
            }
        }
        if !self.is_deposit_fee() && relayer_fee > 0 {
            // pays the relayer fee
            withdraw_sol_cpi(
                self.accounts.unwrap().program_id,
                &self.accounts.unwrap().program_merkle_tree.to_account_info(),
                &self.accounts.unwrap().authority.to_account_info(),
                &self
                    .accounts
                    .unwrap()
                    .sender_sol
                    .as_ref()
                    .unwrap()
                    .to_account_info(),
                &self
                    .accounts
                    .unwrap()
                    .relayer_recipient
                    .as_ref()
                    .unwrap()
                    .to_account_info(),
                &self
                    .accounts
                    .unwrap()
                    .registered_verifier_pda
                    .to_account_info(),
                relayer_fee,
            )?;
        }

        Ok(())
    }

    /// Creates and closes an account such that deposited sol is part of the transaction fees.
    fn deposit_sol(&self, amount_checked: u64, recipient_spl: &AccountInfo) -> Result<()> {
        self.check_sol_pool_account_derivation(
            &recipient_spl.key(),
            &*recipient_spl.data.try_borrow().unwrap(),
        )?;
        // TODO: add check that recipient_spl account is initialized

        msg!("is deposit");
        let rent = <Rent as sysvar::Sysvar>::get()?;

        create_and_check_pda(
            self.accounts.unwrap().program_id,
            &self.accounts.unwrap().signing_address.to_account_info(),
            &self
                .accounts
                .unwrap()
                .sender_sol
                .as_ref()
                .unwrap()
                .to_account_info(),
            &self.accounts.unwrap().system_program.to_account_info(),
            &rent,
            &b"escrow"[..],
            &Vec::new(),
            0,              //bytes
            amount_checked, //lamports
            false,          //rent_exempt
        )?;
        close_account(
            &self
                .accounts
                .unwrap()
                .sender_sol
                .as_ref()
                .unwrap()
                .to_account_info(),
            recipient_spl,
        )
    }

    /// Checks whether a transaction is a deposit by inspecting the public amount.
    pub fn is_deposit(&self) -> bool {
        if self.public_amount_spl[24..] != [0u8; 8] && self.public_amount_spl[..24] == [0u8; 24] {
            return true;
        }
        false
    }

    /// Checks whether a transaction is a deposit by inspecting the public amount.
    pub fn is_deposit_fee(&self) -> bool {
        if self.public_amount_sol[24..] != [0u8; 8] && self.public_amount_sol[..24] == [0u8; 24] {
            return true;
        }
        false
    }

    pub fn check_sol_pool_account_derivation(&self, pubkey: &Pubkey, data: &[u8]) -> Result<()> {
        let derived_pubkey = Pubkey::find_program_address(
            &[&[0u8; 32], self.pool_type, POOL_CONFIG_SEED],
            &MerkleTreeProgram::id(),
        );
        let mut cloned_data = data.clone();
        merkle_tree_program::RegisteredAssetPool::try_deserialize(&mut cloned_data)?;

        if derived_pubkey.0 != *pubkey {
            return err!(VerifierSdkError::InvalidSenderorRecipient);
        }
        Ok(())
    }

    pub fn check_spl_pool_account_derivation(&self, pubkey: &Pubkey, mint: &Pubkey) -> Result<()> {
        let derived_pubkey = Pubkey::find_program_address(
            &[&mint.to_bytes(), self.pool_type, POOL_SEED],
            &MerkleTreeProgram::id(),
        );

        if derived_pubkey.0 != *pubkey {
            return err!(VerifierSdkError::InvalidSenderorRecipient);
        }
        Ok(())
    }

    pub fn check_completion(&self) -> Result<()> {
        if self.transferred_funds
            && self.verified_proof
            && self.inserted_leaves
            && self.inserted_nullifier
        {
            return Ok(());
        }
        msg!("verified_proof {}", self.verified_proof);
        msg!("inserted_leaves {}", self.inserted_leaves);
        msg!("transferred_funds {}", self.transferred_funds);
        err!(VerifierSdkError::TransactionIncomplete)
    }

    #[allow(clippy::comparison_chain)]
    pub fn check_amount(&self, relayer_fee: u64, amount: [u8; 32]) -> Result<(u64, u64)> {
        // pub_amount is the public amount included in public inputs for proof verification
        let pub_amount = <BigInteger256 as FromBytes>::read(&amount[..]).unwrap();
        // Big integers are stored in 4 u64 limbs, if the number is <= U64::max() and encoded in little endian,
        // only the first limb is greater than 0.
        // Amounts in shielded accounts are limited to 64bit therefore a withdrawal will always be greater
        // than one U64::max().
        if pub_amount.0[0] > 0
            && pub_amount.0[1] == 0
            && pub_amount.0[2] == 0
            && pub_amount.0[3] == 0
        {
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

            if field.0[0] < relayer_fee {
                msg!(
                    "Withdrawal invalid relayer_fee: pub amount {} < {} fee",
                    field.0[0],
                    relayer_fee
                );
                return Err(VerifierSdkError::WrongPubAmount.into());
            }

            Ok((field.0[0].saturating_sub(relayer_fee), relayer_fee))
        } else if pub_amount.0[0] == 0
            && pub_amount.0[1] == 0
            && pub_amount.0[2] == 0
            && pub_amount.0[3] == 0
        {
            Ok((0, 0))
        } else {
            Err(VerifierSdkError::WrongPubAmount.into())
        }
    }
}
