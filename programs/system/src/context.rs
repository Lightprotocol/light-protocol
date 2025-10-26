use std::panic::Location;

use light_compressed_account::{
    hash_to_bn254_field_size_be,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        traits::{InputAccount, InstructionData, NewAddress, OutputAccount},
        zero_copy::{ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount},
    },
};
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
    pubkey::Pubkey,
};
use solana_msg::msg;

use crate::{
    cpi_context::state::ZCpiContextAccount2, errors::SystemProgramError,
    utils::transfer_lamports_invoke, Result, MAX_OUTPUT_ACCOUNTS,
};

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
    pub network_fee_is_set: bool,
    pub legacy_merkle_context: Vec<(u8, MerkleTreeContext)>,
    pub invoking_program_id: Option<Pubkey>,
}

/// Helper for legacy trees.
pub struct MerkleTreeContext {
    pub rollover_fee: u64,
    pub hashed_pubkey: [u8; 32],
    pub network_fee: u64,
}

impl SystemContext<'_> {
    #[profile]
    pub fn get_legacy_merkle_context(&mut self, index: u8) -> Option<&MerkleTreeContext> {
        self.legacy_merkle_context
            .iter()
            .find(|a| a.0 == index)
            .map(|a| &a.1)
    }
    pub fn set_legacy_merkle_context(&mut self, index: u8, context: MerkleTreeContext) {
        self.legacy_merkle_context.push((index, context));
    }

    #[profile]
    pub fn set_address_fee(&mut self, fee: u64, index: u8) -> Result<()> {
        self.set_additive_fee(fee, index)
    }

    #[profile]
    pub fn set_network_fee_v1(&mut self, fee: u64, index: u8) -> Result<()> {
        self.set_additive_fee(fee, index)
    }

    #[inline(always)]
    fn set_additive_fee(&mut self, fee: u64, index: u8) -> Result<()> {
        let payment = self.rollover_fee_payments.iter_mut().find(|a| a.0 == index);
        match payment {
            Some(payment) => {
                payment.1 = payment
                    .1
                    .checked_add(fee)
                    .ok_or(ProgramError::ArithmeticOverflow)?;
            }
            None => self.rollover_fee_payments.push((index, fee)),
        };
        Ok(())
    }

    #[profile]
    pub fn set_network_fee_v2(&mut self, fee: u64, index: u8) {
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
                let hashed_pubkey = hash_to_bn254_field_size_be(pubkey.as_ref());
                self.hashed_pubkeys.push((pubkey, hashed_pubkey));
                hashed_pubkey
            }
        }
    }
}

