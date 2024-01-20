use anchor_lang::{
    prelude::*,
    solana_program::{
        hash::{hash, hashv},
        msg,
        program_pack::Pack,
        sysvar,
    },
};
use anchor_spl::token::Transfer;
use ark_bn254::FrParameters;
use ark_ff::{bytes::FromBytes, BigInteger, BigInteger256, FpParameters};
use ark_std::vec::Vec;
use groth16_solana::{
    decompression::{decompress_g1, decompress_g2},
    groth16::{Groth16Verifier, Groth16Verifyingkey},
};
use light_merkle_tree_program::{
    program::LightMerkleTreeProgram,
    state::TransactionMerkleTree,
    utils::{
        accounts::create_and_check_pda,
        constants::{POOL_CONFIG_SEED, POOL_SEED, TRANSACTION_MERKLE_TREE_SEED},
    },
};
use light_sparse_merkle_tree::HashFunction;
use light_utils::{change_endianness, truncate_to_circuit};

use crate::{
    accounts::LightAccounts,
    cpi_instructions::{
        decompress_sol_cpi, decompress_spl_cpi, insert_nullifiers_cpi, insert_two_leaves_cpi,
        insert_two_leaves_event_cpi, invoke_indexer_transaction_event,
    },
    errors::VerifierSdkError,
    state::TransactionIndexerEvent,
    utils::close_account::close_account,
};
pub const VERIFIER_STATE_SEED: &[u8] = b"VERIFIER_STATE";

pub trait Config {
    /// Program ID of the verifier program.
    const ID: Pubkey;
}

#[derive(Clone)]
pub struct Transaction<
    'a,
    'b,
    'c,
    'info,
    const NR_CHECKED_INPUTS: usize,
    const NR_LEAVES: usize,
    const NR_NULLIFIERS: usize,
    const NR_PUBLIC_INPUTS: usize,
    A: LightAccounts<'info>,
> {
    // Client input.
    pub input: TransactionInput<'a, 'b, 'c, 'info, NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, A>,
    // State of transaction.
    pub merkle_root: [u8; 32],
    pub tx_integrity_hash: [u8; 32],
    pub mint_pubkey: [u8; 32],
    pub transferred_funds: bool,
    pub computed_tx_integrity_hash: bool,
    pub verified_proof: bool,
    pub inserted_leaves: bool,
    pub inserted_nullifier: bool,
    pub fetched_root: bool,
    pub fetched_mint: bool,
}

pub struct Message<'a> {
    pub content: &'a Vec<u8>,
    pub hash: [u8; 32],
}

impl<'a> Message<'a> {
    pub fn new(content: &'a Vec<u8>) -> Self {
        let hash = hash(content).to_bytes();
        Message { hash, content }
    }
}

pub struct ProofCompressed {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}

pub struct Proof {
    pub a: [u8; 64],
    pub b: [u8; 128],
    pub c: [u8; 64],
}

pub struct Amounts {
    pub spl: [u8; 32],
    pub sol: [u8; 32],
}

#[derive(Clone)]
pub struct TransactionInput<
    'a,
    'b,
    'c,
    'info,
    const NR_CHECKED_INPUTS: usize,
    const NR_LEAVES: usize,
    const NR_NULLIFIERS: usize,
    A: LightAccounts<'info>,
> {
    pub ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    pub proof: &'a ProofCompressed,
    pub public_amount: &'a Amounts,
    pub message: Option<&'a Message<'a>>,
    pub checked_public_inputs: &'a [[u8; 32]; NR_CHECKED_INPUTS],
    pub nullifiers: &'a [[u8; 32]; NR_NULLIFIERS],
    pub leaves: &'a [[u8; 32]; NR_LEAVES],
    pub encrypted_utxos: &'a Vec<u8>,
    pub rpc_fee: u64,
    pub merkle_root_index: usize,
    pub pool_type: &'a [u8; 32],
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
}

