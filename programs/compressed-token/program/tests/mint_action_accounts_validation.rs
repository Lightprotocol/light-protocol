// use anchor_lang::prelude::AccountMeta;
// use light_account_checks::account_info::test_account_info::pinocchio::{
//     get_account_info, pubkey_unique,
// };
// use light_compressed_token::mint_action::accounts::{AccountsConfig, MintActionAccounts};
// use light_compressed_token::ErrorCode;
// use light_ctoken_types::CMINT_ADDRESS_TREE;
// use pinocchio::account_info::AccountInfo;
// use pinocchio::pubkey::Pubkey;

// /// Trait for converting test state structs to AccountInfo arrays
// pub trait ToAccountInfos {
//     fn to_account_infos(&self) -> Vec<AccountInfo>;
// }

// // Known program accounts
// pub fn get_light_system_program_meta() -> AccountMeta {
//     AccountMeta {
//         pubkey: light_sdk_types::LIGHT_SYSTEM_PROGRAM_ID.into(),
//         is_signer: false,
//         is_writable: false,
//     }
// }

// pub fn get_spl_token_program_meta() -> AccountMeta {
//     AccountMeta {
//         pubkey: spl_token_2022::ID,
//         is_signer: false,
//         is_writable: false,
//     }
// }

// // Address tree for compressed mint creation (checked in accounts.rs:166)
// pub fn get_address_tree_account_meta() -> AccountMeta {
//     AccountMeta {
//         pubkey: CMINT_ADDRESS_TREE.into(),
//         is_signer: false,
//         is_writable: true,
//     }
// }

// // Helper for creating mint account with specific pubkey (checked in accounts.rs:159)
// pub fn get_mint_account_meta(mint_pubkey: solana_pubkey::Pubkey) -> AccountMeta {
//     AccountMeta {
//         pubkey: mint_pubkey,
//         is_signer: false,
//         is_writable: true,
//     }
// }

// // PDA derivation helper for token pool (checked in accounts.rs:136-155)
// pub fn derive_token_pool_pda(
//     mint: &solana_pubkey::Pubkey,
//     index: u8,
// ) -> (solana_pubkey::Pubkey, u8) {
//     solana_pubkey::Pubkey::find_program_address(
//         &[b"ctoken_token_pool", mint.as_ref(), &[index]],
//         &light_compressed_token::ID,
//     )
// }

