use std::ops::Deref;

use crate::error::CTokenSdkError;
use light_compressed_token_types::{InputTokenDataWithContext, PackedTokenTransferOutputData};
use solana_pubkey::Pubkey;

/// Compress, decompress, new
/// Questions:
/// 1. do we need to implement compress?
#[derive(Debug, PartialEq)]
pub struct CTokenAccount {
    inputs: Vec<InputTokenDataWithContext>,
    output: PackedTokenTransferOutputData,
    compression_amount: Option<u64>,
    is_compress: bool,
    is_decompress: bool,
}

impl CTokenAccount {
    pub fn new(
        owner: Pubkey,
        token_data: Vec<InputTokenDataWithContext>,
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
        }
    }

    pub fn new_empty(owner: Pubkey, output_merkle_tree_index: u8) -> Self {
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
        }
    }

    pub fn transfer(
        &mut self,
        recipient: &Pubkey,
        amount: u64,
        output_merkle_tree_index: Option<u8>,
    ) -> Result<Self, CTokenSdkError> {
        if amount > self.output.amount {
            return Err(CTokenSdkError::InsufficientBalance);
        }
        // TODO: skip outputs with zero amount when creating the instruction data.
        self.output.amount -= amount;
        let merkle_tree_index = output_merkle_tree_index.unwrap_or(self.output.merkle_tree_index);

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
        })
    }

    pub fn compress(&mut self, amount: u64) -> Result<(), CTokenSdkError> {
        self.output.amount += amount;
        self.is_compress = true;

        match self.compression_amount.as_mut() {
            Some(amount_ref) => *amount_ref += amount,
            None => self.compression_amount = Some(amount),
        }
        Ok(())
    }

    pub fn decompress(&mut self, amount: u64) -> Result<(), CTokenSdkError> {
        self.output.amount -= amount;
        self.is_decompress = true;

        match self.compression_amount.as_mut() {
            Some(amount_ref) => *amount_ref -= amount,
            None => self.compression_amount = Some(amount),
        }
        Ok(())
    }

    /// Consumes token account for instruction creation.
    pub fn into_inputs_and_outputs(
        self,
    ) -> (
        Vec<InputTokenDataWithContext>,
        PackedTokenTransferOutputData,
    ) {
        (self.inputs, self.output)
    }

    //     /// 1. Serializes the account data and sets the output data hash.
    //     /// 2. Returns CompressedAccountInfo.
    //     ///
    //     /// Note this is an expensive operation
    //     /// that should only be called once per instruction.
    //     pub fn to_account_info(mut self) -> Result<CompressedAccountInfo, LightSdkError> {
    //         if let Some(output) = self.account_info.output.as_mut() {
    //             output.data_hash = self.account.hash::<Poseidon>()?;
    //             output.data = self
    //                 .account
    //                 .try_to_vec()
    //                 .map_err(|_| LightSdkError::Borsh)?;
    //         }
    //         Ok(self.account_info)
    //     }
    // }
}

impl Deref for CTokenAccount {
    type Target = PackedTokenTransferOutputData;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}
