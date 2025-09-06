use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, Ident, Item, ItemFn, ItemStruct, ItemMod, LitStr, Result, Token,
};

/// Parse seed specification for a token account variant
struct TokenSeedSpec {
    variant: Ident,
    _eq: Token![=],
    is_token: Option<bool>, // Optional explicit token flag
    seeds: Punctuated<SeedElement, Token![,]>,
}

impl Parse for TokenSeedSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let variant = input.parse()?;
        let _eq = input.parse()?;
        
        let content;
        syn::parenthesized!(content in input);
        
        // Check if first element is an explicit token flag
        let (is_token, seeds) = if content.peek(Ident) {
            let first_ident: Ident = content.parse()?;
            
            match first_ident.to_string().as_str() {
                "is_token" | "true" => {
                    // Explicit token flag
                    let _comma: Token![,] = content.parse()?;
                    let seeds = Punctuated::parse_terminated(&content)?;
                    (Some(true), seeds)
                }
                "is_pda" | "false" => {
                    // Explicit PDA flag  
                    let _comma: Token![,] = content.parse()?;
                    let seeds = Punctuated::parse_terminated(&content)?;
                    (Some(false), seeds)
                }
                _ => {
                    // Not a flag, treat as first seed element
                    let mut seeds = Punctuated::new();
                    seeds.push(SeedElement::Expression(syn::Expr::Path(syn::ExprPath {
                        attrs: vec![],
                        qself: None,
                        path: syn::Path::from(first_ident),
                    })));
                    
                    if content.peek(Token![,]) {
                        let _comma: Token![,] = content.parse()?;
                        let rest: Punctuated<SeedElement, Token![,]> = Punctuated::parse_terminated(&content)?;
                        seeds.extend(rest);
                    }
                    
                    (None, seeds)
                }
            }
        } else {
            // No identifier first, parse all as seeds
            let seeds = Punctuated::parse_terminated(&content)?;
            (None, seeds)
        };
        
        Ok(TokenSeedSpec {
            variant,
            _eq,
            is_token,
            seeds,
        })
    }
}

enum SeedElement {
    /// String literal like "user_record"
    Literal(LitStr),
    /// Any expression: data.owner, ctx.fee_payer, data.session_id.to_le_bytes(), etc.
    Expression(Expr),
}

impl Parse for SeedElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(SeedElement::Literal(input.parse()?))
        } else {
            // Parse everything else as an expression
            // This will handle ctx.fee_payer, data.session_id.to_le_bytes(), etc.
            Ok(SeedElement::Expression(input.parse()?))
        }
    }
}

/// Parse instruction data field specification: field_name = Type
struct InstructionDataSpec {
    field_name: Ident,
    field_type: syn::Type,
}

impl Parse for InstructionDataSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse: field_name = Type (e.g., session_id = u64)
        let field_name: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let field_type: syn::Type = input.parse()?;
        
        Ok(InstructionDataSpec {
            field_name,
            field_type,
        })
    }
}

/// Parse enhanced macro arguments with mixed account types, PDA seeds, token seeds, and instruction data
struct EnhancedMacroArgs {
    account_types: Vec<Ident>,
    pda_seeds: Vec<TokenSeedSpec>,
    token_seeds: Vec<TokenSeedSpec>,
    instruction_data: Vec<InstructionDataSpec>,
}

impl Parse for EnhancedMacroArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut account_types = Vec::new();
        let mut pda_seeds = Vec::new();
        let mut token_seeds = Vec::new();
        let mut instruction_data = Vec::new();
        
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            
            if input.peek(Token![=]) {
                let _eq: Token![=] = input.parse()?;
                
                if input.peek(syn::token::Paren) {
                    // This is a seed specification (either PDA or CToken)
                    let content;
                    syn::parenthesized!(content in input);
                    
                    // Check for explicit token flag as first element
                    let (is_token_explicit, seeds) = if content.peek(Ident) {
                        let first_ident: Ident = content.parse()?;
                        
                        if first_ident == "is_token" {
                            let _comma: Token![,] = content.parse()?;
                            let seeds = Punctuated::parse_terminated(&content)?;
                            (Some(true), seeds)
                        } else if first_ident == "is_pda" {
                            let _comma: Token![,] = content.parse()?;
                            let seeds = Punctuated::parse_terminated(&content)?;
                            (Some(false), seeds)
                        } else {
                            // Not a flag, treat as first seed element
                            let mut seeds = Punctuated::new();
                            seeds.push(SeedElement::Expression(syn::Expr::Path(syn::ExprPath {
                                attrs: vec![],
                                qself: None,
                                path: syn::Path::from(first_ident),
                            })));
                            
                            if content.peek(Token![,]) {
                                let _comma: Token![,] = content.parse()?;
                                let rest: Punctuated<SeedElement, Token![,]> = Punctuated::parse_terminated(&content)?;
                                seeds.extend(rest);
                            }
                            
                            (None, seeds)
                        }
                    } else {
                        // No identifier first, parse all as seeds
                        let seeds = Punctuated::parse_terminated(&content)?;
                        (None, seeds)
                    };
                    
                    let seed_spec = TokenSeedSpec {
                        variant: ident.clone(),
                        _eq: Token![=]([proc_macro2::Span::call_site()]),
                        is_token: is_token_explicit,
                        seeds,
                    };
                    
                    let is_token_account = is_token_explicit.unwrap_or_else(|| {
                        // Default to PDA if no explicit flag provided
                        false
                    });
                    
                    if is_token_account {
                        token_seeds.push(seed_spec);
                    } else {
                        // This is a PDA seed specification
                        pda_seeds.push(seed_spec);
                        account_types.push(ident);
                    }
                } else {
                    // This is an instruction data type specification: field_name = Type
                    let field_type: syn::Type = input.parse()?;
                    instruction_data.push(InstructionDataSpec {
                        field_name: ident,
                        field_type,
                    });
                }
            } else {
                // This is a regular account type without seed specification
                account_types.push(ident);
            }
            
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            } else {
                break;
            }
        }

        Ok(EnhancedMacroArgs {
            account_types,
            pda_seeds,
            token_seeds,
            instruction_data,
        })
    }
}

// Legacy parsing removed - only declarative syntax supported now! 🎉

