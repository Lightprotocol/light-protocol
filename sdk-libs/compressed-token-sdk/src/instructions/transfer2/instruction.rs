use light_compressed_token_types::{constants::TRANSFER2, ValidityProof};
use light_ctoken_types::{
    instructions::transfer2::{CompressedCpiContext, CompressedTokenInstructionDataTransfer2},
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_program_profiler::profile;
use solana_instruction::Instruction;
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

    pub fn with_cpi_context(mut self, cpi_context: CompressedCpiContext) -> Self {
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
    pub output_queue: u8,
}

/// Create the instruction for compressed token multi-transfer operations
#[profile]
pub fn create_transfer2_instruction(inputs: Transfer2Inputs) -> Result<Instruction> {
    let Transfer2Inputs {
        token_accounts,
        validity_proof,
        transfer_config,
        meta_config,
        in_lamports,
        out_lamports,
        output_queue,
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
        output_queue,
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
    let account_metas = get_transfer2_instruction_account_metas(meta_config);

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
