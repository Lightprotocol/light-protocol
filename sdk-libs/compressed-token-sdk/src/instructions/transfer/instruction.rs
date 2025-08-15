use light_compressed_token_types::{
    constants::TRANSFER, instruction::transfer::CompressedTokenInstructionDataTransfer,
    CompressedCpiContext, ValidityProof,
};
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    account::CTokenAccount,
    error::{Result, TokenSdkError},
    instructions::transfer::account_metas::{
        get_transfer_instruction_account_metas, TokenAccountsMetaConfig,
    },
    AnchorSerialize,
};
// CTokenAccount abstraction to bundle inputs and create outputs.
// Users don't really need to interact with this struct directly.
// Counter point for an anchor like TokenAccount we need the CTokenAccount
//
// Rename TokenAccountMeta -> TokenAccountMeta
//

// We should have a create instruction function that works onchain and offchain.
// - account infos don't belong into the create instruction function.
// One difference between spl and compressed token program is that you don't want to make a separate cpi per transfer.
// -> transfer(from, to, amount) doesn't work well
//    -
// -> compress(token_account, Option<amount>) could be compressed token account
// -> decompress()
// TODO:
// - test decompress and compress in the same instruction

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub struct TransferConfig {
    pub cpi_context_pubkey: Option<Pubkey>,
    pub cpi_context: Option<CompressedCpiContext>,
    pub with_transaction_hash: bool,
    pub filter_zero_amount_outputs: bool,
}

/// Create instruction function should only take Pubkeys as inputs not account infos.
///
/// Create the instruction for compressed token operations
pub fn create_transfer_instruction_raw(
    mint: Pubkey,
    token_accounts: Vec<CTokenAccount>,
    validity_proof: ValidityProof,
    transfer_config: TransferConfig,
    meta_config: TokenAccountsMetaConfig,
    tree_pubkeys: Vec<Pubkey>,
) -> Result<Instruction> {
    // Determine if this is a compress operation by checking any token account
    let is_compress = token_accounts.iter().any(|acc| acc.is_compress());
    let is_decompress = token_accounts.iter().any(|acc| acc.is_decompress());

    let mut compress_or_decompress_amount: Option<u64> = None;
    for acc in token_accounts.iter() {
        if let Some(amount) = acc.compression_amount() {
            if let Some(compress_or_decompress_amount) = compress_or_decompress_amount.as_mut() {
                (*compress_or_decompress_amount) += amount;
            } else {
                compress_or_decompress_amount = Some(amount);
            }
        }
    }

    // Check 1: cpi accounts must be decompress or compress consistent with accounts
    if (is_compress && !meta_config.is_compress) || (is_decompress && !meta_config.is_decompress) {
        return Err(TokenSdkError::InconsistentCompressDecompressState);
    }

    // Check 2: there can only be compress or decompress not both
    if is_compress && is_decompress {
        return Err(TokenSdkError::BothCompressAndDecompress);
    }

    // Check 3: compress_or_decompress_amount must be Some
    if compress_or_decompress_amount.is_none() && meta_config.is_compress_or_decompress() {
        return Err(TokenSdkError::InvalidCompressDecompressAmount);
    }

    // Extract input and output data from token accounts
    let mut input_token_data_with_context = Vec::new();
    let mut output_compressed_accounts = Vec::new();

    for token_account in token_accounts {
        let (inputs, output) = token_account.into_inputs_and_outputs();
        for input in inputs {
            input_token_data_with_context.push(input.into());
        }
        if output.amount == 0 && transfer_config.filter_zero_amount_outputs {
        } else {
            output_compressed_accounts.push(output);
        }
    }

    // Create instruction data
    let instruction_data = CompressedTokenInstructionDataTransfer {
        proof: validity_proof.into(),
        mint: mint.to_bytes(),
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress,
        compress_or_decompress_amount,
        cpi_context: transfer_config.cpi_context,
        with_transaction_hash: transfer_config.with_transaction_hash,
        delegated_transfer: None, // TODO: support in separate pr
        lamports_change_account_merkle_tree_index: None, // TODO: support in separate pr
    };

    // TODO: calculate exact len.
    let serialized = instruction_data
        .try_to_vec()
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Serialize instruction data
    let mut data = Vec::with_capacity(8 + 4 + serialized.len()); // rough estimate
    data.extend_from_slice(&TRANSFER);
    data.extend(u32::try_from(serialized.len()).unwrap().to_le_bytes());
    data.extend(serialized);
    let mut account_metas = get_transfer_instruction_account_metas(meta_config);
    if let Some(cpi_context_pubkey) = transfer_config.cpi_context_pubkey {
        if transfer_config.cpi_context.is_some() {
            account_metas.push(AccountMeta::new(cpi_context_pubkey, false));
        } else {
            // TODO: throw error
            panic!("cpi_context.is_none() but transfer_config.cpi_context_pubkey is some");
        }
    }

    // let account_metas = to_compressed_token_account_metas(cpi_accounts)?;
    for tree_pubkey in tree_pubkeys {
        account_metas.push(AccountMeta::new(tree_pubkey, false));
    }
    Ok(Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data,
    })
}

