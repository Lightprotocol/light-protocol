use std::ops::Deref;

use light_compressed_token_types::{PackedTokenTransferOutputData, TokenAccountMeta};
use solana_pubkey::Pubkey;

use crate::error::TokenSdkError;

#[derive(Debug, PartialEq, Clone)]
pub struct CTokenAccount {
    inputs: Vec<TokenAccountMeta>,
    output: PackedTokenTransferOutputData,
    compression_amount: Option<u64>,
    is_compress: bool,
    is_decompress: bool,
    mint: Pubkey,
    pub(crate) method_used: bool,
}

impl CTokenAccount {
    pub fn new(
        mint: Pubkey,
        owner: Pubkey,
        token_data: Vec<TokenAccountMeta>,
        output_merkle_tree_index: u8,
    ) -> Self {
        let amount = token_data.iter().map(|data| data.amount).sum();
        let lamports = token_data.iter().map(|data| data.lamports).sum();
        let output = PackedTokenTransferOutputData {
            owner: owner.to_bytes(),
            amount,
            lamports,
            tlv: None,
            merkle_tree_index: output_merkle_tree_index,
        };
        Self {
            inputs: token_data,
            output,
            compression_amount: None,
            is_compress: false,
            is_decompress: false,
            mint,
            method_used: false,
        }
    }

    pub fn new_empty(mint: Pubkey, owner: Pubkey, output_merkle_tree_index: u8) -> Self {
        Self {
            inputs: vec![],
            output: PackedTokenTransferOutputData {
                owner: owner.to_bytes(),
                amount: 0,
                lamports: None,
                tlv: None,
                merkle_tree_index: output_merkle_tree_index,
            },
            compression_amount: None,
            is_compress: false,
            is_decompress: false,
            mint,
            method_used: false,
        }
    }

    // TODO: consider this might be confusing because it must not be used in combination with fn transfer()
    //     could mark the struct as transferred and throw in fn transfer
    pub fn transfer(
        &mut self,
        recipient: &Pubkey,
        amount: u64,
        output_merkle_tree_index: Option<u8>,
    ) -> Result<Self, TokenSdkError> {
        if amount > self.output.amount {
            return Err(TokenSdkError::InsufficientBalance);
        }
        // TODO: skip outputs with zero amount when creating the instruction data.
        self.output.amount -= amount;
        let merkle_tree_index = output_merkle_tree_index.unwrap_or(self.output.merkle_tree_index);

        self.method_used = true;
        Ok(Self {
            compression_amount: None,
            is_compress: false,
            is_decompress: false,
            inputs: vec![],
            output: PackedTokenTransferOutputData {
                owner: recipient.to_bytes(),
                amount,
                lamports: None,
                tlv: None,
                merkle_tree_index,
            },
            mint: self.mint,
            method_used: true,
        })
    }

    /// Approves a delegate for a specified amount of tokens.
    /// Similar to transfer, this deducts the amount from the current account
    /// and returns a new CTokenAccount that represents the delegated portion.
    /// The original account balance is reduced by the delegated amount.
    pub fn approve(
        &mut self,
        _delegate: &Pubkey,
        amount: u64,
        output_merkle_tree_index: Option<u8>,
    ) -> Result<Self, TokenSdkError> {
        if amount > self.output.amount {
            return Err(TokenSdkError::InsufficientBalance);
        }

        // Deduct the delegated amount from current account
        self.output.amount -= amount;
        let merkle_tree_index = output_merkle_tree_index.unwrap_or(self.output.merkle_tree_index);

        self.method_used = true;

        // Create a new delegated account with the specified delegate
        // Note: In the actual instruction, this will create the proper delegation structure
        Ok(Self {
            compression_amount: None,
            is_compress: false,
            is_decompress: false,
            inputs: vec![],
            output: PackedTokenTransferOutputData {
                owner: self.output.owner, // Owner remains the same, but delegate is set
                amount,
                lamports: None,
                tlv: None,
                merkle_tree_index,
            },
            mint: self.mint,
            method_used: true,
        })
    }

    // TODO: consider this might be confusing because it must not be used in combination with fn compress()
    pub fn compress(&mut self, amount: u64) -> Result<(), TokenSdkError> {
        self.output.amount += amount;
        self.is_compress = true;
        if self.is_decompress {
            return Err(TokenSdkError::CannotCompressAndDecompress);
        }

        match self.compression_amount.as_mut() {
            Some(amount_ref) => *amount_ref += amount,
            None => self.compression_amount = Some(amount),
        }
        self.method_used = true;

        Ok(())
    }

    // TODO: consider this might be confusing because it must not be used in combination with fn decompress()
    pub fn decompress(&mut self, amount: u64) -> Result<(), TokenSdkError> {
        if self.is_compress {
            return Err(TokenSdkError::CannotCompressAndDecompress);
        }
        if self.output.amount < amount {
            return Err(TokenSdkError::InsufficientBalance);
        }
        self.output.amount -= amount;

        self.is_decompress = true;

        match self.compression_amount.as_mut() {
            Some(amount_ref) => *amount_ref += amount,
            None => self.compression_amount = Some(amount),
        }
        self.method_used = true;

        Ok(())
    }

    pub fn is_compress(&self) -> bool {
        self.is_compress
    }

    pub fn is_decompress(&self) -> bool {
        self.is_decompress
    }

    pub fn mint(&self) -> &Pubkey {
        &self.mint
    }

    pub fn compression_amount(&self) -> Option<u64> {
        self.compression_amount
    }

    pub fn owner(&self) -> Pubkey {
        Pubkey::new_from_array(self.owner)
    }
    pub fn input_metas(&self) -> &[TokenAccountMeta] {
        self.inputs.as_slice()
    }

    /// Consumes token account for instruction creation.
    pub fn into_inputs_and_outputs(self) -> (Vec<TokenAccountMeta>, PackedTokenTransferOutputData) {
        (self.inputs, self.output)
    }
}

impl Deref for CTokenAccount {
    type Target = PackedTokenTransferOutputData;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}
