use light_zero_copy::borsh::Deserialize;
use zerocopy::little_endian::U16;

use crate::{
    compressed_account::PackedMerkleContext,
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{ZNewAddressParamsAssignedPackedMut, ZOutputCompressedAccountWithPackedContextMut},
        with_readonly::{ZInAccountMut, ZInstructionDataInvokeCpiWithReadOnlyMut},
    },
    CompressedAccountError, Pubkey,
};

// TODO: unit test
impl ZOutputCompressedAccountWithPackedContextMut<'_> {
    #[inline]
    pub fn set(
        &mut self,
        owner: Pubkey,
        lamports: u64,
        address: Option<[u8; 32]>,
        merkle_tree_index: u8,
        discriminator: [u8; 8],
        data_hash: [u8; 32],
    ) -> Result<(), CompressedAccountError> {
        self.compressed_account.owner = owner;
        self.compressed_account.lamports = lamports.into();
        if let Some(self_address) = self.compressed_account.address.as_deref_mut() {
            let input_address =
                address.ok_or(CompressedAccountError::InstructionDataExpectedAddress)?;
            *self_address = input_address;
        }
        if self.compressed_account.address.is_none() && address.is_some() {
            return Err(CompressedAccountError::ZeroCopyExpectedAddress);
        }
        *self.merkle_tree_index = merkle_tree_index;
        let data = self
            .compressed_account
            .data
            .as_mut()
            .ok_or(CompressedAccountError::CompressedAccountDataNotInitialized)?;
        data.discriminator = discriminator;
        *data.data_hash = data_hash;

        Ok(())
    }
}

// TODO: unit test
impl ZInAccountMut<'_> {
    #[inline]
    pub fn set_z(
        &mut self,
        discriminator: [u8; 8],
        data_hash: [u8; 32],
        merkle_context: &<PackedMerkleContext as Deserialize>::Output,
        root_index: U16,
        lamports: u64,
        address: Option<&[u8]>,
    ) -> Result<(), CompressedAccountError> {
        self.discriminator = discriminator;
        // Set merkle context fields manually due to mutability constraints
        self.merkle_context.merkle_tree_pubkey_index = merkle_context.merkle_tree_pubkey_index;
        self.merkle_context.queue_pubkey_index = merkle_context.queue_pubkey_index;
        self.merkle_context
            .leaf_index
            .set(merkle_context.leaf_index.get());
        self.merkle_context.prove_by_index = merkle_context.prove_by_index() as u8;
        *self.root_index = root_index;
        self.data_hash = data_hash;
        *self.lamports = lamports.into();
        if let Some(address) = address {
            self.address
                .as_mut()
                .ok_or(CompressedAccountError::InstructionDataExpectedAddress)?
                .copy_from_slice(address);
        }
        if self.address.is_some() && address.is_none() {
            return Err(CompressedAccountError::ZeroCopyExpectedAddress);
        }
        Ok(())
    }

    #[inline]
    pub fn set(
        &mut self,
        discriminator: [u8; 8],
        data_hash: [u8; 32],
        merkle_context: &PackedMerkleContext,
        root_index: U16,
        lamports: u64,
        address: Option<&[u8]>,
    ) -> Result<(), CompressedAccountError> {
        self.discriminator = discriminator;
        // Set merkle context fields manually due to mutability constraints
        self.merkle_context.merkle_tree_pubkey_index = merkle_context.merkle_tree_pubkey_index;
        self.merkle_context.queue_pubkey_index = merkle_context.queue_pubkey_index;
        self.merkle_context
            .leaf_index
            .set(merkle_context.leaf_index);
        self.merkle_context.prove_by_index = merkle_context.prove_by_index as u8;
        *self.root_index = root_index;
        self.data_hash = data_hash;
        *self.lamports = lamports.into();
        if let Some(address) = address {
            self.address
                .as_mut()
                .ok_or(CompressedAccountError::InstructionDataExpectedAddress)?
                .copy_from_slice(address);
        }
        if self.address.is_some() && address.is_none() {
            return Err(CompressedAccountError::ZeroCopyExpectedAddress);
        }
        Ok(())
    }
}

impl ZInstructionDataInvokeCpiWithReadOnlyMut<'_> {
    #[inline]
    pub fn initialize(
        &mut self,
        bump: u8,
        invoking_program_id: &Pubkey,
        input_proof: Option<<CompressedProof as Deserialize>::Output>,
        cpi_context: Option<CompressedCpiContext>,
    ) -> Result<(), CompressedAccountError> {
        self.mode = 1; // Small ix mode
        self.bump = bump;
        self.invoking_program_id = *invoking_program_id;
        if let Some(proof) = self.proof.as_deref_mut() {
            let input_proof =
                input_proof.ok_or(CompressedAccountError::InstructionDataExpectedProof)?;
            proof.a = input_proof.a;
            proof.b = input_proof.b;
            proof.c = input_proof.c;
        }
        if self.proof.is_none() && input_proof.is_some() {
            return Err(CompressedAccountError::ZeroCopyExpectedProof);
        }
        if let Some(cpi_context) = cpi_context {
            self.with_cpi_context = 1;
            self.cpi_context.cpi_context_account_index = cpi_context.cpi_context_account_index;
            self.cpi_context.first_set_context = cpi_context.first_set_context as u8;
            self.cpi_context.set_context = cpi_context.set_context as u8;
        }

        Ok(())
    }
}

impl ZNewAddressParamsAssignedPackedMut<'_> {
    #[inline]
    pub fn set(
        &mut self,
        seed: [u8; 32],
        address_merkle_tree_root_index: U16,
        assigned_account_index: Option<u8>,
        address_merkle_tree_account_index: u8,
    ) {
        self.seed = seed;
        self.address_merkle_tree_root_index = address_merkle_tree_root_index;
        self.address_queue_account_index = 0; // always 0 for v2 address trees.
        if let Some(assigned_account_index) = assigned_account_index {
            self.assigned_account_index = assigned_account_index;
            self.assigned_to_account = 1; // set to true
        }
        // Note we can skip address derivation since we are assigning it to the account in index 0.
        self.address_merkle_tree_account_index = address_merkle_tree_account_index;
    }
}
