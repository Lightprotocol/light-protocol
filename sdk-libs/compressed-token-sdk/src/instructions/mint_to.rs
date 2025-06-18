// /// Get account metas for mint_to instruction
// pub fn get_mint_to_instruction_account_metas(
//     fee_payer: Pubkey,
//     authority: Pubkey,
//     mint: Pubkey,
//     token_pool_pda: Pubkey,
//     merkle_tree: Pubkey,
//     token_program: Option<Pubkey>,
// ) -> Vec<AccountMeta> {
//     let default_pubkeys = CTokenDefaultAccounts::default();
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
