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
use light_macros::heap_neutral;
use light_merkle_tree_program::{
    emit_indexer_event,
    program::LightMerkleTreeProgram,
    state_merkle_tree_from_bytes,
    utils::{
        accounts::create_and_check_pda,
        constants::{POOL_CONFIG_SEED, POOL_SEED},
    },
};
use light_utils::{change_endianness, truncate_to_circuit};

use crate::{
    accounts::LightAccounts,
    cpi_instructions::{
        decompress_sol_cpi, decompress_spl_cpi, insert_nullifiers_cpi, insert_two_leaves_cpi,
        insert_two_leaves_event_cpi,
    },
    errors::VerifierSdkError,
    state::TransactionIndexerEventV1,
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
        }
    }

    /// Transact is a wrapper function which computes the integrity hash, checks the root,
    /// verifies the zero knowledge proof, inserts leaves, inserts nullifiers, transfers funds and fees.
    #[inline(never)]
    pub fn transact(&mut self) -> Result<()> {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("pre transact");

        self.emit_indexer_transaction_event()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("emit_indexer_transaction_event");
        self.compute_tx_integrity_hash()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("compute_tx_integrity_hash");
        self.fetch_root()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("fetch_root");
        self.fetch_mint()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("fetch_mint");
        self.verify()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("verify");
        self.check_remaining_accounts()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("check_remaining_accounts");
        self.insert_leaves()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("insert_leaves");
        self.insert_nullifiers()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("insert_nullifiers");
        self.transfer_user_funds()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("transfer_user_funds");
        self.transfer_fee()
    }

    #[heap_neutral]
    #[inline(never)]
    pub fn emit_indexer_transaction_event(&self) -> Result<()> {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("pre assemble TransactionIndexerEvent");

        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("pre load MerkleTreeSet");
        let merkle_tree_set = self.input.ctx.accounts.get_merkle_tree_set().load()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("post load MerkleTreeSet");

        // Initialize the vector of leaves
        let first_leaf_index =
            state_merkle_tree_from_bytes(&merkle_tree_set.state_merkle_tree).next_index;
        drop(merkle_tree_set);
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("drop MerkleTreeSet");

        let message = match &self.input.message {
            Some(message) => message.content.clone(),
            None => Vec::<u8>::new(),
        };
        let leaves = self.input.leaves.to_vec();
        let transaction_data_event = TransactionIndexerEventV1 {
            leaves: &leaves,
            public_amount_sol: self.input.public_amount.sol,
            public_amount_spl: self.input.public_amount.spl,
            rpc_fee: self.input.rpc_fee,
            encrypted_utxos: self.input.encrypted_utxos.clone(),
            nullifiers: self.input.nullifiers.to_vec(),
            first_leaf_index,
            message,
        };

        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("post assemble TransactionIndexerEvent");

        emit_indexer_event(
            transaction_data_event.try_to_vec()?,
            &self.input.ctx.accounts.get_log_wrapper().to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_merkle_tree_set()
                .to_account_info(),
        )?;

        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("post invoke_indexer_transaction_event");

        let event_hash = self.compute_event_hash(first_leaf_index);
        self.insert_event_leaves(event_hash)?;

        Ok(())
    }

    #[heap_neutral]
    #[inline(never)]
    fn compute_event_hash(&self, first_leaf_index: u64) -> [u8; 32] {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("pre compute hash");

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

        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("post compute hash");

        event_hash.to_bytes()
    }

    /// Calls the Merkle tree program via CPI to insert event leaves.
    #[heap_neutral]
    #[inline(never)]
    fn insert_event_leaves(&self, event_hash: [u8; 32]) -> Result<()> {
        insert_two_leaves_event_cpi(
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
                .get_merkle_tree_set()
                .to_account_info(),
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

    #[inline(never)]
    pub fn verify(&self) -> Result<()> {
        #[cfg(all(target_os = "solana", feature = "custom-heap"))]
        let pos = custom_heap::get_heap_pos();
        // 4(spl, sol, dataHash, mint) + nullifiers + leaves + nullifier roots + leaves roots
        // assert_eq!(
        //     NR_PUBLIC_INPUTS,
        //     4 + NR_NULLIFIERS + NR_LEAVES + NR_CHECKED_INPUTS + NR_NULLIFIERS + NR_LEAVES,
        // );
        // TODO: we should autogenerate this if we go for more rust sdk
        #[derive(AnchorSerialize, Debug)]
        pub struct PrivateTransactionPublicInputs<
            const NR_NULLIFIERS: usize,
            const NR_LEAVES: usize,
            const NR_CHECKED_INPUTS: usize,
        > {
            pub merkle_root: [[u8; 32]; NR_NULLIFIERS],
            pub nullifier_root: [[u8; 32]; NR_NULLIFIERS],
            pub public_amount_spl: [u8; 32],
            pub tx_integrity_hash: [u8; 32],
            pub public_amount_sol: [u8; 32],
            pub mint_pubkey: [u8; 32],
            pub nullifiers: [[u8; 32]; NR_NULLIFIERS],
            pub leaves: [[u8; 32]; NR_LEAVES],
            // pub new_adresses: [[u8; 32]; NR_LEAVES],
            pub checked_public_inputs: [[u8; 32]; NR_CHECKED_INPUTS],
        }

        let public_inputs_struct = PrivateTransactionPublicInputs {
            merkle_root: [self.merkle_root; NR_NULLIFIERS],
            nullifier_root: [[0u8; 32]; NR_NULLIFIERS], // placeholder value replace when we have the nullifier merkle tree
            public_amount_spl: self.input.public_amount.spl,
            tx_integrity_hash: self.tx_integrity_hash,
            public_amount_sol: self.input.public_amount.sol,
            mint_pubkey: self.mint_pubkey,
            nullifiers: *self.input.nullifiers,
            leaves: *self.input.leaves,
            checked_public_inputs: *self.input.checked_public_inputs,
        };
        let public_inputs: Vec<[u8; 32]> = public_inputs_struct
            .try_to_vec()?
            .chunks(32)
            .map(|x| {
                let y: [u8; 32] = x.try_into().unwrap();
                y
            })
            .collect();
        let public_inputs: [[u8; 32]; NR_PUBLIC_INPUTS] = public_inputs.try_into().unwrap();

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
                #[cfg(all(target_os = "solana", feature = "custom-heap"))]
                custom_heap::free_heap(pos);
                Ok(())
            }
            Err(e) => {
                msg!("Public Inputs: {:?} ", public_inputs);
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
        Ok(())
    }

    /// Fetches the root according to an index from the passed-in Merkle tree.
    pub fn fetch_root(&mut self) -> Result<()> {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("pre load MerkleTreeSet");
        let merkle_tree_set = self.input.ctx.accounts.get_merkle_tree_set().load()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("post load MerkleTreeSet");

        self.merkle_root = state_merkle_tree_from_bytes(&merkle_tree_set.state_merkle_tree).roots
            [self.input.merkle_root_index];
        drop(merkle_tree_set);
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("drop MerkleTreeSet");

        Ok(())
    }

    /// Fetches the token mint from passed in sender_spl account. If the sender_spl account is not a
    /// token account, native mint is assumed.
    pub fn fetch_mint(&mut self) -> Result<()> {
        match &self.input.ctx.accounts.get_sender_spl() {
            Some(sender_spl) => {
                let unpacked_account =
                    spl_token::state::Account::unpack(sender_spl.data.borrow().as_ref());
                match unpacked_account {
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

                        Ok(())
                    }
                    Err(_) => {
                        self.mint_pubkey = [0u8; 32];
                        Ok(())
                    }
                }
            }
            None => {
                self.mint_pubkey = [0u8; 32];
                Ok(())
            }
        }
    }

    /// Checks the expected number of remaning accounts:
    ///
    /// * Nullifier and leaf accounts (mandatory).
    /// * Merkle tree accounts (optional).
    #[heap_neutral]
    #[inline(never)]
    fn check_remaining_accounts(&self) -> Result<()> {
        let remaining_accounts_len = self.input.ctx.remaining_accounts.len();
        if remaining_accounts_len != NR_NULLIFIERS {
            msg!(
                "remaining_accounts.len() {} (expected {})",
                remaining_accounts_len,
                NR_NULLIFIERS,
            );
            return err!(VerifierSdkError::InvalidNrRemainingAccounts);
        }

        Ok(())
    }

    /// Calls the Merkle tree program via cpi to insert transaction leaves.
    #[heap_neutral]
    #[inline(never)]
    pub fn insert_leaves(&self) -> Result<()> {
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
            &self
                .input
                .ctx
                .accounts
                .get_merkle_tree_set()
                .to_account_info(),
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
            &self.input.ctx.accounts.get_log_wrapper().to_account_info(),
            // TODO: remove vector or instantiate once for the whole struct
            self.input.leaves.to_vec(),
        )?;
        Ok(())
    }

    /// Calls merkle tree via cpi to insert nullifiers.
    #[heap_neutral]
    #[inline(never)]
    pub fn insert_nullifiers(&self) -> Result<()> {
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
        Ok(())
    }

    /// Transfers user funds either to or from a merkle tree liquidity pool.
    #[heap_neutral]
    #[inline(never)]
    pub fn transfer_user_funds(&self) -> Result<()> {
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
        }
        Ok(())
    }

    /// Transfers the rpc fee  to or from a merkle tree liquidity pool.
    #[heap_neutral]
    #[inline(never)]
    pub fn transfer_fee(&self) -> Result<()> {
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
    #[heap_neutral]
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

    #[heap_neutral]
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

    #[heap_neutral]
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

    #[heap_neutral]
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

#[cfg(feature = "custom-heap")]
pub mod custom_heap {
    use std::{alloc::Layout, mem::size_of, ptr::null_mut, usize};

    #[cfg(target_os = "solana")]
    use anchor_lang::{
        prelude::*,
        solana_program::entrypoint::{HEAP_LENGTH, HEAP_START_ADDRESS},
    };

    #[cfg(target_os = "solana")]
    #[global_allocator]
    static A: BumpAllocator = BumpAllocator {
        start: HEAP_START_ADDRESS as usize,
        len: HEAP_LENGTH,
    };

    pub struct BumpAllocator {
        pub start: usize,
        pub len: usize,
    }

    impl BumpAllocator {
        const RESERVED_MEM: usize = size_of::<*mut u8>();

        /// Return heap position as of this call/// Returns the current position of the heap.
        ///
        /// # Safety
        /// This function is unsafe because it returns a raw pointer.
        pub unsafe fn pos(&self) -> usize {
            let pos_ptr = self.start as *mut usize;
            *pos_ptr
        }

        /// Reset heap start cursor to position.
        /// # Safety
        /// Do not use this function if you initialized heap memory after pos which you still need.
        pub unsafe fn move_cursor(&self, pos: usize) {
            let pos_ptr = self.start as *mut usize;
            *pos_ptr = pos;
        }
    }

    unsafe impl std::alloc::GlobalAlloc for BumpAllocator {
        #[inline]
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let pos_ptr = self.start as *mut usize;

            let mut pos = *pos_ptr;
            if pos == 0 {
                // First time, set starting position
                pos = self.start + self.len;
            }
            pos = pos.saturating_sub(layout.size());
            pos &= !(layout.align().wrapping_sub(1));
            if pos < self.start + BumpAllocator::RESERVED_MEM {
                return null_mut();
            }
            *pos_ptr = pos;
            pos as *mut u8
        }
        #[inline]
        unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
            // no dellaoc in Solana runtime :*(
        }
    }
    #[cfg(target_os = "solana")]
    pub fn log_total_heap(string: &str) -> u64 {
        const HEAP_END_ADDRESS: u64 = HEAP_START_ADDRESS as u64 + HEAP_LENGTH as u64;

        msg!("{}", string);
        let heap_start = unsafe { A.pos() } as u64;
        let heap_used = HEAP_END_ADDRESS - heap_start;
        msg!("total heap used: {}", heap_used);
        heap_used
    }
    #[cfg(target_os = "solana")]
    pub fn get_heap_pos() -> usize {
        let heap_start = unsafe { A.pos() } as usize;
        heap_start
    }
    #[cfg(target_os = "solana")]
    pub fn free_heap(pos: usize) {
        unsafe { A.move_cursor(pos) };
    }
}