pub struct CompressInputs {
    pub fee_payer: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub recipient: Pubkey,
    pub output_tree_index: u8,
    pub sender_token_account: Pubkey,
    pub amount: u64,
    // pub output_queue_pubkey: Pubkey,
    pub token_pool_pda: Pubkey,
    pub transfer_config: Option<TransferConfig>,
    pub spl_token_program: Pubkey,
    pub tree_accounts: Vec<Pubkey>,
}

// TODO: consider adding compress to existing token accounts
//      (effectively compress and merge)
// TODO: wrap batch compress instead.
pub fn compress(inputs: CompressInputs) -> Result<Instruction> {
    let CompressInputs {
        fee_payer,
        authority,
        mint,
        recipient,
        sender_token_account,
        amount,
        token_pool_pda,
        transfer_config,
        spl_token_program,
        output_tree_index,
        tree_accounts,
    } = inputs;
    let mut token_account =
        crate::account::CTokenAccount::new_empty(mint, recipient, output_tree_index);
    token_account.compress(amount).unwrap();
    solana_msg::msg!("spl_token_program {:?}", spl_token_program);
    let config = transfer_config.unwrap_or_default();
    let meta_config = TokenAccountsMetaConfig::compress(
        fee_payer,
        authority,
        token_pool_pda,
        sender_token_account,
        spl_token_program,
    );
    create_transfer_instruction_raw(
        mint,
        vec![token_account],
        ValidityProof::default(),
        config,
        meta_config,
        tree_accounts,
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferInputs {
    pub fee_payer: Pubkey,
    pub validity_proof: ValidityProof,
    pub sender_account: CTokenAccount,
    pub amount: u64,
    pub recipient: Pubkey,
    pub tree_pubkeys: Vec<Pubkey>,
    pub config: Option<TransferConfig>,
}

pub fn transfer(inputs: TransferInputs) -> Result<Instruction> {
    let TransferInputs {
        fee_payer,
        validity_proof,
        amount,
        mut sender_account,
        recipient,
        tree_pubkeys,
        config,
    } = inputs;
    // Sanity check.
    if sender_account.method_used {
        return Err(TokenSdkError::MethodUsed);
    }
    let account_meta_config = TokenAccountsMetaConfig::new(fee_payer, sender_account.owner());
    // None is the same output_tree_index as token account
    let recipient_token_account = sender_account.transfer(&recipient, amount, None).unwrap();

    create_transfer_instruction_raw(
        *sender_account.mint(),
        vec![recipient_token_account, sender_account],
        validity_proof,
        config.unwrap_or_default(),
        account_meta_config,
        tree_pubkeys,
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecompressInputs {
    pub fee_payer: Pubkey,
    pub validity_proof: ValidityProof,
    pub sender_account: CTokenAccount,
    pub amount: u64,
    pub tree_pubkeys: Vec<Pubkey>,
    pub config: Option<TransferConfig>,
    pub token_pool_pda: Pubkey,
    pub recipient_token_account: Pubkey,
    pub spl_token_program: Pubkey,
}

pub fn decompress(inputs: DecompressInputs) -> Result<Instruction> {
    let DecompressInputs {
        amount,
        fee_payer,
        validity_proof,
        mut sender_account,
        tree_pubkeys,
        config,
        token_pool_pda,
        recipient_token_account,
        spl_token_program,
    } = inputs;
    // Sanity check.
    if sender_account.method_used {
        return Err(TokenSdkError::MethodUsed);
    }
    let account_meta_config = TokenAccountsMetaConfig::decompress(
        fee_payer,
        sender_account.owner(),
        token_pool_pda,
        recipient_token_account,
        spl_token_program,
    );
    sender_account.decompress(amount).unwrap();

    create_transfer_instruction_raw(
        *sender_account.mint(),
        vec![sender_account],
        validity_proof,
        config.unwrap_or_default(),
        account_meta_config,
        tree_pubkeys,
    )
}
