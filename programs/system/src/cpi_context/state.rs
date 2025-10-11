use std::slice;

use borsh::BorshDeserialize;
use light_account_checks::{checks::check_owner, discriminator::Discriminator};
use light_compressed_account::instruction_data::{
    data::OutputCompressedAccountWithPackedContext,
    zero_copy::{ZPackedMerkleContext, ZPackedReadOnlyAddress, ZPackedReadOnlyCompressedAccount},
};
use light_profiler::profile;
use light_zero_copy::{errors::ZeroCopyError, slice_mut::ZeroCopySliceMut, vec::ZeroCopyVecU8};
use pinocchio::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use zerocopy::{little_endian::U16, Ref};

use crate::{
    cpi_context::{
        account::{CpiContextInAccount, CpiContextOutAccount},
        address::CpiContextNewAddressParamsAssignedPacked,
    },
    errors::SystemProgramError,
    CPI_CONTEXT_ACCOUNT_DISCRIMINATOR, CPI_CONTEXT_ACCOUNT_DISCRIMINATOR_V1, ID,
};

/// Collects instruction data without executing a compressed transaction.
/// Signer checks are performed on instruction data.
/// Collected instruction data is combined with the instruction data of the executing cpi,
/// and executed as a single transaction.
/// This enables to use input compressed accounts that are owned by multiple programs,
/// with one zero-knowledge proof.
#[derive(Debug, PartialEq, Default, BorshDeserialize, Clone)]
#[repr(C)]
pub struct CpiContextAccountLegacy {
    pub fee_payer: Pubkey,
    pub associated_merkle_tree: Pubkey,
}

#[derive(Debug)]
pub struct ZCpiContextAccount<'a> {
    pub fee_payer: Ref<&'a mut [u8], light_compressed_account::pubkey::Pubkey>,
    pub associated_merkle_tree: Ref<&'a mut [u8], light_compressed_account::pubkey::Pubkey>,
    pub new_addresses: ZeroCopyVecU8<'a, CpiContextNewAddressParamsAssignedPacked>,
    pub readonly_addresses: ZeroCopyVecU8<'a, ZPackedReadOnlyAddress>,
    pub readonly_accounts: ZeroCopyVecU8<'a, ZPackedReadOnlyCompressedAccount>,
    pub in_accounts: ZeroCopyVecU8<'a, CpiContextInAccount>,
    pub out_accounts: ZeroCopyVecU8<'a, CpiContextOutAccount>,
    output_data_len: Ref<&'a mut [u8], U16>,
    pub output_data: Vec<ZeroCopySliceMut<'a, U16, u8>>,
    remaining_data: &'a mut [u8],
}

impl<'a> ZCpiContextAccount<'a> {
    #[profile]
    pub fn is_empty(&self) -> bool {
        self.new_addresses.is_empty()
            && self.readonly_addresses.is_empty()
            && self.readonly_accounts.is_empty()
            && self.in_accounts.is_empty()
            && self.out_accounts.is_empty()
            && self.output_data.is_empty()
    }

    /// Get the output data length
    #[inline]
    pub fn output_data_len(&self) -> u16 {
        self.output_data_len.get()
    }

