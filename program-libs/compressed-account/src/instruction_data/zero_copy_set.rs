use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAt;
use zerocopy::little_endian::U16;

use crate::{
    compressed_account::PackedMerkleContext,
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{ZNewAddressParamsAssignedPackedMut, ZOutputCompressedAccountWithPackedContextMut},
        with_readonly::{ZInAccountMut, ZInstructionDataInvokeCpiWithReadOnlyMut},
    },
    CompressedAccountError, Pubkey,
};

impl ZOutputCompressedAccountWithPackedContextMut<'_> {
    #[profile]
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

impl ZInAccountMut<'_> {
    #[inline]
    #[profile]
    pub fn set_z(
        &mut self,
        discriminator: [u8; 8],
        data_hash: [u8; 32],
        merkle_context: &<PackedMerkleContext as ZeroCopyAt>::ZeroCopyAt,
        root_index: U16,
        lamports: u64,
        address: Option<&[u8; 32]>,
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
    #[profile]
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

pub trait CompressedCpiContextTrait {
    fn set_context(&self) -> u8;
    fn first_set_context(&self) -> u8;
}

impl ZInstructionDataInvokeCpiWithReadOnlyMut<'_> {
    #[inline]
    pub fn initialize(
        &mut self,
        bump: u8,
        invoking_program_id: &Pubkey,
        input_proof: Option<<CompressedProof as ZeroCopyAt>::ZeroCopyAt>,
        cpi_context: &Option<impl CompressedCpiContextTrait>,
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
        // self.cpi_context is constant, always allocated
        //      -> no reverse ok_or check necessary
        if let Some(cpi_context) = cpi_context {
            self.with_cpi_context = 1;
            self.cpi_context.cpi_context_account_index = 0;
            self.cpi_context.first_set_context = cpi_context.first_set_context();
            self.cpi_context.set_context = cpi_context.set_context();
        }

        Ok(())
    }
}

impl ZNewAddressParamsAssignedPackedMut<'_> {
    #[inline]
    #[profile]
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
        } else {
            self.assigned_account_index = 0;
            self.assigned_to_account = 0; // set to false
        }
        // Note we can skip address derivation since we are assigning it to the account in index 0.
        self.address_merkle_tree_account_index = address_merkle_tree_account_index;
    }
}
