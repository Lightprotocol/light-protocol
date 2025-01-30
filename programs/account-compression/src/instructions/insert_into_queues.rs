use light_zero_copy::{errors::ZeroCopyError, slice_mut::ZeroCopySliceMut};
use std::mem::size_of;
use std::ops::{Deref, DerefMut};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Ref, Unaligned};

use crate::processor::insert_addresses::InsertAddressInput;
use crate::processor::insert_leaves::AppendLeavesInput;
use crate::processor::insert_nullifiers::InsertNullifierInput;
use crate::{context::AcpAccount, errors::AccountCompressionErrorCode};

#[repr(C)]
#[derive(
    FromBytes, IntoBytes, KnownLayout, Immutable, Copy, Clone, PartialEq, Debug, Unaligned,
)]
pub struct AppendNullifyCreateAddressInputsMeta {
    is_invoked_by_program: u8,
    pub bump: u8,
    pub num_queues: u8,
    pub num_unique_appends: u8,
    pub num_address_appends: u8,
    pub tx_hash: [u8; 32],
}

#[derive(Debug)]
pub struct AppendNullifyCreateAddressInputs<'a> {
    meta: Ref<&'a mut [u8], AppendNullifyCreateAddressInputsMeta>,
    pub leaves: ZeroCopySliceMut<'a, u8, AppendLeavesInput, false>,
    pub nullifiers: ZeroCopySliceMut<'a, u8, InsertNullifierInput, false>,
    pub addresses: ZeroCopySliceMut<'a, u8, InsertAddressInput, false>,
    // Don't add sequence numbers we don't want to deserialize these here.
}

impl<'a> AppendNullifyCreateAddressInputs<'a> {
    pub fn is_invoked_by_program(&self) -> bool {
        self.meta.is_invoked_by_program == 1
    }

    pub fn set_invoked_by_program(&mut self, value: bool) {
        self.meta.is_invoked_by_program = value as u8;
    }

    pub fn required_size_for_capacity(
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
    ) -> usize {
        size_of::<AppendNullifyCreateAddressInputsMeta>()
            + ZeroCopySliceMut::<u8, AppendLeavesInput, false>::required_size_for_capacity(
                leaves_capacity,
            )
            + ZeroCopySliceMut::<u8, InsertNullifierInput, false>::required_size_for_capacity(
                nullifiers_capacity,
            )
            + ZeroCopySliceMut::<u8, InsertAddressInput, false>::required_size_for_capacity(
                addresses_capacity,
            )
    }

    pub fn new(
        bytes: &'a mut [u8],
        leaves_capacity: u8,
        nullifiers_capacity: u8,
        addresses_capacity: u8,
    ) -> std::result::Result<Self, ZeroCopyError> {
        let (meta, bytes) = bytes.split_at_mut(size_of::<AppendNullifyCreateAddressInputsMeta>());
        let meta = Ref::<&mut [u8], AppendNullifyCreateAddressInputsMeta>::from_bytes(meta)?;
        let (leaves, bytes) =
            ZeroCopySliceMut::<u8, AppendLeavesInput, false>::new_at(leaves_capacity, bytes)?;
        let (nullifiers, bytes) = ZeroCopySliceMut::<u8, InsertNullifierInput, false>::new_at(
            nullifiers_capacity,
            bytes,
        )?;
        let addresses =
            ZeroCopySliceMut::<u8, InsertAddressInput, false>::new(addresses_capacity, bytes)?;
        Ok(AppendNullifyCreateAddressInputs {
            meta,
            leaves,
            nullifiers,
            addresses,
        })
    }
}

impl Deref for AppendNullifyCreateAddressInputs<'_> {
    type Target = AppendNullifyCreateAddressInputsMeta;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

impl DerefMut for AppendNullifyCreateAddressInputs<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.meta
    }
}

pub fn deserialize_nullify_append_create_address_inputs<'a>(
    bytes: &'a mut [u8],
) -> std::result::Result<AppendNullifyCreateAddressInputs<'a>, ZeroCopyError> {
    let (metadata, bytes) = bytes.split_at_mut(size_of::<AppendNullifyCreateAddressInputsMeta>());
    let meta = Ref::<&mut [u8], AppendNullifyCreateAddressInputsMeta>::from_bytes(metadata)?;

    let (leaves, bytes) = ZeroCopySliceMut::<u8, AppendLeavesInput, false>::from_bytes_at(bytes)?;

    let (nullifiers, bytes) =
        ZeroCopySliceMut::<u8, InsertNullifierInput, false>::from_bytes_at(bytes)?;
    let (addresses, _bytes) =
        ZeroCopySliceMut::<u8, InsertAddressInput, false>::from_bytes_at(bytes)?;
    Ok(AppendNullifyCreateAddressInputs {
        meta,
        leaves,
        nullifiers,
        addresses,
    })
}

pub fn get_queue_and_tree_accounts<'a, 'b, 'info>(
    accounts: &'b mut [AcpAccount<'a, 'info>],
    queue_index: usize,
    tree_index: usize,
) -> std::result::Result<
    (&'b mut AcpAccount<'a, 'info>, &'b mut AcpAccount<'a, 'info>),
    AccountCompressionErrorCode,
> {
    let (smaller, bigger) = if queue_index < tree_index {
        (queue_index, tree_index)
    } else {
        (tree_index, queue_index)
    };
    let (left, right) = accounts.split_at_mut(bigger);
    let smaller_ref = &mut left[smaller];
    let bigger_ref = &mut right[0];
    Ok(if queue_index < tree_index {
        (smaller_ref, bigger_ref)
    } else {
        (bigger_ref, smaller_ref)
    })
}
