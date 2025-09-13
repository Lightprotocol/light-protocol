use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::Result;

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
    pub rent_recipient: Pubkey,
    /// Number of epochs of rent to prepay
    pub pre_pay_num_epochs: u64,
    /// Initial lamports to top up for rent payments (optional)
    pub write_top_up_lamports: Option<u32>,
}

pub fn create_compressible_token_account(
    inputs: CreateCompressibleTokenAccount,
) -> Result<Instruction> {
    // Format: [opcode, owner_pubkey, option_byte, CompressibleExtensionInstructionData]
    let mut data = Vec::new();
    data.push(18u8); // InitializeAccount3 opcode
    data.extend_from_slice(&inputs.owner_pubkey.to_bytes());
    data.push(1); // Some option byte for compressible_config

    // CompressibleExtensionInstructionData fields (simplified - no rent_authority/recipient)
    data.extend_from_slice(&inputs.pre_pay_num_epochs.to_le_bytes()); // rent_payment in epochs

    // Handle write_top_up_lamports
    match inputs.write_top_up_lamports {
        Some(lamports) => {
            data.push(1); // has_top_up = true
            data.extend_from_slice(&lamports.to_le_bytes());
        }
        None => {
            data.push(0); // has_top_up = false
            data.extend_from_slice(&0u32.to_le_bytes()); // write_top_up = 0
        }
    }

    // Account order based on processor:
    // 1. token_account (signer)
    // 2. mint
    // 3. payer (signer)
    // 4. compressible_config
    // 5. system_program
    // 6. fee_payer_pda (rent_recipient)
    let accounts = vec![
        solana_instruction::AccountMeta::new(inputs.account_pubkey, true),
        solana_instruction::AccountMeta::new_readonly(inputs.mint_pubkey, false),
        solana_instruction::AccountMeta::new(inputs.payer, true),
        solana_instruction::AccountMeta::new_readonly(inputs.compressible_config, false),
        solana_instruction::AccountMeta::new_readonly(Pubkey::default(), false),
        solana_instruction::AccountMeta::new(inputs.rent_recipient, false), // fee_payer_pda
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
    // Create InitializeAccount3 instruction data manually
    // Format: [18, owner_pubkey_32_bytes, 0]
    let mut data = Vec::with_capacity(1 + 32);
    data.push(18u8); // InitializeAccount3 opcode
    data.extend_from_slice(&owner_pubkey.to_bytes());

    Ok(Instruction {
        program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: vec![
            solana_instruction::AccountMeta::new(account_pubkey, false),
            solana_instruction::AccountMeta::new_readonly(mint_pubkey, false),
        ],
        data,
    })
}
