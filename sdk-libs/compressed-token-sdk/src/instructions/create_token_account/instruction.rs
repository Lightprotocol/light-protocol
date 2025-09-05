use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::error::Result;

/// Input parameters for creating a token account with compressible extension
#[derive(Debug, Clone)]
pub struct CreateCompressibleTokenAccount {
    /// The account to be created
    pub payer: Pubkey,
    /// The account to be created
    pub account_pubkey: Pubkey,
    /// The mint for the token account
    pub mint_pubkey: Pubkey,
    /// The owner of the token account
    pub owner_pubkey: Pubkey,
    /// The authority that can close this account (in addition to owner)
    pub rent_authority: Pubkey,
    /// The recipient of lamports when the account is closed by rent authority
    pub rent_recipient: Pubkey,
    /// Number of epochs of rent to prepay
    pub pre_pay_num_epochs: u64,
    /// Initial lamports to top up for rent payments (optional)
    pub write_top_up_lamports: Option<u32>,
    /// Bump seed for the pool PDA
    pub payer_pda_bump: u8,
}

pub fn create_compressible_token_account(
    inputs: CreateCompressibleTokenAccount,
) -> Result<Instruction> {
    // Format: [opcode, owner_pubkey, option_byte, CompressibleExtensionInstructionData]
    let mut data = Vec::new();
    data.push(18u8); // InitializeAccount3 opcode
    data.extend_from_slice(&inputs.owner_pubkey.to_bytes());
    data.push(1); // Some option byte for compressible_config

    // CompressibleExtensionInstructionData fields:
    data.extend_from_slice(&inputs.pre_pay_num_epochs.to_le_bytes()); // rent_payment in epochs
    data.push(1); // has_rent_authority = true
    data.extend_from_slice(&inputs.rent_authority.to_bytes());
    data.push(1); // has_rent_recipient = true
    data.extend_from_slice(&inputs.rent_recipient.to_bytes());

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
    data.push(inputs.payer_pda_bump); // payer_pda_bump
    let mut accounts = vec![
        solana_instruction::AccountMeta::new(inputs.account_pubkey, true),
        solana_instruction::AccountMeta::new_readonly(inputs.mint_pubkey, false),
    ];
    accounts.push(solana_instruction::AccountMeta::new(inputs.payer, true));

    accounts.push(solana_instruction::AccountMeta::new_readonly(
        Pubkey::default(),
        false,
    ));
    // pda that funds account creation
    accounts.push(solana_instruction::AccountMeta::new(
        inputs.rent_recipient,
        false,
    ));
    println!("accounts {:?}", accounts);
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
