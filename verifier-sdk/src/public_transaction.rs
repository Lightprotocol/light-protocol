use std::{collections::HashMap, marker::PhantomData};

#[cfg(all(target_os = "solana", feature = "custom-heap"))]
use crate::light_transaction::custom_heap;
use crate::{
    accounts::LightPublicAccounts,
    cpi_instructions::{
        decompress_sol_cpi, decompress_spl_cpi, insert_public_nullifier_into_indexed_array_cpi,
        insert_two_leaves_parallel_cpi, invoke_indexer_transaction_event,
    },
    errors::VerifierSdkError,
    light_transaction::{Message, ProofCompressed},
    utils::close_account::close_account,
    utxo::{Utxo, DEFAULT_PUBKEY, DEFAULT_UTXO_HASH},
};
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
    program::LightMerkleTreeProgram,
    utils::{
        accounts::create_and_check_pda,
        constants::{POOL_CONFIG_SEED, POOL_SEED},
    },
};
use light_utils::{change_endianness, truncate_to_circuit};
use psp_account_compression::{state::ConcurrentMerkleTreeAccount, state_merkle_tree_from_bytes};

pub const VERIFIER_STATE_SEED: &[u8] = b"VERIFIER_STATE";

pub trait Config {
    /// Program ID of the verifier program.
    const ID: Pubkey;
}

#[derive(AnchorSerialize, Debug)]
pub struct PublicTransactionPublicInputsTransfer<
    const NR_IN_UTXOS: usize,
    const NR_OUT_UTXOS: usize,
> {
    pub state_merkle_roots: [[u8; 32]; NR_IN_UTXOS],
    // pub public_amount_spl: [u8; 32],
    pub tx_integrity_hash: [u8; 32],
    // pub public_amount_sol: [u8; 32],
    // pub mint_pubkey: [u8; 32],
    pub in_utxo_hashes: [[u8; 32]; NR_IN_UTXOS],
    pub out_utxo_hashes: [[u8; 32]; NR_OUT_UTXOS],
    // pub new_adresses: [[u8; 32]; NR_OUT_UTXOS],
    // pub in_utxo_data_hashes: [[u8; 32]; NR_IN_UTXOS],
}

impl<
        'a,
        'b,
        'c,
        'info,
        const NR_CHECKED_INPUTS: usize,
        const NR_OUT_UTXOS: usize,
        const NR_IN_UTXOS: usize,
        const NR_PUBLIC_INPUTS: usize,
        A: LightPublicAccounts<'info>,
        P,
    >
    TransactionConvertible<
        'a,
        'b,
        'c,
        'info,
        NR_CHECKED_INPUTS,
        NR_OUT_UTXOS,
        NR_IN_UTXOS,
        NR_PUBLIC_INPUTS,
        A,
        P,
    > for PublicTransactionPublicInputsTransfer<NR_IN_UTXOS, NR_OUT_UTXOS>
where
    P: AnchorSerialize,
{
    fn from_transaction(
        input: &PublicTransaction<
            'a,
            'b,
            'c,
            'info,
            NR_CHECKED_INPUTS,
            NR_OUT_UTXOS,
            NR_IN_UTXOS,
            NR_PUBLIC_INPUTS,
            A,
            P,
        >,
    ) -> Self {
        PublicTransactionPublicInputsTransfer {
            state_merkle_roots: input.state_merkle_roots,
            // public_amount_spl: input
            //     .input
            //     .public_amount
            //     .unwrap_or(&Amounts::default())
            //     .spl
            //     .unwrap_or([0u8; 32]),
            tx_integrity_hash: input.tx_integrity_hash,
            // public_amount_sol: input
            //     .input
            //     .public_amount
            //     .unwrap_or(&Amounts::default())
            //     .sol
            //     .unwrap_or([0u8; 32]),
            // mint_pubkey: input.mint_pubkey,
            in_utxo_hashes: input.in_utxo_hashes_proof,
            out_utxo_hashes: input.out_utxo_hashes_proof,
            // checked_public_inputs: *input.checked_public_inputs,
            // in_utxo_data_hashes: input
            //     .input
            //     .in_utxo_data_hashes
            //     .map(|x| x.unwrap_or([0u8; 32])),
            // new_adresses: input.input.new_addresses.map(|x| x.unwrap_or([0u8; 32])),
        }
    }
}

