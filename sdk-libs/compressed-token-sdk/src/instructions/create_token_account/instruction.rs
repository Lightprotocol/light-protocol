use light_ctoken_types::CTokenError;
use solana_account_info::AccountInfo;
use solana_cpi::invoke_signed;
use solana_instruction::Instruction;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::error::{Result, TokenSdkError};

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

#[derive(Debug, Clone)]
pub struct CreateCompressibleTokenAccountSigned<'info> {
    /// The account to be created
    pub payer: AccountInfo<'info>,
    /// The account to be created
    pub token_account: AccountInfo<'info>,
    /// The mint for the token account
    pub mint: AccountInfo<'info>,
    /// The owner of the token account
    pub owner: AccountInfo<'info>,
    /// The authority that can close this account (in addition to owner)
    pub rent_authority: AccountInfo<'info>,
    /// The recipient of lamports when the account is closed by rent authority
    pub rent_recipient: AccountInfo<'info>,
    /// Number of epochs of rent to prepay
    pub pre_pay_num_epochs: u64,
    /// Initial lamports to top up for rent payments (optional)
    pub write_top_up_lamports: Option<u32>,
    /// Bump seed for the pool PDA
    pub payer_pda_bump: u8,

    // Owned seeds
    pub signer_seeds: Vec<Vec<Vec<u8>>>,
}
pub fn create_compressible_token_account_signed<'info>(
    inputs: CreateCompressibleTokenAccountSigned<'info>,
) -> std::result::Result<(), solana_program_error::ProgramError> {
    let params = CreateCompressibleTokenAccount {
        payer: *inputs.payer.key,
        account_pubkey: *inputs.token_account.key,
        mint_pubkey: *inputs.mint.key,
        owner_pubkey: *inputs.owner.key,
        rent_authority: *inputs.rent_authority.key,
        rent_recipient: *inputs.rent_recipient.key,
        pre_pay_num_epochs: inputs.pre_pay_num_epochs,
        write_top_up_lamports: inputs.write_top_up_lamports,
        payer_pda_bump: inputs.payer_pda_bump,
    };
    let ix = create_compressible_token_account(params)
        .map_err(|_| TokenSdkError::CTokenError(CTokenError::InvalidInstructionData))?;
    let account_infos = vec![
        inputs.payer,
        inputs.token_account,
        inputs.mint,
        inputs.owner,
        inputs.rent_authority,
        inputs.rent_recipient,
    ];
    // Convert owned Vec<Vec<Vec<u8>>> —> &[&[&[u8]]]
    let seed_refs_level1: Vec<Vec<&[u8]>> = inputs
        .signer_seeds
        .iter()
        .map(|inner| inner.iter().map(|v| v.as_slice()).collect())
        .collect();
    let seed_refs_level2: Vec<&[&[u8]]> = seed_refs_level1.iter().map(|v| v.as_slice()).collect();

    invoke_signed(&ix, &account_infos, seed_refs_level2.as_slice())
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
