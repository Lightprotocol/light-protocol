use std::ops::Deref;

use light_compressed_token_types::ValidityProof;
use light_ctoken_types::instructions::transfer2::{
    Compression, CompressionMode, MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
};
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    error::TokenSdkError,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
    utils::get_token_account_balance,
};

#[derive(Debug, PartialEq, Clone)]
pub struct CTokenAccount2 {
    pub inputs: Vec<MultiInputTokenDataWithContext>,
    pub output: MultiTokenTransferOutputData,
    pub compression: Option<Compression>,
    pub delegate_is_set: bool,
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
        authority: u8,
    ) -> Result<(), TokenSdkError> {
        // Check if there's already a compression set
        if self.compression.is_some() {
            return Err(TokenSdkError::CompressionCannotBeSetTwice);
        }

        self.output.amount += amount;
        self.compression = Some(Compression::compress(
            amount,
            self.output.mint,
            source_or_recipient_index,
            authority,
        ));
        self.method_used = true;

        Ok(())
    }

    pub fn compress_spl(
        &mut self,
        amount: u64,
        source_or_recipient_index: u8,
        authority: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
    ) -> Result<(), TokenSdkError> {
        // Check if there's already a compression set
        if self.compression.is_some() {
            return Err(TokenSdkError::CompressionCannotBeSetTwice);
        }

        self.output.amount += amount;
        self.compression = Some(Compression::compress_spl(
            amount,
            self.output.mint,
            source_or_recipient_index,
            authority,
            pool_account_index,
            pool_index,
            bump,
        ));
        self.method_used = true;

        Ok(())
    }

    // TODO: consider this might be confusing because it must not be used in combination with fn decompress()
    pub fn decompress(&mut self, amount: u64, source_index: u8) -> Result<(), TokenSdkError> {
        // Check if there's already a compression set
        if self.compression.is_some() {
            return Err(TokenSdkError::CompressionCannotBeSetTwice);
        }

        if self.output.amount < amount {
            return Err(TokenSdkError::InsufficientBalance);
        }
        self.output.amount -= amount;

        self.compression = Some(Compression::decompress(
            amount,
            self.output.mint,
            source_index,
        ));
        self.method_used = true;

        Ok(())
    }

    pub fn decompress_spl(
        &mut self,
        amount: u64,
        source_index: u8,
        pool_account_index: u8,
        pool_index: u8,
        bump: u8,
    ) -> Result<(), TokenSdkError> {
        // Check if there's already a compression set
        if self.compression.is_some() {
            return Err(TokenSdkError::CompressionCannotBeSetTwice);
        }

        if self.output.amount < amount {
            return Err(TokenSdkError::InsufficientBalance);
        }
        self.output.amount -= amount;

        self.compression = Some(Compression::decompress_spl(
            amount,
            self.output.mint,
            source_index,
            pool_account_index,
            pool_index,
            bump,
        ));
        self.method_used = true;

        Ok(())
    }

    pub fn compress_full(
        &mut self,
        source_or_recipient_index: u8,
        authority: u8,
        token_account_info: &AccountInfo,
    ) -> Result<(), TokenSdkError> {
        // Check if there's already a compression set
        if self.compression.is_some() {
            return Err(TokenSdkError::CompressionCannotBeSetTwice);
        }

        // Get the actual token account balance to add to output
        let token_balance = get_token_account_balance(token_account_info)?;

        // Add the full token balance to the output amount
        self.output.amount += token_balance;

        // For compress_full, set amount to the actual balance for instruction data
        self.compression = Some(Compression {
            amount: token_balance,
            mode: CompressionMode::Compress, // Use regular compress mode with actual amount
            mint: self.output.mint,
            source_or_recipient: source_or_recipient_index,
            authority,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
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

pub fn create_spl_to_ctoken_transfer_instruction(
    source_spl_token_account: Pubkey,
    to: Pubkey,
    amount: u64,
    authority: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    token_pool_pda: Pubkey,
    token_pool_pda_bump: u8,
) -> Result<Instruction, TokenSdkError> {
    let mut packed_accounts = Vec::with_capacity(6);

    // Mint (index 0)
    packed_accounts.push(AccountMeta::new_readonly(mint, false));

    // Destination token account (index 1)
    packed_accounts.push(AccountMeta::new(to, false));

    // Authority for compression (index 2) - signer
    packed_accounts.push(AccountMeta::new_readonly(authority, true));

    // Source SPL token account (index 3) - writable
    packed_accounts.push(AccountMeta::new(source_spl_token_account, false));

    // Token pool PDA (index 4) - writable
    packed_accounts.push(AccountMeta::new(token_pool_pda, false));

    // SPL Token program (index 5) - needed for CPI
    packed_accounts.push(AccountMeta::new_readonly(
        Pubkey::from(light_compressed_token_types::constants::SPL_TOKEN_PROGRAM_ID),
        false,
    ));

    let wrap_spl_to_ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::compress_spl(
            amount,
            0, // mint
            3, // source or recpient
            2, // authority
            4, //
            0,
            token_pool_pda_bump,
        )),
        delegate_is_set: false,
        method_used: true,
    };

    let ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::decompress_ctoken(amount, 0, 1)),
        delegate_is_set: false,
        method_used: true,
    };

    // Create Transfer2Inputs following the test pattern
    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(packed_accounts),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![wrap_spl_to_ctoken_account, ctoken_account],
    };

    // Create the actual transfer2 instruction
    create_transfer2_instruction(inputs)
}

pub fn create_ctoken_to_spl_transfer_instruction(
    source_ctoken_account: Pubkey,
    destination_spl_token_account: Pubkey,
    amount: u64,
    authority: Pubkey,
    mint: Pubkey,
    payer: Pubkey,
    token_pool_pda: Pubkey,
    token_pool_pda_bump: u8,
) -> Result<Instruction, TokenSdkError> {
    let mut packed_accounts = Vec::with_capacity(6);

    // Mint (index 0)
    packed_accounts.push(AccountMeta::new_readonly(mint, false));

    // Source ctoken account (index 1) - writable
    packed_accounts.push(AccountMeta::new(source_ctoken_account, false));

    // Destination SPL token account (index 2) - writable
    packed_accounts.push(AccountMeta::new(destination_spl_token_account, false));

    // Authority (index 3) - signer
    packed_accounts.push(AccountMeta::new_readonly(authority, true));

    // Token pool PDA (index 4) - writable
    packed_accounts.push(AccountMeta::new(token_pool_pda, false));

    // SPL Token program (index 5) - needed for CPI
    packed_accounts.push(AccountMeta::new_readonly(
        Pubkey::from(light_compressed_token_types::constants::SPL_TOKEN_PROGRAM_ID),
        false,
    ));

    // First operation: compress from ctoken account to pool using compress_spl
    let compress_to_pool = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::compress_ctoken(
            amount, 0, // mint index
            1, // source ctoken account index
            3, // authority index
        )),
        delegate_is_set: false,
        method_used: true,
    };

    // Second operation: decompress from pool to SPL token account using decompress_spl
    let decompress_to_spl = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::decompress_spl(
            amount,
            0, // mint index
            2, // destination SPL token account index
            4, // pool_account_index
            0, // pool_index (TODO: make dynamic)
            token_pool_pda_bump,
        )),
        delegate_is_set: false,
        method_used: true,
    };

    // Create Transfer2Inputs
    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(packed_accounts),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![compress_to_pool, decompress_to_spl],
    };

    // Create the actual transfer2 instruction
    create_transfer2_instruction(inputs)
}