impl<'info> SystemContext<'info> {
    #[track_caller]
    pub fn get_index_or_insert(
        &mut self,
        ix_data_index: u8,
        remaining_accounts: &'info [AccountInfo],
        name: &str,
    ) -> std::result::Result<u8, SystemProgramError> {
        let queue_index = self
            .account_indices
            .iter()
            .position(|a| *a == ix_data_index);
        match queue_index {
            Some(index) => Ok(index as u8),
            None => {
                self.account_indices.push(ix_data_index);
                let account_info =
                    &remaining_accounts
                        .get(ix_data_index as usize)
                        .ok_or_else(|| {
                            let location = Location::caller();
                            msg!(
                                "Index: {}, account name: {} {}:{}:{}",
                                ix_data_index,
                                name,
                                location.file(),
                                location.line(),
                                location.column()
                            );
                            SystemProgramError::PackedAccountIndexOutOfBounds
                        })?;
                self.accounts.push(AccountMeta {
                    pubkey: account_info.key(),
                    is_signer: false,
                    is_writable: true,
                });
                self.account_infos.push(account_info);
                Ok(self.account_indices.len() as u8 - 1)
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
    /// - V1 state trees: charge per input (5000 lamports × num_inputs)
    /// - V2 batched state trees: charge once per tree if inputs > 0 OR outputs > 0 (5000 lamports)
    /// - Address creation: charge per address (10000 lamports × num_addresses)
    ///
    /// Examples (V1 state trees):
    /// 1. create account with 1 address, 0 inputs:     network fee 10,000 lamports
    /// 2. token transfer (1 input, 1 output):          network fee 5,000 lamports
    /// 3. transfer with 2 V1 inputs, 1 address:        network fee 20,000 lamports (2×5k + 1×10k)
    ///
    /// Examples (V2 batched state trees):
    /// 1. token transfer (1 input, 0 output):          network fee 5,000 lamports (once per tree)
    /// 2. token transfer (0 input, 1 output):          network fee 5,000 lamports (once per tree)
    /// 3. token transfer (1 input, 1 output):          network fee 5,000 lamports (once per tree)
    /// 4. transfer with 2 V2 inputs, 1 address:        network fee 15,000 lamports (5k + 1×10k)
    ///    Transfers rollover and network fees.
    pub fn transfer_fees(&self, accounts: &[AccountInfo], fee_payer: &AccountInfo) -> Result<()> {
        for (i, fee) in self.rollover_fee_payments.iter() {
            transfer_lamports_invoke(fee_payer, &accounts[*i as usize], *fee)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct WrappedInstructionData<'a, T: InstructionData<'a>> {
    instruction_data: T,
    cpi_context: Option<ZCpiContextAccount2<'a>>,
    address_len: usize,
    input_len: usize,
    outputs_len: usize,
    /// Offsets are used to copy output compressed account data from the cpi context
    /// to the system program -> account compression program cpi instruction data.
    /// This ensures the indexer can index all output account data.
    cpi_context_outputs_start_offset: usize,
    cpi_context_outputs_end_offset: usize,
}

impl<'a, 'b, T> WrappedInstructionData<'a, T>
where
    T: InstructionData<'a>,
{
    #[profile]
    pub fn new(instruction_data: T) -> std::result::Result<Self, SystemProgramError> {
        let outputs_len = instruction_data
            .output_accounts()
            .iter()
            .filter(|x| !x.skip())
            .count();
        if outputs_len > MAX_OUTPUT_ACCOUNTS {
            return Err(SystemProgramError::TooManyOutputAccounts);
        }
        Ok(Self {
            input_len: instruction_data
                .input_accounts()
                .iter()
                .filter(|x| !x.skip())
                .count(),
            outputs_len,
            address_len: instruction_data.new_addresses().len(),
            cpi_context: None,
            instruction_data,
            cpi_context_outputs_start_offset: 0,
            cpi_context_outputs_end_offset: 0,
        })
    }

    #[profile]
    pub fn set_cpi_context(&mut self, cpi_context: ZCpiContextAccount2<'a>) -> Result<()> {
        if self.cpi_context.is_none() {
            self.outputs_len += cpi_context.out_accounts.len();
            if self.outputs_len > MAX_OUTPUT_ACCOUNTS {
                return Err(SystemProgramError::TooManyOutputAccounts.into());
            }
            self.address_len += cpi_context.new_addresses.len();
            self.input_len += cpi_context.in_accounts.len();

            // Calculate offsets from the CPI context account
            let (outputs_start_offset, outputs_end_offset) = cpi_context.calculate_output_offsets();

            self.cpi_context = Some(cpi_context);
            self.cpi_context_outputs_start_offset = outputs_start_offset;
            self.cpi_context_outputs_end_offset = outputs_end_offset;
        } else {
            return Err(SystemProgramError::CpiContextAlreadySet.into());
        }
        Ok(())
    }

    pub fn get_cpi_context_outputs_start_offset(&self) -> usize {
        self.cpi_context_outputs_start_offset
    }

    pub fn get_cpi_context_outputs_end_offset(&self) -> usize {
        self.cpi_context_outputs_end_offset
    }

    pub fn get_cpi_context_account(&'b self) -> &'b Option<ZCpiContextAccount2<'a>> {
        &self.cpi_context
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
    pub fn bump(&self) -> Option<u8> {
        self.instruction_data.bump()
    }

    pub fn with_transaction_hash(&self) -> bool {
        self.instruction_data.with_transaction_hash()
    }

    #[profile]
    pub fn get_output_account(&'b self, index: usize) -> Option<&'b (dyn OutputAccount<'a> + 'b)> {
        // Check CPI context first
        if let Some(cpi_context) = self.cpi_context.as_ref() {
            let cpi_outputs_len = cpi_context.output_accounts().len();
            if index < cpi_outputs_len {
                return cpi_context.output_accounts().get(index).map(|account| {
                    let output_account_trait_object: &'b (dyn OutputAccount<'a> + 'b) = account;
                    output_account_trait_object
                });
            }
            // Adjust index for instruction data
            let ix_index = index - cpi_outputs_len;
            return self
                .instruction_data
                .output_accounts()
                .get(ix_index)
                .map(|account| account as &(dyn OutputAccount<'a> + 'b));
        }

        // No CPI context, use instruction data
        self.instruction_data
            .output_accounts()
            .get(index)
            .map(|account| account as &(dyn OutputAccount<'a> + 'b))
    }
}

impl<'a, T: InstructionData<'a>> WrappedInstructionData<'a, T> {
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

    pub fn new_addresses<'b>(&'b self) -> impl Iterator<Item = &'b dyn NewAddress<'a>> {
        if let Some(cpi_context) = &self.cpi_context {
            chain_new_addresses(
                cpi_context.new_addresses(),
                self.instruction_data.new_addresses(),
            )
        } else {
            let empty_slice = &[];
            chain_new_addresses(empty_slice, self.instruction_data.new_addresses())
        }
    }

    pub fn output_accounts<'b>(&'b self) -> impl Iterator<Item = &'b dyn OutputAccount<'a>> {
        if let Some(cpi_context) = &self.cpi_context {
            chain_outputs(
                cpi_context.output_accounts(),
                self.instruction_data.output_accounts(),
            )
        } else {
            chain_outputs(&[], self.instruction_data.output_accounts())
        }
    }

    pub fn input_accounts<'b>(&'b self) -> impl Iterator<Item = &'b dyn InputAccount<'a>> {
        if let Some(cpi_context) = &self.cpi_context {
            chain_inputs(
                cpi_context.input_accounts(),
                self.instruction_data.input_accounts(),
            )
        } else {
            let empty_slice = &[];
            chain_inputs(empty_slice, self.instruction_data.input_accounts())
        }
    }

    pub fn cpi_context(&self) -> Option<CompressedCpiContext> {
        self.instruction_data.cpi_context()
    }

    pub fn read_only_addresses(&self) -> Option<&[ZPackedReadOnlyAddress]> {
        self.instruction_data.read_only_addresses()
    }

    pub fn read_only_accounts(&self) -> Option<&[ZPackedReadOnlyCompressedAccount]> {
        self.instruction_data.read_only_accounts()
    }
}

#[profile]
pub fn chain_outputs<'a, 'b: 'a>(
    slice1: &'a [impl OutputAccount<'b>],
    slice2: &'a [impl OutputAccount<'b>],
) -> impl Iterator<Item = &'a dyn OutputAccount<'b>> {
    slice1
        .iter()
        .filter(|x| !x.skip())
        .map(|item| item as &dyn OutputAccount<'b>)
        .chain(
            slice2
                .iter()
                .filter(|x| !x.skip())
                .map(|item| item as &dyn OutputAccount<'b>),
        )
}

#[profile]
pub fn chain_inputs<'a, 'b: 'a>(
    slice1: &'a [impl InputAccount<'b>],
    slice2: &'a [impl InputAccount<'b>],
) -> impl Iterator<Item = &'a dyn InputAccount<'b>> {
    slice1
        .iter()
        .filter(|x| !x.skip())
        .map(|item| item as &dyn InputAccount<'b>)
        .chain(
            slice2
                .iter()
                .filter(|x| !x.skip())
                .map(|item| item as &dyn InputAccount<'b>),
        )
}

#[profile]
pub fn chain_new_addresses<'a, 'b: 'a>(
    slice1: &'a [impl NewAddress<'b>],
    slice2: &'a [impl NewAddress<'b>],
) -> impl Iterator<Item = &'a dyn NewAddress<'b>> {
    slice1
        .iter()
        .map(|item| item as &dyn NewAddress<'b>)
        .chain(slice2.iter().map(|item| item as &dyn NewAddress<'b>))
}
