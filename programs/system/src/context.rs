use std::iter::{Chain, Repeat, Zip};
use std::slice::Iter;

use crate::{invoke_cpi::account::ZCpiContextAccount, utils::transfer_lamports_cpi};
// use anchor_lang::{prelude::*, Result};
use crate::Result;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_compressed_account::hash_to_bn254_field_size_be;
use light_compressed_account::instruction_data::data::OutputCompressedAccountWithPackedContext;
use light_compressed_account::instruction_data::traits::{
    InputAccountTrait, InstructionDataTrait, OutputAccountTrait,
};
use light_compressed_account::instruction_data::zero_copy::ZNewAddressParamsPacked;
use light_concurrent_merkle_tree::zero_copy::ConcurrentMerkleTreeZeroCopyMut;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::zero_copy::IndexedMerkleTreeZeroCopyMut;
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta, pubkey::Pubkey};

/// AccountCompressionProgramAccount
pub enum AcpAccount<'info> {
    Authority(&'info AccountInfo),
    RegisteredProgramPda(&'info AccountInfo),
    SystemProgram(&'info AccountInfo),
    OutputQueue(BatchedQueueAccount<'info>),
    BatchedStateTree(BatchedMerkleTreeAccount<'info>),
    BatchedAddressTree(BatchedMerkleTreeAccount<'info>),
    StateTree((Pubkey, ConcurrentMerkleTreeZeroCopyMut<'info, Poseidon, 26>)),
    AddressTree(
        (
            Pubkey,
            IndexedMerkleTreeZeroCopyMut<'info, Poseidon, usize, 26, 16>,
        ),
    ),
    AddressQueue(Pubkey, &'info AccountInfo),
    V1Queue(&'info AccountInfo),
    Unknown(),
}

pub struct SystemContext<'info> {
    pub account_indices: Vec<u8>,
    pub accounts: Vec<AccountMeta<'info>>,
    // Would be better to store references.
    pub account_infos: Vec<&'info AccountInfo>,
    pub hashed_pubkeys: Vec<(Pubkey, [u8; 32])>,
    // Addresses for deduplication.
    // Try to find a way without storing the addresses.
    pub addresses: Vec<Option<[u8; 32]>>,
    // Index of account and fee to be paid.
    pub rollover_fee_payments: Vec<(u8, u64)>,
    pub address_fee_is_set: bool,
    pub network_fee_is_set: bool,
    pub legacy_merkle_context: Vec<(u8, MerkleTreeContext)>,
    pub invoking_program_id: Option<Pubkey>,
}

/// Helper for legacy trees.
pub struct MerkleTreeContext {
    pub rollover_fee: u64,
    pub hashed_pubkey: [u8; 32],
}

impl SystemContext<'_> {
    pub fn get_legacy_merkle_context(&mut self, index: u8) -> Option<&MerkleTreeContext> {
        self.legacy_merkle_context
            .iter()
            .find(|a| a.0 == index)
            .map(|a| &a.1)
    }
    pub fn set_legacy_merkle_context(&mut self, index: u8, context: MerkleTreeContext) {
        self.legacy_merkle_context.push((index, context));
    }

    pub fn set_address_fee(&mut self, fee: u64, index: u8) {
        if !self.address_fee_is_set {
            self.address_fee_is_set = true;
            self.rollover_fee_payments.push((index, fee));
        }
    }

    pub fn set_network_fee(&mut self, fee: u64, index: u8) {
        if !self.network_fee_is_set {
            self.network_fee_is_set = true;
            self.rollover_fee_payments.push((index, fee));
        }
    }

    pub fn get_or_hash_pubkey(&mut self, pubkey: Pubkey) -> [u8; 32] {
        let hashed_pubkey = self
            .hashed_pubkeys
            .iter()
            .find(|a| a.0 == pubkey)
            .map(|a| a.1);
        match hashed_pubkey {
            Some(hashed_pubkey) => hashed_pubkey,
            None => {
                let hashed_pubkey = hash_to_bn254_field_size_be(&pubkey.as_ref());
                self.hashed_pubkeys.push((pubkey, hashed_pubkey));
                hashed_pubkey
            }
        }
    }
}

impl<'info> SystemContext<'info> {
    pub fn get_index_or_insert(
        &mut self,
        ix_data_index: u8,
        remaining_accounts: &'info [AccountInfo],
    ) -> u8 {
        let queue_index = self
            .account_indices
            .iter()
            .position(|a| *a == ix_data_index);
        match queue_index {
            Some(index) => index as u8,
            None => {
                self.account_indices.push(ix_data_index);
                let account_info = &remaining_accounts[ix_data_index as usize];
                self.accounts.push(AccountMeta {
                    pubkey: account_info.key(),
                    is_signer: false,
                    is_writable: true,
                });
                self.account_infos.push(account_info);
                self.account_indices.len() as u8 - 1
            }
        }
    }

    pub fn set_rollover_fee(&mut self, ix_data_index: u8, fee: u64) {
        let payment = self
            .rollover_fee_payments
            .iter_mut()
            .find(|a| a.0 == ix_data_index);
        match payment {
            Some(payment) => payment.1 += fee,
            None => self.rollover_fee_payments.push((ix_data_index, fee)),
        };
    }

    /// Network fee distribution:
    /// - if any account is created or modified -> transfer network fee (5000 lamports)
    ///   (Previously we didn't charge for appends now we have to since values go into a queue.)
    /// - if an address is created -> transfer an additional network fee (5000 lamports)
    ///
    /// Examples:
    /// 1. create account with address    network fee 10,000 lamports
    /// 2. token transfer                 network fee 5,000 lamports
    /// 3. mint token                     network fee 5,000 lamports
    ///     Transfers rollover and network fees.
    pub fn transfer_fees(&self, accounts: &[AccountInfo], fee_payer: &AccountInfo) -> Result<()> {
        for (i, fee) in self.rollover_fee_payments.iter() {
            // msg!("paying fee: {:?}", fee);
            // msg!("to account: {:?}", accounts[*i as usize].key());
            transfer_lamports_cpi(fee_payer, &accounts[*i as usize], *fee)?;
        }
        Ok(())
    }
}

/// TODO: refactor cpi context account so that everything is just combined into the first context,
///     the vector must never have more than 1 element.
pub struct WrappedInstructionData<'a, T: InstructionDataTrait<'a>> {
    instruction_data: T,
    cpi_context: Option<ZCpiContextAccount<'a>>,
    address_len: usize,
    input_len: usize,
    outputs_len: usize,
}

impl<'a, T: InstructionDataTrait<'a>> WrappedInstructionData<'a, T> {
    pub fn new(instruction_data: T, cpi_context: Option<ZCpiContextAccount<'a>>) -> Self {
        let (mut address_len, mut input_len, mut outputs_len) =
            if let Some(cpi_context) = cpi_context.as_ref() {
                if cpi_context.context.len() > 1 {
                    unimplemented!();
                }

                (
                    cpi_context.context[0].new_address_params.len(),
                    cpi_context.context[0]
                        .input_compressed_accounts_with_merkle_context
                        .len(),
                    cpi_context.context[0].output_compressed_accounts.len(),
                )
            } else {
                (0, 0, 0)
            };

        address_len += instruction_data.new_addresses().len();
        input_len += instruction_data.input_accounts().len();
        outputs_len += instruction_data.output_accounts().len();

        Self {
            instruction_data,
            input_len,
            outputs_len,
            address_len,
            cpi_context,
        }
    }

    pub fn address_len(&self) -> usize {
        self.address_len
    }

    pub fn input_len(&self) -> usize {
        self.input_len
    }

    pub fn output_len(&self) -> usize {
        self.outputs_len
    }

    pub fn inputs_empty(&self) -> bool {
        self.input_len == 0
    }

    pub fn outputs_empty(&self) -> bool {
        self.outputs_len == 0
    }

    pub fn address_empty(&self) -> bool {
        self.address_len == 0
    }
}

impl<'a, T: InstructionDataTrait<'a>> WrappedInstructionData<'a, T> {
    pub fn owner(&self) -> light_compressed_account::pubkey::Pubkey {
        self.instruction_data.owner()
    }
    pub fn proof(
        &self,
    ) -> Option<
        zerocopy::Ref<
            &'a [u8],
            light_compressed_account::instruction_data::compressed_proof::CompressedProof,
        >,
    > {
        self.instruction_data.proof()
    }
    pub fn is_compress(&self) -> bool {
        self.instruction_data.is_compress()
    }
    pub fn compress_or_decompress_lamports(&self) -> Option<u64> {
        self.instruction_data.compress_or_decompress_lamports()
    }

    pub fn new_addresses<'b>(
        &'b self,
    ) -> Chain<Iter<'b, ZNewAddressParamsPacked>, Iter<'b, ZNewAddressParamsPacked>> {
        if let Some(cpi_context) = &self.cpi_context {
            self.instruction_data
                .new_addresses()
                .iter()
                .chain(cpi_context.context[0].new_addresses().iter())
        } else {
            let empty_slice: &'b [ZNewAddressParamsPacked] = &[];
            self.instruction_data
                .new_addresses()
                .iter()
                .chain(empty_slice.iter())
        }
    }

    pub fn output_accounts<'b>(
        &'b self,
    ) -> impl Iterator<Item = &'b (dyn OutputAccountTrait<'a> + 'b)> {
        if let Some(cpi_context) = &self.cpi_context {
            self.instruction_data
                .output_accounts()
                .iter()
                .chain(cpi_context.context[0].output_accounts().iter())
        } else {
            let empty_slice = &[];
            self.instruction_data
                .output_accounts()
                .iter()
                .chain(empty_slice.iter())
        }
    }

    /// Can introduce wrapper struct.
    pub fn input_accounts<'b>(
        &'b self,
    ) -> Zip<
        std::slice::Iter<'a, impl InputAccountTrait<'b> + 'a>,
        Repeat<light_compressed_account::pubkey::Pubkey>,
    > {
        if let Some(cpi_context) = &self.cpi_context {
            self.instruction_data
                .input_accounts()
                .iter()
                .zip(std::iter::repeat(self.instruction_data.owner()))
                .chain(
                    cpi_context.context[0]
                        .input_accounts()
                        .iter()
                        .zip(std::iter::repeat(self.instruction_data.owner())),
                );
            unimplemented!()
        } else {
            let empty_slice = &[];
            self.instruction_data
                .input_accounts()
                .iter()
                .zip(std::iter::repeat(self.instruction_data.owner()))
                .chain(
                    empty_slice
                        .iter()
                        .zip(std::iter::repeat(self.instruction_data.owner())),
                );
            unimplemented!()
        }
    }
}
