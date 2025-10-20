use light_zero_copy::ZeroCopyMut;

use crate::{
    compressed_account::{CompressedAccount, PackedCompressedAccountWithMerkleContext},
    discriminators::DISCRIMINATOR_INVOKE,
    instruction_data::{compressed_proof::CompressedProof, traits::LightInstructionData},
    InstructionDiscriminator, Pubkey, Vec,
};

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone)]
pub struct InstructionDataInvoke {
    pub proof: Option<CompressedProof>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<PackedCompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    pub relay_fee: Option<u64>,
    pub new_address_params: Vec<NewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
}

impl InstructionDataInvoke {
    pub fn new(proof: Option<CompressedProof>) -> Self {
        Self {
            proof,
            ..Default::default()
        }
    }

    pub fn with_input_compressed_accounts_with_merkle_context(
        mut self,
        input_compressed_accounts_with_merkle_context: &[PackedCompressedAccountWithMerkleContext],
    ) -> Self {
        if !input_compressed_accounts_with_merkle_context.is_empty() {
            self.input_compressed_accounts_with_merkle_context
                .extend_from_slice(input_compressed_accounts_with_merkle_context);
        }
        self
    }

    pub fn with_output_compressed_accounts(
        mut self,
        output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    ) -> Self {
        if !output_compressed_accounts.is_empty() {
            self.output_compressed_accounts
                .extend_from_slice(output_compressed_accounts);
        }
        self
    }

    pub fn with_new_addresses(mut self, new_address_params: &[NewAddressParamsPacked]) -> Self {
        if !new_address_params.is_empty() {
            self.new_address_params
                .extend_from_slice(new_address_params);
        }
        self
    }

    pub fn compress_lamports(mut self, lamports: u64) -> Self {
        self.compress_or_decompress_lamports = Some(lamports);
        self.is_compress = true;
        self
    }

    pub fn decompress_lamports(mut self, lamports: u64) -> Self {
        self.compress_or_decompress_lamports = Some(lamports);
        self.is_compress = false;
        self
    }
}

impl InstructionDiscriminator for InstructionDataInvoke {
    fn discriminator(&self) -> &'static [u8] {
        &DISCRIMINATOR_INVOKE
    }
}

impl LightInstructionData for InstructionDataInvoke {}

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone)]
pub struct OutputCompressedAccountWithContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree: Pubkey,
}

#[repr(C)]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone, ZeroCopyMut)]
pub struct OutputCompressedAccountWithPackedContext {
    pub compressed_account: CompressedAccount,
    pub merkle_tree_index: u8,
}

#[repr(C)]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone, Copy, ZeroCopyMut)]
pub struct NewAddressParamsPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
}

#[repr(C)]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone, Copy, ZeroCopyMut)]
pub struct NewAddressParamsAssignedPacked {
    pub seed: [u8; 32],
    pub address_queue_account_index: u8,
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: u16,
    pub assigned_to_account: bool,
    pub assigned_account_index: u8,
}

impl NewAddressParamsAssignedPacked {
    pub fn new(address_params: NewAddressParamsPacked, index: Option<u8>) -> Self {
        Self {
            seed: address_params.seed,
            address_queue_account_index: address_params.address_queue_account_index,
            address_merkle_tree_account_index: address_params.address_merkle_tree_account_index,
            address_merkle_tree_root_index: address_params.address_merkle_tree_root_index,
            assigned_to_account: index.is_some(),
            assigned_account_index: index.unwrap_or_default(),
        }
    }

    pub fn assigned_account_index(&self) -> Option<u8> {
        if self.assigned_to_account {
            Some(self.assigned_account_index)
        } else {
            None
        }
    }
}

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone)]
pub struct NewAddressParams {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone)]
pub struct NewAddressParamsAssigned {
    pub seed: [u8; 32],
    pub address_queue_pubkey: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
    pub assigned_account_index: Option<u8>,
}

#[repr(C)]
#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone, Copy, ZeroCopyMut)]
pub struct PackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_root_index: u16,
    pub address_merkle_tree_account_index: u8,
}

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Default, Clone)]
pub struct ReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_root_index: u16,
}
