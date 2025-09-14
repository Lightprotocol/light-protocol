#[cfg(feature = "anchor")]
use anchor_lang::{
    prelude::{InterfaceAccount, Signer},
    ToAccountInfo,
};
use light_account_checks::AccountInfoTrait;
use light_ctoken_types::{
    instructions::transfer2::{
        Compression, CompressionMode, MultiInputTokenDataWithContext, MultiTokenTransferOutputData,
    },
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_sdk::{
    compressible::{create_or_allocate_account, CompressibleConfig},
    constants::CPI_AUTHORITY_PDA_SEED,
    cpi::{CpiAccountsSmall, CpiSigner},
    instruction::borsh_compat::ValidityProof,
    AnchorDeserialize, AnchorSerialize,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use crate::{
    account2::CTokenAccount2,
    error::Result,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
};

/// Same as SPL-token discriminator
pub const CLOSE_TOKEN_ACCOUNT_DISCRIMINATOR: u8 = 9;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct PackedCompressedTokenDataWithContext {
    pub mint: u8,
    pub source_or_recipient_token_account: u8,
    pub multi_input_token_data_with_context: MultiInputTokenDataWithContext,
}

pub fn account_meta_from_account_info(account_info: &AccountInfo) -> AccountMeta {
    AccountMeta {
        pubkey: *account_info.key,
        is_signer: account_info.is_signer,
        is_writable: account_info.is_writable,
    }
}

/// Structure to hold token account data for batch compression
#[cfg(feature = "anchor")]
#[derive(Debug, Clone)]
pub struct TokenAccountToCompress<'info> {
    pub token_account: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
    pub signer_seeds: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct AccountInfoToCompress<'info> {
    pub account_info: AccountInfo<'info>,
    pub signer_seeds: Vec<Vec<u8>>,
}

fn add_or_get_index<T: PartialEq>(vec: &mut Vec<T>, item: T) -> u8 {
    if let Some(idx) = vec.iter().position(|x| x == &item) {
        idx as u8
    } else {
        vec.push(item);
        (vec.len() - 1) as u8
    }
}

// /// Input parameters for creating a token account with compressible extension
// #[derive(Debug, Clone)]
// pub struct InitializeCompressibleTokenAccount {
//     /// The account to be created
//     pub account_pubkey: Pubkey,
//     /// The mint for the token account
//     pub mint_pubkey: Pubkey,
//     /// The owner of the token account
//     pub owner_pubkey: Pubkey,
//     /// The authority that can close this account (in addition to owner)
//     pub rent_authority: Pubkey,
//     /// The recipient of lamports when the account is closed by rent authority
//     pub rent_recipient: Pubkey,
//     /// Number of slots that must pass before compression is allowed
//     pub slots_until_compression: u64,
// }

// #[inline(never)]
// pub fn initialize_compressible_token_account(
//     inputs: InitializeCompressibleTokenAccount,
// ) -> Result<Instruction> {
//     // Format: [18, owner_pubkey_32_bytes, 0]
//     // Create compressible extension data manually
//     // Layout: [slots_until_compression: u64, rent_authority: 32 bytes, rent_recipient: 32 bytes]
//     let mut data = Vec::with_capacity(1 + 32 + 1 + 8 + 32 + 32);
//     data.push(18u8); // InitializeAccount3 opcode
//     data.extend_from_slice(&inputs.owner_pubkey.to_bytes());
//     data.push(1); // Some option byte extension
//     data.extend_from_slice(&inputs.slots_until_compression.to_le_bytes());
//     data.extend_from_slice(&inputs.rent_authority.to_bytes());
//     data.extend_from_slice(&inputs.rent_recipient.to_bytes());

//     Ok(Instruction {
//         program_id: Pubkey::from(light_sdk_types::CTOKEN_PROGRAM_ID),
//         accounts: vec![
//             solana_instruction::AccountMeta::new(inputs.account_pubkey, false),
//             solana_instruction::AccountMeta::new_readonly(inputs.mint_pubkey, false),
//         ],
//         data,
//     })
// }

// #[allow(clippy::too_many_arguments)]
// #[cfg(feature = "anchor")]
// #[inline(never)]
// pub fn create_compressible_token_account<'a>(
//     authority: &AccountInfo<'a>,
//     payer: &AccountInfo<'a>,
//     token_account: &AccountInfo<'a>,
//     mint_account: &AccountInfo<'a>,
//     system_program: &AccountInfo<'a>,
//     token_program: &AccountInfo<'a>,
//     signer_seeds: &[&[u8]],
//     rent_authority: &AccountInfo<'a>,
//     rent_recipient: &AccountInfo<'a>,
//     slots_until_compression: u64,
// ) -> std::result::Result<(), solana_program_error::ProgramError> {
//     use anchor_lang::ToAccountInfo;
//     use solana_cpi::invoke;

//     let space = COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize;

//     create_or_allocate_account(
//         token_program.key,
//         payer.to_account_info(),
//         system_program.to_account_info(),
//         token_account.to_account_info(),
//         signer_seeds,
//         space,
//     )?;

//     let init_ix = initialize_compressible_token_account(InitializeCompressibleTokenAccount {
//         account_pubkey: *token_account.key,
//         mint_pubkey: *mint_account.key,
//         owner_pubkey: *authority.key,
//         rent_authority: *rent_authority.key,
//         rent_recipient: *rent_recipient.key,
//         slots_until_compression,
//     })?;

//     invoke(
//         &init_ix,
// &[
//     token_account.to_account_info(),
//     mint_account.to_account_info(),
//     authority.to_account_info(),
//     rent_authority.to_account_info(),
//     rent_recipient.to_account_info(),
// ],
//     )?;

//     Ok(())
// }

// TODO: remove.
// /// CPI function to close a compressed token account
// ///
// /// # Arguments
// /// * `token_account` - The token account to close (must have 0 balance)
// /// * `destination` - The account to receive the lamports
// /// * `authority` - The owner of the token account (must sign)
// /// * `signer_seeds` - Optional signer seeds if calling from a PDA
// pub fn close_compressed_token_account<'info>(
//     token_account: AccountInfo<'info>,
//     destination: AccountInfo<'info>,
//     authority: AccountInfo<'info>,
//     signer_seeds: Option<&[&[&[u8]]]>,
// ) -> std::result::Result<(), solana_program_error::ProgramError> {
//     let instruction_data = vec![CLOSE_TOKEN_ACCOUNT_DISCRIMINATOR];

//     let account_metas = vec![
//         AccountMeta::new(token_account.pubkey(), false), // token_account (mutable)
//         AccountMeta::new(destination.pubkey(), false),   // destination (mutable)
//         AccountMeta::new_readonly(authority.pubkey(), true), // authority (signer)
//     ];

//     let instruction = Instruction {
//         program_id: CTOKEN_PROGRAM_ID.into(),
//         accounts: account_metas,
//         data: instruction_data,
//     };

//     let account_infos = vec![
//         token_account.to_account_info(),
//         destination.to_account_info(),
//         authority.to_account_info(),
//     ];

//     if let Some(seeds) = signer_seeds {
//         invoke_signed(&instruction, &account_infos, seeds)?;
//     } else {
//         invoke(&instruction, &account_infos)?;
//     }

//     Ok(())
// }

// /// Generic function to process token account compression
// ///
// /// This function handles the compression of a native SPL token account into a compressed token account.
// /// It accepts signer seeds as a parameter, making it reusable for different PDA derivation schemes.
// ///
// /// # Arguments
// /// * `fee_payer` - The account paying for transaction fees
// /// * `authority` - The authority account (owner or delegate of the token account)
// /// * `compressed_token_cpi_authority` - The CPI authority for the compressed token program
// /// * `compressed_token_program` - The compressed token program
// /// * `token_account` - The SPL token account to compress
// /// * `config_account` - The compression configuration account
// /// * `rent_recipient` - The account that will receive the reclaimed rent
// /// * `remaining_accounts` - Additional accounts needed for the Light Protocol CPI
// /// * `token_signer_seeds` - The signer seeds for the token account PDA (without bump)
// ///
// /// # Returns
// /// * `Result<()>` - Success or error
// #[allow(clippy::too_many_arguments)]
// #[cfg(feature = "anchor")]
// pub fn compress_and_close_token_account<'info>(
//     program_id: Pubkey,
//     fee_payer: &Signer<'info>,
//     token_account: InterfaceAccount<'info, anchor_spl::token_interface::TokenAccount>,
//     authority: &AccountInfo<'info>,
//     compressed_token_cpi_authority: &AccountInfo<'info>,
//     compressed_token_program: &AccountInfo<'info>,
//     config_account: &AccountInfo<'info>,
//     rent_recipient: &AccountInfo<'info>,
//     remaining_accounts: &[AccountInfo<'info>],
//     cpi_signer: CpiSigner,
//     token_signer_seeds: Vec<Vec<u8>>,
// ) -> std::result::Result<(), solana_program_error::ProgramError> {
//     compress_and_close_token_accounts(
//         program_id,
//         fee_payer,
//         authority,
//         compressed_token_cpi_authority,
//         compressed_token_program,
//         config_account,
//         rent_recipient,
//         remaining_accounts,
//         vec![TokenAccountToCompress {
//             token_account,
//             signer_seeds: token_signer_seeds,
//         }],
//         cpi_signer,
//     )
// }

// /// Compress and close multiple compressible token accounts in a single
// /// instruction.
// ///
// /// All token accounts must be owned by the same program authority.
// ///
// /// # Arguments
// /// * `program_id` - The program ID that owns the token accounts
// /// * `fee_payer` - The account paying for transaction fees
// /// * `authority` - The authority account (must be the same for all token
// ///   accounts)
// /// * `compressed_token_cpi_authority` - The CPI authority for the compressed
// ///   token program
// /// * `compressed_token_program` - The compressed token program
// /// * `config_account` - The compression configuration account
// /// * `rent_recipient` - The account that will receive the reclaimed rent
// /// * `remaining_accounts` - Additional accounts needed for the Light Protocol
// ///   CPI
// /// * `token_accounts_to_compress` - Vector of token accounts with their
// ///   respective signer seeds
// /// * `cpi_signer` - The CPI signer for the program
// ///
// /// # Returns
// /// * `Result<()>` - Success or error
// #[allow(clippy::too_many_arguments)]
// #[cfg(feature = "anchor")]
// pub fn compress_and_close_token_accounts<'info>(
//     program_id: Pubkey,
//     fee_payer: &Signer<'info>,
//     authority: &AccountInfo<'info>,
//     compressed_token_cpi_authority: &AccountInfo<'info>,
//     compressed_token_program: &AccountInfo<'info>,
//     config_account: &AccountInfo<'info>,
//     rent_recipient: &AccountInfo<'info>,
//     remaining_accounts: &[AccountInfo<'info>],
//     token_accounts_to_compress: Vec<TokenAccountToCompress<'info>>,
//     cpi_signer: CpiSigner,
// ) -> std::result::Result<(), solana_program_error::ProgramError> {
//     if token_accounts_to_compress.is_empty() {
//         return Ok(());
//     }

//     // TODO: consider removing this check.
//     let config = CompressibleConfig::load_checked(config_account, &program_id)?;

//     // Verify rent recipient matches config
//     if rent_recipient.pubkey() != config.rent_recipient {
//         return Err(solana_program_error::ProgramError::InvalidAccountData);
//     }

//     let cpi_accounts = CpiAccountsSmall::new(authority, remaining_accounts, cpi_signer);

//     let mut account_metas: Vec<AccountMeta> = Vec::new();

//     // // Fee payer (index 0)
//     // account_metas.push(account_meta_from_account_info(&fee_payer.to_account_info()));

//     // Pack token accounts
//     let mut ctoken_accounts = Vec::with_capacity(token_accounts_to_compress.len());
//     for token_data in &token_accounts_to_compress {
//         let token_account = token_data.token_account.clone();

//         let seeds: Vec<&[u8]> = token_data
//             .signer_seeds
//             .iter()
//             .map(|s| s.as_slice())
//             .collect();

//         solana_msg::msg!("seeds {:?}", seeds);

//         let expected_token_account = Pubkey::create_program_address(&seeds, &program_id)
//             .map_err(|_| solana_program_error::ProgramError::InvalidSeeds)?;
//         solana_msg::msg!("expected_token_account {:?}", expected_token_account);

//         if token_account.to_account_info().key != &expected_token_account {
//             return Err(solana_program_error::ProgramError::InvalidAccountData);
//         }

//         // MERKLE TREE OUTPUT QUEUE
//         let output_queue_index = add_or_get_index(
//             &mut account_metas,
//             AccountMeta {
//                 pubkey: cpi_accounts.tree_accounts().unwrap()[0].pubkey(),
//                 is_writable: true,
//                 is_signer: false,
//             },
//         );
//         // TOKEN ACCOUNT
//         let token_account_index = add_or_get_index(
//             &mut account_metas,
//             account_meta_from_account_info(&token_account.to_account_info()),
//         );

//         // MINT
//         let mint_index = add_or_get_index(
//             &mut account_metas,
//             AccountMeta {
//                 pubkey: token_account.mint,
//                 is_writable: false,
//                 is_signer: false,
//             },
//         );

//         // AUTHORITY
//         let authority_index = add_or_get_index(
//             &mut account_metas,
//             AccountMeta {
//                 pubkey: cpi_accounts.authority().unwrap().pubkey(),
//                 is_writable: false,
//                 is_signer: true,
//             },
//         );

//         // Create the compressed token account structure
//         let ctoken_account = CTokenAccount2 {
//             inputs: vec![],
//             output: MultiTokenTransferOutputData {
//                 owner: token_account_index,
//                 amount: token_account.amount,
//                 merkle_tree: output_queue_index,
//                 mint: mint_index,
//                 version: 2,
//                 delegate: 0,
//                 has_delegate: false,
//             },
//             compression: Some(Compression {
//                 amount: token_account.amount,
//                 mode: CompressionMode::Compress,
//                 mint: mint_index,                         // Index of mint
//                 source_or_recipient: token_account_index, // Index of token account
//                 authority: authority_index,               // Index of authority
//                 pool_account_index: 0,                    // unused
//                 pool_index: 0,                            // unused
//                 bump: 0,                                  // unused
//             }),
//             delegate_is_set: false,
//             method_used: false,
//         };

//         ctoken_accounts.push(ctoken_account);
//     }

//     solana_msg::msg!("ctoken_accounts len: {:?}", ctoken_accounts.len());

//     let inputs = Transfer2Inputs {
//         validity_proof: ValidityProof(None).into(), // TODO: add
//         transfer_config: Transfer2Config::new().filter_zero_amount_outputs(),
//         meta_config: Transfer2AccountsMetaConfig::new(fee_payer.pubkey(), account_metas),
//         in_lamports: None,
//         out_lamports: None,
//         token_accounts: ctoken_accounts,
//     };
//     solana_msg::msg!("inputs BEFORE: {:?}", inputs);
//     let ctoken_ix =
//         create_transfer2_instruction(inputs).map_err(solana_program_error::ProgramError::from)?;
//     // let ctoken_ix = create_transfer2_instruction(Transfer2Inputs {
//     //     validity_proof: ValidityProof(None).into(), // TODO: add
//     //     transfer_config: Transfer2Config::new().filter_zero_amount_outputs(),
//     //     meta_config: Transfer2AccountsMetaConfig::new(fee_payer.pubkey(), account_metas),
//     //     in_lamports: None,
//     //     out_lamports: None,
//     //     token_accounts: ctoken_accounts,
//     // })
//     // .map_err(solana_program_error::ProgramError::from)?;

//     // Account Infos
//     let mut all_account_infos = vec![
//         fee_payer.to_account_info(),
//         compressed_token_cpi_authority.to_account_info(),
//         compressed_token_program.to_account_info(),
//         config_account.to_account_info(),
//     ];
//     all_account_infos.extend(cpi_accounts.to_account_infos());

//     // authority
//     let authority_seeds = &[CPI_AUTHORITY_PDA_SEED, &[cpi_signer.bump]];

//     // msg!("ctoken_ix: {:?}", ctoken_ix);
//     solana_msg::msg!("all_account_infos len: {:?}", all_account_infos.len());
//     // solana_msg::msg!("all_account_infos: {:?}", all_account_infos);
//     solana_msg::msg!("ix_data: {:?}", ctoken_ix.data);

//     invoke_signed(
//         &ctoken_ix,
//         all_account_infos.as_slice(),
//         &[authority_seeds.as_slice()],
//     )?;

//     // Clean up token accounts
//     for token_data in token_accounts_to_compress {
//         close_compressed_token_account(
//             token_data.token_account.to_account_info(),
//             rent_recipient.to_account_info(),
//             cpi_accounts.authority().unwrap().to_account_info(),
//             Some(&[authority_seeds]),
//         )?;
//     }

//     Ok(())
// }
