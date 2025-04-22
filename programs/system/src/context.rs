use light_compressed_account::{
    compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    hash_to_bn254_field_size_be,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::OutputCompressedAccountWithPackedContext,
        invoke_cpi::InstructionDataInvokeCpi,
        traits::{InputAccount, InstructionData, NewAddress, OutputAccount},
        zero_copy::{ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount},
    },
};
use pinocchio::{account_info::AccountInfo, instruction::AccountMeta, pubkey::Pubkey};

use crate::{
    errors::SystemProgramError, invoke_cpi::account::ZCpiContextAccount,
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
                let hashed_pubkey = hash_to_bn254_field_size_be(pubkey.as_ref());
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
            transfer_lamports_invoke(fee_payer, &accounts[*i as usize], *fee)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct WrappedInstructionData<'a, T: InstructionData<'a>> {
    instruction_data: T,
    cpi_context: Option<ZCpiContextAccount<'a>>,
    address_len: usize,
    input_len: usize,
    outputs_len: usize,
    /// Offsets are used to copy output compressed account data from the cpi context
    /// to the system program -> account compression program cpi instruction data.
    /// This ensures the indexer can index all output account data.
    cpi_context_outputs_start_offset: usize,
    cpi_context_outputs_end_offset: usize,
}

impl<'a, 'b, T: InstructionData<'a>> WrappedInstructionData<'a, T> {
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

    pub fn set_cpi_context(
        &mut self,
        cpi_context: ZCpiContextAccount<'a>,
        outputs_start_offset: usize,
        outputs_end_offset: usize,
    ) -> Result<()> {
        if cpi_context.context.len() != 1 {
            return Err(SystemProgramError::InvalidCapacity.into());
        }
        if self.cpi_context.is_none() {
            self.outputs_len += cpi_context.context[0].output_compressed_accounts.len();
            if self.outputs_len > MAX_OUTPUT_ACCOUNTS {
                return Err(SystemProgramError::TooManyOutputAccounts.into());
            }
            self.address_len += cpi_context.context[0].new_address_params.len();
            self.input_len += cpi_context.context[0]
                .input_compressed_accounts_with_merkle_context
                .len();
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

    pub fn get_cpi_context_account(&'b self) -> &'b Option<ZCpiContextAccount<'a>> {
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

    pub fn get_output_account(&'b self, index: usize) -> Option<&'b (dyn OutputAccount<'a> + 'b)> {
        let ix_outputs_len = self.instruction_data.output_accounts().len();
        if index >= ix_outputs_len {
            if let Some(cpi_context) = self.cpi_context.as_ref() {
                if let Some(context) = cpi_context.context.first() {
                    let index = index.saturating_sub(ix_outputs_len);
                    context
                        .output_accounts()
                        .get(index)
                        .map(|account| account as &dyn OutputAccount<'a>)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            self.instruction_data
                .output_accounts()
                .get(index)
                .map(|account| account as &dyn OutputAccount<'a>)
        }
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
                self.instruction_data.new_addresses(),
                cpi_context.context[0].new_addresses(),
            )
        } else {
            let empty_slice = &[];
            chain_new_addresses(self.instruction_data.new_addresses(), empty_slice)
        }
    }

    pub fn output_accounts<'b>(&'b self) -> impl Iterator<Item = &'b dyn OutputAccount<'a>> {
        if let Some(cpi_context) = &self.cpi_context {
            chain_outputs(
                self.instruction_data.output_accounts(),
                cpi_context.context[0].output_accounts(),
            )
        } else {
            chain_outputs(self.instruction_data.output_accounts(), &[])
        }
    }

    pub fn input_accounts<'b>(&'b self) -> impl Iterator<Item = &'b dyn InputAccount<'a>> {
        if let Some(cpi_context) = &self.cpi_context {
            chain_inputs(
                self.instruction_data.input_accounts(),
                cpi_context.context[0].input_accounts(),
            )
        } else {
            let empty_slice = &[];
            chain_inputs(self.instruction_data.input_accounts(), empty_slice)
        }
    }

    pub fn into_instruction_data_invoke_cpi(
        &self,
        cpi_account_data: &mut InstructionDataInvokeCpi,
    ) {
        for input in self.instruction_data.input_accounts() {
            if input.skip() {
                continue;
            }
            let input_account = PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: input.owner().into(),
                    lamports: input.lamports(),
                    address: input.address(),
                    data: input.data(),
                },
                merkle_context: input.merkle_context().into(),
                read_only: false,
                root_index: input.root_index(),
            };
            cpi_account_data
                .input_compressed_accounts_with_merkle_context
                .push(input_account);
        }

        for output in self.instruction_data.output_accounts() {
            if output.skip() {
                continue;
            }
            let output_account = OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: output.owner().into(),
                    lamports: output.lamports(),
                    address: output.address(),
                    data: output.data(),
                },
                merkle_tree_index: output.merkle_tree_index(),
            };
            cpi_account_data
                .output_compressed_accounts
                .push(output_account);
        }

        if !self.instruction_data.new_addresses().is_empty() {
            unimplemented!("Address assignment cannot be guaranteed with cpi context.");
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

pub fn chain_outputs<'a, 'b: 'a>(
    slice1: &'a [impl OutputAccount<'b>],
    slice2: &'a [impl OutputAccount<'b>],
) -> impl Iterator<Item = &'a (dyn OutputAccount<'b>)> {
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

pub fn chain_inputs<'a, 'b: 'a>(
    slice1: &'a [impl InputAccount<'b>],
    slice2: &'a [impl InputAccount<'b>],
) -> impl Iterator<Item = &'a (dyn InputAccount<'b>)> {
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

pub fn chain_new_addresses<'a, 'b: 'a>(
    slice1: &'a [impl NewAddress<'b>],
    slice2: &'a [impl NewAddress<'b>],
) -> impl Iterator<Item = &'a (dyn NewAddress<'b>)> {
    slice1
        .iter()
        .map(|item| item as &dyn NewAddress<'b>)
        .chain(slice2.iter().map(|item| item as &dyn NewAddress<'b>))
}
