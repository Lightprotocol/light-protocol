use std::ops::Deref;

use light_ctoken_types::instructions::multi_transfer::{
    Compression, CompressionMode, MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
};
use solana_account_info::AccountInfo;
use solana_pubkey::Pubkey;

use crate::error::TokenSdkError;

#[derive(Debug, PartialEq, Clone)]
pub struct CTokenAccount2 {
    inputs: Vec<MultiInputTokenDataWithContext>,
    output: MultiTokenTransferOutputData,
    compression: Option<Compression>,
    delegate_is_set: bool,
    pub(crate) method_used: bool,
}

impl CTokenAccount2 {
    pub fn new(
        token_data: Vec<MultiInputTokenDataWithContext>,
        output_merkle_tree_index: u8,
    ) -> Result<Self, TokenSdkError> {
        // all mint indices must be the same
        // all owners must be the same
        let amount = token_data.iter().map(|data| data.amount).sum();
        // Check if token_data is empty
        if token_data.is_empty() {
            return Err(TokenSdkError::InsufficientBalance); // TODO: Add proper error variant
        }

        // Use the indices from the first token data (assuming they're all the same mint/owner)
        let mint_index = token_data[0].mint;
        let owner_index = token_data[0].owner;
        let output = MultiTokenTransferOutputData {
            owner: owner_index,
            amount,
            merkle_tree: output_merkle_tree_index,
            delegate: 0, // Default delegate index
            mint: mint_index,
            version: 2, // V2 for batched Merkle trees
        };
        Ok(Self {
            inputs: token_data,
            output,
            delegate_is_set: false,
            compression: None,
            method_used: false,
        })
    }

    pub fn new_empty(owner_index: u8, mint_index: u8, output_merkle_tree_index: u8) -> Self {
        Self {
            inputs: vec![],
            output: MultiTokenTransferOutputData {
                owner: owner_index,
                amount: 0,
                merkle_tree: output_merkle_tree_index,
                delegate: 0, // Default delegate index
                mint: mint_index,
                version: 2, // V2 for batched Merkle trees
            },
            compression: None,
            delegate_is_set: false,
            method_used: false,
        }
    }

    // TODO: consider this might be confusing because it must not be used in combination with fn transfer()
    //     could mark the struct as transferred and throw in fn transfer
    pub fn transfer(
        &mut self,
        recipient_index: u8,
        amount: u64,
        output_merkle_tree_index: Option<u8>,
    ) -> Result<Self, TokenSdkError> {
        if amount > self.output.amount {
            return Err(TokenSdkError::InsufficientBalance);
        }
        // TODO: skip outputs with zero amount when creating the instruction data.
        self.output.amount -= amount;
        let merkle_tree_index = output_merkle_tree_index.unwrap_or(self.output.merkle_tree);

        self.method_used = true;
        Ok(Self {
            compression: None,
            inputs: vec![],
            output: MultiTokenTransferOutputData {
                owner: recipient_index,
                amount,
                merkle_tree: merkle_tree_index,
                delegate: 0,
                mint: self.output.mint,
                version: self.output.version,
            },
            delegate_is_set: false,
            method_used: false,
        })
    }

    /// Approves a delegate for a specified amount of tokens.
    /// Similar to transfer, this deducts the amount from the current account
    /// and returns a new CTokenAccount that represents the delegated portion.
    /// The original account balance is reduced by the delegated amount.
    pub fn approve(
        &mut self,
        delegate_index: u8,
        amount: u64,
        output_merkle_tree_index: Option<u8>,
    ) -> Result<Self, TokenSdkError> {
        if amount > self.output.amount {
            return Err(TokenSdkError::InsufficientBalance);
        }

        // Deduct the delegated amount from current account
        self.output.amount -= amount;
        let merkle_tree_index = output_merkle_tree_index.unwrap_or(self.output.merkle_tree);

        self.method_used = true;

        // Create a new delegated account with the specified delegate
        // Note: In the actual instruction, this will create the proper delegation structure
        Ok(Self {
            compression: None,
            inputs: vec![],
            output: MultiTokenTransferOutputData {
                owner: self.output.owner, // Owner remains the same
                amount,
                merkle_tree: merkle_tree_index,
                delegate: delegate_index,
                mint: self.output.mint,
                version: self.output.version,
            },
            delegate_is_set: true,
            method_used: false,
        })
    }

    // TODO: consider this might be confusing because it must not be used in combination with fn compress()
    pub fn compress(
        &mut self,
        amount: u64,
        source_or_recipient_index: u8,
    ) -> Result<(), TokenSdkError> {
        // Check if there's already a compression with different mode
        if let Some(compression) = &self.compression {
            if compression.mode != CompressionMode::Compress {
                return Err(TokenSdkError::CannotCompressAndDecompress);
            }
        }

        self.output.amount += amount;
        self.compression = Some(Compression {
            amount,
            mode: CompressionMode::Compress,
            mint: self.output.mint,
            source_or_recipient: source_or_recipient_index,
        });
        self.method_used = true;

        Ok(())
    }

    // TODO: consider this might be confusing because it must not be used in combination with fn decompress()
    pub fn decompress(&mut self, amount: u64, source_index: u8) -> Result<(), TokenSdkError> {
        // Check if there's already a compression with different mode
        if let Some(compression) = &self.compression {
            if compression.mode != CompressionMode::Decompress {
                return Err(TokenSdkError::CannotCompressAndDecompress);
            }
        }

        if self.output.amount < amount {
            return Err(TokenSdkError::InsufficientBalance);
        }
        self.output.amount -= amount;

        self.compression = Some(Compression {
            amount,
            mode: CompressionMode::Decompress,
            mint: self.output.mint,
            source_or_recipient: source_index,
        });
        self.method_used = true;

        Ok(())
    }

    pub fn is_compress(&self) -> bool {
        self.compression
            .as_ref()
            .map(|c| c.mode == CompressionMode::Compress)
            .unwrap_or(false)
    }

    pub fn is_decompress(&self) -> bool {
        self.compression
            .as_ref()
            .map(|c| c.mode == CompressionMode::Decompress)
            .unwrap_or(false)
    }

    pub fn mint(&self, account_infos: &[AccountInfo]) -> Pubkey {
        *account_infos[self.mint as usize].key
    }

    pub fn compression_amount(&self) -> Option<u64> {
        self.compression.as_ref().map(|c| c.amount)
    }

    pub fn compression(&self) -> Option<&Compression> {
        self.compression.as_ref()
    }

    pub fn owner(&self, account_infos: &[AccountInfo]) -> Pubkey {
        *account_infos[self.owner as usize].key
    }
    // TODO: make option and take from self
    //pub fn delegate_account<'b>(&self, account_infos: &'b [&'b AccountInfo]) -> &'b Pubkey {
    //    account_infos[self.output.delegate as usize].key
    // }

    pub fn input_metas(&self) -> &[MultiInputTokenDataWithContext] {
        self.inputs.as_slice()
    }

    /// Consumes token account for instruction creation.
    pub fn into_inputs_and_outputs(
        self,
    ) -> (
        Vec<MultiInputTokenDataWithContext>,
        MultiTokenTransferOutputData,
    ) {
        (self.inputs, self.output)
    }
}

impl Deref for CTokenAccount2 {
    type Target = MultiTokenTransferOutputData;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}
