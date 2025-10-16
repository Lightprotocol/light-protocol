use borsh::BorshSerialize;
use light_ctoken_types::{
    instructions::{
        create_ctoken_account::CreateTokenAccountInstructionData,
        extensions::compressible::{CompressToPubkey, CompressibleExtensionInstructionData},
    },
    state::TokenDataVersion,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::{Result, TokenSdkError};

/// Input parameters for creating a token account with compressible extension
#[derive(Debug, Clone)]
pub struct CreateCompressibleTokenAccount {
    pub payer: Pubkey,
    /// The account to be created
    pub account_pubkey: Pubkey,
    /// The mint for the token account
    pub mint_pubkey: Pubkey,
    /// The owner of the token account
    pub owner_pubkey: Pubkey,
    /// The CompressibleConfig account
    pub compressible_config: Pubkey,
    /// The rent recipient PDA (fee_payer_pda in processor)
    pub rent_sponsor: Pubkey,
    /// Number of epochs of rent to prepay
    pub pre_pay_num_epochs: u8,
    /// Initial lamports to top up for rent payments (optional)
    pub lamports_per_write: Option<u32>,
    pub compress_to_account_pubkey: Option<CompressToPubkey>,
    /// Version of the compressed token account when ctoken account is
    /// compressed and closed. (The version specifies the hashing scheme.)
    pub token_account_version: TokenDataVersion,
}

pub fn create_compressible_token_account(
    inputs: CreateCompressibleTokenAccount,
) -> Result<Instruction> {
    // Create the CompressibleExtensionInstructionData
    let compressible_extension = CompressibleExtensionInstructionData {
        token_account_version: inputs.token_account_version as u8,
        rent_payment: inputs.pre_pay_num_epochs,
        has_top_up: if inputs.lamports_per_write.is_some() {
            1
        } else {
            0
        },
        write_top_up: inputs.lamports_per_write.unwrap_or(0),
        compress_to_account_pubkey: inputs.compress_to_account_pubkey, // Not used for regular create_token_account
    };

    // Create the instruction data struct
    let instruction_data = CreateTokenAccountInstructionData {
        owner: light_compressed_account::Pubkey::from(inputs.owner_pubkey.to_bytes()),
        compressible_config: Some(compressible_extension),
    };

    // Serialize with Borsh
    let mut data = Vec::new();
    data.push(18u8); // InitializeAccount3 opcode
    instruction_data
        .serialize(&mut data)
        .map_err(|_| TokenSdkError::SerializationError)?;

    // Account order based on processor:
    // 1. token_account (signer)
    // 2. mint
    // 3. payer (signer)
    // 4. compressible_config
    // 5. system_program
    // 6. fee_payer_pda (rent_sponsor)
    let accounts = vec![
        solana_instruction::AccountMeta::new(inputs.account_pubkey, true),
        solana_instruction::AccountMeta::new_readonly(inputs.mint_pubkey, false),
        solana_instruction::AccountMeta::new(inputs.payer, true),
        solana_instruction::AccountMeta::new_readonly(inputs.compressible_config, false),
        solana_instruction::AccountMeta::new_readonly(Pubkey::default(), false),
        solana_instruction::AccountMeta::new(inputs.rent_sponsor, false), // fee_payer_pda
    ];

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data,
    })
}

pub fn create_token_account(
    account_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
) -> Result<Instruction> {
    // Create the instruction data struct without compressible config
    let instruction_data = CreateTokenAccountInstructionData {
        owner: light_compressed_account::Pubkey::from(owner_pubkey.to_bytes()),
        compressible_config: None,
    };

    // Serialize with Borsh
    let mut data = Vec::new();
    data.push(18u8); // InitializeAccount3 opcode
    instruction_data
        .serialize(&mut data)
        .map_err(|_| TokenSdkError::SerializationError)?;

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(account_pubkey, false),
            solana_instruction::AccountMeta::new_readonly(mint_pubkey, false),
        ],
        data,
    })
}