#[cfg(test)]
mod test {
    use std::{
        alloc::{GlobalAlloc, Layout},
        mem::size_of,
        ptr::null_mut,
    };

    use custom_heap::BumpAllocator;

    use super::*;

    #[test]
    fn test_pos_move_cursor_heap() {
        use std::mem::size_of;

        use super::custom_heap::BumpAllocator;
        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            let pos = unsafe { allocator.pos() };
            assert_eq!(pos, unsafe { allocator.pos() });
            assert_eq!(pos, 0);
            let mut pos_64 = 0;
            for i in 0..128 - size_of::<*mut u8>() {
                if i == 64 {
                    pos_64 = unsafe { allocator.pos() };
                }
                let ptr = unsafe {
                    allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap())
                };
                assert_eq!(
                    ptr as *const _ as usize,
                    heap.as_ptr() as *const _ as usize + heap.len() - 1 - i
                );
                assert_eq!(ptr as *const _ as usize, unsafe { allocator.pos() });
            }
            let pos_128 = unsafe { allocator.pos() };
            // free half of the heap
            unsafe { allocator.move_cursor(pos_64) };
            assert_eq!(pos_64, unsafe { allocator.pos() });
            assert_ne!(pos_64 + 1, unsafe { allocator.pos() });
            // allocate second half of the heap again
            for i in 0..64 - size_of::<*mut u8>() {
                let ptr = unsafe {
                    allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap())
                };
                assert_eq!(
                    ptr as *const _ as usize,
                    heap.as_ptr() as *const _ as usize + heap.len() - 1 - (i + 64)
                );
                assert_eq!(ptr as *const _ as usize, unsafe { allocator.pos() });
            }
            assert_eq!(pos_128, unsafe { allocator.pos() });
            // free all of the heap
            unsafe { allocator.move_cursor(pos) };
            assert_eq!(pos, unsafe { allocator.pos() });
            assert_ne!(pos + 1, unsafe { allocator.pos() });
        }
    }

    /// taken from solana-program https://github.com/solana-labs/solana/blob/9a520fd5b42bafefa4815afe3e5390b4ea7482ca/sdk/program/src/entrypoint.rs#L374
    #[test]
    fn test_bump_allocator() {
        // alloc the entire
        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            for i in 0..128 - size_of::<*mut u8>() {
                let ptr = unsafe {
                    allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap())
                };
                assert_eq!(
                    ptr as *const _ as usize,
                    heap.as_ptr() as *const _ as usize + heap.len() - 1 - i
                );
            }
            assert_eq!(null_mut(), unsafe {
                allocator.alloc(Layout::from_size_align(1, 1).unwrap())
            });
        }
        // check alignment
        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u8>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u16>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u16>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u32>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u32>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u64>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u64>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u128>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u128>()));
            let ptr = unsafe { allocator.alloc(Layout::from_size_align(1, 64).unwrap()) };
            assert_eq!(0, ptr.align_offset(64));
        }
        // alloc entire block (minus the pos ptr)
        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(120, size_of::<u8>()).unwrap()) };
            assert_ne!(ptr, null_mut());
            assert_eq!(0, ptr.align_offset(size_of::<u64>()));
        }
    }
}