    #[profile]
    pub fn store_data<
        'b,
        T: light_compressed_account::instruction_data::traits::InstructionData<'b>,
    >(
        &'a mut self,
        instruction_data: &crate::context::WrappedInstructionData<'b, T>,
    ) -> Result<(), light_zero_copy::errors::ZeroCopyError> {
        let pre_address_len = self.new_addresses.len();
        // Cache owner bytes to avoid repeated calls
        let owner_bytes = instruction_data.owner().to_bytes();

        // Store new addresses
        for address in instruction_data.new_addresses() {
            let assigned_index = address.assigned_compressed_account_index();
            // Use checked arithmetic to prevent overflow
            let assigned_account_index = (assigned_index.unwrap_or(0) as u8)
                .checked_add(pre_address_len as u8)
                .ok_or(ZeroCopyError::Size)?;
            let new_address = CpiContextNewAddressParamsAssignedPacked {
                owner: owner_bytes, // Use cached owner bytes
                seed: address.seed(),
                address_queue_account_index: address.address_queue_index(),
                address_merkle_tree_account_index: address.address_merkle_tree_account_index(),
                address_merkle_tree_root_index: address.address_merkle_tree_root_index().into(),
                assigned_to_account: assigned_index.is_some() as u8,
                assigned_account_index,
            };
            self.new_addresses.push(new_address)?;
        }

        // Store input accounts
        for input in instruction_data.input_accounts() {
            if input.skip() {
                continue;
            }
            // Cache data and address calls
            let data = input.data();
            let address = input.address();
            let merkle_context = input.merkle_context();

            let in_account = CpiContextInAccount {
                owner: *input.owner(),
                discriminator: data.as_ref().map_or([0; 8], |d| d.discriminator),
                data_hash: data.as_ref().map_or([0; 32], |d| d.data_hash),
                merkle_context: ZPackedMerkleContext {
                    merkle_tree_pubkey_index: merkle_context.merkle_tree_pubkey_index,
                    queue_pubkey_index: merkle_context.queue_pubkey_index,
                    leaf_index: merkle_context.leaf_index,
                    prove_by_index: merkle_context.prove_by_index() as u8, // Direct bool to u8
                },
                root_index: input.root_index().into(),
                lamports: input.lamports().into(),
                with_address: address.is_some() as u8, // Direct bool to u8
                address: address.unwrap_or([0; 32]),
            };
            self.in_accounts.push(in_account)?;
        }

        // Store read-only addresses if any
        if let Some(readonly_addresses) = instruction_data.read_only_addresses() {
            for readonly_address in readonly_addresses {
                self.readonly_addresses.push(*readonly_address)?;
            }
        }

        // Store read-only accounts if any
        if let Some(readonly_accounts) = instruction_data.read_only_accounts() {
            for readonly_account in readonly_accounts {
                self.readonly_accounts.push(*readonly_account)?;
            }
        }
        // Store output accounts
        for output in instruction_data.output_accounts() {
            if output.skip() {
                // Skipping none accounts from CompressedAccountInfo based instruction data
                continue;
            }
            // Cache data and address calls
            let data = output.data();
            let address = output.address();

            let out_account = CpiContextOutAccount {
                owner: output.owner(),
                discriminator: data.as_ref().map_or([0; 8], |d| d.discriminator),
                data_hash: data.as_ref().map_or([0; 32], |d| d.data_hash),
                output_merkle_tree_index: output.merkle_tree_index(),
                lamports: output.lamports().into(),
                with_address: address.is_some() as u8, // Direct bool to u8
                address: address.unwrap_or([0; 32]),
            };
            self.out_accounts.push(out_account)?;
            // Add output data
            {
                *self.output_data_len += 1;

                let data_len = data
                    .as_ref()
                    .map_or(Ok(0), |d| d.data.len().try_into())
                    .map_err(|_| ZeroCopyError::InvalidConversion)?;
                let (mut new_data, remaining_data) =
                    ZeroCopySliceMut::<U16, u8>::new_at(data_len.into(), self.remaining_data)?;

                if let Some(d) = data.as_ref() {
                    new_data.as_mut_slice().copy_from_slice(&d.data);
                }
                self.output_data.push(new_data);
                self.remaining_data = remaining_data;
            }
        }

        Ok(())
    }
}

impl Discriminator for ZCpiContextAccount<'_> {
    const LIGHT_DISCRIMINATOR: [u8; 8] = CPI_CONTEXT_ACCOUNT_DISCRIMINATOR;
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

#[profile]
pub fn deserialize_cpi_context_account<'a>(
    account_info: &AccountInfo,
) -> Result<ZCpiContextAccount<'a>, ProgramError> {
    deserialize_cpi_context_account_inner::<false>(account_info)
}

