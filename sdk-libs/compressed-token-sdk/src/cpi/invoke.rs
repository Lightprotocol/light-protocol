use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_compressed_token_types::{
    constants::{PROGRAM_ID as COMPRESSED_TOKEN_PROGRAM_ID, TRANSFER},
    cpi_accounts::CpiAccounts,
    instruction::transfer::CompressedTokenInstructionDataTransfer,
    CompressedCpiContext,
};
use solana_account_info::AccountInfo;
use solana_instruction::Instruction;
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    account::CTokenAccount,
    cpi::accounts::to_compressed_token_account_metas,
    error::{Result, TokenSdkError},
    AnchorSerialize,
};

/// CPI inputs for compressed token operations
#[derive(Debug, Clone, Default)]
pub struct CpiInputs {
    pub token_accounts: Vec<CTokenAccount>,
    pub validity_proof: ValidityProof,
    pub cpi_context: Option<CompressedCpiContext>,
    pub with_transaction_hash: bool,
}

impl CpiInputs {
    /// Create new CPI inputs for compress operation
    pub fn new_compress(token_accounts: Vec<CTokenAccount>) -> Self {
        Self {
            token_accounts,
            ..Default::default()
        }
    }

    /// Create new CPI inputs for transfer operation
    pub fn new(token_accounts: Vec<CTokenAccount>, validity_proof: ValidityProof) -> Self {
        Self {
            token_accounts,
            validity_proof,
            ..Default::default()
        }
    }
}

/// Create the instruction for compressed token operations
pub fn create_compressed_token_instruction(
    cpi_inputs: CpiInputs,
    cpi_accounts: &CpiAccounts<'_, AccountInfo>,
) -> Result<Instruction> {
    // Determine if this is a compress operation by checking any token account
    let is_compress = cpi_inputs
        .token_accounts
        .iter()
        .any(|acc| acc.is_compress());
    let is_decompress = cpi_inputs
        .token_accounts
        .iter()
        .any(|acc| acc.is_decompress());
    let mint = *cpi_inputs.token_accounts[0].mint();
    let mut compress_or_decompress_amount: Option<u64> = None;
    for acc in cpi_inputs.token_accounts.iter() {
        if let Some(amount) = acc.compression_amount() {
            if let Some(compress_or_decompress_amount) = compress_or_decompress_amount.as_mut() {
                (*compress_or_decompress_amount) += amount;
            } else {
                compress_or_decompress_amount = Some(amount);
            }
        }
    }

    // Check 1: cpi accounts must be decompress or compress consistent with accounts
    if (is_compress && !cpi_accounts.config().compress)
        || (is_decompress && !cpi_accounts.config().decompress)
    {
        return Err(TokenSdkError::InconsistentCompressDecompressState);
    }

    // Check 2: there can only be compress or decompress not both
    if is_compress && is_decompress {
        return Err(TokenSdkError::BothCompressAndDecompress);
    }

    // Check 3: compress_or_decompress_amount must be Some
    if compress_or_decompress_amount.is_none() && cpi_accounts.config().is_compress_or_decompress()
    {
        return Err(TokenSdkError::InvalidCompressDecompressAmount);
    }

    // Extract input and output data from token accounts
    let mut input_token_data_with_context = Vec::new();
    let mut output_compressed_accounts = Vec::new();

    for token_account in cpi_inputs.token_accounts {
        let (inputs, output) = token_account.into_inputs_and_outputs();
        input_token_data_with_context.extend(inputs);
        output_compressed_accounts.push(output);
    }
    msg!("inputs {:?}", input_token_data_with_context);
    msg!("outputs {:?}", output_compressed_accounts);
    msg!(
        "compress_or_decompress_amount {:?}",
        compress_or_decompress_amount
    );
    // Create instruction data
    let instruction_data = CompressedTokenInstructionDataTransfer {
        proof: cpi_inputs.validity_proof.into(),
        mint: mint.to_bytes(),
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress,
        compress_or_decompress_amount,
        cpi_context: cpi_inputs.cpi_context,
        with_transaction_hash: cpi_inputs.with_transaction_hash,
        delegated_transfer: None, // TODO: support in separate pr
        lamports_change_account_merkle_tree_index: None, // TODO: support in separate pr
    };
    msg!("instruction_data {:?}", instruction_data);
    // TODO: calculate exact len.
    let serialized = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Serialize instruction data
    let mut data = Vec::with_capacity(8 + 4 + serialized.len()); // rough estimate
    data.extend_from_slice(&TRANSFER);
    data.extend(u32::try_from(serialized.len()).unwrap().to_le_bytes());
    data.extend(serialized);

    let account_metas = to_compressed_token_account_metas(cpi_accounts)?;

    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data,
    })
}