/// Enhanced version of add_compressible_instructions that generates both compress and decompress instructions
/// 
/// Now supports automatic CToken seed derivation:
/// - Specify token seeds directly in the macro
/// - Eliminates need for manual CTokenSeedProvider implementation
/// - Completely automatic seed generation
/// 
/// Usage:
/// ```rust
/// #[add_compressible_instructions(
///     MyAccount = ("my_account", data.field),
///     AnotherAccount = ("another", data.id.to_le_bytes()),
///     MyToken = (is_token, "my_token", ctx.fee_payer, ctx.mint),
///     field = Pubkey,
///     id = u64
/// )]
/// #[program]
/// pub mod my_program {
///     // Your other instructions...
/// }
/// ```
///
/// ## Explicit Token/PDA Flags:
/// - Use `is_token` as first element for token accounts (REQUIRED for tokens!)
/// - Use `is_pda` as first element for PDA accounts (optional, defaults to PDA)
/// - NO naming convention fallbacks - be explicit!
pub fn add_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    // Parse with enhanced format - no legacy fallback!
    let enhanced_args = syn::parse2::<EnhancedMacroArgs>(args)?;
    let account_types = enhanced_args.account_types;
    let pda_seeds = Some(enhanced_args.pda_seeds);
    let token_seeds = Some(enhanced_args.token_seeds);
    let instruction_data = enhanced_args.instruction_data;

    if module.content.is_none() {
        return Err(syn::Error::new_spanned(&module, "Module must have a body"));
    }
    
    if account_types.is_empty() {
        return Err(syn::Error::new_spanned(&module, "At least one account type must be specified"));
    }

    let content = module.content.as_mut().unwrap();

    // Generate the CTokenAccountVariant enum automatically from token_seeds
    let ctoken_enum = if let Some(ref token_seed_specs) = token_seeds {
        if !token_seed_specs.is_empty() {
            generate_ctoken_account_variant_enum(token_seed_specs)?
        } else {
            quote! {
                // No CToken variants - generate empty enum for compatibility
                #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
                #[repr(u8)]
                pub enum CTokenAccountVariant {}
            }
        }
    } else {
        quote! {
            // No CToken variants - generate empty enum for compatibility
            #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
            #[repr(u8)]
            pub enum CTokenAccountVariant {}
        }
    };

    // Generate the compressed_account_variant enum automatically
    let mut account_types_stream = TokenStream::new();
    for (i, account_type) in account_types.iter().enumerate() {
        if i > 0 {
            account_types_stream.extend(quote! { , });
        }
        account_types_stream.extend(quote! { #account_type });
    }
    let enum_and_traits = crate::variant_enum::compressed_account_variant(account_types_stream)?;

    // Extract required accounts from seed expressions
    let required_accounts = extract_required_accounts_from_seeds(&pda_seeds, &token_seeds)?;

    // Generate the DecompressAccountsIdempotent accounts struct with required accounts
    let decompress_accounts = generate_decompress_accounts_struct(&required_accounts)?;

    // Generate match arms for decompress instruction using the account types
    let decompress_match_arms: Result<Vec<_>> = account_types.iter().map(|name| {
        let name_str = name.to_string();
        
        // Generate seed derivation from PDA seed specification - NO FALLBACKS!
        let seed_call = if let Some(ref pda_seed_specs) = pda_seeds {
            if let Some(spec) = pda_seed_specs.iter().find(|s| s.variant.to_string() == name_str) {
                // Generate dynamic seed derivation from the specification
                generate_pda_seed_derivation(spec, &instruction_data)?
            } else {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("No seed specification provided for account type '{}'. All accounts must have seed specifications.", name_str)
                ))
            }
        } else {
            return Err(syn::Error::new_spanned(
                name,
                "No seed specifications provided. Use the new syntax: AccountType = (\"seed\", data.field)"
            ))
        };
        
        Ok(quote! {
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
        })
    }).collect();
    let decompress_match_arms = decompress_match_arms?;

    // Generate unreachable match arms for Packed variants (PDA types are unpacked, not packed)
    let packed_unreachable_arms = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#packed_name(_) => {
                unreachable!();
            }
        }
    });

    // Generate trait-based system for TRULY generic CToken variant handling
    let ctoken_trait_system: syn::ItemMod = syn::parse_quote! {
        /// Trait-based system for generic CToken variant seed handling
        /// Users implement this trait for their CTokenAccountVariant enum
        pub mod ctoken_seed_system {
            use super::*;
            
            /// Context struct providing access to ALL instruction accounts
            /// This gives users access to any account in the instruction context
            pub struct CTokenSeedContext<'a, 'info> {
                pub accounts: &'a DecompressAccountsIdempotent<'info>,
                pub remaining_accounts: &'a [anchor_lang::prelude::AccountInfo<'info>],
                pub fee_payer: &'a Pubkey,
                pub owner: &'a Pubkey,
                pub mint: &'a Pubkey,
                // Users can access any account via ctx.accounts.field_name
            }
            
            /// Trait that CToken variants implement to provide seed derivation
            /// Completely extensible - users can implement ANY seed logic with access to ALL accounts
            pub trait CTokenSeedProvider {
                fn get_seeds<'a, 'info>(&self, ctx: &CTokenSeedContext<'a, 'info>) -> (Vec<Vec<u8>>, Pubkey);
            }
        }
    };

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
                    #(#packed_unreachable_arms)*
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

                // ✅ TRULY GENERIC CToken variant handling using trait dispatch
                // Users get access to ALL instruction accounts via ctx.accounts
                // NO NEED TO MODIFY THE MACRO - completely extensible by users
                use crate::ctoken_seed_system::{CTokenSeedProvider, CTokenSeedContext};
                
                let seed_context = CTokenSeedContext {
                    accounts: &ctx.accounts,
                    remaining_accounts: ctx.remaining_accounts,
                    fee_payer: &fee_payer.key(),
                    owner: &owner_info.key(), 
                    mint: &mint_info.key(),
                };
                
                let ctoken_signer_seeds = token_data.variant.get_seeds(&seed_context).0;

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

    // Generate the CompressAccountsIdempotent accounts struct
    let compress_accounts: syn::ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct CompressAccountsIdempotent<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            /// The global config account
            /// CHECK: Config is validated by the SDK's load_checked method
            pub config: AccountInfo<'info>,
            /// Rent recipient - must match config
            /// CHECK: Rent recipient is validated against the config
            #[account(mut)]
            pub rent_recipient: AccountInfo<'info>,

            /// CHECK: compression_authority must be the rent_authority defined when creating the token account.
            #[account(mut)]
            pub token_compression_authority: AccountInfo<'info>,

            // Optional token-specific accounts (only needed when compressing token accounts)
            /// Compressed token program
            /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
            pub compressed_token_program: Option<UncheckedAccount<'info>>,

            /// CPI authority PDA of the compressed token program
            /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
            pub compressed_token_cpi_authority: Option<UncheckedAccount<'info>>,
        }
    };

    // Generate compress match arms for each account type with dedicated vectors
    let compress_match_arms = account_types.iter().map(|name| {
        quote! {
            d if d == #name::discriminator() => {
                let mut anchor_account = anchor_lang::prelude::Account::<#name>::try_from(account_info)?;

                let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<#name>(
                    &crate::ID,
                    &mut anchor_account,
                    &meta,
                    &cpi_accounts,
                    &compression_config.compression_delay,
                    &compression_config.address_space,
                )?;

                // Store in type-specific vector for proper closing
                #name.push(anchor_account);
                compressed_pda_infos.push(compressed_info);
            }
        }
    });

    // Generate the compress instruction
    let compress_instruction: syn::ItemFn = syn::parse_quote! {
        /// Auto-generated compress_accounts_idempotent instruction
        pub fn compress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, 'info, 'info, CompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            signer_seeds: Vec<Vec<Vec<u8>>>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            let compression_config = light_sdk::compressible::CompressibleConfig::load_checked(
                &ctx.accounts.config,
                &crate::ID,
            )?;
            if ctx.accounts.rent_recipient.key() != compression_config.rent_recipient {
                return err!(ErrorCode::InvalidRentRecipient);
            }

            let cpi_accounts = light_sdk_types::CpiAccountsSmall::new(
                ctx.accounts.fee_payer.as_ref(),
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                LIGHT_CPI_SIGNER,
            );

            // We use signer_seeds because compressed_accounts can be != accounts to compress
            let pda_accounts_start = ctx.remaining_accounts.len() - signer_seeds.len();
            let solana_accounts = &ctx.remaining_accounts[pda_accounts_start..];

            // Initialize collections for different account types
            let mut token_accounts_to_compress = Vec::new();
            let mut compressed_pda_infos = Vec::new();
            // Create dedicated vectors for each account type for proper closing
            #(let mut #account_types = Vec::new();)*

            for (i, account_info) in solana_accounts.iter().enumerate() {
                if account_info.data_is_empty() {
                    msg!("No data. Account already compressed or uninitialized. Skipping.");
                    continue;
                }
                if account_info.owner == &light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID.into() {
                    if let Ok(token_account) = anchor_lang::prelude::InterfaceAccount::<anchor_spl::token_interface::TokenAccount>::try_from(account_info) {
                        let account_signer_seeds = signer_seeds[i].clone();
                        token_accounts_to_compress.push(
                            light_compressed_token_sdk::TokenAccountToCompress {
                                token_account,
                                signer_seeds: account_signer_seeds,
                            },
                        );
                    }
                } else if account_info.owner == &crate::ID {
                    let data = account_info.try_borrow_data()?;
                    let discriminator = &data[0..8];
                    let meta = compressed_accounts[i];

                    // Generic PDA account handling
                    match discriminator {
                        #(#compress_match_arms)*
                        _ => {
                            panic!("Trying to compress with invalid account discriminator");
                        }
                    }
                }
            }

            let has_pdas = !compressed_pda_infos.is_empty();
            let has_tokens = !token_accounts_to_compress.is_empty();

            // 1. Compress and close token accounts in one CPI (no proof)
            if has_tokens {
                light_compressed_token_sdk::compress_and_close_token_accounts(
                    crate::ID,
                    &ctx.accounts.fee_payer,
                    cpi_accounts.authority().unwrap(),
                    ctx.accounts.compressed_token_cpi_authority.as_ref().unwrap(),
                    ctx.accounts.compressed_token_program.as_ref().unwrap(),
                    &ctx.accounts.config,
                    &ctx.accounts.rent_recipient,
                    ctx.remaining_accounts,
                    token_accounts_to_compress,
                    LIGHT_CPI_SIGNER,
                )?;
            }
            
            // 2. Compress and close PDAs in another CPI (with proof)
            if has_pdas {
                let cpi_inputs = light_sdk::cpi::CpiInputs::new(proof, compressed_pda_infos);
                cpi_inputs.invoke_light_system_program_small(cpi_accounts)?;
            }

            // Close all PDA accounts using Anchor's proper close method
            #(
                for anchor_account in #account_types.iter() {
                    anchor_account.close(ctx.accounts.rent_recipient.clone())?;
                }
            )*

            Ok(())
        }
    };

    // Generate compression config instructions (same as old add_compressible_instructions macro)
    let init_config_accounts: syn::ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct InitializeCompressionConfig<'info> {
            #[account(mut)]
            pub payer: Signer<'info>,
            /// CHECK: Config PDA is created and validated by the SDK
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// The program's data account
            /// CHECK: Program data account is validated by the SDK
            pub program_data: AccountInfo<'info>,
            /// The program's upgrade authority (must sign)
            pub authority: Signer<'info>,
            pub system_program: Program<'info, System>,
        }
    };

    let update_config_accounts: syn::ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct UpdateCompressionConfig<'info> {
            /// CHECK: config account is validated by the SDK
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// CHECK: authority must be the current update authority
            pub authority: Signer<'info>,
        }
    };

    let init_config_instruction: syn::ItemFn = syn::parse_quote! {
        /// Initialize compression config for the program
        pub fn initialize_compression_config<'info>(
            ctx: Context<'_, '_, '_, 'info, InitializeCompressionConfig<'info>>,
            compression_delay: u32,
            rent_recipient: Pubkey,
            address_space: Vec<Pubkey>,
        ) -> Result<()> {
            light_sdk::compressible::process_initialize_compression_config_checked(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.program_data.to_account_info(),
                &rent_recipient,
                address_space,
                compression_delay,
                0, // one global config for now, so bump is 0.
                &ctx.accounts.payer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &crate::ID,
            ).map_err(|e| anchor_lang::error::Error::from(e))
        }
    };

    let update_config_instruction: syn::ItemFn = syn::parse_quote! {
        /// Update compression config for the program
        pub fn update_compression_config<'info>(
            ctx: Context<'_, '_, '_, 'info, UpdateCompressionConfig<'info>>,
            new_compression_delay: Option<u32>,
            new_rent_recipient: Option<Pubkey>,
            new_address_space: Option<Vec<Pubkey>>,
            new_update_authority: Option<Pubkey>,
        ) -> Result<()> {
            light_sdk::compressible::process_update_compression_config(
                ctx.accounts.config.as_ref(),
                ctx.accounts.authority.as_ref(),
                new_update_authority.as_ref(),
                new_rent_recipient.as_ref(),
                new_address_space,
                new_compression_delay,
                &crate::ID,
            ).map_err(|e| anchor_lang::error::Error::from(e))
        }
    };

    // Add all generated items to the module
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(Item::Struct(compress_accounts));
    content.1.push(Item::Fn(compress_instruction));
    content.1.push(Item::Struct(init_config_accounts));
    content.1.push(Item::Struct(update_config_accounts));
    content.1.push(Item::Fn(init_config_instruction));
    content.1.push(Item::Fn(update_config_instruction));

    // Generate automatic CTokenSeedProvider implementation
    let ctoken_implementation = if let Some(ref seeds) = token_seeds {
        if !seeds.is_empty() {
            generate_ctoken_seed_provider_implementation(seeds)?
        } else {
            quote! {
                // No CToken variants specified - implementation not needed
            }
        }
    } else {
        quote! {
            // No CToken variants specified - implementation not needed
        }
    };

    // Generate public client-side seed functions for external consumption
    let client_seed_functions = generate_client_seed_functions(&account_types, &pda_seeds, &token_seeds, &instruction_data)?;

    Ok(quote! {
        // Auto-generated CTokenAccountVariant enum
        #ctoken_enum
        
        // Auto-generated CompressedAccountVariant enum and traits
        #enum_and_traits
        
        // Auto-generated public seed functions for client consumption
        #client_seed_functions
        
        // Generate the trait system OUTSIDE the module so users can implement it
        #ctoken_trait_system
        
        // Auto-generated CTokenSeedProvider implementation
        #ctoken_implementation
        
        // Suppress snake_case warnings for account type names in macro usage
        #[allow(non_snake_case)]
        #module
    })
}