#[derive(AnchorSerialize, Debug)]
pub struct PublicTransactionPublicInputs<const NR_IN_UTXOS: usize, const NR_OUT_UTXOS: usize> {
    pub state_merkle_roots: [[u8; 32]; NR_IN_UTXOS],
    pub public_amount_spl: [u8; 32],
    pub tx_integrity_hash: [u8; 32],
    pub public_amount_sol: [u8; 32],
    pub mint_pubkey: [u8; 32],
    pub in_utxo_hashes: [[u8; 32]; NR_IN_UTXOS],
    pub out_utxo_hashes_proof: [[u8; 32]; NR_OUT_UTXOS],
    pub new_adresses: [[u8; 32]; NR_OUT_UTXOS],
    pub in_utxo_data_hashes: [[u8; 32]; NR_IN_UTXOS],
}

impl<
        'a,
        'b,
        'c,
        'info,
        const NR_CHECKED_INPUTS: usize,
        const NR_OUT_UTXOS: usize,
        const NR_IN_UTXOS: usize,
        const NR_PUBLIC_INPUTS: usize,
        A: LightPublicAccounts<'info>,
        P,
    >
    TransactionConvertible<
        'a,
        'b,
        'c,
        'info,
        NR_CHECKED_INPUTS,
        NR_OUT_UTXOS,
        NR_IN_UTXOS,
        NR_PUBLIC_INPUTS,
        A,
        P,
    > for PublicTransactionPublicInputs<NR_IN_UTXOS, NR_OUT_UTXOS>
where
    P: AnchorSerialize,
{
    fn from_transaction(
        input: &PublicTransaction<
            'a,
            'b,
            'c,
            'info,
            NR_CHECKED_INPUTS,
            NR_OUT_UTXOS,
            NR_IN_UTXOS,
            NR_PUBLIC_INPUTS,
            A,
            P,
        >,
    ) -> Self {
        PublicTransactionPublicInputs {
            state_merkle_roots: input.state_merkle_roots,
            public_amount_spl: input
                .input
                .public_amount
                .unwrap_or(&Amounts::default())
                .spl
                .unwrap_or([0u8; 32]),
            tx_integrity_hash: input.tx_integrity_hash,
            public_amount_sol: input
                .input
                .public_amount
                .unwrap_or(&Amounts::default())
                .sol
                .unwrap_or([0u8; 32]),
            mint_pubkey: input.mint_pubkey,
            in_utxo_hashes: input.in_utxo_hashes_proof,
            out_utxo_hashes_proof: input.out_utxo_hashes_proof,
            // checked_public_inputs: *input.checked_public_inputs,
            in_utxo_data_hashes: input
                .input
                .in_utxo_data_hashes
                .map(|x| x.unwrap_or([0u8; 32])),
            new_adresses: input.input.new_addresses.map(|x| x.unwrap_or([0u8; 32])),
        }
    }
}

#[derive(Clone)]
pub struct PublicTransaction<
    'a,
    'b,
    'c,
    'info,
    const NR_CHECKED_INPUTS: usize,
    const NR_OUT_UTXOS: usize,
    const NR_IN_UTXOS: usize,
    const NR_PUBLIC_INPUTS: usize,
    A: LightPublicAccounts<'info>,
    P,
> where
    P: AnchorSerialize,
{
    // Client input.
    pub input:
        PublicTransactionInput<'a, 'b, 'c, 'info, NR_CHECKED_INPUTS, NR_OUT_UTXOS, NR_IN_UTXOS, A>,
    // State of transaction.
    pub state_merkle_roots: [[u8; 32]; NR_IN_UTXOS],
    pub tx_integrity_hash: [u8; 32],
    pub mint_pubkey: [u8; 32],
    pub out_utxo_hashes: Vec<[u8; 32]>,
    pub out_utxo_index: Vec<u64>,
    pub out_utxo_hashes_proof: [[u8; 32]; NR_OUT_UTXOS],
    pub in_utxo_hashes_proof: [[u8; 32]; NR_IN_UTXOS],
    _p: PhantomData<P>,
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize, Default)]
pub struct Amounts {
    pub spl: Option<[u8; 32]>,
    pub sol: Option<[u8; 32]>,
}

