use light_compressed_token_types::{constants::TRANSFER2, CompressedCpiContext, ValidityProof};
use light_ctoken_types::{
    instructions::transfer2::{
        CompressedTokenInstructionDataTransfer2, Compression, CompressionMode,
        MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
    },
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::cpi::{CpiAccountsConfig, CpiSigner};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    account2::CTokenAccount2,
    error::{Result, TokenSdkError},
    instructions::transfer2::account_metas::{
        get_transfer2_instruction_account_metas, Transfer2AccountsMetaConfig,
    },
    AnchorSerialize,
};

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub struct Transfer2Config {
    pub cpi_context_pubkey: Option<Pubkey>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub with_transaction_hash: bool,
    pub sol_pool_pda: bool,
    pub sol_decompression_recipient: Option<Pubkey>,
    pub filter_zero_amount_outputs: bool,
}

impl Transfer2Config {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_cpi_context(
        mut self,
        cpi_context_pubkey: Pubkey,
        cpi_context: CompressedCpiContext,
    ) -> Self {
        self.cpi_context_pubkey = Some(cpi_context_pubkey);
        self.cpi_context = Some(cpi_context);
        self
    }

    pub fn with_transaction_hash(mut self) -> Self {
        self.with_transaction_hash = true;
        self
    }

    pub fn with_sol_pool(mut self, sol_decompression_recipient: Pubkey) -> Self {
        self.sol_pool_pda = true;
        self.sol_decompression_recipient = Some(sol_decompression_recipient);
        self
    }

    pub fn filter_zero_amount_outputs(mut self) -> Self {
        self.filter_zero_amount_outputs = true;
        self
    }
}

/// Multi-transfer input parameters
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transfer2Inputs {
    pub token_accounts: Vec<CTokenAccount2>,
    pub validity_proof: ValidityProof,
    pub transfer_config: Transfer2Config,
    pub meta_config: Transfer2AccountsMetaConfig,
    // pub tree_pubkeys: Vec<Pubkey>,
    // pub packed_pubkeys: Vec<Pubkey>, // Owners, Delegates, Mints
    pub in_lamports: Option<Vec<u64>>,
    pub out_lamports: Option<Vec<u64>>,
}

pub fn create_spl_to_ctoken_transfer_instruction(
    source: u8,
    destination: u8,
    amount: u64,
    authority: Pubkey,
    mint: u8,
    payer: Pubkey,
    packed_accounts: CpiAccountsSmall,
    cpi_signer: CpiSigner,
) -> Result<Instruction> {
    let cpi_accounts = CpiAccountsSmall::new_with_config(
        &payer,
        packed_accounts,
        CpiAccountsConfig::new(cpi_signer),
    );
    // every value in Transfer2Inputs is default or none or 0 except for token_accounts
    // so we can just create the instruction with the given values

    let ctoken_account = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData {
            owner: destination, // recipeitn of output
            amount: 0,
            merkle_tree: 0,
            delegate: 0,
            mint: 0,
            version: 0,
        },
        compression: Some(Compression {
            amount,
            mode: CompressionMode::Compress,
            mint,
            source_or_recipient: source, // index of account source
            authority: 0,
            pool_account_index: 0,
            pool_index: 0,
            bump: 0,
        }),
        delegate_is_set: false,
        method_used: true,
    };

    let transfer2_inputs = Transfer2Inputs {
        token_accounts: vec![ctoken_account],
        validity_proof: ValidityProof::default(),
        transfer_config: Transfer2Config::default(),
        meta_config: Transfer2AccountsMetaConfig::default(),
        in_lamports: None,
        out_lamports: None,
    };

    let instruction = Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: vec![3u8, 3u8, amount.to_le_bytes()],
    };

    Ok(instruction)
}

/// Create the instruction for compressed token multi-transfer operations
pub fn create_transfer2_instruction(inputs: Transfer2Inputs) -> Result<Instruction> {
    let Transfer2Inputs {
        token_accounts,
        validity_proof,
        transfer_config,
        meta_config,
        in_lamports,
        out_lamports,
    } = inputs;
    let mut input_token_data_with_context = Vec::new();
    let mut output_compressed_accounts = Vec::new();
    let mut collected_compressions = Vec::new();

    // Process each token account and convert to multi-transfer format
    for token_account in token_accounts {
        // Collect compression if present
        if let Some(compression) = token_account.compression() {
            collected_compressions.push(*compression);
        }
        let (inputs, output) = token_account.into_inputs_and_outputs();

        // Collect inputs directly (they're already in the right format)
        input_token_data_with_context.extend(inputs);

        // Add output if not zero amount (when filtering is enabled)
        if !transfer_config.filter_zero_amount_outputs || output.amount > 0 {
            output_compressed_accounts.push(output);
        }
    }

    // Create instruction data
    let instruction_data = CompressedTokenInstructionDataTransfer2 {
        with_transaction_hash: transfer_config.with_transaction_hash,
        with_lamports_change_account_merkle_tree_index: false, // TODO: support in future
        lamports_change_account_merkle_tree_index: 0,
        lamports_change_account_owner_index: 0,
        proof: validity_proof.into(),
        in_token_data: input_token_data_with_context,
        out_token_data: output_compressed_accounts,
        in_lamports,
        out_lamports,
        in_tlv: None,  // TLV is unimplemented
        out_tlv: None, // TLV is unimplemented
        compressions: if collected_compressions.is_empty() {
            None
        } else {
            Some(collected_compressions)
        },
        cpi_context: transfer_config.cpi_context,
    };

    // Serialize instruction data
    let serialized = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Build instruction data with discriminator
    let mut data = Vec::with_capacity(1 + serialized.len());
    data.push(TRANSFER2);
    data.extend(serialized);

    // Get account metas
    let mut account_metas = get_transfer2_instruction_account_metas(meta_config);

    // Add CPI context account if configured
    if let Some(cpi_context_pubkey) = transfer_config.cpi_context_pubkey {
        if transfer_config.cpi_context.is_some() {
            account_metas.push(AccountMeta::new(cpi_context_pubkey, false));
        }
    }

    // Moved assignment to account meta config
    // Add tree accounts first
    //for tree_pubkey in tree_pubkeys {
    //     account_metas.push(AccountMeta::new(tree_pubkey, false));
    // }
    // Add packed accounts second
    // for packed_pubkey in packed_pubkeys {
    //     account_metas.push(AccountMeta::new(packed_pubkey, false));
    // }

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data,
    })
}

/*
/// Create a multi-transfer instruction
pub fn transfer2(inputs: create_transfer2_instruction) -> Result<Instruction> {
    let create_transfer2_instruction {
        fee_payer,
        authority,
        validity_proof,
        token_accounts,
        tree_pubkeys,
        config,
    } = inputs;

    // Validate that no token account has been used
    for token_account in &token_accounts {
        if token_account.method_used {
            return Err(TokenSdkError::MethodUsed);
        }
    }

    let config = config.unwrap_or_default();
    let meta_config = Transfer2AccountsMetaConfig::new(fee_payer, authority)
        .with_sol_pool(
            config.sol_pool_pda.unwrap_or_default(),
            config.sol_decompression_recipient.unwrap_or_default(),
        )
        .with_cpi_context();

    create_transfer2_instruction(
        token_accounts,
        validity_proof,
        config,
        meta_config,
        tree_pubkeys,
    )
}
*/
