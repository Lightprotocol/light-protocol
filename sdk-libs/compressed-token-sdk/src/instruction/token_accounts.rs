use light_compressed_token_types::{
    ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA, LIGHT_SYSTEM_PROGRAM_ID, NOOP_PROGRAM_ID,
    PROGRAM_ID as LIGHT_COMPRESSED_TOKEN_PROGRAM_ID, SPL_TOKEN_2022_PROGRAM_ID,
    SPL_TOKEN_PROGRAM_ID,
};
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

/// Account metadata configuration for compressed token instructions
#[derive(Debug, Default, Copy, Clone)]
pub struct TokenAccountsMetaConfig {
    pub fee_payer: Pubkey,
    pub authority: Pubkey,
    pub token_pool_pda: Option<Pubkey>,
    pub compress_or_decompress_token_account: Option<Pubkey>,
    pub token_program: Option<Pubkey>,
}

impl TokenAccountsMetaConfig {
    pub fn new(fee_payer: Pubkey, authority: Pubkey) -> Self {
        Self {
            fee_payer,
            authority,
            token_pool_pda: None,
            compress_or_decompress_token_account: None,
            token_program: None,
        }
    }

    pub fn compress(
        fee_payer: Pubkey,
        authority: Pubkey,
        token_pool_pda: Pubkey,
        sender_token_account: Pubkey,
        is_token22: bool,
    ) -> Self {
        Self {
            fee_payer,
            authority,
            token_pool_pda: Some(token_pool_pda),
            compress_or_decompress_token_account: Some(sender_token_account),
            token_program: Some(if is_token22 {
                Pubkey::from(SPL_TOKEN_2022_PROGRAM_ID)
            } else {
                Pubkey::from(SPL_TOKEN_PROGRAM_ID)
            }),
        }
    }

    pub fn decompress(
        fee_payer: Pubkey,
        authority: Pubkey,
        token_pool_pda: Pubkey,
        recipient_token_account: Pubkey,
        is_token22: bool,
    ) -> Self {
        Self {
            fee_payer,
            authority,
            token_pool_pda: Some(token_pool_pda),
            compress_or_decompress_token_account: Some(recipient_token_account),
            token_program: Some(if is_token22 {
                Pubkey::from(SPL_TOKEN_2022_PROGRAM_ID)
            } else {
                Pubkey::from(SPL_TOKEN_PROGRAM_ID)
            }),
        }
    }
}

/// Standard pubkeys for compressed token instructions
#[derive(Debug, Copy, Clone)]
pub struct TokenAccountPubkeys {
    pub light_system_program: Pubkey,
    pub registered_program_pda: Pubkey,
    pub noop_program: Pubkey,
    pub account_compression_authority: Pubkey,
    pub account_compression_program: Pubkey,
    pub self_program: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub system_program: Pubkey,
}

impl Default for TokenAccountPubkeys {
    fn default() -> Self {
        // For the registered_program_pda, we need to derive it properly
        // For now, using a placeholder - this should be computed at runtime
        let registered_program_pda = Pubkey::new_unique(); // TODO: compute properly

        // For account_compression_authority, we can use the pre-computed CPI_AUTHORITY_PDA
        // but for the light system program context, we need to derive it
        let account_compression_authority = Pubkey::new_unique(); // TODO: compute properly

        Self {
            light_system_program: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
            registered_program_pda,
            noop_program: Pubkey::from(NOOP_PROGRAM_ID),
            account_compression_authority,
            account_compression_program: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
            self_program: Pubkey::from(LIGHT_COMPRESSED_TOKEN_PROGRAM_ID),
            cpi_authority_pda: Pubkey::from(CPI_AUTHORITY_PDA),
            system_program: Pubkey::new_from_array([0u8; 32]), // System program ID (11111111111111111111111111111111)
        }
    }
}