// TODO: add functions from TransferUtxo to PublicTransactionInput with auto padding for in utxo hashes, out utxo hashes, new_addresses, in_data_hashes

// Remaining accounts layout:
// all remainging accounts need to be set regardless whether less utxos are actually used
// 0..NR_IN_Utxos: in utxos
// NR_IN_Utxos..NR_IN_Utxos+NR_IN_Utxos: indexed arrays to nullify in utxos
// NR_IN_Utxos+NR_IN_Utxos..NR_IN_Utxos+NR_IN_Utxos+NR_OUT_Utxos: out utxos
#[derive(Clone)]
pub struct PublicTransactionInput<
    'a,
    'b,
    'c,
    'info,
    const NR_CHECKED_INPUTS: usize,
    const NR_OUT_UTXOS: usize,
    const NR_IN_UTXOS: usize,
    A: LightPublicAccounts<'info>,
> {
    pub ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    pub proof: &'a ProofCompressed,
    pub public_amount: Option<&'a Amounts>,
    pub message: Option<&'a Vec<u8>>,
    pub transaction_hash: Option<&'a [u8; 32]>,
    pub program_id: Option<&'a Pubkey>,
    pub in_utxo_hashes: &'a Vec<[u8; 32]>,
    pub in_utxo_data_hashes: [Option<[u8; 32]>; NR_IN_UTXOS],
    pub low_element_indexes: &'a Vec<u16>,
    // using a vector on purpose since Utxos can be large and could lead to stack frame issues
    pub out_utxos: Vec<Utxo>,
    pub new_addresses: &'a [Option<[u8; 32]>; NR_OUT_UTXOS],
    pub rpc_fee: Option<u64>,
    pub merkle_root_indexes: [usize; NR_IN_UTXOS],
    pub pool_type: &'a [u8; 32],
    pub verifyingkey: &'a Groth16Verifyingkey<'a>,
}
#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PublicTransactionEvent {
    pub in_utxo_hashes: Vec<[u8; 32]>,
    pub out_utxos: Vec<Utxo>,
    pub out_utxo_indexes: Vec<u64>,

    // Fields used when (de)compression
    pub public_amount_sol: Option<[u8; 32]>,
    pub public_amount_spl: Option<[u8; 32]>,
    pub rpc_fee: Option<u64>,
    // Program utxo fields
    pub message: Option<Vec<u8>>,
    pub transaction_hash: Option<[u8; 32]>,
    pub program_id: Option<Pubkey>,
}
pub trait TransactionConvertible<
    'a,
    'b,
    'c,
    'info,
    const NR_CHECKED_INPUTS: usize,
    const NR_OUT_UTXOS: usize,
    const NR_IN_UTXOS: usize,
    const NR_PUBLIC_INPUTS: usize,
    A: LightPublicAccounts<'info>,
    P,
> where
    P: AnchorSerialize,
{
    fn from_transaction(
        transaction: &PublicTransaction<
            'a,
            'b,
            'c,
            'info,
            NR_CHECKED_INPUTS,
            NR_OUT_UTXOS,
            NR_IN_UTXOS,
            NR_PUBLIC_INPUTS,
            A,
            P,
        >,
    ) -> Self;
}

impl<
        'a,
        'b,
        'c,
        'info,
        const NR_CHECKED_INPUTS: usize,
        const NR_OUT_UTXOS: usize,
        const NR_IN_UTXOS: usize,
        const NR_PUBLIC_INPUTS: usize,
        A: LightPublicAccounts<'info>,
        P,
    >
    PublicTransaction<
        'a,
        'b,
        'c,
        'info,
        NR_CHECKED_INPUTS,
        NR_OUT_UTXOS,
        NR_IN_UTXOS,
        NR_PUBLIC_INPUTS,
        A,
        P,
    >