/// Generate CTokenAccountVariant enum automatically from token seed specifications
fn generate_ctoken_account_variant_enum(token_seeds: &[TokenSeedSpec]) -> Result<TokenStream> {
    let variants = token_seeds.iter().enumerate().map(|(index, spec)| {
        let variant_name = &spec.variant;
        let index_u8 = index as u8;
        quote! {
            #variant_name = #index_u8,
        }
    });

    Ok(quote! {
        /// Auto-generated CTokenAccountVariant enum from token seed specifications
        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        #[repr(u8)]
        pub enum CTokenAccountVariant {
            #(#variants)*
        }
    })
}

/// Generate CTokenSeedProvider implementation from token seed specifications
fn generate_ctoken_seed_provider_implementation(
    token_seeds: &[TokenSeedSpec],
) -> Result<TokenStream> {
    let mut match_arms = Vec::new();

    for spec in token_seeds {
        let variant_name = &spec.variant;
        
        // Generate bindings for any expressions that need them
        let mut bindings = Vec::new();
        let mut seed_refs = Vec::new();
        
        for (i, seed) in spec.seeds.iter().enumerate() {
            match seed {
                SeedElement::Literal(lit) => {
                    let value = lit.value();
                    seed_refs.push(quote! { #value.as_bytes() });
                }
                SeedElement::Expression(expr) => {
                    // For CToken seeds, we need to handle account references specially
                    // ctx.accounts.mint -> ctx.accounts.mint.key().as_ref()
                    let mut handled = false;
                    
                    match expr {
                        syn::Expr::Field(field_expr) => {
                            // Check if this is ctx.accounts.field_name
                            if let syn::Member::Named(field_name) = &field_expr.member {
                                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                    if let syn::Member::Named(base_name) = &nested_field.member {
                                        if base_name == "accounts" {
                                            if let syn::Expr::Path(path) = &*nested_field.base {
                                                if let Some(segment) = path.path.segments.first() {
                                                    if segment.ident == "ctx" {
                                                        // This is ctx.accounts.field_name
                                                        // In CTokenSeedContext, accounts are accessed via ctx.accounts.field_name
                                                        let binding_name = syn::Ident::new(&format!("seed_{}", i), expr.span());
                                                        bindings.push(quote! {
                                                            let #binding_name = ctx.accounts.#field_name.key().to_bytes();
                                                        });
                                                        seed_refs.push(quote! { &#binding_name });
                                                        handled = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if let syn::Expr::Path(path) = &*field_expr.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "ctx" {
                                            // This is ctx.field_name
                                            let field_str = field_name.to_string();
                                            
                                            // Check if it's a standard CTokenSeedContext field
                                            if field_str == "fee_payer" || field_str == "owner" || field_str == "mint" {
                                                // Standard field - use directly from ctx
                                                let binding_name = syn::Ident::new(&format!("seed_{}", i), expr.span());
                                                bindings.push(quote! {
                                                    let #binding_name = ctx.#field_name.to_bytes();
                                                });
                                                seed_refs.push(quote! { &#binding_name });
                                            } else {
                                                // Custom field - access via ctx.accounts
                                                let binding_name = syn::Ident::new(&format!("seed_{}", i), expr.span());
                                                bindings.push(quote! {
                                                    let #binding_name = ctx.accounts.#field_name.key().to_bytes();
                                                });
                                                seed_refs.push(quote! { &#binding_name });
                                            }
                                            handled = true;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    
                    if !handled {
                        // Not a ctx.accounts reference, use as-is
                        seed_refs.push(quote! { (#expr).as_ref() });
                    }
                }
            }
        }

        let match_arm = quote! {
            CTokenAccountVariant::#variant_name => {
                #(#bindings)*
                let seeds: &[&[u8]] = &[#(#seed_refs),*];
                let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                let seeds_vec = seeds.iter().map(|s| s.to_vec()).collect::<Vec<_>>();
                let mut seeds_vec = seeds_vec;
                seeds_vec.push(vec![bump]);
                (seeds_vec, pda)
            }
        };
        match_arms.push(match_arm);
    }

    Ok(quote! {
        /// Auto-generated CTokenSeedProvider implementation
        impl ctoken_seed_system::CTokenSeedProvider for CTokenAccountVariant {
            fn get_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                match self {
                    #(#match_arms)*
                    _ => {
                        unreachable!("CToken variant not configured with seeds")
                    }
                }
            }
        }
    })
}

/// Generate seed expressions from SeedElement specifications
fn generate_seed_expressions(
    seeds: &Punctuated<SeedElement, Token![,]>,
) -> Result<Vec<TokenStream>> {
    let mut expressions = Vec::new();

    for seed in seeds {
        let expr = match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                quote! { #value.as_bytes() }
            }
            SeedElement::Expression(expr) => {
                // Handle ctx.accounts.field_name specially
                match expr {
                    syn::Expr::Field(field_expr) => {
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                if let syn::Member::Named(base_name) = &nested_field.member {
                                    if base_name == "accounts" {
                                        if let syn::Expr::Path(path) = &*nested_field.base {
                                            if let Some(segment) = path.path.segments.first() {
                                                if segment.ident == "ctx" {
                                                    // This is ctx.accounts.field_name - convert to key
                                                    quote! { ctx.accounts.#field_name.key().as_ref() }
                                                } else {
                                                    quote! { (#expr).as_ref() }
                                                }
                                            } else {
                                                quote! { (#expr).as_ref() }
                                            }
                                        } else {
                                            quote! { (#expr).as_ref() }
                                        }
                                    } else {
                                        quote! { (#expr).as_ref() }
                                    }
                                } else {
                                    quote! { (#expr).as_ref() }
                                }
                            } else if let syn::Expr::Path(path) = &*field_expr.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        // This is ctx.field_name - convert to key
                                        quote! { ctx.accounts.#field_name.key().as_ref() }
                                    } else {
                                        quote! { (#expr).as_ref() }
                                    }
                                } else {
                                    quote! { (#expr).as_ref() }
                                }
                            } else {
                                quote! { (#expr).as_ref() }
                            }
                        } else {
                            quote! { (#expr).as_ref() }
                        }
                    }
                    syn::Expr::Path(path_expr) => {
                        if let Some(ident) = path_expr.path.get_ident() {
                            // This is a direct account reference - convert to key
                            quote! { ctx.accounts.#ident.key().as_ref() }
                        } else {
                            quote! { (#expr).as_ref() }
                        }
                    }
                    _ => {
                        quote! { (#expr).as_ref() }
                    }
                }
            }
        };
        expressions.push(expr);
    }

    Ok(expressions)
}

/// Generate seed expressions with proper type handling
fn generate_seed_expressions_with_types(
    seeds: &Punctuated<SeedElement, Token![,]>,
    instruction_data: &[InstructionDataSpec],
) -> Result<Vec<TokenStream>> {
    let mut expressions = Vec::new();

    for seed in seeds {
        let expr = match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                quote! { #value.as_bytes() }
            }
            SeedElement::Expression(expr) => {
                match expr {
                    syn::Expr::Field(field_expr) => {
                        // Handle ctx.accounts.field_name, ctx.field_name, data.field
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            match &*field_expr.base {
                                syn::Expr::Field(nested_field) => {
                                    // Handle ctx.accounts.field_name
                                    if let syn::Member::Named(base_name) = &nested_field.member {
                                        if base_name == "accounts" {
                                            if let syn::Expr::Path(path) = &*nested_field.base {
                                                if let Some(segment) = path.path.segments.first() {
                                                    if segment.ident == "ctx" {
                                                        // This is ctx.accounts.field_name
                                                        quote! { ctx.accounts.#field_name.key().as_ref() }
                                                    } else {
                                                        quote! { (#expr).as_ref() }
                                                    }
                                                } else {
                                                    quote! { (#expr).as_ref() }
                                                }
                                            } else {
                                                quote! { (#expr).as_ref() }
                                            }
                                        } else {
                                            quote! { (#expr).as_ref() }
                                        }
                                    } else {
                                        quote! { (#expr).as_ref() }
                                    }
                                }
                                syn::Expr::Path(path) => {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "ctx" {
                                            // This is ctx.field_name
                                            quote! { ctx.accounts.#field_name.key().as_ref() }
                                        } else if segment.ident == "data" {
                                            // This is data.field - check type from instruction_data
                                            if let Some(data_spec) = instruction_data.iter().find(|d| d.field_name == *field_name) {
                                                if is_pubkey_type(&data_spec.field_type) {
                                                    quote! { data.#field_name.as_ref() }
                                                } else {
                                                    // Numeric type needs to_le_bytes
                                                    quote! { data.#field_name.to_le_bytes().as_ref() }
                                                }
                                            } else {
                                                // Default to as_ref if type not found
                                                quote! { data.#field_name.as_ref() }
                                            }
                                        } else {
                                            // Other
                                            quote! { (#expr).as_ref() }
                                        }
                                    } else {
                                        quote! { (#expr).as_ref() }
                                    }
                                }
                                _ => {
                                    quote! { (#expr).as_ref() }
                                }
                            }
                        } else {
                            quote! { (#expr).as_ref() }
                        }
                    }
                    syn::Expr::Path(path_expr) => {
                        // Handle direct account references
                        if let Some(ident) = path_expr.path.get_ident() {
                            // This is a direct account reference
                            quote! { ctx.accounts.#ident.key().as_ref() }
                        } else {
                            quote! { (#expr).as_ref() }
                        }
                    }
                    _ => {
                        quote! { (#expr).as_ref() }
                    }
                }
            }
        };
        expressions.push(expr);
    }

    Ok(expressions)
}

/// Generate PDA seed derivation from specification
fn generate_pda_seed_derivation(spec: &TokenSeedSpec, _instruction_data: &[InstructionDataSpec]) -> Result<TokenStream> {
    // First, generate bindings for any expressions that need them
    let mut bindings = Vec::new();
    let mut seed_refs = Vec::new();
    
    for (i, seed) in spec.seeds.iter().enumerate() {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                seed_refs.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                // We need to handle different types of expressions differently
                let mut handled = false;
                
                // Check for expressions that need special handling
                match expr {
                    syn::Expr::MethodCall(mc) if mc.method == "to_le_bytes" => {
                        // This creates a temporary array, needs binding
                        let binding_name = syn::Ident::new(&format!("seed_binding_{}", i), expr.span());
                        bindings.push(quote! {
                            let #binding_name = #expr;
                        });
                        seed_refs.push(quote! { #binding_name.as_ref() });
                        handled = true;
                    }
                    syn::Expr::Field(field_expr) => {
                        // Check if this is ctx.accounts.field_name
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                if let syn::Member::Named(base_name) = &nested_field.member {
                                    if base_name == "accounts" {
                                        if let syn::Expr::Path(path) = &*nested_field.base {
                                            if let Some(segment) = path.path.segments.first() {
                                                if segment.ident == "ctx" {
                                                    // This is ctx.accounts.field_name - create binding for the key
                                                    let binding_name = syn::Ident::new(&format!("seed_binding_{}", i), expr.span());
                                                    bindings.push(quote! {
                                                        let #binding_name = ctx.accounts.#field_name.key().to_bytes();
                                                    });
                                                    seed_refs.push(quote! { &#binding_name });
                                                    handled = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let syn::Expr::Path(path) = &*field_expr.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        // This is ctx.field_name - create binding
                                        let binding_name = syn::Ident::new(&format!("seed_binding_{}", i), expr.span());
                                        bindings.push(quote! {
                                            let #binding_name = ctx.accounts.#field_name.key().to_bytes();
                                        });
                                        seed_refs.push(quote! { &#binding_name });
                                        handled = true;
                                    } else if segment.ident == "data" {
                                        // This is data.field - might need to_le_bytes
                                        // Just use the expression as-is, will be handled by generate_seed_expressions
                                        seed_refs.push(quote! { (#expr).as_ref() });
                                        handled = true;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                
                if !handled {
                    // Other expressions - use as-is
                    seed_refs.push(quote! { (#expr).as_ref() });
                }
            }
        }
    }
    
    // Generate indices for accessing seeds array
    let indices: Vec<usize> = (0..seed_refs.len()).collect();
    
    Ok(quote! {
        {
            #(#bindings)*
            let seeds: &[&[u8]] = &[
                #(#seed_refs,)*
            ];
            let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
            let seeds_vec: Vec<Vec<u8>> = vec![
                #(
                    seeds[#indices].to_vec(),
                )*
                vec![bump],
            ];
            (seeds_vec, pda)
        }
    })
}

/// Generate temporary bindings and references for seeds to avoid lifetime issues
fn generate_seed_bindings(
    seeds: &Punctuated<SeedElement, Token![,]>,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut temp_bindings = Vec::new();
    let mut seed_refs = Vec::new();

    for (i, seed) in seeds.iter().enumerate() {
        let temp_var = syn::Ident::new(&format!("seed_{}", i), proc_macro2::Span::call_site());
        
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                temp_bindings.push(quote! {
                    let #temp_var = #value.as_bytes();
                });
                seed_refs.push(quote! { #temp_var });
            }
            SeedElement::Expression(expr) => {
                match expr {
                    syn::Expr::Field(field_expr) => {
                        // Handle ctx.accounts.field_name, ctx.field_name, data.field
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            match &*field_expr.base {
                                syn::Expr::Field(nested_field) => {
                                    // Handle ctx.accounts.field_name
                                    if let syn::Member::Named(base_name) = &nested_field.member {
                                        if base_name == "accounts" {
                                            if let syn::Expr::Path(path) = &*nested_field.base {
                                                if let Some(segment) = path.path.segments.first() {
                                                    if segment.ident == "ctx" {
                                                        // This is ctx.accounts.field_name
                                                        temp_bindings.push(quote! {
                                                            let #temp_var = ctx.accounts.#field_name.key().to_bytes();
                                                        });
                                                        seed_refs.push(quote! { #temp_var.as_ref() });
                                                    } else {
                                                        temp_bindings.push(quote! {
                                                            let #temp_var = (#expr).as_ref();
                                                        });
                                                        seed_refs.push(quote! { #temp_var });
                                                    }
                                                } else {
                                                    temp_bindings.push(quote! {
                                                        let #temp_var = (#expr).as_ref();
                                                    });
                                                    seed_refs.push(quote! { #temp_var });
                                                }
                                            } else {
                                                temp_bindings.push(quote! {
                                                    let #temp_var = (#expr).as_ref();
                                                });
                                                seed_refs.push(quote! { #temp_var });
                                            }
                                        } else {
                                            temp_bindings.push(quote! {
                                                let #temp_var = (#expr).as_ref();
                                            });
                                            seed_refs.push(quote! { #temp_var });
                                        }
                                    } else {
                                        temp_bindings.push(quote! {
                                            let #temp_var = (#expr).as_ref();
                                        });
                                        seed_refs.push(quote! { #temp_var });
                                    }
                                }
                                syn::Expr::Path(path) => {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "ctx" {
                                            // This is ctx.field_name
                                            temp_bindings.push(quote! {
                                                let #temp_var = ctx.accounts.#field_name.key().to_bytes();
                                            });
                                            seed_refs.push(quote! { #temp_var.as_ref() });
                                        } else if segment.ident == "data" {
                                            // This is data.field - use as-ref for Pubkey, to_le_bytes for numbers
                                            temp_bindings.push(quote! {
                                                let #temp_var = (#expr).as_ref();
                                            });
                                            seed_refs.push(quote! { #temp_var });
                                        } else {
                                            // Other expressions
                                            temp_bindings.push(quote! {
                                                let #temp_var = (#expr).as_ref();
                                            });
                                            seed_refs.push(quote! { #temp_var });
                                        }
                                    } else {
                                        temp_bindings.push(quote! {
                                            let #temp_var = (#expr).as_ref();
                                        });
                                        seed_refs.push(quote! { #temp_var });
                                    }
                                }
                                _ => {
                                    temp_bindings.push(quote! {
                                        let #temp_var = (#expr).as_ref();
                                    });
                                    seed_refs.push(quote! { #temp_var });
                                }
                            }
                        } else {
                            temp_bindings.push(quote! {
                                let #temp_var = (#expr).as_ref();
                            });
                            seed_refs.push(quote! { #temp_var });
                        }
                    }
                    syn::Expr::Path(path_expr) => {
                        // Handle direct account references
                        if let Some(ident) = path_expr.path.get_ident() {
                            temp_bindings.push(quote! {
                                let #temp_var = ctx.accounts.#ident.key().to_bytes();
                            });
                            seed_refs.push(quote! { #temp_var.as_ref() });
                        } else {
                            temp_bindings.push(quote! {
                                let #temp_var = (#expr).as_ref();
                            });
                            seed_refs.push(quote! { #temp_var });
                        }
                    }
                    _ => {
                        temp_bindings.push(quote! {
                            let #temp_var = (#expr).as_ref();
                        });
                        seed_refs.push(quote! { #temp_var });
                    }
                }
            }
        }
    }

    Ok((temp_bindings, seed_refs))
}

/// Generate public client-side seed functions for external consumption
fn generate_client_seed_functions(
    _account_types: &[Ident],
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
    instruction_data: &[InstructionDataSpec],
) -> Result<TokenStream> {
    let mut functions = Vec::new();

    // Generate PDA seed functions - FULLY GENERIC based on seed specifications
    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            let variant_name = &spec.variant;
            let function_name = format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());
            
            // Extract parameters and expressions from the seed specification
            let (parameters, seed_expressions) = analyze_seed_spec_for_client(spec, instruction_data)?;
            
            let function = quote! {
                /// Auto-generated client-side seed function
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                    let seed_values: Vec<Vec<u8>> = vec![
                        #(
                            (#seed_expressions).to_vec(),
                        )*
                    ];
                    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
                    let mut seeds_vec = seed_values;
                    seeds_vec.push(vec![bump]);
                    (seeds_vec, pda)
                }
            };
            functions.push(function);
        }
    }

    // Generate CToken seed functions - FULLY GENERIC based on seed specifications
    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let variant_name = &spec.variant;
            let function_name = format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());
            
            // Extract parameters and expressions from the seed specification
            let (parameters, seed_expressions) = analyze_seed_spec_for_client(spec, instruction_data)?;
            
            let function = quote! {
                /// Auto-generated client-side CToken seed function
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                    let seed_values: Vec<Vec<u8>> = vec![
                        #(
                            (#seed_expressions).to_vec(),
                        )*
                    ];
                    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
                    let mut seeds_vec = seed_values;
                    seeds_vec.push(vec![bump]);
                    (seeds_vec, pda)
                }
            };
            functions.push(function);
        }
    }

    Ok(quote! {
        #(#functions)*
    })
}

/// Analyze seed specification and generate parameters + expressions for client functions
fn analyze_seed_spec_for_client(
    spec: &TokenSeedSpec, 
    instruction_data: &[InstructionDataSpec]
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut parameters = Vec::new();
    let mut expressions = Vec::new();
    
    for seed in &spec.seeds {
        match seed {
            SeedElement::Literal(lit) => {
                // String literals don't need parameters
                let value = lit.value();
                expressions.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                // Analyze the expression to extract parameter and generate client expression
                match expr {
                    syn::Expr::Field(field_expr) => {
                        // Handle data.field or ctx.field
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            if let syn::Expr::Path(path) = &*field_expr.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "data" {
                                        // This is a data field - look up the type from instruction_data
                                        if let Some(data_spec) = instruction_data.iter().find(|d| d.field_name == *field_name) {
                                            let param_type = &data_spec.field_type;
                                            // Use references for Pubkey, direct values for numeric types
                                            let param_with_ref = if is_pubkey_type(param_type) {
                                                quote! { #field_name: &#param_type }
                                            } else {
                                                quote! { #field_name: #param_type }
                                            };
                                            parameters.push(param_with_ref);
                                            expressions.push(quote! { #field_name.as_ref() });
                                        } else {
                                            return Err(syn::Error::new_spanned(
                                                field_name,
                                                format!("data.{} used in seeds but no type specified. Add: {} = Pubkey (or u8, u16, u64)", field_name, field_name)
                                            ));
                                        }
                                    } else {
                                        // ctx.field - determine type by field name
                                        let param_type = if field_name.to_string().contains("owner") || 
                                                          field_name.to_string().contains("fee_payer") || 
                                                          field_name.to_string().contains("mint") {
                                            quote! { &anchor_lang::prelude::Pubkey }
                                        } else {
                                            quote! { &anchor_lang::prelude::Pubkey } // Default to Pubkey
                                        };
                                        parameters.push(quote! { #field_name: #param_type });
                                        expressions.push(quote! { #field_name.as_ref() });
                                    }
                                }
                            }
                        }
                    }
                    syn::Expr::Path(path_expr) => {
                        // Handle direct account field references like: amm_config, token_0_mint, pool_state
                        if let Some(ident) = path_expr.path.get_ident() {
                            // This is an account field reference - assume it's a Pubkey for client functions
                            parameters.push(quote! { #ident: &anchor_lang::prelude::Pubkey });
                            expressions.push(quote! { #ident.as_ref() });
                        }
                    }
                    syn::Expr::MethodCall(method_call) => {
                        // Handle method calls like amm_config.key().as_ref(), data.session_id.to_le_bytes(), etc.
                        if let syn::Expr::Field(field_expr) = &*method_call.receiver {
                            if let syn::Member::Named(field_name) = &field_expr.member {
                                if let syn::Expr::Path(path) = &*field_expr.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "data" {
                                            // This is a data field - look up the type from instruction_data
                                            if let Some(data_spec) = instruction_data.iter().find(|d| d.field_name == *field_name) {
                                                let param_type = &data_spec.field_type;
                                                // Use references for Pubkey, direct values for numeric types
                                                let param_with_ref = if is_pubkey_type(param_type) {
                                                    quote! { #field_name: &#param_type }
                                                } else {
                                                    quote! { #field_name: #param_type }
                                                };
                                                parameters.push(param_with_ref);
                                                
                                                // Generate expression for client function  
                                                let method_name = &method_call.method;
                                                expressions.push(quote! { #field_name.#method_name().as_ref() });
                                            } else {
                                                return Err(syn::Error::new_spanned(
                                                    field_name,
                                                    format!("data.{} used in seeds but no type specified. Add: {} = Pubkey (or u8, u16, u64)", field_name, field_name)
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        } else if let syn::Expr::Path(path_expr) = &*method_call.receiver {
                            // Handle direct account method calls like amm_config.key().as_ref()
                            if let Some(ident) = path_expr.path.get_ident() {
                                // This is an account field reference - assume it's a Pubkey for client functions
                                parameters.push(quote! { #ident: &anchor_lang::prelude::Pubkey });
                                expressions.push(quote! { #ident.as_ref() });
                            }
                        }
                    }
                    _ => {
                        // For other expressions, try to use as-is
                        expressions.push(quote! { (#expr).as_ref() });
                    }
                }
            }
        }
    }
    
    Ok((parameters, expressions))
}

/// Check if a type is Pubkey-like
fn is_pubkey_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            type_name == "Pubkey" || type_name.contains("Pubkey")
        } else {
            false
        }
    } else {
        false
    }
}

/// Extract required account names from seed expressions
fn extract_required_accounts_from_seeds(
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
) -> Result<Vec<String>> {
    let mut required_accounts = std::collections::HashSet::new();

    // Extract from PDA seeds
    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
        }
    }

    // Extract from token seeds
    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
        }
    }

    Ok(required_accounts.into_iter().collect())
}

/// Extract account names from a single seed specification
/// Extract account name from an expression, handling method chains
/// Simply looks for ctx.accounts.FIELD_NAME pattern and extracts FIELD_NAME
fn extract_account_from_expr(
    expr: &syn::Expr,
    required_accounts: &mut std::collections::HashSet<String>,
) {
    match expr {
        syn::Expr::MethodCall(method_call) => {
            // For method calls, check the receiver
            // e.g., ctx.accounts.mint.key().as_ref() -> check ctx.accounts.mint.key()
            extract_account_from_expr(&*method_call.receiver, required_accounts);
        }
        syn::Expr::Field(field_expr) => {
            // Check if this is ctx.accounts.FIELD_NAME
            if let syn::Member::Named(field_name) = &field_expr.member {
                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                    if let syn::Member::Named(base_name) = &nested_field.member {
                        if base_name == "accounts" {
                            if let syn::Expr::Path(path) = &*nested_field.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        // Found ctx.accounts.FIELD_NAME - extract FIELD_NAME
                                        required_accounts.insert(field_name.to_string());
                                        return; // Found it, no need to recurse further
                                    }
                                }
                            }
                        }
                    }
                } else if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "ctx" && field_name != "accounts" {
                            // Found ctx.FIELD_NAME (shorthand) - treat as account
                            required_accounts.insert(field_name.to_string());
                            return;
                        }
                    }
                }
            }
        }
        syn::Expr::Path(path_expr) => {
            // Handle direct account references (just an identifier)
            if let Some(ident) = path_expr.path.get_ident() {
                let name = ident.to_string();
                // Skip "ctx" and "data" as they're not accounts
                if name != "ctx" && name != "data" {
                    required_accounts.insert(name);
                }
            }
        }
        _ => {}
    }
}

fn extract_accounts_from_seed_spec(
    spec: &TokenSeedSpec,
    required_accounts: &mut std::collections::HashSet<String>,
) -> Result<()> {
    for seed in &spec.seeds {
        match seed {
            SeedElement::Literal(_) => {
                // String literals don't require accounts
            }
            SeedElement::Expression(expr) => {
                match expr {
                    syn::Expr::MethodCall(method_call) => {
                        // Recursively find the base account through method call chains
                        // e.g., ctx.accounts.mint.key().as_ref() -> extract "mint"
                        extract_account_from_expr(&*method_call.receiver, required_accounts);
                    }
                    syn::Expr::Path(_) | syn::Expr::Field(_) => {
                        // Use the helper function for all expressions
                        extract_account_from_expr(expr, required_accounts);
                    }
                    _ => {
                        // Other expressions - try to extract identifiers
                    }
                }
            }
        }
    }
    Ok(())
}

/// Generate DecompressAccountsIdempotent struct with required accounts
fn generate_decompress_accounts_struct(required_accounts: &[String]) -> Result<syn::ItemStruct> {
    let mut account_fields = vec![
        // Standard fields
        quote! {
            #[account(mut)]
            pub fee_payer: Signer<'info>
        },
        quote! {
            /// UNCHECKED: Anyone can pay to init.
            #[account(mut)]
            pub rent_payer: Signer<'info>
        },
        quote! {
            /// The global config account
            /// CHECK: load_checked.
            pub config: AccountInfo<'info>
        },
        quote! {
            /// Compressed token program
            /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
            pub compressed_token_program: Option<UncheckedAccount<'info>>
        },
        quote! {
            /// CPI authority PDA of the compressed token program
            /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
            pub compressed_token_cpi_authority: Option<UncheckedAccount<'info>>
        },
    ];

    // Add required accounts as unchecked accounts (skip standard fields)
    let standard_fields = ["fee_payer", "rent_payer", "config", "compressed_token_program", "compressed_token_cpi_authority"];
    
    for account_name in required_accounts {
        if !standard_fields.contains(&account_name.as_str()) {
            let account_ident = syn::Ident::new(account_name, proc_macro2::Span::call_site());
            account_fields.push(quote! {
                /// CHECK: Required for seed derivation - validated by program logic
                pub #account_ident: UncheckedAccount<'info>
            });
        }
    }

    let struct_def = quote! {
        #[derive(Accounts)]
        pub struct DecompressAccountsIdempotent<'info> {
            #(#account_fields,)*
        }
    };

    Ok(syn::parse2(struct_def)?)
}

// Client seed function generation complete! 🎉

// No more hardcoded fallbacks! Everything is now auto-generated! 🎉
