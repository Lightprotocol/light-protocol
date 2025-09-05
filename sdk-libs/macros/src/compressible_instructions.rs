use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Item, ItemFn, ItemStruct, ItemMod, Result, Token,
};

/// Parse a comma-separated list of account type identifiers
struct AccountTypeList {
    types: Punctuated<Ident, Token![,]>,
}

impl Parse for AccountTypeList {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(AccountTypeList {
            types: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Enhanced version of add_compressible_instructions that generates both compress and decompress instructions
/// 
/// Usage:
/// ```rust
/// #[add_compressible_instructions_enhanced(UserRecord, GameSession, PlaceholderRecord)]
/// #[program]
/// pub mod my_program {
///     // Your other instructions...
/// }
/// ```
pub fn add_compressible_instructions_enhanced(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    let type_list = syn::parse2::<AccountTypeList>(args)?;

    if module.content.is_none() {
        return Err(syn::Error::new_spanned(&module, "Module must have a body"));
    }

    let account_types: Vec<&Ident> = type_list.types.iter().collect();
    
    if account_types.is_empty() {
        return Err(syn::Error::new_spanned(&module, "At least one account type must be specified"));
    }

    let content = module.content.as_mut().unwrap();

    // Generate the DecompressAccountsIdempotent accounts struct
    let decompress_accounts: ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct DecompressAccountsIdempotent<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            /// UNCHECKED: Anyone can pay to init.
            #[account(mut)]
            pub rent_payer: Signer<'info>,
            /// The global config account
            /// CHECK: load_checked.
            pub config: AccountInfo<'info>,
            /// Compressed token program
            /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
            pub compressed_token_program: Option<UncheckedAccount<'info>>,
            /// CPI authority PDA of the compressed token program
            /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
            pub compressed_token_cpi_authority: Option<UncheckedAccount<'info>>,
        }
    };

    // Generate match arms for decompress instruction using the account types
    let decompress_match_arms = account_types.iter().map(|name| {
        let name_str = name.to_string();
        
        // Generate the appropriate seed function call based on the account type name
        let seed_call = match name_str.as_str() {
            "UserRecord" => quote! { get_user_record_seeds(&data.owner) },
            "GameSession" => quote! { get_game_session_seeds(data.session_id) },
            "PlaceholderRecord" => quote! { get_placeholder_record_seeds(data.placeholder_id) },
            _ => quote! { 
                return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
            },
        };
        
        quote! {
            CompressedAccountVariant::#name(data) => {
                let (seeds_vec, _) = #seed_call;

                let compressed_infos = light_sdk::compressible::prepare_account_for_decompression_idempotent::<#name>(
                    &crate::ID,
                    data,
                    light_sdk::compressible::into_compressed_meta_with_address(
                        &compressed_data.meta,
                        &solana_accounts[i],
                        address_space,
                        &crate::ID,
                    ),
                    &solana_accounts[i],
                    &ctx.accounts.rent_payer,
                    &cpi_accounts,
                    seeds_vec
                        .iter()
                        .map(|v| v.as_slice())
                        .collect::<Vec<&[u8]>>()
                        .as_slice(),
                )?;
                compressed_pda_infos.extend(compressed_infos);
            }
        }
    });

    // Generate the decompress instruction
    let decompress_instruction: ItemFn = syn::parse_quote! {
        /// Auto-generated decompress_accounts_idempotent instruction
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            let compression_config = light_sdk::compressible::CompressibleConfig::load_checked(
                &ctx.accounts.config,
                &crate::ID,
            )?;
            let address_space = compression_config.address_space[0];

            let (mut has_tokens, mut has_pdas) = (false, false);
            for c in &compressed_accounts {
                match c.data {
                    CompressedAccountVariant::CompressibleTokenAccountPacked(_) => {
                        has_tokens = true;
                    }
                    _ => has_pdas = true,
                }
                if has_tokens && has_pdas {
                    break;
                }
            }

            let cpi_accounts = if has_tokens && has_pdas {
                light_sdk_types::CpiAccountsSmall::new_with_config(
                    ctx.accounts.fee_payer.as_ref(),
                    &ctx.remaining_accounts[system_accounts_offset as usize..],
                    light_sdk_types::CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
                )
            } else {
                light_sdk_types::CpiAccountsSmall::new(
                    ctx.accounts.fee_payer.as_ref(),
                    &ctx.remaining_accounts[system_accounts_offset as usize..],
                    LIGHT_CPI_SIGNER,
                )
            };

            let pda_accounts_start = ctx.remaining_accounts.len() - compressed_accounts.len();
            let solana_accounts = &ctx.remaining_accounts[pda_accounts_start..];

            let mut compressed_token_accounts = Vec::new();
            let mut compressed_pda_infos = Vec::new();

            for (i, compressed_data) in compressed_accounts.clone().into_iter().enumerate() {
                let unpacked_data = compressed_data
                    .data
                    .unpack(cpi_accounts.post_system_accounts().unwrap())?;

                match unpacked_data {
                    #(#decompress_match_arms)*
                    CompressedAccountVariant::PackedUserRecord(_) => {
                        unreachable!();
                    }
                    CompressedAccountVariant::PackedGameSession(_) => {
                        unreachable!();
                    }
                    CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                        unreachable!();
                    }
                    CompressedAccountVariant::CompressibleTokenAccountPacked(data) => {
                        compressed_token_accounts.push((data, compressed_data.meta));
                    }
                    CompressedAccountVariant::CompressibleTokenData(_) => {
                        unreachable!();
                    }
                }
            }

            let has_pdas = !compressed_pda_infos.is_empty();
            let has_tokens = !compressed_token_accounts.is_empty();

            if !has_pdas && !has_tokens {
                msg!("All accounts already initialized.");
                return Ok(());
            }

            let fee_payer = ctx.accounts.fee_payer.as_ref();
            let authority = cpi_accounts.authority().unwrap();
            let cpi_context = cpi_accounts.cpi_context().unwrap();

            if has_pdas && has_tokens {
                let system_cpi_accounts = light_sdk_types::cpi_context_write::CpiContextWriteAccounts {
                    fee_payer,
                    authority,
                    cpi_context,
                    cpi_signer: LIGHT_CPI_SIGNER,
                };

                let cpi_inputs = light_sdk::cpi::CpiInputs::new_first_cpi(
                    compressed_pda_infos,
                    Vec::new(),
                );
                cpi_inputs.invoke_light_system_program_cpi_context(system_cpi_accounts)?;
            } else if has_pdas {
                let cpi_inputs = light_sdk::cpi::CpiInputs::new(proof, compressed_pda_infos);
                cpi_inputs.invoke_light_system_program_small(cpi_accounts.clone())?;
            }

            // Handle token account decompression
            let mut token_decompress_indices = Vec::new();
            let mut token_signers_seeds = Vec::new();
            let packed_accounts = cpi_accounts.post_system_accounts().unwrap();

            for (token_data, meta) in compressed_token_accounts.into_iter() {
                let owner_index: u8 = token_data.token_data.owner;
                let mint_index: u8 = token_data.token_data.mint;

                let mint_info = packed_accounts[mint_index as usize].to_account_info();
                let owner_info = packed_accounts[owner_index as usize].to_account_info();

                // seeds for ctoken. match on variant.
                let ctoken_signer_seeds = match token_data.variant {
                    CTokenAccountVariant::CTokenSigner => {
                        let (seeds, _) = get_ctoken_signer_seeds(&fee_payer.key(), &mint_info.key());
                        seeds
                    }
                    CTokenAccountVariant::AssociatedTokenAccount => unreachable!(),
                };

                light_compressed_token_sdk::create_compressible_token_account(
                    authority,
                    fee_payer,
                    &owner_info,
                    &mint_info,
                    cpi_accounts.system_program().unwrap(),
                    ctx.accounts.compressed_token_program.as_ref().unwrap(),
                    &ctoken_signer_seeds
                        .iter()
                        .map(|s| s.as_slice())
                        .collect::<Vec<&[u8]>>(),
                    fee_payer, // rent_auth
                    fee_payer, // rent_recipient
                    0,         // slots_until_compression
                )?;

                let decompress_index = light_compressed_token_sdk::instructions::DecompressFullIndices::from((token_data.token_data, meta, owner_index));

                token_decompress_indices.push(decompress_index);
                token_signers_seeds.extend(ctoken_signer_seeds);
            }

            if has_tokens {
                let ctoken_ix = light_compressed_token_sdk::instructions::decompress_full_ctoken_accounts_with_indices(
                    fee_payer.key(),
                    proof,
                    if has_pdas {
                        Some(cpi_context.key())
                    } else {
                        None
                    },
                    &token_decompress_indices,
                    packed_accounts,
                )
                .map_err(anchor_lang::prelude::ProgramError::from)?;

                let mut all_account_infos = vec![fee_payer.to_account_info()];
                all_account_infos.extend(
                    ctx.accounts
                        .compressed_token_cpi_authority
                        .to_account_infos(),
                );
                all_account_infos.extend(ctx.accounts.compressed_token_program.to_account_infos());
                all_account_infos.extend(ctx.accounts.rent_payer.to_account_infos());
                all_account_infos.extend(ctx.accounts.config.to_account_infos());
                all_account_infos.extend(cpi_accounts.to_account_infos());

                let seed_refs = token_signers_seeds
                    .iter()
                    .map(|s| s.as_slice())
                    .collect::<Vec<&[u8]>>();
                anchor_lang::solana_program::program::invoke_signed(
                    &ctoken_ix,
                    all_account_infos.as_slice(),
                    &[seed_refs.as_slice()],
                )?;
            }
            Ok(())
        }
    };

    // Add the generated items to the module
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Fn(decompress_instruction));

    Ok(quote! {
        #module
    })
}