where
    P: AnchorSerialize
        + TransactionConvertible<
            'a,
            'b,
            'c,
            'info,
            NR_CHECKED_INPUTS,
            NR_OUT_UTXOS,
            NR_IN_UTXOS,
            NR_PUBLIC_INPUTS,
            A,
            P,
        >,
{
    pub fn new(
        input: PublicTransactionInput<
            'a,
            'b,
            'c,
            'info,
            NR_CHECKED_INPUTS,
            NR_OUT_UTXOS,
            NR_IN_UTXOS,
            A,
        >,
    ) -> PublicTransaction<
        'a,
        'b,
        'c,
        'info,
        NR_CHECKED_INPUTS,
        NR_OUT_UTXOS,
        NR_IN_UTXOS,
        NR_PUBLIC_INPUTS,
        A,
        P,
    > {
        PublicTransaction::<
            'a,
            'b,
            'c,
            'info,
            NR_CHECKED_INPUTS,
            NR_OUT_UTXOS,
            NR_IN_UTXOS,
            NR_PUBLIC_INPUTS,
            A,
            P,
        > {
            input,
            state_merkle_roots: [[0u8; 32]; NR_IN_UTXOS],
            tx_integrity_hash: [0u8; 32],
            mint_pubkey: [0u8; 32],
            out_utxo_hashes: Vec::new(),
            out_utxo_index: Vec::new(),
            in_utxo_hashes_proof: [[0u8; 32]; NR_IN_UTXOS],
            out_utxo_hashes_proof: [DEFAULT_UTXO_HASH; NR_OUT_UTXOS],
            _p: PhantomData,
        }
    }

    /// Transact is a wrapper function which computes the integrity hash, checks the root,
    /// verifies the zero knowledge proof, inserts out_utxo_hashes, inserts in_utxo_hashes, transfers funds and fees.
    #[inline(never)]
    pub fn transact(&mut self) -> Result<()> {
        self.fill_in_utxo_hashes_proof();
        self.hash_out_utxos_and_fetch_out_utxo_index()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("hash_out_utxos_and_fetch_out_utxo_index");
        self.emit_indexer_transaction_event()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("emit_indexer_transaction_event");
        self.compute_tx_integrity_hash()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("compute_tx_integrity_hash");
        self.fetch_state_merkle_tree_roots()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("fetch_state_merkle_tree_roots");
        self.fetch_mint()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("fetch_mint");
        self.verify()?;
        // #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        // custom_heap::log_total_heap("verify");
        // self.check_remaining_accounts()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("check_remaining_accounts");
        self.insert_out_utxos_into_merkle_tree()?;
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("insert_out_utxos_into_merkle_tree");
        self.nullify_in_utxos()?;
        // #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        // custom_heap::log_total_heap("nullify_in_utxos");
        // self.transfer_user_funds()?;
        // #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        // custom_heap::log_total_heap("transfer_user_funds");
        // self.transfer_fee()?;
        Ok(())
    }

    #[inline(never)]
    pub fn fill_in_utxo_hashes_proof(&mut self) {
        msg!("in_utxo_hashes: {:?}", self.input.in_utxo_hashes);
        for (i, hash) in self.input.in_utxo_hashes.iter().enumerate() {
            self.in_utxo_hashes_proof[i] = *hash;
        }
    }

    #[heap_neutral]
    #[inline(never)]
    pub fn emit_indexer_transaction_event(&self) -> Result<()> {
        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("pre assemble TransactionIndexerEvent");
        let mut out_utxos = Vec::new();
        for utxo in self.input.out_utxos.iter() {
            if utxo.owner != DEFAULT_PUBKEY {
                out_utxos.push(utxo.clone());
            }
        }
        // Initialize the vector of out_utxo_hashes
        let transaction_data_event = PublicTransactionEvent {
            in_utxo_hashes: self.input.in_utxo_hashes.to_vec(),
            out_utxos,
            program_id: if self.input.program_id.is_some() {
                Some(*self.input.program_id.unwrap())
            } else {
                None
            },
            transaction_hash: if self.input.transaction_hash.is_some() {
                Some(*self.input.transaction_hash.unwrap())
            } else {
                None
            },
            message: if self.input.message.is_some() {
                Some(self.input.message.unwrap().to_vec())
            } else {
                None
            },
            out_utxo_indexes: self.out_utxo_index.to_vec(),
            public_amount_sol: self.input.public_amount.map(|x| x.sol.unwrap_or([0u8; 32])),
            public_amount_spl: self.input.public_amount.map(|x| x.spl.unwrap_or([0u8; 32])),
            rpc_fee: self.input.rpc_fee,
        };

        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("post assemble TransactionIndexerEvent");

        invoke_indexer_transaction_event::<PublicTransactionEvent>(
            &transaction_data_event,
            &self.input.ctx.accounts.get_log_wrapper().to_account_info(),
        )?;

        #[cfg(all(target_os = "solana", feature = "mem-profiling"))]
        custom_heap::log_total_heap("post invoke_indexer_transaction_event");

        Ok(())
    }

    #[inline(never)]
    pub fn hash_out_utxos_and_fetch_out_utxo_index(&mut self) -> Result<()> {
        let mut merkle_tree_indexes = HashMap::<Pubkey, usize>::new();
        for (i, utxo) in self.input.out_utxos.iter_mut().enumerate() {
            self.out_utxo_hashes_proof[i] = utxo.hash()?;

            let index = merkle_tree_indexes
                .get_mut(&self.input.ctx.remaining_accounts[i + NR_IN_UTXOS * 2].key());
            match index {
                Some(index) => {
                    self.out_utxo_index.push(index.clone() as u64);
                }
                None => {
                    let merkle_tree = AccountLoader::<ConcurrentMerkleTreeAccount>::try_from(
                        &self.input.ctx.remaining_accounts[i + NR_IN_UTXOS * 2].to_account_info(),
                    )
                    .unwrap();
                    let merkle_tree_account = merkle_tree.load()?;
                    let merkle_tree =
                        state_merkle_tree_from_bytes(&merkle_tree_account.state_merkle_tree);
                    let index = merkle_tree.next_index as usize;
                    merkle_tree_indexes.insert(
                        self.input.ctx.remaining_accounts[i + NR_IN_UTXOS * 2].key(),
                        index,
                    );

                    self.out_utxo_index.push(index.clone() as u64);
                }
            }
            utxo.update_blinding(
                self.input.ctx.remaining_accounts[i + NR_IN_UTXOS * 2].key(),
                self.out_utxo_index[i] as usize,
            )
            .unwrap();
            self.out_utxo_hashes.push(utxo.hash()?);
        }
        Ok(())
    }

    /// Verifies a Goth16 zero knowledge proof over the bn254 curve.
    #[inline(never)]
    pub fn verify(&self) -> Result<()> {
        #[cfg(all(target_os = "solana", feature = "custom-heap"))]
        let pos = custom_heap::get_heap_pos();
        // 4(spl, sol, dataHash, mint) + in_utxo_hashes + out_utxo_hashes + nullifier roots + out_utxo_hashes roots
        // assert_eq!(
        //     NR_PUBLIC_INPUTS,
        //     4 + NR_IN_UTXOS + NR_OUT_UTXOS + NR_CHECKED_INPUTS + NR_IN_UTXOS + NR_OUT_UTXOS,
        // );
        // TODO: we should autogenerate this if we go for more rust sdk

        let public_inputs_struct = P::from_transaction(self);
        // TODO: return a public inputs array from_transaction
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
            Some(message) => Message::new(message).hash,
            None => [0u8; 32],
        };
        let recipient_spl = match self.input.ctx.accounts.get_recipient_spl().as_ref() {
            Some(recipient_spl) => recipient_spl.key().to_bytes(),
            None => [0u8; 32],
        };
        let recipient_sol = match self.input.ctx.accounts.get_recipient_sol().as_ref() {
            Some(recipient_spl) => recipient_spl.key().to_bytes(),
            None => [0u8; 32],
        };
        let tx_integrity_hash = hashv(&[
            &message_hash,
            &recipient_spl,
            &recipient_sol,
            &self
                .input
                .ctx
                .accounts
                .get_signing_address()
                .key()
                .to_bytes(),
            &self.input.rpc_fee.unwrap_or(0u64).to_be_bytes(),
            // self.input.encrypted_utxos,
        ]);
        msg!("message_hash: {:?}", message_hash.to_vec());
        msg!("recipient_spl: {:?}", recipient_spl.to_vec());
        msg!("recipient_sol: {:?}", recipient_sol);
        msg!(
            "signing_address: {:?}",
            self.input
                .ctx
                .accounts
                .get_signing_address()
                .key()
                .to_bytes()
                .to_vec()
        );
        msg!(
            "rpc_fee: {:?}",
            self.input.rpc_fee.unwrap_or(0u64).to_be_bytes().to_vec()
        );

        self.tx_integrity_hash = truncate_to_circuit(&tx_integrity_hash.to_bytes());
        Ok(())
    }

    /// Fetches the root according to an index from the passed-in Merkle tree.
    pub fn fetch_state_merkle_tree_roots(&mut self) -> Result<()> {
        for i in 0..NR_IN_UTXOS {
            let merkle_tree = AccountLoader::<ConcurrentMerkleTreeAccount>::try_from(
                &self.input.ctx.remaining_accounts[i].to_account_info(),
            )
            .unwrap();
            let merkle_tree_account = merkle_tree.load()?;
            let merkle_tree = state_merkle_tree_from_bytes(&merkle_tree_account.state_merkle_tree);

            self.state_merkle_roots[i] = merkle_tree.roots[merkle_tree.current_root_index as usize];
        }
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
                        if self.input.public_amount.is_none()
                            || self.input.public_amount.unwrap().spl.is_none()
                        {
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

    // /// Checks the expected number of remaning accounts:
    // ///
    // /// * Nullifier and leaf accounts (mandatory).
    // /// * Merkle tree accounts (optional).
    // #[heap_neutral]
    // #[inline(never)]
    // fn check_remaining_accounts(&self) -> Result<()> {
    //     let nr_nullifiers_leaves = NR_IN_UTXOS + NR_OUT_UTXOS / 2;
    //     let remaining_accounts_len = self.input.ctx.remaining_accounts.len();
    //     if remaining_accounts_len != nr_nullifiers_leaves // Only in_utxo_hashes and out_utxo_hashes.
    //         // Nullifiers, out_utxo_hashes and next Merkle trees (transaction, event).
    //         && remaining_accounts_len != nr_nullifiers_leaves + 2
    //     {
    //         msg!(
    //             "remaining_accounts.len() {} (expected {} or {})",
    //             remaining_accounts_len,
    //             nr_nullifiers_leaves,
    //             nr_nullifiers_leaves + 1
    //         );
    //         return err!(VerifierSdkError::InvalidNrRemainingAccounts);
    //     }

    //     Ok(())
    // }

    /// Calls the Merkle tree program via cpi to insert transaction out_utxo_hashes.
    #[heap_neutral]
    #[inline(never)]
    pub fn insert_out_utxos_into_merkle_tree(&self) -> Result<()> {
        insert_two_leaves_parallel_cpi(
            self.input.ctx.program_id,
            &self
                .input
                .ctx
                .accounts
                .get_psp_account_compression()
                .to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_account_compression_authority()
                .to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_registered_verifier_pda()
                .to_account_info(),
            // TODO: remove vector or instantiate once for the whole struct
            // switch to out utxo hashes assoon as we can insert signle hashes
            self.out_utxo_hashes.to_vec(),
            self.input.ctx.remaining_accounts
                [NR_IN_UTXOS * 2..NR_IN_UTXOS * 2 + self.out_utxo_hashes.len()]
                .to_vec(),
        )?;
        Ok(())
    }

    /// Calls account compression program via cpi to nullify in_utxo_hashes.
    #[heap_neutral]
    #[inline(never)]
    pub fn nullify_in_utxos(&self) -> Result<()> {
        insert_public_nullifier_into_indexed_array_cpi(
            self.input.ctx.program_id,
            &self
                .input
                .ctx
                .accounts
                .get_psp_account_compression()
                .to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_account_compression_authority()
                .to_account_info(),
            &self
                .input
                .ctx
                .accounts
                .get_registered_verifier_pda()
                .to_account_info(),
            self.input.in_utxo_hashes.to_vec(),
            self.input.low_element_indexes.to_vec(),
            self.input.ctx.remaining_accounts
                [NR_IN_UTXOS..NR_IN_UTXOS + self.input.in_utxo_hashes.len()]
                .to_vec(),
        )?;
        Ok(())
    }

    /// Transfers user funds either to or from a merkle tree liquidity pool.
    #[heap_neutral]
    #[inline(never)]
    pub fn transfer_user_funds(&self) -> Result<()> {
        let is_compress_spl = self.is_compress_spl();
        if is_compress_spl.is_none() {
            return Ok(());
        }
        let (spl_amount, is_deposit): ([u8; 32], bool) = self.is_compress_spl().unwrap();
        msg!("transferring user funds");
        // check mintPubkey
        let (pub_amount_checked, _) = check_amount(0, change_endianness(&spl_amount))?;

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
            if is_deposit {
                self.compress_spl(pub_amount_checked, sender_spl, recipient_spl)?;
            } else {
                check_spl_pool_account_derivation(
                    &self.input.pool_type,
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
        let is_compress_sol = self.is_compress_spl();
        if is_compress_sol.is_none() {
            return Ok(());
        }
        let (sol_amount, is_compress): ([u8; 32], bool) = self.is_compress_sol().unwrap();

        // check that it is the native token pool
        let (fee_amount_checked, rpc_fee) = check_amount(
            self.input.rpc_fee.unwrap_or(0u64),
            change_endianness(&sol_amount),
        )?;
        msg!("fee amount {} ", fee_amount_checked);
        if fee_amount_checked > 0 {
            if is_compress {
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

                check_sol_pool_account_derivation(
                    &self.input.pool_type,
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
        if !is_compress && rpc_fee > 0 {
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
        check_sol_pool_account_derivation(
            &self.input.pool_type,
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
        check_spl_pool_account_derivation(
            &self.input.pool_type,
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

    /// Checks whether a transaction is a compression or decompression transaction by inspecting the public amount.
    pub fn is_compress_spl(&self) -> Option<([u8; 32], bool)> {
        match self.input.public_amount {
            Some(amounts) => match amounts.spl {
                Some(spl) => Some((spl, spl[24..] != [0u8; 8] && spl[..24] == [0u8; 24])),
                None => None,
            },
            _ => None,
        }
    }

    /// Checks whether a transaction is a compression or decompression transaction by inspecting the public amount.
    pub fn is_compress_sol(&self) -> Option<([u8; 32], bool)> {
        match self.input.public_amount {
            Some(amounts) => match amounts.sol {
                Some(sol) => Some((sol, sol[24..] != [0u8; 8] && sol[..24] == [0u8; 24])),
                None => None,
            },
            _ => None,
        }
    }
}

//    #[heap_neutral]
pub fn check_spl_pool_account_derivation(
    pool_type: &[u8; 32],
    pubkey: &Pubkey,
    mint: &Pubkey,
) -> Result<()> {
    let derived_pubkey = Pubkey::find_program_address(
        &[&mint.to_bytes(), pool_type, POOL_SEED],
        &LightMerkleTreeProgram::id(),
    );

    if derived_pubkey.0 != *pubkey {
        return err!(VerifierSdkError::InvalidSenderOrRecipient);
    }
    Ok(())
}
//    #[heap_neutral] TODO: edit macro so that it can be used for functions
pub fn check_sol_pool_account_derivation(
    pool_type: &[u8; 32],
    pubkey: &Pubkey,
    data: &[u8],
) -> Result<()> {
    let derived_pubkey = Pubkey::find_program_address(
        &[&[0u8; 32], pool_type, POOL_CONFIG_SEED],
        &LightMerkleTreeProgram::id(),
    );
    let mut cloned_data = data;
    light_merkle_tree_program::RegisteredAssetPool::try_deserialize(&mut cloned_data)?;

    if derived_pubkey.0 != *pubkey {
        return err!(VerifierSdkError::InvalidSenderOrRecipient);
    }
    Ok(())
}

#[allow(clippy::comparison_chain)]
pub fn check_amount(rpc_fee: u64, amount: [u8; 32]) -> Result<(u64, u64)> {
    // pub_amount is the public amount included in public inputs for proof verification
    let pub_amount = <BigInteger256 as FromBytes>::read(&amount[..]).unwrap();
    // Big integers are stored in 4 u64 limbs, if the number is <= U64::max() and encoded in little endian,
    // only the first limb is greater than 0.
    // Amounts in compressed accounts are limited to 64bit therefore a decompression will always be greater
    // than one U64::max().
    if pub_amount.0[0] > 0 && pub_amount.0[1] == 0 && pub_amount.0[2] == 0 && pub_amount.0[3] == 0 {
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

#[cfg(test)]
mod test {
    use light_hasher::{Hasher, Poseidon};
    use num_bigint::BigUint;

    #[test]
    fn poseidon_test() {
        // let inputs = [
        //     &[0][..],
        //     &[
        //         0, 4, 206, 231, 74, 191, 249, 123, 216, 118, 108, 32, 100, 122, 18, 33, 122, 14,
        //         253, 86, 13, 209, 7, 85, 47, 55, 199, 43, 102, 113, 161, 105,
        //     ][..],
        //     // &[
        //     //     124, 35, 94, 81, 59, 167, 199, 9, 139, 251, 8, 52, 72, 231, 132, 134, 120, 140,
        //     //     149, 45, 85, 144, 24, 9, 13, 204, 143, 49, 180, 190, 254, 4,
        //     // ][..],
        //     &[
        //         97, 62, 42, 87, 170, 177, 223, 108, 235, 90, 3, 60, 153, 211, 84, 169, 47, 169,
        //         165, 171, 85, 206, 149, 159, 18, 251, 190, 94, 37, 179, 162,
        //     ][..],
        //     // &[
        //     //     9, 21, 108, 8, 217, 255, 193, 117, 171, 30, 78, 225, 163, 80, 240, 173, 38, 58, 40,
        //     //     21, 46, 236, 67, 48, 41, 215, 29, 246, 171, 54, 10, 235,
        //     // ][..],
        //     &[
        //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        //         0, 0, 0, 0,
        //     ][..],
        //     &[0][..],
        //     &[
        //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        //         0, 0, 0, 0,
        //     ][..],
        //     &[
        //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        //         0, 0, 0, 0,
        //     ][..],
        // ];
        let inputs = [
            &[0][..],
            &[
                0, 4, 206, 231, 74, 191, 249, 123, 216, 118, 108, 32, 100, 122, 18, 33, 122, 14,
                253, 86, 13, 209, 7, 85, 47, 55, 199, 43, 102, 113, 161, 105,
            ][..],
            &[
                4, 254, 190, 180, 49, 143, 204, 13, 9, 24, 144, 85, 45, 149, 140, 120, 134, 132,
                231, 72, 52, 8, 251, 139, 9, 199, 167, 59, 81, 94, 35, 124,
            ][..],
            &[
                6, 136, 77, 101, 34, 229, 37, 20, 45, 173, 215, 161, 26, 156, 195, 70, 208, 185,
                127, 45, 61, 176, 192, 42, 102, 133, 55, 121, 132, 32, 17, 32,
            ][..],
            &[
                44, 179, 81, 40, 126, 231, 214, 116, 172, 174, 178, 187, 68, 51, 218, 18, 184, 228,
                249, 221, 203, 153, 163, 215, 82, 15, 47, 107, 93, 73, 244, 12,
            ][..],
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ][..],
            &[0][..],
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ][..],
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ][..],
        ];
        let inputs_refs: Vec<&[u8]> = inputs.iter().map(|&slice| slice).collect();
        Poseidon::hashv(&inputs_refs).unwrap();
    }

    #[test]
    fn poseid_test() {
        let spl_circuit = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];
        let res = Poseidon::hashv(&[
            &BigUint::parse_bytes(
                b"6686672797465227418401714772753289406522066866583537086457438811846503839916",
                10,
            )
            .unwrap()
            .to_bytes_be()
            .as_slice(),
            &spl_circuit,
        ])
        .unwrap();
        let res_res = Poseidon::hash(&res).unwrap();
    }
}