/// Get the standard account metas for a compressed token transfer instruction
pub fn get_transfer_instruction_account_metas(config: TokenAccountsMetaConfig) -> Vec<AccountMeta> {
    let default_pubkeys = TokenAccountPubkeys::default();

    let mut metas = vec![
        // fee_payer (mut, signer)
        AccountMeta::new(config.fee_payer, true),
        // authority (signer)
        AccountMeta::new_readonly(config.authority, true),
        // cpi_authority_pda
        AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
        // light_system_program
        AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
        // registered_program_pda
        AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
        // noop_program
        AccountMeta::new_readonly(default_pubkeys.noop_program, false),
        // account_compression_authority
        AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
        // account_compression_program
        AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        // self_program (compressed token program)
        AccountMeta::new_readonly(default_pubkeys.self_program, false),
    ];

    // Optional token pool PDA (for compression/decompression)
    if let Some(token_pool_pda) = config.token_pool_pda {
        metas.push(AccountMeta::new(token_pool_pda, false));
    }

    // Optional compress/decompress token account
    if let Some(token_account) = config.compress_or_decompress_token_account {
        metas.push(AccountMeta::new(token_account, false));
    }

    // Optional token program
    if let Some(token_program) = config.token_program {
        metas.push(AccountMeta::new_readonly(token_program, false));
    }

    // system_program (always last)
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));

    metas
}

// /// Get account metas for mint_to instruction
// pub fn get_mint_to_instruction_account_metas(
//     fee_payer: Pubkey,
//     authority: Pubkey,
//     mint: Pubkey,
//     token_pool_pda: Pubkey,
//     merkle_tree: Pubkey,
//     token_program: Option<Pubkey>,
// ) -> Vec<AccountMeta> {
//     let default_pubkeys = TokenAccountPubkeys::default();
//     let token_program = token_program.unwrap_or(Pubkey::from(SPL_TOKEN_PROGRAM_ID));

//     vec![
//         // fee_payer (mut, signer)
//         AccountMeta::new(fee_payer, true),
//         // authority (signer)
//         AccountMeta::new_readonly(authority, true),
//         // cpi_authority_pda
//         AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
//         // mint (optional, mut)
//         AccountMeta::new(mint, false),
//         // token_pool_pda (mut)
//         AccountMeta::new(token_pool_pda, false),
//         // token_program
//         AccountMeta::new_readonly(token_program, false),
//         // light_system_program
//         AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
//         // registered_program_pda
//         AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
//         // noop_program
//         AccountMeta::new_readonly(default_pubkeys.noop_program, false),
//         // account_compression_authority
//         AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
//         // account_compression_program
//         AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
//         // merkle_tree (mut)
//         AccountMeta::new(merkle_tree, false),
//         // self_program
//         AccountMeta::new_readonly(default_pubkeys.self_program, false),
//         // system_program
//         AccountMeta::new_readonly(default_pubkeys.system_program, false),
//     ]
// }

// /// Get account metas for burn instruction
// pub fn get_burn_instruction_account_metas(
//     fee_payer: Pubkey,
//     authority: Pubkey,
//     mint: Pubkey,
//     token_pool_pda: Pubkey,
//     token_program: Option<Pubkey>,
// ) -> Vec<AccountMeta> {
//     let default_pubkeys = TokenAccountPubkeys::default();
//     let token_program = token_program.unwrap_or(Pubkey::from(SPL_TOKEN_PROGRAM_ID));

//     vec![
//         // fee_payer (mut, signer)
//         AccountMeta::new(fee_payer, true),
//         // authority (signer)
//         AccountMeta::new_readonly(authority, true),
//         // cpi_authority_pda
//         AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
//         // mint (mut)
//         AccountMeta::new(mint, false),
//         // token_pool_pda (mut)
//         AccountMeta::new(token_pool_pda, false),
//         // token_program
//         AccountMeta::new_readonly(token_program, false),
//         // light_system_program
//         AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
//         // registered_program_pda
//         AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
//         // noop_program
//         AccountMeta::new_readonly(default_pubkeys.noop_program, false),
//         // account_compression_authority
//         AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
//         // account_compression_program
//         AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
//         // self_program
//         AccountMeta::new_readonly(default_pubkeys.self_program, false),
//         // system_program
//         AccountMeta::new_readonly(default_pubkeys.system_program, false),
//     ]
// }