impl<
        'a,
        'b,
        'c,
        'info,
        const NR_CHECKED_INPUTS: usize,
        const NR_LEAVES: usize,
        const NR_NULLIFIERS: usize,
        const NR_PUBLIC_INPUTS: usize,
        A: LightAccounts<'info>,
    >
    Transaction<'a, 'b, 'c, 'info, NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, NR_PUBLIC_INPUTS, A>
{
    pub fn new(
        input: TransactionInput<'a, 'b, 'c, 'info, NR_CHECKED_INPUTS, NR_LEAVES, NR_NULLIFIERS, A>,
    ) -> Transaction<
        'a,
        'b,
        'c,
        'info,
        NR_CHECKED_INPUTS,
        NR_LEAVES,
        NR_NULLIFIERS,
        NR_PUBLIC_INPUTS,
        A,
    > {
        Transaction {
            input,
            merkle_root: [0u8; 32],
            tx_integrity_hash: [0u8; 32],
            mint_pubkey: [0u8; 32],
            transferred_funds: false,
            computed_tx_integrity_hash: false,
            verified_proof: false,
            inserted_leaves: false,
            inserted_nullifier: false,
            fetched_root: false,
            fetched_mint: false,
        }
    }

    /// Transact is a wrapper function which computes the integrity hash, checks the root,
    /// verifies the zero knowledge proof, inserts leaves, inserts nullifiers, transfers funds and fees.
    pub fn transact(&mut self) -> Result<()> {
        self.emit_indexer_transaction_event()?;
        self.compute_tx_integrity_hash()?;
        self.fetch_root()?;
        self.fetch_mint()?;
        self.verify()?;
        self.check_remaining_accounts()?;
        self.insert_leaves()?;
        self.insert_nullifiers()?;
        self.transfer_user_funds()?;
        self.transfer_fee()?;
        self.check_completion()
    }

    pub fn emit_indexer_transaction_event(&mut self) -> Result<()> {
        // Initialize the vector of leaves

        let merkle_tree = self.input.ctx.accounts.get_transaction_merkle_tree();
        let merkle_tree = merkle_tree.load_mut()?;

        let first_leaf_index = merkle_tree.merkle_tree.next_index;

        let message = match &self.input.message {
            Some(message) => message.content.clone(),
            None => Vec::<u8>::new(),
        };
        let transaction_data_event = TransactionIndexerEvent {
            leaves: &self.input.leaves.to_vec(),
            public_amount_sol: self.input.public_amount.sol,
            public_amount_spl: self.input.public_amount.spl,
            rpc_fee: self.input.rpc_fee,
            encrypted_utxos: self.input.encrypted_utxos.clone(),
            nullifiers: self.input.nullifiers.to_vec(),
            first_leaf_index,
            message,
        };

        invoke_indexer_transaction_event(
            &transaction_data_event,
            &self.input.ctx.accounts.get_log_wrapper().to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_transaction_merkle_tree()
                .to_account_info(),
        )?;

        let event_hash = self.compute_event_hash(first_leaf_index);
        self.insert_event_leaves(event_hash)?;

        Ok(())
    }

    fn compute_event_hash(&mut self, first_leaf_index: u64) -> [u8; 32] {
        // TODO: remove vector
        let nullifiers_hash = hashv(
            self.input
                .nullifiers
                .iter()
                .map(|arr| arr.as_slice())
                .collect::<Vec<_>>()
                .as_slice(),
        );
        // TODO: remove vector
        let leaves_hash = hashv(
            self.input
                .leaves
                .iter()
                .map(|two_d| two_d.as_slice())
                .collect::<Vec<_>>()
                .as_slice(),
        );

        let message_hash = match self.input.message {
            Some(message) => message.hash,
            None => [0u8; 32],
        };
        let encrypted_utxos_hash = hash(self.input.encrypted_utxos.as_slice());

        let event_hash = hashv(&[
            leaves_hash.to_bytes().as_slice(),
            self.input.public_amount.spl.as_slice(),
            self.input.public_amount.sol.as_slice(),
            self.input.rpc_fee.to_le_bytes().as_slice(),
            encrypted_utxos_hash.to_bytes().as_slice(),
            nullifiers_hash.to_bytes().as_slice(),
            first_leaf_index.to_le_bytes().as_slice(),
            message_hash.as_slice(),
        ]);

        event_hash.to_bytes()
    }

    /// Calls the Merkle tree program via CPI to insert event leaves.
    fn insert_event_leaves(&mut self, event_hash: [u8; 32]) -> Result<()> {
        let event_merkle_tree = self.input.ctx.accounts.get_event_merkle_tree();
        if event_merkle_tree.load()?.merkle_tree.hash_function != HashFunction::Sha256 as u64 {
            return err!(VerifierSdkError::EventMerkleTreeInvalidHashFunction);
        }
        insert_two_leaves_event_cpi(
            self.input.ctx.program_id,
            &self
                .input
                .ctx
                .accounts
                .get_program_merkle_tree()
                .to_account_info(),
            &self.input.ctx.accounts.get_authority().to_account_info(),
            &event_merkle_tree.to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_system_program()
                .to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_registered_verifier_pda()
                .to_account_info(),
            &event_hash,
            &[0; 32],
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
        msg!("verifying proof");
        assert_eq!(
            NR_PUBLIC_INPUTS,
            5 + NR_NULLIFIERS + NR_LEAVES + NR_CHECKED_INPUTS,
        );

        let mut public_inputs: [[u8; 32]; NR_PUBLIC_INPUTS] = [[0u8; 32]; NR_PUBLIC_INPUTS];

        public_inputs[0] = self.merkle_root;
        public_inputs[1] = self.input.public_amount.spl;
        public_inputs[2] = self.tx_integrity_hash;
        public_inputs[3] = self.input.public_amount.sol;
        public_inputs[4] = self.mint_pubkey;

        for (i, input) in self.input.nullifiers.iter().enumerate() {
            public_inputs[5 + i] = *input;
        }

        for (i, input) in self.input.leaves.chunks(2).enumerate() {
            public_inputs[5 + NR_NULLIFIERS + i * 2] = input[0];
            public_inputs[5 + NR_NULLIFIERS + i * 2 + 1] = input[1];
        }

        for (i, input) in self.input.checked_public_inputs.iter().enumerate() {
            public_inputs[5 + NR_NULLIFIERS + NR_LEAVES + i] = *input;
        }

        // we negate proof a offchain
        let proof_a = decompress_g1(&self.input.proof.a).unwrap();
        let proof_b = decompress_g2(&self.input.proof.b).unwrap();
        let proof_c = decompress_g1(&self.input.proof.c).unwrap();

        let mut verifier = Groth16Verifier::new(
            &proof_a,
            &proof_b,
            &proof_c,
            &public_inputs,
            self.input.verifyingkey,
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
                msg!("public_amount_spl {:?}", self.input.public_amount.spl);
                msg!("tx_integrity_hash {:?}", self.tx_integrity_hash);
                msg!("public_amount_sol {:?}", self.input.public_amount.sol);
                msg!("mint_pubkey {:?}", self.mint_pubkey);
                msg!("nullifiers {:?}", self.input.nullifiers);
                msg!("leaves {:?}", self.input.leaves);
                msg!(
                    "checked_public_inputs {:?}",
                    self.input.checked_public_inputs
                );
                msg!("error {:?}", e);
                err!(VerifierSdkError::ProofVerificationFailed)
            }
        }
    }

    /// Computes the integrity hash of the transaction. This hash is an input to the ZKP, and
    /// ensures that the rpc cannot change parameters of the internal or decompress transaction.
    /// H(recipient_spl||recipient_sol||signer||rpc_fee||encrypted_utxos).
    pub fn compute_tx_integrity_hash(&mut self) -> Result<()> {
        let message_hash = match self.input.message {
            Some(message) => message.hash,
            None => [0u8; 32],
        };
        let recipient_spl = match self.input.ctx.accounts.get_recipient_spl().as_ref() {
            Some(recipient_spl) => recipient_spl.key().to_bytes(),
            None => [0u8; 32],
        };
        let tx_integrity_hash = hashv(&[
            &message_hash,
            &recipient_spl,
            &self
                .input
                .ctx
                .accounts
                .get_recipient_sol()
                .as_ref()
                .unwrap()
                .key()
                .to_bytes(),
            &self
                .input
                .ctx
                .accounts
                .get_signing_address()
                .key()
                .to_bytes(),
            &self.input.rpc_fee.to_be_bytes(),
            self.input.encrypted_utxos,
        ]);
        // msg!("message_hash: {:?}", message_hash.to_vec());
        // msg!("recipient_spl: {:?}", recipient_spl.to_vec());
        // msg!(
        //     "recipient_sol: {:?}",
        //     self.input
        //         .ctx
        //         .accounts
        //         .get_recipient_sol()
        //         .as_ref()
        //         .unwrap()
        //         .key()
        //         .to_bytes()
        //         .to_vec()
        // );
        // msg!(
        //     "signing_address: {:?}",
        //     self.input
        //         .ctx
        //         .accounts
        //         .get_signing_address()
        //         .key()
        //         .to_bytes()
        //         .to_vec()
        // );
        // msg!(
        //     "rpc_fee: {:?}",
        //     self.input.rpc_fee.to_be_bytes().to_vec()
        // );
        // msg!("rpc_fee {}", self.input.rpc_fee);
        // msg!("encrypted_utxos: {:?}", self.input.encrypted_utxos);

        self.tx_integrity_hash = truncate_to_circuit(&tx_integrity_hash.to_bytes());
        self.computed_tx_integrity_hash = true;
        Ok(())
    }

    /// Fetches the root according to an index from the passed-in Merkle tree.
    pub fn fetch_root(&mut self) -> Result<()> {
        let merkle_tree = self.input.ctx.accounts.get_transaction_merkle_tree();
        let merkle_tree = merkle_tree.load()?;
        self.merkle_root = merkle_tree.merkle_tree.roots[self.input.merkle_root_index];
        self.fetched_root = true;
        Ok(())
    }

    /// Fetches the token mint from passed in sender_spl account. If the sender_spl account is not a
    /// token account, native mint is assumed.
    pub fn fetch_mint(&mut self) -> Result<()> {
        match &self.input.ctx.accounts.get_sender_spl() {
            Some(sender_spl) => {
                match spl_token::state::Account::unpack(sender_spl.data.borrow().as_ref()) {
                    Ok(sender_spl) => {
                        // Omits the last byte for the mint pubkey bytes to fit into the bn254 field.
                        // msg!(
                        //     "{:?}",
                        //     [vec![0u8], sender_mint.mint.to_bytes()[..31].to_vec()].concat()
                        // );
                        if self.input.public_amount.spl[24..32] == vec![0u8; 8] {
                            self.mint_pubkey = [0u8; 32];
                        } else {
                            self.mint_pubkey = [
                                vec![0u8],
                                hash(&sender_spl.mint.to_bytes()).try_to_vec()?[1..].to_vec(),
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

    /// Checks the expected number of remaning accounts:
    ///
    /// * Nullifier and leaf accounts (mandatory).
    /// * Merkle tree accounts (optional).
    fn check_remaining_accounts(&self) -> Result<()> {
        let nr_nullifiers_leaves = NR_NULLIFIERS + NR_LEAVES / 2;
        let remaining_accounts_len = self.input.ctx.remaining_accounts.len();
        if remaining_accounts_len != nr_nullifiers_leaves // Only nullifiers and leaves.
            // Nullifiers, leaves and next Merkle trees (transaction, event).
            && remaining_accounts_len != nr_nullifiers_leaves + 2
        {
            msg!(
                "remaining_accounts.len() {} (expected {} or {})",
                remaining_accounts_len,
                nr_nullifiers_leaves,
                nr_nullifiers_leaves + 1
            );
            return err!(VerifierSdkError::InvalidNrRemainingAccounts);
        }

        Ok(())
    }

    /// Calls the Merkle tree program via cpi to insert transaction leaves.
    pub fn insert_leaves(&mut self) -> Result<()> {
        if !self.verified_proof {
            msg!("Tried to insert leaves without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        let transaction_merkle_tree = self.get_transaction_merkle_tree()?;

        // check account integrities
        insert_two_leaves_cpi(
            self.input.ctx.program_id,
            &self
                .input
                .ctx
                .accounts
                .get_program_merkle_tree()
                .to_account_info(),
            &self.input.ctx.accounts.get_authority().to_account_info(),
            &transaction_merkle_tree,
            &self
                .input
                .ctx
                .accounts
                .get_system_program()
                .to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_registered_verifier_pda()
                .to_account_info(),
            // TODO: remove vector or instantiate once for the whole struct
            self.input.leaves.to_vec(),
        )?;

        self.inserted_leaves = true;
        Ok(())
    }

    /// Returns a Transaction Merkle Tree which should be used for the current
    /// transaction. It might be either:
    ///
    /// * Transaction Merkle Tree provided in the context.
    /// * A new Transaction Merkle Tree provided as a remaining account, which
    ///   usually is the case when we reached the switch threshold (255_000).
    ///   We pass a new Transaction Merkle Tree as remaining account, because
    ///   Anchor does not support passing two accounts of the same type in the
    ///   same instruction. We need a second Transacton Merkle Tree account for
    ///   the UTXOs we want to spend in the transaction are in the old
    ///   Transaction Merkle Tree. Since the old Merkle Tree is almost full we
    ///   need to insert the new utxo commitments into the new Transaction
    ///   Merkle Tree.
    fn get_transaction_merkle_tree(&self) -> Result<AccountInfo<'info>> {
        // Index of a new Transaction Merkle Tree in remaining accounts.
        let index = NR_NULLIFIERS + self.input.leaves.len() / 2;

        match self.input.ctx.remaining_accounts.get(index) {
            Some(transaction_merkle_tree) => {
                let transaction_merkle_tree = transaction_merkle_tree.to_account_info();
                self.validate_transaction_merkle_tree(&transaction_merkle_tree)?;
                Ok(transaction_merkle_tree)
            }
            None => Ok(self
                .input
                .ctx
                .accounts
                .get_transaction_merkle_tree()
                .to_account_info()),
        }
    }

    fn validate_transaction_merkle_tree(
        &self,
        transaction_merkle_tree: &AccountInfo,
    ) -> Result<()> {
        let transaction_merkle_tree: AccountLoader<TransactionMerkleTree> =
            AccountLoader::try_from(transaction_merkle_tree)?;
        let index = transaction_merkle_tree.load()?.merkle_tree_nr;
        let (pubkey, _) = Pubkey::find_program_address(
            &[TRANSACTION_MERKLE_TREE_SEED, index.to_le_bytes().as_ref()],
            &LightMerkleTreeProgram::id(),
        );
        if transaction_merkle_tree.key() != pubkey {
            msg!(
                "Transaction Merkle tree address is invalid, expected: {}, got: {}",
                pubkey,
                transaction_merkle_tree.key()
            );
            return err!(VerifierSdkError::InvalidTransactionMerkleTreeAddress);
        }
        Ok(())
    }

    /// Calls merkle tree via cpi to insert nullifiers.
    pub fn insert_nullifiers(&mut self) -> Result<()> {
        if !self.verified_proof {
            msg!("Tried to insert nullifiers without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        insert_nullifiers_cpi(
            self.input.ctx.program_id,
            &self
                .input
                .ctx
                .accounts
                .get_program_merkle_tree()
                .to_account_info(),
            &self.input.ctx.accounts.get_authority().to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_system_program()
                .to_account_info()
                .clone(),
            &self
                .input
                .ctx
                .accounts
                .get_registered_verifier_pda()
                .to_account_info(),
            self.input.nullifiers.to_vec(),
            self.input.ctx.remaining_accounts.to_vec(),
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
        let (pub_amount_checked, _) =
            self.check_amount(0, change_endianness(&self.input.public_amount.spl))?;

        // Only transfer if pub amount is greater than zero otherwise recipient_spl and sender_spl accounts are not checked
        if pub_amount_checked > 0 {
            let recipient_spl = spl_token::state::Account::unpack(
                &self
                    .input
                    .ctx
                    .accounts
                    .get_recipient_spl()
                    .as_ref()
                    .unwrap()
                    .data
                    .borrow(),
            )?;
            let sender_spl = spl_token::state::Account::unpack(
                &self
                    .input
                    .ctx
                    .accounts
                    .get_sender_spl()
                    .as_ref()
                    .unwrap()
                    .data
                    .borrow(),
            )?;

            // check mint
            if self.mint_pubkey[1..] != hash(&recipient_spl.mint.to_bytes()).try_to_vec()?[1..] {
                msg!(
                    "*self.mint_pubkey[..31] {:?}, {:?}, recipient_spl mint",
                    self.mint_pubkey[1..].to_vec(),
                    hash(&recipient_spl.mint.to_bytes()).try_to_vec()?[1..].to_vec()
                );
                return err!(VerifierSdkError::InconsistentMintProofSenderOrRecipient);
            }

            // is a token compress or decompress
            if self.is_compress_spl() {
                self.compress_spl(pub_amount_checked, sender_spl, recipient_spl)?;
            } else {
                self.check_spl_pool_account_derivation(
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_sender_spl()
                        .as_ref()
                        .unwrap()
                        .key(),
                    &sender_spl.mint,
                )?;

                // compress_spl_cpi
                decompress_spl_cpi(
                    self.input.ctx.program_id,
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_program_merkle_tree()
                        .to_account_info(),
                    &self.input.ctx.accounts.get_authority().to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_sender_spl()
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_recipient_spl()
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_token_authority()
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_token_program()
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_registered_verifier_pda()
                        .to_account_info(),
                    pub_amount_checked,
                )?;
            }
            msg!("transferred");
        }

        self.transferred_funds = true;
        Ok(())
    }

    /// Transfers the rpc fee  to or from a merkle tree liquidity pool.
    pub fn transfer_fee(&self) -> Result<()> {
        if !self.verified_proof {
            msg!("Tried to transfer fees without verifying the proof.");
            return err!(VerifierSdkError::ProofNotVerified);
        }

        // check that it is the native token pool
        let (fee_amount_checked, rpc_fee) = self.check_amount(
            self.input.rpc_fee,
            change_endianness(&self.input.public_amount.sol),
        )?;
        msg!("fee amount {} ", fee_amount_checked);
        if fee_amount_checked > 0 {
            if self.is_compress_sol() {
                msg!("is compress");
                self.compress_sol(
                    fee_amount_checked,
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_recipient_sol()
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                )?;
            } else {
                msg!("is decompress");

                self.check_sol_pool_account_derivation(
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_sender_sol()
                        .as_ref()
                        .unwrap()
                        .key(),
                    *self
                        .input
                        .ctx
                        .accounts
                        .get_sender_sol()
                        .as_ref()
                        .unwrap()
                        .to_account_info()
                        .data
                        .try_borrow()
                        .unwrap(),
                )?;
                // Decompress sol for the user
                msg!("decompress sol cpi");
                decompress_sol_cpi(
                    self.input.ctx.program_id,
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_program_merkle_tree()
                        .to_account_info(),
                    &self.input.ctx.accounts.get_authority().to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_sender_sol()
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_recipient_sol()
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    &self
                        .input
                        .ctx
                        .accounts
                        .get_registered_verifier_pda()
                        .to_account_info(),
                    fee_amount_checked,
                )?;
                msg!("decompressed sol for the user");
            }
        }
        if !self.is_compress_sol() && rpc_fee > 0 {
            // pays the rpc fee
            decompress_sol_cpi(
                self.input.ctx.program_id,
                &self
                    .input
                    .ctx
                    .accounts
                    .get_program_merkle_tree()
                    .to_account_info(),
                &self.input.ctx.accounts.get_authority().to_account_info(),
                &self
                    .input
                    .ctx
                    .accounts
                    .get_sender_sol()
                    .as_ref()
                    .unwrap()
                    .to_account_info(),
                &self
                    .input
                    .ctx
                    .accounts
                    .get_rpc_recipient_sol()
                    .as_ref()
                    .to_account_info(),
                &self
                    .input
                    .ctx
                    .accounts
                    .get_registered_verifier_pda()
                    .to_account_info(),
                rpc_fee,
            )?;
        }

        Ok(())
    }

    /// Creates and closes an account such that compressed sol is part of the transaction fees.
    fn compress_sol(&self, amount_checked: u64, recipient_sol: &AccountInfo) -> Result<()> {
        self.check_sol_pool_account_derivation(
            &recipient_sol.key(),
            *recipient_sol.data.try_borrow().unwrap(),
        )?;

        let signing_address = self
            .input
            .ctx
            .accounts
            .get_signing_address()
            .to_account_info();
        let sender_sol = self
            .input
            .ctx
            .accounts
            .get_sender_sol()
            .unwrap()
            .to_account_info();

        msg!("is compress");
        let rent = <Rent as sysvar::Sysvar>::get()?;

        create_and_check_pda(
            self.input.ctx.program_id,
            &signing_address,
            &sender_sol,
            &self
                .input
                .ctx
                .accounts
                .get_system_program()
                .to_account_info(),
            &rent,
            &b"escrow"[..],
            &Vec::new(),
            0,              //bytes
            amount_checked, //lamports
            false,          //rent_exempt
        )?;
        close_account(
            &self
                .input
                .ctx
                .accounts
                .get_sender_sol()
                .as_ref()
                .unwrap()
                .to_account_info(),
            recipient_sol,
        )
    }

    fn compress_spl(
        &self,
        pub_amount_checked: u64,
        sender_spl: spl_token::state::Account,
        recipient_spl: spl_token::state::Account,
    ) -> Result<()> {
        self.check_spl_pool_account_derivation(
            &self
                .input
                .ctx
                .accounts
                .get_recipient_spl()
                .as_ref()
                .unwrap()
                .key(),
            &recipient_spl.mint,
        )?;

        let signing_address = self
            .input
            .ctx
            .accounts
            .get_signing_address()
            .to_account_info();

        if sender_spl.owner.as_ref() != signing_address.key().as_ref() {
            msg!(
                "sender_spl owned by: {}, expected signer: {}",
                sender_spl.owner,
                signing_address.key()
            );
            return err!(VerifierSdkError::InvalidSenderOrRecipient);
        }

        let seed = light_merkle_tree_program::ID.to_bytes();
        let (_, bump) = anchor_lang::prelude::Pubkey::find_program_address(
            &[seed.as_ref()],
            self.input.ctx.program_id,
        );
        let bump = &[bump];
        let seeds = &[&[seed.as_slice(), bump][..]];

        let accounts = Transfer {
            from: self
                .input
                .ctx
                .accounts
                .get_sender_spl()
                .as_ref()
                .unwrap()
                .to_account_info()
                .clone(),
            to: self
                .input
                .ctx
                .accounts
                .get_recipient_spl()
                .as_ref()
                .unwrap()
                .to_account_info()
                .clone(),
            authority: self
                .input
                .ctx
                .accounts
                .get_authority()
                .to_account_info()
                .clone(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            self.input
                .ctx
                .accounts
                .get_token_program()
                .unwrap()
                .to_account_info()
                .clone(),
            accounts,
            seeds,
        );
        anchor_spl::token::transfer(cpi_ctx, pub_amount_checked)?;

        Ok(())
    }

    /// Checks whether a transaction is a compression by inspecting the public amount.
    pub fn is_compress_spl(&self) -> bool {
        if self.input.public_amount.spl[24..] != [0u8; 8]
            && self.input.public_amount.spl[..24] == [0u8; 24]
        {
            return true;
        }
        false
    }

    /// Checks whether a transaction is a deposit by inspecting the public amount.
    pub fn is_compress_sol(&self) -> bool {
        if self.input.public_amount.sol[24..] != [0u8; 8]
            && self.input.public_amount.sol[..24] == [0u8; 24]
        {
            return true;
        }
        false
    }

    pub fn check_sol_pool_account_derivation(&self, pubkey: &Pubkey, data: &[u8]) -> Result<()> {
        let derived_pubkey = Pubkey::find_program_address(
            &[&[0u8; 32], self.input.pool_type, POOL_CONFIG_SEED],
            &LightMerkleTreeProgram::id(),
        );
        let mut cloned_data = data;
        light_merkle_tree_program::RegisteredAssetPool::try_deserialize(&mut cloned_data)?;

        if derived_pubkey.0 != *pubkey {
            return err!(VerifierSdkError::InvalidSenderOrRecipient);
        }
        Ok(())
    }

    pub fn check_spl_pool_account_derivation(&self, pubkey: &Pubkey, mint: &Pubkey) -> Result<()> {
        let derived_pubkey = Pubkey::find_program_address(
            &[&mint.to_bytes(), self.input.pool_type, POOL_SEED],
            &LightMerkleTreeProgram::id(),
        );

        if derived_pubkey.0 != *pubkey {
            return err!(VerifierSdkError::InvalidSenderOrRecipient);
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
    pub fn check_amount(&self, rpc_fee: u64, amount: [u8; 32]) -> Result<(u64, u64)> {
        // pub_amount is the public amount included in public inputs for proof verification
        let pub_amount = <BigInteger256 as FromBytes>::read(&amount[..]).unwrap();
        // Big integers are stored in 4 u64 limbs, if the number is <= U64::max() and encoded in little endian,
        // only the first limb is greater than 0.
        // Amounts in compressed accounts are limited to 64bit therefore a decompression will always be greater
        // than one U64::max().
        if pub_amount.0[0] > 0
            && pub_amount.0[1] == 0
            && pub_amount.0[2] == 0
            && pub_amount.0[3] == 0
        {
            if rpc_fee != 0 {
                msg!("rpc_fee {}", rpc_fee);
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

            if field.0[0] < rpc_fee {
                msg!(
                    "Decompress invalid rpc_fee: pub amount {} < {} fee",
                    field.0[0],
                    rpc_fee
                );
                return Err(VerifierSdkError::WrongPubAmount.into());
            }

            Ok((field.0[0].saturating_sub(rpc_fee), rpc_fee))
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