#[profile]
pub fn deserialize_cpi_context_account_cleared<'a>(
    account_info: &AccountInfo,
) -> Result<ZCpiContextAccount<'a>, ProgramError> {
    deserialize_cpi_context_account_inner::<true>(account_info)
}

#[profile]
fn deserialize_cpi_context_account_inner<'a, const CLEARED: bool>(
    account_info: &AccountInfo,
) -> Result<ZCpiContextAccount<'a>, ProgramError> {
    check_owner(&ID, account_info).map_err(|_| SystemProgramError::InvalidCpiContextOwner)?;
    let mut account_data = account_info
        .try_borrow_mut_data()
        .map_err(|_| SystemProgramError::BorrowingDataFailed)?;
    // SAFETY: account_data is a valid RefMut<[u8]>, pointer and length are valid
    let data = unsafe { slice::from_raw_parts_mut(account_data.as_mut_ptr(), account_data.len()) };
    let (discriminator, data) = data.split_at_mut(8);
    if discriminator != CPI_CONTEXT_ACCOUNT_DISCRIMINATOR {
        msg!("Invalid cpi context account discriminator.");
        return Err(SystemProgramError::InvalidCpiContextDiscriminator.into());
    }
    let (fee_payer, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(data)
            .map_err(ZeroCopyError::from)?;

    let (associated_merkle_tree, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(data)
            .map_err(ZeroCopyError::from)?;

    let (mut new_addresses, data) =
        ZeroCopyVecU8::<'a, CpiContextNewAddressParamsAssignedPacked>::from_bytes_at(data)?;
    let (mut readonly_addresses, data) =
        ZeroCopyVecU8::<'a, ZPackedReadOnlyAddress>::from_bytes_at(data)?;
    let (mut readonly_accounts, data) =
        ZeroCopyVecU8::<'a, ZPackedReadOnlyCompressedAccount>::from_bytes_at(data)?;
    let (mut in_accounts, data) = ZeroCopyVecU8::<'a, CpiContextInAccount>::from_bytes_at(data)?;
    let (mut out_accounts, data) = ZeroCopyVecU8::<'a, CpiContextOutAccount>::from_bytes_at(data)?;
    let (mut output_data_len, mut data) =
        Ref::<&'a mut [u8], U16>::from_prefix(data).map_err(ZeroCopyError::from)?;
    let output_data = if CLEARED {
        new_addresses.zero_out();
        readonly_addresses.zero_out();
        readonly_accounts.zero_out();
        in_accounts.zero_out();
        out_accounts.zero_out();
        output_data_len.set(0);
        // 65 CU
        data.fill(0);
        Vec::new()
    } else {
        let mut output_data = Vec::with_capacity(output_data_len.get() as usize);
        for _ in 0..output_data_len.get() {
            let (output_data_slice, inner_data) = ZeroCopySliceMut::<U16, u8>::from_bytes_at(data)?;
            output_data.push(output_data_slice);
            data = inner_data;
        }
        output_data
    };

    Ok(ZCpiContextAccount {
        fee_payer,
        associated_merkle_tree,
        new_addresses,
        readonly_addresses,
        readonly_accounts,
        in_accounts,
        out_accounts,
        output_data_len,
        output_data,
        remaining_data: data,
    })
}
pub struct CpiContextAccountInitParams {
    pub associated_merkle_tree: Pubkey,
    pub new_addresses_len: u8,
    pub readonly_addresses_len: u8,
    pub readonly_accounts_len: u8,
    pub in_accounts_len: u8,
    pub out_accounts_len: u8,
}

impl CpiContextAccountInitParams {
    #[profile]
    pub fn new(associated_merkle_tree: Pubkey) -> Self {
        Self {
            associated_merkle_tree,
            new_addresses_len: 10,
            readonly_addresses_len: 10,
            readonly_accounts_len: 10,
            in_accounts_len: 20,
            out_accounts_len: 30,
        }
    }
}

/// 1. Check owner.
/// 2. Check discriminator is zero.
/// 3. Set discriminator.
/// 4. Set fee payer.
/// 5. Set associated merkle tree.
/// 6. Set new addresses length.
/// 7. Set readonly addresses length.
/// 8. Set readonly accounts length.
/// 9. Set in accounts length.
/// 10. Set out accounts length.
#[profile]
pub fn cpi_context_account_new<'a, const RE_INIT: bool>(
    account_info: &AccountInfo,
    params: CpiContextAccountInitParams,
) -> Result<ZCpiContextAccount<'a>, ProgramError> {
    check_owner(&ID, account_info).map_err(|_| {
        msg!("Invalid cpi context account owner.");
        SystemProgramError::InvalidCpiContextOwner
    })?;
    let mut account_data = account_info.try_borrow_mut_data().map_err(|_| {
        msg!("Cpi context account data borrow failed.");
        SystemProgramError::BorrowingDataFailed
    })?;

    // SAFETY: account_data is a valid RefMut<[u8]>, pointer and length are valid
    let data = unsafe { slice::from_raw_parts_mut(account_data.as_mut_ptr(), account_data.len()) };
    let (discriminator, data) = data.split_at_mut(8);
    if RE_INIT {
        // Check discriminator matches
        if discriminator != CPI_CONTEXT_ACCOUNT_DISCRIMINATOR_V1 {
            msg!("Invalid cpi context account discriminator.");
            return Err(SystemProgramError::InvalidCpiContextDiscriminator.into());
        }
        // Zero out account data
        data.fill(0);
    } else if discriminator != [0u8; 8] {
        msg!("Invalid cpi context account discriminator.");
        return Err(SystemProgramError::InvalidCpiContextDiscriminator.into());
    }
    discriminator.copy_from_slice(&CPI_CONTEXT_ACCOUNT_DISCRIMINATOR);

    let (mut fee_payer, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(data)
            .map_err(ZeroCopyError::from)?;
    *fee_payer = [0u8; 32].into(); // Is set during operation.

    let (mut associated_merkle_tree, data) =
        Ref::<&'a mut [u8], light_compressed_account::pubkey::Pubkey>::from_prefix(data)
            .map_err(ZeroCopyError::from)?;
    *associated_merkle_tree = params.associated_merkle_tree.into();

    let (new_addresses, data) =
        ZeroCopyVecU8::<'a, CpiContextNewAddressParamsAssignedPacked>::new_at(
            params.new_addresses_len,
            data,
        )?;
    let (readonly_addresses, data) =
        ZeroCopyVecU8::<'a, ZPackedReadOnlyAddress>::new_at(params.readonly_addresses_len, data)?;
    let (readonly_accounts, data) = ZeroCopyVecU8::<'a, ZPackedReadOnlyCompressedAccount>::new_at(
        params.readonly_accounts_len,
        data,
    )?;
    let (in_accounts, data) =
        ZeroCopyVecU8::<'a, CpiContextInAccount>::new_at(params.in_accounts_len, data)?;
    let (out_accounts, data) =
        ZeroCopyVecU8::<'a, CpiContextOutAccount>::new_at(params.out_accounts_len, data)?;
    let (output_data_len, data) =
        Ref::<&'a mut [u8], U16>::from_prefix(data).map_err(ZeroCopyError::from)?;
    if data.len()
        < params.out_accounts_len as usize
            * std::mem::size_of::<OutputCompressedAccountWithPackedContext>()
            + 1024
    // arbitrary minimum for Data bytes because OutputCompressedAccountWithPackedContext
    // contains vectors which are only considered with their pointer size in size_of
    {
        return Err(ZeroCopyError::InsufficientCapacity.into());
    }

    Ok(ZCpiContextAccount {
        fee_payer,
        associated_merkle_tree,
        new_addresses,
        readonly_addresses,
        readonly_accounts,
        in_accounts,
        out_accounts,
        output_data_len,
        output_data: Vec::new(),
        remaining_data: data,
    })
}