// /// Possible states for an account in validation testing
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum AccountState {
//     // Basic states based on presence and permissions
//     None,      // Account not present (missing)
//     NonMut,    // NonMut, non-signer, non-mutable (read-only)
//     Signer,    // NonMut, signer, non-mutable
//     Mutable,   // NonMut, non-signer, mutable
//     SignerMut, // NonMut, signer, mutable

//     // Wrong configurations for testing specific errors
//     WrongKey,   // NonMut but wrong pubkey
//     WrongOwner, // NonMut but owned by wrong program
//     WrongData,  // NonMut but wrong data/discriminator
//     WrongSize,  // NonMut but wrong account size

//     // Edge cases
//     Executable,        // NonMut, marked as executable (for program accounts)
//     SignerWrongKey,    // Signer but wrong pubkey (should fail PDA/key checks)
//     MutableWrongKey,   // Mutable but wrong pubkey
//     SignerMutWrongKey, // Signer+Mutable but wrong pubkey

//     // Uninitialized/closed states
//     Uninitialized, // NonMut but data is all zeros/uninitialized
//     Closed,        // NonMut but account was closed (0 lamports, empty data)
//     AccountMeta(AccountMeta),
// }

// impl AccountState {
//     /// Create an AccountInfo based on the state configuration
//     /// Returns None if the account should not exist (AccountState::None)
//     pub fn to_account_info(&self) -> Option<AccountInfo> {
//         self.to_account_info_with_key(pubkey_unique())
//     }

//     /// Create an AccountInfo with a specific pubkey based on the state configuration
//     pub fn to_account_info_with_key(&self, key: Pubkey) -> Option<AccountInfo> {
//         match self {
//             AccountState::None => None,

//             AccountState::NonMut => Some(get_account_info(
//                 key,
//                 Pubkey::default(), // System program owner
//                 false,             // not signer
//                 false,             // not writable
//                 false,             // not executable
//                 vec![],
//             )),

//             AccountState::Signer => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 true,  // signer
//                 false, // not writable
//                 false,
//                 vec![],
//             )),

//             AccountState::Mutable => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 false, // not signer
//                 true,  // writable
//                 false,
//                 vec![],
//             )),

//             AccountState::SignerMut => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 true, // signer
//                 true, // writable
//                 false,
//                 vec![],
//             )),

//             AccountState::WrongKey => Some(get_account_info(
//                 pubkey_unique(), // Different key than expected
//                 Pubkey::default(),
//                 false,
//                 false,
//                 false,
//                 vec![],
//             )),

//             AccountState::WrongOwner => Some(get_account_info(
//                 key,
//                 pubkey_unique(), // Wrong owner
//                 false,
//                 false,
//                 false,
//                 vec![],
//             )),

//             AccountState::WrongData => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 false,
//                 false,
//                 false,
//                 vec![0xFF; 32], // Invalid data
//             )),

//             AccountState::WrongSize => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 false,
//                 false,
//                 false,
//                 vec![0; 1], // Wrong size data
//             )),

//             AccountState::Executable => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 false,
//                 false,
//                 true, // executable
//                 vec![],
//             )),

//             AccountState::SignerWrongKey => Some(get_account_info(
//                 pubkey_unique(), // Wrong key
//                 Pubkey::default(),
//                 true, // signer
//                 false,
//                 false,
//                 vec![],
//             )),

//             AccountState::MutableWrongKey => Some(get_account_info(
//                 pubkey_unique(), // Wrong key
//                 Pubkey::default(),
//                 false,
//                 true, // writable
//                 false,
//                 vec![],
//             )),

//             AccountState::SignerMutWrongKey => Some(get_account_info(
//                 pubkey_unique(), // Wrong key
//                 Pubkey::default(),
//                 true, // signer
//                 true, // writable
//                 false,
//                 vec![],
//             )),

//             AccountState::Uninitialized => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 false,
//                 false,
//                 false,
//                 vec![0; 100], // All zeros - uninitialized
//             )),

//             AccountState::Closed => Some(get_account_info(
//                 key,
//                 Pubkey::default(),
//                 false,
//                 false,
//                 false,
//                 vec![], // Empty data - closed account
//             )),

//             AccountState::AccountMeta(meta) => Some(get_account_info(
//                 meta.pubkey.to_bytes(), // Use pubkey from AccountMeta
//                 Pubkey::default(),      // System program owner
//                 meta.is_signer,
//                 meta.is_writable,
//                 false, // not executable (unless it's a program, but AccountMeta doesn't specify)
//                 vec![],
//             )),
//         }
//     }
// }

// /// LightSystemAccounts state configuration
// #[derive(Debug, Clone)]
// pub struct LightSystemAccountsState {
//     pub fee_payer: AccountState,
//     pub cpi_authority_pda: AccountState,
//     pub registered_program_pda: AccountState,
//     pub account_compression_authority: AccountState,
//     pub account_compression_program: AccountState,
//     pub system_program: AccountState,
//     pub sol_pool_pda: AccountState, // Option<&AccountInfo>
//     pub sol_decompression_recipient: AccountState, // Option<&AccountInfo>
//     pub cpi_context: AccountState,  // Option<&AccountInfo>
// }

// impl Default for LightSystemAccountsState {
//     /// Returns a valid default configuration for LightSystemAccounts
//     /// with all required accounts present and properly configured
//     fn default() -> Self {
//         Self {
//             fee_payer: AccountState::SignerMut,
//             cpi_authority_pda: AccountState::NonMut,
//             registered_program_pda: AccountState::NonMut,
//             account_compression_authority: AccountState::NonMut,
//             account_compression_program: AccountState::NonMut,
//             system_program: AccountState::NonMut,
//             sol_pool_pda: AccountState::None, // Optional
//             sol_decompression_recipient: AccountState::None, // Optional
//             cpi_context: AccountState::None,  // Optional
//         }
//     }
// }

// impl ToAccountInfos for LightSystemAccountsState {
//     fn to_account_infos(&self) -> Vec<AccountInfo> {
//         let mut accounts = Vec::new();

//         // Required accounts - always add (or None becomes missing account error)
//         if let Some(account) = self.fee_payer.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.cpi_authority_pda.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.registered_program_pda.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.account_compression_authority.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.account_compression_program.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.system_program.to_account_info() {
//             accounts.push(account);
//         }

//         // Optional accounts - only add if not None
//         if let Some(account) = self.sol_pool_pda.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.sol_decompression_recipient.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.cpi_context.to_account_info() {
//             accounts.push(account);
//         }

//         accounts
//     }
// }

// /// CpiContextLightSystemAccounts state configuration
// #[derive(Debug, Clone)]
// pub struct CpiContextLightSystemAccountsState {
//     pub fee_payer: AccountState,
//     pub cpi_authority_pda: AccountState,
//     pub cpi_context: AccountState,
// }

// impl Default for CpiContextLightSystemAccountsState {
//     /// Returns a valid default configuration for CpiContextLightSystemAccounts
//     /// with all required accounts present and properly configured
//     fn default() -> Self {
//         Self {
//             fee_payer: AccountState::SignerMut,
//             cpi_authority_pda: AccountState::NonMut,
//             cpi_context: AccountState::Mutable,
//         }
//     }
// }

// impl ToAccountInfos for CpiContextLightSystemAccountsState {
//     fn to_account_infos(&self) -> Vec<AccountInfo> {
//         let mut accounts = Vec::new();

//         // All accounts are required for CpiContextLightSystemAccounts
//         if let Some(account) = self.fee_payer.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.cpi_authority_pda.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.cpi_context.to_account_info() {
//             accounts.push(account);
//         }

//         accounts
//     }
// }

// /// ExecutingAccounts state configuration
// #[derive(Debug, Clone)]
// pub struct ExecutingAccountsState {
//     pub mint: AccountState,           // Option<&AccountInfo>
//     pub token_pool_pda: AccountState, // Option<&AccountInfo>
//     pub token_program: AccountState,  // Option<&AccountInfo>
//     pub system: LightSystemAccountsState,
//     pub out_output_queue: AccountState,
//     pub in_merkle_tree: AccountState,      // Option<&AccountInfo>
//     pub address_merkle_tree: AccountState, // Option<&AccountInfo>
//     pub in_output_queue: AccountState,     // Option<&AccountInfo>
//     pub tokens_out_queue: AccountState,    // Option<&AccountInfo>
// }

// impl ExecutingAccountsState {
//     /// Generate account infos with specific mint and token pool pubkeys
//     /// Returns accounts for the executing path
//     pub fn to_account_infos_with_keys(
//         &self,
//         cmint_pubkey: &solana_pubkey::Pubkey,
//         token_pool_pubkey: &solana_pubkey::Pubkey,
//     ) -> Vec<AccountInfo> {
//         let mut accounts = Vec::new();

//         // Optional SPL mint accounts with specific pubkeys
//         if !matches!(self.mint, AccountState::None) {
//             accounts.push(match &self.mint {
//                 AccountState::AccountMeta(meta) => get_account_info(
//                     meta.pubkey.to_bytes(),
//                     Pubkey::default(),
//                     meta.is_signer,
//                     meta.is_writable,
//                     false,
//                     vec![],
//                 ),
//                 _ => self
//                     .mint
//                     .to_account_info_with_key(cmint_pubkey.to_bytes())
//                     .unwrap(),
//             });
//         }

//         if !matches!(self.token_pool_pda, AccountState::None) {
//             accounts.push(match &self.token_pool_pda {
//                 AccountState::AccountMeta(meta) => get_account_info(
//                     meta.pubkey.to_bytes(),
//                     Pubkey::default(),
//                     meta.is_signer,
//                     meta.is_writable,
//                     false,
//                     vec![],
//                 ),
//                 _ => self
//                     .token_pool_pda
//                     .to_account_info_with_key(token_pool_pubkey.to_bytes())
//                     .unwrap(),
//             });
//         }

//         if let Some(account) = self.token_program.to_account_info() {
//             accounts.push(account);
//         }

//         // Add all LightSystemAccounts
//         accounts.extend(self.system.to_account_infos());

//         // Required output queue
//         if let Some(account) = self.out_output_queue.to_account_info() {
//             accounts.push(account);
//         }

//         // Either in_merkle_tree or address_merkle_tree should be present (not both)
//         if let Some(account) = self.in_merkle_tree.to_account_info() {
//             accounts.push(account);
//         } else if let Some(account) = self.address_merkle_tree.to_account_info() {
//             accounts.push(account);
//         }

//         // Optional queues
//         if let Some(account) = self.in_output_queue.to_account_info() {
//             accounts.push(account);
//         }
//         if let Some(account) = self.tokens_out_queue.to_account_info() {
//             accounts.push(account);
//         }

//         accounts
//     }
// }

// /// Account presence and state configuration for MintActionAccounts validation testing
// /// Each field represents the state of that account in the test
// #[derive(Debug, Clone)]
// pub struct MintActionAccountsPresence {
//     // Top-level MintActionAccounts fields
//     pub light_system_program: AccountState,
//     pub mint_signer: AccountState, // Option<&AccountInfo>
//     pub authority: AccountState,

//     // ExecutingAccounts - Option<ExecutingAccounts>
//     pub executing: Option<ExecutingAccountsState>,

//     // CpiContextLightSystemAccounts - Option<CpiContextLightSystemAccounts>
//     pub write_to_cpi_context_system: Option<CpiContextLightSystemAccountsState>,

//     // Packed accounts (can be empty slice)
//     pub packed_accounts_count: usize,
// }

// impl MintActionAccountsPresence {
//     /// Create a default configuration for the executing path (no CPI write context)
//     /// This represents a valid configuration for executing an instruction with existing mint
//     pub fn default_executing() -> Self {
//         Self {
//             // Top-level MintActionAccounts fields
//             light_system_program: AccountState::Executable,
//             mint_signer: AccountState::None, // Not needed for existing mint
//             authority: AccountState::Signer,

//             // ExecutingAccounts - present for executing path
//             executing: Some(ExecutingAccountsState {
//                 mint: AccountState::None, // Optional - only needed if SPL mint initialized
//                 token_pool_pda: AccountState::None, // Optional - only needed if SPL mint initialized
//                 token_program: AccountState::None, // Optional - only needed if SPL mint initialized
//                 system: LightSystemAccountsState::default(),
//                 out_output_queue: AccountState::Mutable,
//                 in_merkle_tree: AccountState::Mutable, // For existing mint
//                 address_merkle_tree: AccountState::None, // Not used for existing mint
//                 in_output_queue: AccountState::Mutable, // Required for existing mint
//                 tokens_out_queue: AccountState::None,  // Optional - only for MintTo actions
//             }),

//             // CpiContextLightSystemAccounts - None for executing path
//             write_to_cpi_context_system: None,

//             // No packed accounts by default
//             packed_accounts_count: 0,
//         }
//     }

//     /// Generate AccountsConfig based on the MintActionAccountsPresence state
//     pub fn to_accounts_config(&self) -> AccountsConfig {
//         // Determine write_to_cpi_context based on presence of write_to_cpi_context_system
//         let write_to_cpi_context = self.write_to_cpi_context_system.is_some();

//         // Determine with_cpi_context - true if either:
//         // 1. write_to_cpi_context_system is present, OR
//         // 2. executing.system.cpi_context is not None
//         let with_cpi_context = write_to_cpi_context
//             || self
//                 .executing
//                 .as_ref()
//                 .map(|e| !matches!(e.system.cpi_context, AccountState::None))
//                 .unwrap_or(false);

//         // Check if SPL mint is initialized based on mint/token_pool_pda/token_program presence
//         let spl_mint_initialized = self
//             .executing
//             .as_ref()
//             .map(|e| {
//                 !matches!(e.mint, AccountState::None)
//                     || !matches!(e.token_pool_pda, AccountState::None)
//                     || !matches!(e.token_program, AccountState::None)
//             })
//             .unwrap_or(false);

//         // Check if there are mint-to actions based on tokens_out_queue presence
//         let has_mint_to_actions = self
//             .executing
//             .as_ref()
//             .map(|e| !matches!(e.tokens_out_queue, AccountState::None))
//             .unwrap_or(false);

//         // Check if mint_signer is present (needed for create mint or create SPL mint)
//         let with_mint_signer = !matches!(self.mint_signer, AccountState::None);

//         // Determine if creating mint based on address_merkle_tree vs in_merkle_tree
//         let create_mint = self
//             .executing
//             .as_ref()
//             .map(|e| {
//                 // Creating mint uses address_merkle_tree
//                 !matches!(e.address_merkle_tree, AccountState::None)
//                     && matches!(e.in_merkle_tree, AccountState::None)
//             })
//             .unwrap_or(false);

//         AccountsConfig {
//             with_cpi_context,
//             write_to_cpi_context,
//             spl_mint_initialized,
//             has_mint_to_actions,
//             with_mint_signer,
//             create_mint,
//         }
//     }
// }

// impl MintActionAccountsPresence {
//     /// Generate account infos and related validation parameters
//     /// Returns (accounts, cmint_pubkey, token_pool_index, token_pool_bump)
//     pub fn to_account_infos(&self) -> (Vec<AccountInfo>, solana_pubkey::Pubkey, u8, u8) {
//         let mut accounts = Vec::new();

//         // Generate a consistent cmint_pubkey for this test
//         let cmint_pubkey = solana_pubkey::Pubkey::new_unique();
//         let token_pool_index = 0u8;

//         // Derive token pool PDA if needed
//         let (token_pool_pubkey, token_pool_bump) = if self
//             .executing
//             .as_ref()
//             .map(|e| !matches!(e.token_pool_pda, AccountState::None))
//             .unwrap_or(false)
//         {
//             derive_token_pool_pda(&cmint_pubkey, token_pool_index)
//         } else {
//             (solana_pubkey::Pubkey::default(), 0)
//         };

//         // Always required: light_system_program
//         if let Some(account) = self.light_system_program.to_account_info() {
//             accounts.push(account);
//         }

//         // Optional: mint_signer
//         if let Some(account) = self.mint_signer.to_account_info() {
//             accounts.push(account);
//         }

//         // Always required: authority
//         if let Some(account) = self.authority.to_account_info() {
//             accounts.push(account);
//         }

//         // Either executing OR write_to_cpi_context_system (but not both)
//         if let Some(executing) = &self.executing {
//             accounts
//                 .extend(executing.to_account_infos_with_keys(&cmint_pubkey, &token_pool_pubkey));
//         } else if let Some(cpi_context) = &self.write_to_cpi_context_system {
//             accounts.extend(cpi_context.to_account_infos());
//         }

//         // Add packed accounts (create dummy accounts for testing)
//         for _ in 0..self.packed_accounts_count {
//             accounts.push(get_account_info(
//                 pubkey_unique(),
//                 Pubkey::default(),
//                 false,
//                 false,
//                 false,
//                 vec![],
//             ));
//         }

//         (accounts, cmint_pubkey, token_pool_index, token_pool_bump)
//     }
// }

// #[test]
// fn test_validate_with_generated_accounts() {
//     let state = MintActionAccountsPresence::default_executing();
//     let (accounts, cmint_pubkey, token_pool_index, token_pool_bump) = state.to_account_infos();
//     let config = state.to_accounts_config();

//     // This should succeed with proper accounts
//     let result = MintActionAccounts::validate_and_parse(
//         &accounts,
//         &config,
//         &cmint_pubkey,
//         token_pool_index,
//         token_pool_bump,
//     );

//     assert!(result.is_ok());
// }

// #[test]
// fn test_expected_cpi_authority_error() {
//     // Create accounts for a successful parse
//     let state = MintActionAccountsPresence::default_executing();
//     let (accounts, cmint_pubkey, _token_pool_index, _token_pool_bump) = state.to_account_infos();
//     // Now test cpi_authority() method when both executing and write_to_cpi_context are None
//     // This can't happen in normal flow, but we can construct it manually for testing
//     let broken_accounts = MintActionAccounts {
//         light_system_program: &accounts[0],
//         mint_signer: None,
//         authority: &accounts[1],
//         executing: None,
//         write_to_cpi_context_system: None,
//         packed_accounts: light_account_checks::packed_accounts::ProgramPackedAccounts {
//             accounts: &[],
//         },
//     };

//     let result = broken_accounts.cpi_authority();
//     assert!(result.is_err());
//     assert_eq!(
//         result.err().unwrap(),
//         ErrorCode::ExpectedCpiAuthority.into()
//     );

//     println!("✅ ExpectedCpiAuthority error test passed!");
// }

// #[test]
// fn test_invalid_token_program() {
//     // Setup state with SPL mint initialized
//     let mut state = MintActionAccountsPresence::default_executing();

//     // Set up SPL mint accounts
//     state.executing.as_mut().unwrap().mint = AccountState::Mutable;
//     state.executing.as_mut().unwrap().token_pool_pda = AccountState::Mutable;
//     // Use wrong token program (not SPL Token 2022)
//     state.executing.as_mut().unwrap().token_program = AccountState::AccountMeta(AccountMeta {
//         pubkey: solana_pubkey::Pubkey::new_unique(), // Wrong program ID
//         is_signer: false,
//         is_writable: false,
//     });

//     let (accounts, cmint_pubkey, token_pool_index, token_pool_bump) = state.to_account_infos();
//     let config = state.to_accounts_config();

//     let result = MintActionAccounts::validate_and_parse(
//         &accounts,
//         &config,
//         &cmint_pubkey,
//         token_pool_index,
//         token_pool_bump,
//     );

//     assert!(result.is_err());
//     assert_eq!(
//         result.err().unwrap(),
//         anchor_lang::prelude::ProgramError::InvalidAccountData
//     );

//     println!("✅ Invalid token program test passed!");
// }

// #[test]
// fn test_invalid_token_pool_pda() {
//     // Setup state with SPL mint initialized
//     let mut state = MintActionAccountsPresence::default_executing();

//     // Set up SPL mint accounts
//     state.executing.as_mut().unwrap().mint = AccountState::Mutable;
//     // Use wrong token pool PDA (random pubkey instead of correct PDA)
//     state.executing.as_mut().unwrap().token_pool_pda = AccountState::AccountMeta(AccountMeta {
//         pubkey: solana_pubkey::Pubkey::new_unique(), // Wrong PDA
//         is_signer: false,
//         is_writable: true,
//     });
//     state.executing.as_mut().unwrap().token_program =
//         AccountState::AccountMeta(get_spl_token_program_meta());

//     let (accounts, cmint_pubkey, token_pool_index, token_pool_bump) = state.to_account_infos();
//     let config = state.to_accounts_config();

//     let result = MintActionAccounts::validate_and_parse(
//         &accounts,
//         &config,
//         &cmint_pubkey,
//         token_pool_index,
//         token_pool_bump,
//     );

//     assert!(result.is_err());
//     assert_eq!(
//         result.err().unwrap(),
//         anchor_lang::prelude::ProgramError::InvalidAccountData
//     );

//     println!("✅ Invalid token pool PDA test passed!");
// }

// #[test]
// fn test_mint_account_mismatch() {
//     // Setup state with SPL mint initialized
//     let mut state = MintActionAccountsPresence::default_executing();

//     // Set up SPL mint accounts with wrong mint pubkey
//     let wrong_mint_pubkey = solana_pubkey::Pubkey::new_unique();
//     state.executing.as_mut().unwrap().mint = AccountState::AccountMeta(AccountMeta {
//         pubkey: solana_pubkey::Pubkey::new_unique(), // Different from cmint_pubkey
//         is_signer: false,
//         is_writable: true,
//     });
//     let (pubkey, token_pool_bump) = derive_token_pool_pda(&wrong_mint_pubkey, 0);
//     // Set token_pool_pda and token_program to None to avoid PDA validation before mint mismatch check
//     state.executing.as_mut().unwrap().token_pool_pda = AccountState::AccountMeta(AccountMeta {
//         pubkey,
//         is_signer: false,
//         is_writable: true,
//     });
//     state.executing.as_mut().unwrap().token_program =
//         AccountState::AccountMeta(get_spl_token_program_meta());

//     let (accounts, cmint_pubkey, token_pool_index, _token_pool_bump) = state.to_account_infos();
//     let config = state.to_accounts_config();

//     let result = MintActionAccounts::validate_and_parse(
//         &accounts,
//         &config,
//         &wrong_mint_pubkey, // Use the cmint_pubkey generated by to_account_infos
//         token_pool_index,
//         token_pool_bump,
//     );

//     assert!(result.is_err());
//     assert_eq!(result.err().unwrap(), ErrorCode::MintAccountMismatch.into());

//     println!("✅ Mint account mismatch test passed!");
// }

// #[test]
// fn test_invalid_address_tree() {
//     // Setup state for creating new mint
//     let mut state = MintActionAccountsPresence::default_executing();

//     // Configure for mint creation (address_merkle_tree instead of in_merkle_tree)
//     state.executing.as_mut().unwrap().address_merkle_tree =
//         AccountState::AccountMeta(AccountMeta {
//             pubkey: solana_pubkey::Pubkey::new_unique(), // Wrong address tree
//             is_signer: false,
//             is_writable: true,
//         });
//     state.executing.as_mut().unwrap().in_merkle_tree = AccountState::None;
//     state.executing.as_mut().unwrap().in_output_queue = AccountState::None; // Not needed for create

//     let (accounts, cmint_pubkey, token_pool_index, token_pool_bump) = state.to_account_infos();
//     let config = state.to_accounts_config();

//     let result = MintActionAccounts::validate_and_parse(
//         &accounts,
//         &config,
//         &cmint_pubkey,
//         token_pool_index,
//         token_pool_bump,
//     );

//     assert!(result.is_err());
//     assert_eq!(result.err().unwrap(), ErrorCode::InvalidAddressTree.into());

//     println!("✅ Invalid address tree test passed!");
// }
