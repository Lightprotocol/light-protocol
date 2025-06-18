use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::instructions::CTokenDefaultAccounts;

/// Account metadata configuration for batch compress instruction
#[derive(Debug, Default, Copy, Clone)]
pub struct BatchCompressMetaConfig {
    pub fee_payer: Option<Pubkey>,
    pub authority: Option<Pubkey>,
    pub token_pool_pda: Option<Pubkey>,
    pub sender_token_account: Option<Pubkey>,
    pub token_program: Option<Pubkey>,
    pub merkle_tree: Option<Pubkey>,
    pub sol_pool_pda: Option<Pubkey>,
}

impl BatchCompressMetaConfig {
    pub fn new(
        fee_payer: Pubkey,
        authority: Pubkey,
        token_pool_pda: Pubkey,
        sender_token_account: Pubkey,
        token_program: Pubkey,
        merkle_tree: Pubkey,
    ) -> Self {
        Self {
            fee_payer: Some(fee_payer),
            authority: Some(authority),
            token_pool_pda: Some(token_pool_pda),
            sender_token_account: Some(sender_token_account),
            token_program: Some(token_program),
            merkle_tree: Some(merkle_tree),
            sol_pool_pda: None,
        }
    }

    pub fn new_client(
        token_pool_pda: Pubkey,
        sender_token_account: Pubkey,
        token_program: Pubkey,
        merkle_tree: Pubkey,
    ) -> Self {
        Self {
            fee_payer: None,
            authority: None,
            token_pool_pda: Some(token_pool_pda),
            sender_token_account: Some(sender_token_account),
            token_program: Some(token_program),
            merkle_tree: Some(merkle_tree),
            sol_pool_pda: None,
        }
    }

    pub fn with_sol_pool_pda(mut self, sol_pool_pda: Pubkey) -> Self {
        self.sol_pool_pda = Some(sol_pool_pda);
        self
    }
}

/// Get the standard account metas for a batch compress instruction
/// Matches the MintToInstruction account structure used by batch_compress
pub fn get_batch_compress_instruction_account_metas(config: BatchCompressMetaConfig) -> Vec<AccountMeta> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    
    // Start building the account metas to match MintToInstruction structure
    let mut metas = Vec::new();

    // Add fee_payer and authority if provided (for direct invoke)
    if let Some(fee_payer) = config.fee_payer {
        let authority = config.authority.expect("Missing authority");
        metas.extend_from_slice(&[
            // fee_payer (mut, signer)
            AccountMeta::new(fee_payer, true),
            // authority (signer)
            AccountMeta::new_readonly(authority, true),
        ]);
    }

    // cpi_authority_pda
    metas.push(AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false));
    
    // mint: Option<UncheckedAccount> - None for batch_compress, so we skip this account
    
    // token_pool_pda (mut)
    let token_pool_pda = config.token_pool_pda.expect("Missing token_pool_pda");
    metas.push(AccountMeta::new(token_pool_pda, false));
    
    // token_program
    let token_program = config.token_program.expect("Missing token_program");
    metas.push(AccountMeta::new_readonly(token_program, false));
    
    // light_system_program
    metas.push(AccountMeta::new_readonly(default_pubkeys.light_system_program, false));
    
    // registered_program_pda
    metas.push(AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false));
    
    // noop_program
    metas.push(AccountMeta::new_readonly(default_pubkeys.noop_program, false));
    
    // account_compression_authority 
    metas.push(AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false));
    
    // account_compression_program
    metas.push(AccountMeta::new_readonly(default_pubkeys.account_compression_program, false));
    
    // merkle_tree (mut)
    let merkle_tree = config.merkle_tree.expect("Missing merkle_tree");
    metas.push(AccountMeta::new(merkle_tree, false));
    
    // self_program (compressed token program)
    metas.push(AccountMeta::new_readonly(default_pubkeys.self_program, false));
    
    // system_program
    metas.push(AccountMeta::new_readonly(default_pubkeys.system_program, false));
    
    // sol_pool_pda (optional, mut)
    if let Some(sol_pool_pda) = config.sol_pool_pda {
        metas.push(AccountMeta::new(sol_pool_pda, false));
    }
    
    // Remaining accounts: sender_token_account (first remaining account for batch_compress)
    let sender_token_account = config.sender_token_account.expect("Missing sender_token_account");
    metas.push(AccountMeta::new(sender_token_account, false));

    metas
}
