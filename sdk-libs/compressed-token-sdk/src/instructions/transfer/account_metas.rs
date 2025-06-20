use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for compressed token instructions
#[derive(Debug, Default, Copy, Clone)]
pub struct TokenAccountsMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub authority: Option<Pubkey>,
    pub token_pool_pda: Option<Pubkey>,
    pub compress_or_decompress_token_account: Option<Pubkey>,
    pub token_program: Option<Pubkey>,
    pub is_compress: bool,
    pub is_decompress: bool,
    pub with_anchor_none: bool,
}

impl TokenAccountsMetaConfig {
    pub fn new(fee_payer: Pubkey, authority: Pubkey) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            authority: Some(authority),
            token_pool_pda: None,
            compress_or_decompress_token_account: None,
            token_program: None,
            is_compress: false,
            is_decompress: false,
            with_anchor_none: false,
        }
    }

    pub fn new_client() -> Self {
        Self {
            fee_payer: None,
            authority: None,
            token_pool_pda: None,
            compress_or_decompress_token_account: None,
            token_program: None,
            is_compress: false,
            is_decompress: false,
            with_anchor_none: false,
        }
    }

    pub fn new_with_anchor_none() -> Self {
        Self {
            fee_payer: None,
            authority: None,
            token_pool_pda: None,
            compress_or_decompress_token_account: None,
            token_program: None,
            is_compress: false,
            is_decompress: false,
            with_anchor_none: true,
        }
    }

    pub fn compress(
        fee_payer: Pubkey,
        authority: Pubkey,
        token_pool_pda: Pubkey,
        sender_token_account: Pubkey,
        spl_program_id: Pubkey,
    ) -> Self {
        // TODO: derive token_pool_pda here and pass mint instead.
        Self {
            fee_payer: Some(fee_payer),
            authority: Some(authority),
            token_pool_pda: Some(token_pool_pda),
            compress_or_decompress_token_account: Some(sender_token_account),
            token_program: Some(spl_program_id),
            is_compress: true,
            is_decompress: false,
            with_anchor_none: false,
        }
    }

    pub fn compress_client(
        token_pool_pda: Pubkey,
        sender_token_account: Pubkey,
        spl_program_id: Pubkey,
    ) -> Self {
        Self {
            fee_payer: None,
            authority: None,
            token_pool_pda: Some(token_pool_pda),
            compress_or_decompress_token_account: Some(sender_token_account),
            token_program: Some(spl_program_id),
            is_compress: true,
            is_decompress: false,
            with_anchor_none: false,
        }
    }

    pub fn decompress(
        fee_payer: Pubkey,
        authority: Pubkey,
        token_pool_pda: Pubkey,
        recipient_token_account: Pubkey,
        spl_program_id: Pubkey,
    ) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            authority: Some(authority),
            token_pool_pda: Some(token_pool_pda),
            compress_or_decompress_token_account: Some(recipient_token_account),
            token_program: Some(spl_program_id),
            is_compress: false,
            is_decompress: true,
            with_anchor_none: false,
        }
    }

    pub fn decompress_client(
        token_pool_pda: Pubkey,
        recipient_token_account: Pubkey,
        spl_program_id: Pubkey,
    ) -> Self {
        Self {
            fee_payer: None,
            authority: None,
            token_pool_pda: Some(token_pool_pda),
            compress_or_decompress_token_account: Some(recipient_token_account),
            token_program: Some(spl_program_id),
            is_compress: false,
            is_decompress: true,
            with_anchor_none: false,
        }
    }

    pub fn is_compress_or_decompress(&self) -> bool {
        self.is_compress || self.is_decompress
    }
}

/// Get the standard account metas for a compressed token transfer instruction
pub fn get_transfer_instruction_account_metas(config: TokenAccountsMetaConfig) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    // Direct invoke adds fee_payer, and authority
    let mut metas = if let Some(fee_payer) = config.fee_payer {
        let authority = if let Some(authority) = config.authority {
            authority
        } else {
            panic!("Missing authority");
        };
        vec![
            AccountMeta::new(fee_payer, true),
            AccountMeta::new_readonly(authority, true),
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
        ]
    } else {
        vec![
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
        ]
    };

    // Optional token pool PDA (for compression/decompression)
    if let Some(token_pool_pda) = config.token_pool_pda {
        metas.push(AccountMeta::new(token_pool_pda, false));
    } else if config.fee_payer.is_some() || config.with_anchor_none {
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.compressed_token_program,
            false,
        ));
    }
    println!("config.with_anchor_none {}", config.with_anchor_none);
    // Optional compress/decompress token account
    if let Some(token_account) = config.compress_or_decompress_token_account {
        metas.push(AccountMeta::new(token_account, false));
    } else if config.fee_payer.is_some() || config.with_anchor_none {
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.compressed_token_program,
            false,
        ));
    }

    // Optional token program
    if let Some(token_program) = config.token_program {
        metas.push(AccountMeta::new_readonly(token_program, false));
    } else if config.fee_payer.is_some() || config.with_anchor_none {
        metas.push(AccountMeta::new_readonly(
            default_pubkeys.compressed_token_program,
            false,
        ));
    }

    // system_program (always last)
    metas.push(AccountMeta::new_readonly(
        default_pubkeys.system_program,
        false,
    ));

    metas
}
