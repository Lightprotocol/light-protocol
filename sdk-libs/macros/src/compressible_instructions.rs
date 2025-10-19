use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, Ident, Item, ItemFn, ItemMod, LitStr, Result, Token,
};

/// Helper macro to create syn::Error with file:line information
/// This helps track exactly where in the macro the error originated
macro_rules! macro_error {
    ($span:expr, $msg:expr) => {
        syn::Error::new_spanned(
            $span,
            format!(
                "{}\n  --> macro location: {}:{}",
                $msg,
                file!(),
                line!()
            )
        )
    };
    ($span:expr, $fmt:expr, $($arg:tt)*) => {
        syn::Error::new_spanned(
            $span,
            format!(
                concat!($fmt, "\n  --> macro location: {}:{}"),
                $($arg)*,
                file!(),
                line!()
            )
        )
    };
}

/// Determines which type of instruction to generate based on seed specifications
#[derive(Debug, Clone, Copy)]
enum InstructionVariant {
    /// Only PDA seeds specified - generate PDA-only instructions
    PdaOnly,
    /// Only token seeds specified - generate token-only instructions  
    TokenOnly,
    /// Both PDA and token seeds specified - generate mixed instructions
    Mixed,
}

/// Parse seed specification for a token account variant
#[derive(Clone)]
struct TokenSeedSpec {
    variant: Ident,
    _eq: Token![=],
    is_token: Option<bool>, // Optional explicit token flag
    seeds: Punctuated<SeedElement, Token![,]>,
    authority: Option<Vec<SeedElement>>, // Optional authority seeds for CToken accounts
}

impl Parse for TokenSeedSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let variant = input.parse()?;
        let _eq = input.parse()?;

        let content;
        syn::parenthesized!(content in input);

        // Check if first element is an explicit token flag
        let (is_token, seeds, authority) = if content.peek(Ident) {
            let first_ident: Ident = content.parse()?;

            match first_ident.to_string().as_str() {
                "is_token" | "true" => {
                    // Explicit token flag
                    let _comma: Token![,] = content.parse()?;
                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(true), seeds, authority)
                }
                "is_pda" | "false" => {
                    // Explicit PDA flag
                    let _comma: Token![,] = content.parse()?;
                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(false), seeds, authority)
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
                        let (rest, authority) = parse_seeds_with_authority(&content)?;
                        seeds.extend(rest);
                        (None, seeds, authority)
                    } else {
                        (None, seeds, None)
                    }
                }
            }
        } else {
            // No identifier first, parse all as seeds
            let (seeds, authority) = parse_seeds_with_authority(&content)?;
            (None, seeds, authority)
        };

        Ok(TokenSeedSpec {
            variant,
            _eq,
            is_token,
            seeds,
            authority,
        })
    }
}

// Helper function to parse seeds and extract authority if present
fn parse_seeds_with_authority(
    content: ParseStream,
) -> Result<(Punctuated<SeedElement, Token![,]>, Option<Vec<SeedElement>>)> {
    let mut seeds = Punctuated::new();
    let mut authority = None;

    while !content.is_empty() {
        // Check for "authority = <expr>" pattern
        if content.peek(Ident) {
            let fork = content.fork();
            if let Ok(ident) = fork.parse::<Ident>() {
                if ident == "authority" && fork.peek(Token![=]) {
                    // Found authority assignment
                    let _: Ident = content.parse()?;
                    let _: Token![=] = content.parse()?;

                    // Check if authority is a tuple (multiple seeds) or single seed
                    if content.peek(syn::token::Paren) {
                        // Parse tuple: authority = ("auth", ctx.accounts.mint)
                        let auth_content;
                        syn::parenthesized!(auth_content in content);
                        let mut auth_seeds = Vec::new();

                        while !auth_content.is_empty() {
                            auth_seeds.push(auth_content.parse::<SeedElement>()?);
                            if auth_content.peek(Token![,]) {
                                let _: Token![,] = auth_content.parse()?;
                            } else {
                                break;
                            }
                        }
                        authority = Some(auth_seeds);
                    } else {
                        // Parse single seed: authority = LIGHT_CPI_SIGNER
                        authority = Some(vec![content.parse::<SeedElement>()?]);
                    }

                    // Check if there's more after authority
                    if content.peek(Token![,]) {
                        let _: Token![,] = content.parse()?;
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }

        // Regular seed element
        seeds.push(content.parse::<SeedElement>()?);

        if content.peek(Token![,]) {
            let _: Token![,] = content.parse()?;
            if content.is_empty() {
                break;
            }
        } else {
            break;
        }
    }

    Ok((seeds, authority))
}

#[derive(Clone)]
enum SeedElement {
    /// String literal like "user_record"
    Literal(LitStr),
    /// Any expression: data.owner, ctx.fee_payer, data.session_id.to_le_bytes(), CONST_NAME, etc.
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

        let mut item_count = 0;
        while !input.is_empty() {
            let ident: Ident = input.parse().map_err(|e| e)?;

            if input.peek(Token![=]) {
                let _eq: Token![=] = input.parse()?;

                if input.peek(syn::token::Paren) {
                    // This is a seed specification (either PDA or CToken). Reuse TokenSeedSpec parser to avoid mis-parsing
                    let content;
                    syn::parenthesized!(content in input);
                    let inside: TokenStream = content.parse()?;
                    let seed_spec: TokenSeedSpec = syn::parse2(quote! { #ident = (#inside) })?;

                    let is_token_account = seed_spec.is_token.unwrap_or(false);
                    if is_token_account {
                        token_seeds.push(seed_spec);
                    } else {
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
            item_count += 1;
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
#[inline(never)]
pub fn add_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    // Parse with enhanced format - no legacy fallback!
    let enhanced_args = match syn::parse2::<EnhancedMacroArgs>(args.clone()) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("ERROR: Failed to parse macro args: {}", e);
            eprintln!("Args were: {}", args);
            return Err(e);
        }
    };

    let account_types = enhanced_args.account_types;
    let pda_seeds = Some(enhanced_args.pda_seeds);
    let token_seeds = Some(enhanced_args.token_seeds);
    let instruction_data = enhanced_args.instruction_data;

    if module.content.is_none() {
        return Err(macro_error!(&module, "Module must have a body"));
    }

    if account_types.is_empty() {
        return Err(macro_error!(
            &module,
            "At least one account type must be specified"
        ));
    }

    // Generate compile-time size validation for compressed accounts
    let size_validation_checks = validate_compressed_account_sizes(&account_types)?;

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

    // Validate that token variants have authority specified for compression signing
    if let Some(ref token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            if spec.authority.is_none() {
                return Err(macro_error!(
                    &spec.variant,
                    "Token account '{}' must specify authority = <seed_expr> for compression signing",
                    spec.variant
                ));
            }
        }
    }

    // Generate the compressed_account_variant enum automatically
    let mut account_types_stream = TokenStream::new();
    for (i, account_type) in account_types.iter().enumerate() {
        if i > 0 {
            account_types_stream.extend(quote! { , });
        }
        account_types_stream.extend(quote! { #account_type });
    }
    let enum_and_traits = crate::variant_enum::compressed_account_variant(account_types_stream)?;

    // Determine instruction variant based on seed types
    let has_pda_seeds = pda_seeds.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    let has_token_seeds = token_seeds.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    let instruction_variant = match (has_pda_seeds, has_token_seeds) {
        (true, true) => InstructionVariant::Mixed,
        (true, false) => InstructionVariant::PdaOnly,
        (false, true) => InstructionVariant::TokenOnly,
        (false, false) => {
            return Err(macro_error!(
                &module,
                "At least one PDA or token seed specification must be provided"
            ))
        }
    };

    // Generate error codes automatically based on instruction variant
    let error_codes = generate_error_codes(instruction_variant)?;

    // Extract required accounts from seed expressions and track dependencies
    let (required_accounts, account_dependencies) =
        extract_required_accounts_from_seeds(&pda_seeds, &token_seeds)?;

    // Generate the DecompressAccountsIdempotent accounts struct with required accounts
    let decompress_accounts = generate_decompress_accounts_struct(
        &required_accounts,
        &account_dependencies,
        instruction_variant,
    )?;

    // Generate helper functions for packed variants
    let helper_packed_fns: Result<Vec<_>> = account_types.iter().map(|name| {
        let name_str = name.to_string();
        // Generate dynamic seed derivation for this type as well
        let seed_call = if let Some(ref pda_seed_specs) = pda_seeds {
            if let Some(spec) = pda_seed_specs.iter().find(|s| s.variant.to_string() == name_str) {
                generate_pda_seed_derivation(spec, &instruction_data, &format_ident!("accounts"))?
            } else {
                return Err(macro_error!(
                    name,
                    "No seed specification provided for account type '{}'. All accounts must have seed specifications.", 
                    name_str
                ))
            }
        } else {
            return Err(macro_error!(
                name,
                "No seed specifications provided. Use the new syntax: AccountType = (\"seed\", data.field)"
            ))
        };
        let packed_name = format_ident!("Packed{}", name);
        let func_name = format_ident!("handle_packed_{}", name);
        Ok(quote! {
            #[inline(never)]
            fn #func_name<'a, 'b, 'info>(
                accounts: &DecompressAccountsIdempotent<'info>,
                cpi_accounts: &light_sdk::cpi::CpiAccountsSmall<'b, 'info>,
                address_space: anchor_lang::prelude::Pubkey,
                solana_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
                i: usize,
                packed: &#packed_name,
                meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                post_system_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
                compressed_pda_infos: &mut Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
            ) -> Result<()> {
                let data: #name = <#packed_name as light_sdk::compressible::Unpack>::unpack(packed, post_system_accounts)
                    .map_err(anchor_lang::prelude::ProgramError::from)?;
                let (seeds_vec, derived_pda) = #seed_call;

                if derived_pda != *solana_accounts[i].key {
                   anchor_lang::solana_program::log::msg!(
                        "Derived PDA does not match account at index {}: expected {:?}, got {:?}, seeds: {:?}",
                        i,
                        solana_accounts[i].key,
                        derived_pda,
                        seeds_vec
                    );
                }

                let compressed_infos = {
                    let seed_refs: Vec<&[u8]> = seeds_vec.iter().map(|v| v.as_slice()).collect();
                    light_sdk::compressible::prepare_account_for_decompression_idempotent::<#name>(
                        &crate::ID,
                        data,
                        light_sdk::compressible::into_compressed_meta_with_address(
                            meta,
                            &solana_accounts[i],
                            address_space,
                            &crate::ID,
                        ),
                        &solana_accounts[i],
                        &accounts.rent_payer,
                        cpi_accounts,
                        seed_refs.as_slice(),
                    )?
                };

                compressed_pda_infos.extend(compressed_infos);
                Ok(())
            }
        })
    }).collect();
    let helper_packed_fns = helper_packed_fns?;

    // Generate match arms for unpacked variants - should be unreachable in decompression
    let call_unpacked_arms: Vec<_> = account_types.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(_) => {
                unreachable!("Unpacked variants should not be present during decompression - accounts are always packed in-flight");
            }
        }
    }).collect();
    let call_packed_arms: Vec<_> = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        let func_name = format_ident!("handle_packed_{}", name);
        quote! {
            CompressedAccountVariant::#packed_name(packed) => {
                #func_name(accounts, &cpi_accounts, address_space, solana_accounts, i, &packed, &meta, post_system_accounts, &mut compressed_pda_infos)?;
            }
        }
    }).collect();

    // Generate trait-based system for TRULY generic CToken variant handling
    let ctoken_trait_system: syn::ItemMod = syn::parse_quote! {
        /// Trait-based system for generic CToken variant seed handling
        /// Users implement this trait for their CTokenAccountVariant enum
        pub mod ctoken_seed_system {
            use super::*;

            pub struct CTokenSeedContext<'a, 'info> {
                pub accounts: &'a DecompressAccountsIdempotent<'info>,
                pub remaining_accounts: &'a [anchor_lang::prelude::AccountInfo<'info>],
            }

            pub trait CTokenSeedProvider {
                /// Get seeds for the token account PDA (used for decompression - the owner of the compressed token)
                fn get_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, Pubkey)>;

                /// Get authority seeds for signing during compression (if authority is specified)
                fn get_authority_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, Pubkey)>;
            }
        }
    };

    // Generate helper functions inside a private submodule to avoid Anchor treating them as instructions
    let helpers_module: syn::ItemMod = {
        let helper_packed_fns = helper_packed_fns.clone();
        let call_unpacked_arms = call_unpacked_arms.clone();
        let call_packed_arms = call_packed_arms.clone();
        syn::parse_quote! {
        mod __macro_helpers {
            use super::*;
            #(#helper_packed_fns)*
                #[inline(never)]
                pub fn collect_pda_and_token<'a, 'b, 'info>(
                    accounts: &DecompressAccountsIdempotent<'info>,
                    cpi_accounts: &light_sdk::cpi::CpiAccountsSmall<'b, 'info>,
                    address_space: anchor_lang::prelude::Pubkey,
                    compressed_accounts: Vec<CompressedAccountData>,
                    solana_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
                ) -> Result<(
                    Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                    Vec<(
                        light_sdk::token::PackedCTokenData<CTokenAccountVariant>,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    )>,
                )> {
                    let post_system_accounts = cpi_accounts.post_system_accounts().unwrap();
                    let estimated_capacity = compressed_accounts.len();
                    let mut compressed_pda_infos = Vec::with_capacity(estimated_capacity);
                    let mut compressed_token_accounts: Vec<(
                        light_sdk::token::PackedCTokenData<CTokenAccountVariant>,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    )> = Vec::with_capacity(estimated_capacity);

                    for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                       anchor_lang::solana_program::log::msg!("CU before unpack idx={}", i);
                        anchor_lang::solana_program::log::sol_log_compute_units();
                        let meta = compressed_data.meta;
                        match compressed_data.data {
                            #(#call_unpacked_arms)*
                            #(#call_packed_arms)*
                            CompressedAccountVariant::PackedCTokenData(mut data) => {
                                // Fix version field: The TS client doesn't include version in packed data,
                                // but on-chain expects it. Set to 3 (TokenDataVersion::ShaFlat) which is
                                // the default for compressed token accounts.
                                data.token_data.version = 3;
                                compressed_token_accounts.push((data, meta));
                            }
                            CompressedAccountVariant::CTokenData(_) => {
                                unreachable!();
                            }
                        }
                       anchor_lang::solana_program::log::msg!("CU after unpack idx={}", i);
                        anchor_lang::solana_program::log::sol_log_compute_units();
                    }

                    Ok((compressed_pda_infos, compressed_token_accounts))
                }
            }
        }
    };

    // Generate the decompress instruction based on variant
    let decompress_instruction: ItemFn = match instruction_variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
        InstructionVariant::Mixed => syn::parse_quote! {
        /// Auto-generated decompress_accounts_idempotent instruction
        #[inline(never)]
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, 'info, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {

            let compression_config = light_sdk::compressible::CompressibleConfig::load_checked(
                &ctx.accounts.config,
                &crate::ID,
            )?;
            let address_space = compression_config.address_space[0];

           anchor_lang::solana_program::log::msg!("CU after load_checked");
            anchor_lang::solana_program::log::sol_log_compute_units();

            #[inline(never)]
            fn check_account_types(compressed_accounts: &[CompressedAccountData]) -> (bool, bool) {
                let (mut has_tokens, mut has_pdas) = (false, false);
                for c in compressed_accounts {
                    match c.data {
                        CompressedAccountVariant::PackedCTokenData(_) => {
                            has_tokens = true;
                        }
                        _ => has_pdas = true,
                    }
                    if has_tokens && has_pdas {
                        break;
                    }
                }
                (has_tokens, has_pdas)
            }

            #[inline(never)]
            fn process_tokens<'a, 'b, 'info>(
                accounts: &DecompressAccountsIdempotent<'info>,
                remaining_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
                fee_payer: &anchor_lang::prelude::AccountInfo<'info>,
                ctoken_program: &anchor_lang::prelude::UncheckedAccount<'info>,
                ctoken_rent_sponsor: &anchor_lang::prelude::AccountInfo<'info>,
                ctoken_cpi_authority: &anchor_lang::prelude::UncheckedAccount<'info>,
                ctoken_config: &anchor_lang::prelude::AccountInfo<'info>,
                config: &anchor_lang::prelude::AccountInfo<'info>,
                ctoken_accounts: Vec<(
                    light_sdk::token::PackedCTokenData<CTokenAccountVariant>,
                    light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                )>,
                proof: light_sdk::instruction::ValidityProof,
                cpi_accounts: light_sdk::cpi::CpiAccountsSmall<'b, 'info>,
                has_pdas: bool,
            ) -> Result<()> {
                let mut token_decompress_indices = Box::new(Vec::with_capacity(ctoken_accounts.len()));
                let mut token_signers_seed_groups = Box::new(Vec::with_capacity(ctoken_accounts.len()));
                let packed_accounts = cpi_accounts.post_system_accounts().unwrap();

                use crate::ctoken_seed_system::{CTokenSeedContext, CTokenSeedProvider};
                let seed_context = CTokenSeedContext { accounts, remaining_accounts };
                let authority = cpi_accounts.authority().unwrap();
                let cpi_context = cpi_accounts.cpi_context().unwrap();

                for (token_data, meta) in ctoken_accounts.into_iter() {
                    let owner_index: u8 = token_data.token_data.owner;
                    let mint_index: u8 = token_data.token_data.mint;

                    let mint_info = packed_accounts[mint_index as usize].to_account_info();
                    let owner_info = packed_accounts[owner_index as usize].to_account_info();

                    // Idempotency: if the token account already exists/initialized, skip creating it.
                    let already_exists = !owner_info.data_is_empty();

                    let (ctoken_signer_seeds, derived_token_account_address) = token_data.variant.get_seeds(&seed_context)?;
                    let (ctoken_authority_seeds, ctoken_authority_pda) = token_data.variant.get_authority_seeds(&seed_context)?;

                    if derived_token_account_address != *owner_info.key {
                       anchor_lang::solana_program::log::msg!("Derived token account address (PDA) does not match provided owner account");
                       anchor_lang::solana_program::log::msg!("derived_token_account_address: {:?}", derived_token_account_address);
                       anchor_lang::solana_program::log::msg!("owner_info.key: {:?}", owner_info.key);
                        return err!(CompressibleInstructionError::CTokenDecompressionNotImplemented);
                    }

                    if !already_exists {
                        let seed_refs: Vec<&[u8]> = ctoken_signer_seeds.iter().map(|s| s.as_slice()).collect();
                        let seeds_slice: &[&[u8]] = &seed_refs;

                        light_compressed_token_sdk::instructions::create_token_account::create_ctoken_account_signed(
                            crate::ID,
                            fee_payer.clone().to_account_info(),
                            owner_info.clone(),
                            mint_info.clone(),
                            ctoken_authority_pda,
                            seeds_slice,
                            ctoken_rent_sponsor.clone().to_account_info(),
                            ctoken_config.to_account_info(),
                            Some(1), // pre_pay_num_epochs TODO: make this configurable
                            None,    // write_top_up_lamports
                        )?;

                        let decompress_index = light_compressed_token_sdk::instructions::DecompressFullIndices::from((
                            token_data.token_data,
                            meta,
                            owner_index,
                        ));
                        token_decompress_indices.push(decompress_index);
                        token_signers_seed_groups.push(ctoken_signer_seeds);
                    } else {
                       anchor_lang::solana_program::log::msg!("CToken account already initialized, skipping creation and decompress");
                        continue;
                    }
                }

                // If there are no token accounts to process, return early to avoid unnecessary CPI.
                if token_decompress_indices.is_empty() {
                    return Ok(());
                }

                let ctoken_ix = light_compressed_token_sdk::instructions::decompress_full_ctoken_accounts_with_indices(
                    fee_payer.key(),
                    proof,
                    if has_pdas { Some(cpi_context.key()) } else { None },
                    &token_decompress_indices,
                    packed_accounts,
                )
                .map_err(anchor_lang::prelude::ProgramError::from)?;

                {
                    let signer_seed_refs: Vec<Vec<&[u8]>> = token_signers_seed_groups
                        .iter()
                        .map(|group| group.iter().map(|s| s.as_slice()).collect())
                        .collect();
                    let signer_seed_slices: Vec<&[&[u8]]> =
                        signer_seed_refs.iter().map(|g| g.as_slice()).collect();

                    let cpi_slice = cpi_accounts.account_infos_slice();
                    let mut account_infos = Vec::with_capacity(5 + cpi_slice.len().saturating_sub(1));
                    account_infos.push(fee_payer.to_account_info());
                    account_infos.push(ctoken_cpi_authority.to_account_info());
                    account_infos.push(ctoken_program.to_account_info());
                    account_infos.push(ctoken_rent_sponsor.to_account_info());
                    account_infos.push(config.to_account_info());
                    if cpi_slice.len() > 1 {
                        account_infos.extend_from_slice(&cpi_slice[1..]);
                    }
                    anchor_lang::solana_program::program::invoke_signed(
                        &ctoken_ix,
                        account_infos.as_slice(),
                        signer_seed_slices.as_slice(),
                    )?;
                }
                Ok(())
            }

            let (has_tokens, has_pdas) = check_account_types(&compressed_accounts);
            if !has_tokens && !has_pdas {
                return Ok(());
            }

           anchor_lang::solana_program::log::msg!("CU after check_account_types: has_tokens={}, has_pdas={}", has_tokens, has_pdas);
            anchor_lang::solana_program::log::sol_log_compute_units();


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

           anchor_lang::solana_program::log::msg!("CU after alloc CpiAccountsSmall");
            anchor_lang::solana_program::log::sol_log_compute_units();

            let solana_accounts = &ctx.remaining_accounts[ctx.remaining_accounts.len() - compressed_accounts.len()..];

            let (mut compressed_pda_infos, compressed_token_accounts) = __macro_helpers::collect_pda_and_token(
                &ctx.accounts,
                &cpi_accounts,
                address_space,
                compressed_accounts,
                solana_accounts,
            )?;

           anchor_lang::solana_program::log::msg!(
                "CU after collect_pda_and_ctoken: pdas={}, tokens={}",
                compressed_pda_infos.len(),
                compressed_token_accounts.len()
            );
            anchor_lang::solana_program::log::sol_log_compute_units();

            let has_pdas = !compressed_pda_infos.is_empty();
            let has_tokens = !compressed_token_accounts.is_empty();
            if !has_pdas && !has_tokens {
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

            if has_tokens {
                anchor_lang::solana_program::log::msg!("CU before process_tokens");
                anchor_lang::solana_program::log::sol_log_compute_units();
                process_tokens(
                    &ctx.accounts,
                    &ctx.remaining_accounts,
                    &fee_payer,
                    &ctx.accounts.ctoken_program,
                    &ctx.accounts.ctoken_rent_sponsor,
                    &ctx.accounts.ctoken_cpi_authority,
                    &ctx.accounts.ctoken_config,
                    &ctx.accounts.config,
                    compressed_token_accounts,
                    proof,
                    cpi_accounts,
                    has_pdas
                )?;
               anchor_lang::solana_program::log::msg!("CU after process_tokens");
                anchor_lang::solana_program::log::sol_log_compute_units();
            }
            Ok(())
        }
        },
    };

    // Generate the CompressAccountsIdempotent accounts struct based on variant
    let compress_accounts: syn::ItemStruct = match instruction_variant {
        InstructionVariant::PdaOnly => generate_pda_only_compress_accounts_struct()?,
        InstructionVariant::TokenOnly => generate_token_only_compress_accounts_struct()?,
        InstructionVariant::Mixed => syn::parse_quote! {
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

            /// CHECK: compression_authority must be the rent_authority defined when creating the PDA account.
            #[account(mut)]
            pub compression_authority: AccountInfo<'info>,

            /// CHECK: token_compression_authority must be the rent_authority defined when creating the token account.
            #[account(mut)]
            pub ctoken_compression_authority: AccountInfo<'info>,

            /// Token rent recipient - must match config
            /// CHECK: Token rent recipient is validated against the config
            #[account(mut)]
            pub ctoken_rent_sponsor: AccountInfo<'info>,

            // Required token-specific accounts (always needed in mixed variant for simplicity)
            /// Compressed token program (always required in mixed variant)
            /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
            pub ctoken_program: UncheckedAccount<'info>,

            /// CPI authority PDA of the compressed token program (always required in mixed variant)
            /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
            pub ctoken_cpi_authority: UncheckedAccount<'info>,
        }
        },
    };

    // Generate compress match arms for each account type with dedicated vectors
    let compress_match_arms: Vec<_> = account_types.iter().map(|name| {
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
                compressed_pda_infos.push(compressed_info);
                // Record index for closing later using native close (to reduce stack usage)
                pda_indices_to_close.push(i);
            }
        }
    }).collect();

    // Generate the compress instruction based on variant
    let compress_instruction: syn::ItemFn = match instruction_variant {
        InstructionVariant::PdaOnly => {
            unreachable!()
            // generate_pda_only_compress_instruction(&compress_match_arms, &account_types)?
        }
        InstructionVariant::TokenOnly => {
            unreachable!()
            // generate_token_only_compress_instruction()?
        }
        InstructionVariant::Mixed => syn::parse_quote! {
        /// Auto-generated compress_accounts_idempotent instruction
        #[inline(never)]
        pub fn compress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, 'info, 'info, CompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            signer_seeds: Vec<Vec<Vec<u8>>>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            // Note: we need to override proof here for now temporarily. TODO:
            // fix and remove.
            let proof = light_sdk::instruction::ValidityProof::new(None);
            let compression_config = light_sdk::compressible::CompressibleConfig::load_checked(
                &ctx.accounts.config,
                &crate::ID,
            )?;
            if ctx.accounts.rent_recipient.key() != compression_config.rent_recipient {
                return err!(CompressibleInstructionError::InvalidRentRecipient);
            }
            // Identify solana accounts slice (PDAs and/or token accounts). Tokens must always come at the end.
            let pda_and_token_accounts_start = ctx.remaining_accounts.len() - signer_seeds.len();
            let solana_accounts = &ctx.remaining_accounts[pda_and_token_accounts_start..];

            #[inline(never)]
            fn has_pdas_and_tokens<'info>(
                solana_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
            ) -> (bool, bool) {
                let (mut has_tokens, mut has_pdas) = (false, false);
                for account_info in solana_accounts.iter() {
                    if account_info.data_is_empty() {
                        continue;
                    }
                    if account_info.owner == &light_sdk_types::CTOKEN_PROGRAM_ID.into() {
                        has_tokens = true;
                    } else if account_info.owner == &crate::ID {
                        has_pdas = true;
                    }
                    if has_tokens && has_pdas {
                        break;
                    }
                }
                (has_tokens, has_pdas)
            }

            let (has_tokens, has_pdas) = has_pdas_and_tokens(solana_accounts);
            if !has_tokens && !has_pdas {
                return Ok(());
            }

            // Build CPI accounts (no CPI context needed for compression flow)
            let cpi_accounts = light_sdk_types::CpiAccountsSmall::new(
                ctx.accounts.fee_payer.as_ref(),
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                LIGHT_CPI_SIGNER,
            );

            // Collections (keep tiny stack footprint; heap grows as needed)
            let mut compressed_pda_infos = Vec::with_capacity(0);
            let mut token_accounts_to_compress: Vec<light_compressed_token_sdk::AccountInfoToCompress<'info>> = Vec::with_capacity(0);
            // Track PDA indices to close later to avoid storing typed Anchor accounts on the stack
            let mut pda_indices_to_close: Vec<usize> = Vec::with_capacity(0);

            // Map metas only to PDA accounts (tokens do not have entries in compressed_accounts)

            #[inline(never)]
            fn collect_accounts_to_compress<'b, 'info>(
                cpi_accounts: &light_sdk::cpi::CpiAccountsSmall<'b, 'info>,
                compression_config: &light_sdk::compressible::CompressibleConfig,
                solana_accounts: &'info [anchor_lang::prelude::AccountInfo<'info>],
                signer_seeds: &[Vec<Vec<u8>>],
                compressed_accounts: &[light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress],
                token_accounts_to_compress: &mut Vec<light_compressed_token_sdk::AccountInfoToCompress<'info>>,
                compressed_pda_infos: &mut Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                pda_indices_to_close: &mut Vec<usize>,
            ) -> Result<()> {
                let mut pda_meta_index: usize = 0;
                for (i, account_info) in solana_accounts.iter().enumerate() {
                    if account_info.data_is_empty() {
                        continue;
                    }
                    if account_info.owner == &light_sdk_types::CTOKEN_PROGRAM_ID.into() {
                        if let Ok(token_account) = anchor_lang::prelude::InterfaceAccount::<anchor_spl::token_interface::TokenAccount>::try_from(account_info) {
                            let account_signer_seeds = signer_seeds[i].clone();
                            token_accounts_to_compress.push(
                                light_compressed_token_sdk::AccountInfoToCompress {
                                    account_info: token_account.to_account_info(),
                                    signer_seeds: account_signer_seeds,
                                }
                            );
                        }
                    } else if account_info.owner == &crate::ID {
                        let data = account_info.try_borrow_data()?;
                        let discriminator = &data[0..8];
                        let meta = compressed_accounts[pda_meta_index];
                        pda_meta_index += 1;

                        match discriminator {
                            #(#compress_match_arms)*
                            _ => {
                                panic!("Trying to compress with invalid account discriminator");
                            }
                        }
                    }
                }
                Ok(())
            }

            collect_accounts_to_compress(
                &cpi_accounts,
                &compression_config,
                solana_accounts,
                &signer_seeds,
                &compressed_accounts,
                &mut token_accounts_to_compress,
                &mut compressed_pda_infos,
                &mut pda_indices_to_close,
            )?;

            let has_pdas = !compressed_pda_infos.is_empty();
            let has_tokens = !token_accounts_to_compress.is_empty();

            // 1) Compress and close token accounts (tokens must always come at the end)
            if has_tokens {
                let system_offset = cpi_accounts.system_accounts_end_offset();
                let post_system = &cpi_accounts.to_account_infos()[system_offset..];
                let output_queue = cpi_accounts.tree_accounts().unwrap()[0].to_account_info();
                let cpi_authority = cpi_accounts.authority().unwrap().to_account_info();
                light_compressed_token_sdk::instructions::compress_and_close::compress_and_close_ctoken_accounts_signed(
                    &token_accounts_to_compress,
                    ctx.accounts.fee_payer.to_account_info(),
                    output_queue,
                    ctx.accounts.ctoken_rent_sponsor.to_account_info(),
                    ctx.accounts.ctoken_cpi_authority.to_account_info(),
                    cpi_authority,
                    post_system,
                    &cpi_accounts.to_account_infos(),
                )?;
            }

            // 2) Compress PDAs (if any) and close on-chain PDAs to reclaim rent
            if has_pdas {
                let cpi_inputs = light_sdk::cpi::CpiInputs::new(proof, compressed_pda_infos);
                cpi_inputs.invoke_light_system_program_small(cpi_accounts.clone())?;

                // Close each PDA using native close to minimize stack usage
                for idx in pda_indices_to_close.into_iter() {
                    let mut info = solana_accounts[idx].clone();
                    light_sdk::compressible::compress_account_on_init_native::close(
                        &mut info,
                        ctx.accounts.rent_recipient.clone(),
                    ).map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
                }
            }

            Ok(())
        }
        },
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
        #[inline(never)]
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
        #[inline(never)]
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
    content.1.push(Item::Mod(helpers_module));
    content.1.push(Item::Mod(ctoken_trait_system));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(Item::Struct(compress_accounts));
    content.1.push(Item::Fn(compress_instruction));
    content.1.push(Item::Struct(init_config_accounts));
    content.1.push(Item::Struct(update_config_accounts));
    content.1.push(Item::Fn(init_config_instruction));
    content.1.push(Item::Fn(update_config_instruction));

    // Generate automatic CTokenSeedProvider implementation (authority removed)
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
    let client_seed_functions = generate_client_seed_functions(
        &account_types,
        &pda_seeds,
        &token_seeds,
        &instruction_data,
    )?;

    Ok(quote! {
        // Compile-time size validation for compressed accounts (must be first)
        #size_validation_checks

        // Auto-generated error codes for the macro
        #error_codes

        // Auto-generated CTokenAccountVariant enum
        #ctoken_enum

        // Auto-generated CompressedAccountVariant enum and traits
        #enum_and_traits

        // Auto-generated public seed functions for client consumption
        #client_seed_functions

        // Auto-generated CTokenSeedProvider implementation
        #ctoken_implementation

        // Suppress snake_case warnings for account type names in macro usage
        #[allow(non_snake_case)]
        #module
    })
}

/// Generate CTokenAccountVariant enum automatically from token seed specifications
#[inline(never)]
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
#[inline(never)]
fn generate_ctoken_seed_provider_implementation(
    token_seeds: &[TokenSeedSpec],
) -> Result<TokenStream> {
    let mut get_seeds_match_arms = Vec::new();
    let mut get_authority_seeds_match_arms = Vec::new();

    for spec in token_seeds {
        let variant_name = &spec.variant;

        // Generate bindings for token account seeds (always use the main seeds, not authority)
        let mut token_bindings = Vec::new();
        let mut token_seed_refs = Vec::new();

        for (i, seed) in spec.seeds.iter().enumerate() {
            match seed {
                SeedElement::Literal(lit) => {
                    let value = lit.value();
                    token_seed_refs.push(quote! { #value.as_bytes() });
                }
                SeedElement::Expression(expr) => {
                    // Check if this is a simple const identifier (like POOL_VAULT_SEED)
                    if let syn::Expr::Path(path_expr) = expr {
                        if let Some(ident) = path_expr.path.get_ident() {
                            // Check if it's all uppercase (likely a const)
                            let ident_str = ident.to_string();
                            if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                                // This looks like a const - use it as a seed
                                token_seed_refs.push(quote! { #ident.as_bytes() });
                                continue;
                            }
                        }
                    }

                    // For CToken seeds, we need to handle account references
                    // specially ctx.accounts.mint -> ctx.accounts.mint.key().as_ref()
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
                                                        // This is ctx.accounts.field_name - handle optional
                                                        let binding_name = syn::Ident::new(
                                                            &format!("seed_{}", i),
                                                            expr.span(),
                                                        );
                                                        token_bindings.push(quote! {
                                                            let #binding_name = ctx.accounts.#field_name.as_ref()
                                                                .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                                                                .key();
                                                        });
                                                        token_seed_refs.push(
                                                            quote! { #binding_name.as_ref() },
                                                        );
                                                        handled = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if let syn::Expr::Path(path) = &*field_expr.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "ctx" {
                                            // This is ctx.field_name - all fields accessed via ctx.accounts
                                            let binding_name = syn::Ident::new(
                                                &format!("seed_{}", i),
                                                expr.span(),
                                            );
                                            token_bindings.push(quote! {
                                                let #binding_name = ctx.accounts.#field_name.as_ref()
                                                    .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                                                    .key();
                                            });
                                            token_seed_refs.push(quote! { #binding_name.as_ref() });
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
                        token_seed_refs.push(quote! { (#expr).as_ref() });
                    }
                }
            }
        }

        // Always generate get_seeds to return TOKEN ACCOUNT seeds (for decompression)
        let get_seeds_arm = quote! {
            CTokenAccountVariant::#variant_name => {
                #(#token_bindings)*
                let seeds: &[&[u8]] = &[#(#token_seed_refs),*];
                let (token_account_pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                // Pre-allocate on heap with known capacity to minimize stack usage
                let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                seeds_vec.push(vec![bump]);
                Ok((seeds_vec, token_account_pda))
            }
        };
        get_seeds_match_arms.push(get_seeds_arm);

        // Generate get_authority_seeds if authority is specified (for compression signing)
        if let Some(authority_seeds) = &spec.authority {
            let mut auth_bindings: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut auth_seed_refs = Vec::new();

            for (i, authority_seed) in authority_seeds.iter().enumerate() {
                match authority_seed {
                    SeedElement::Literal(lit) => {
                        let value = lit.value();
                        auth_seed_refs.push(quote! { #value.as_bytes() });
                    }
                    SeedElement::Expression(expr) => {
                        let mut handled = false;
                        match expr {
                            // Handle ctx.accounts.field -> use .key().as_ref()
                            syn::Expr::Field(field_expr) => {
                                if let syn::Member::Named(field_name) = &field_expr.member {
                                    if let syn::Expr::Field(nested_field) = &*field_expr.base {
                                        if let syn::Member::Named(base_name) = &nested_field.member
                                        {
                                            if base_name == "accounts" {
                                                if let syn::Expr::Path(path) = &*nested_field.base {
                                                    if let Some(segment) =
                                                        path.path.segments.first()
                                                    {
                                                        if segment.ident == "ctx" {
                                                            let binding_name = syn::Ident::new(
                                                                &format!("authority_seed_{}", i),
                                                                expr.span(),
                                                            );
                                                            auth_bindings.push(quote! {
                                                                let #binding_name = ctx.accounts.#field_name.as_ref()
                                                                    .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                                                                    .key();
                                                            });
                                                            auth_seed_refs.push(
                                                                quote! { #binding_name.as_ref() },
                                                            );
                                                            handled = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if let syn::Expr::Path(path) = &*field_expr.base {
                                        if let Some(segment) = path.path.segments.first() {
                                            if segment.ident == "ctx" {
                                                let binding_name = syn::Ident::new(
                                                    &format!("authority_seed_{}", i),
                                                    expr.span(),
                                                );
                                                auth_bindings.push(quote! {
                                                    let #binding_name = ctx.accounts.#field_name.as_ref()
                                                        .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                                                        .key();
                                                });
                                                auth_seed_refs
                                                    .push(quote! { #binding_name.as_ref() });
                                                handled = true;
                                            }
                                        }
                                    }
                                }
                            }
                            // Handle method calls like ctx.accounts.mint.key() -> as_ref()
                            syn::Expr::MethodCall(_mc) => {
                                auth_seed_refs.push(quote! { (#expr).as_ref() });
                                handled = true;
                            }
                            // Handle uppercase consts
                            syn::Expr::Path(path_expr) => {
                                if let Some(ident) = path_expr.path.get_ident() {
                                    let ident_str = ident.to_string();
                                    if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                                        auth_seed_refs.push(quote! { #ident.as_bytes() });
                                        handled = true;
                                    }
                                }
                            }
                            _ => {}
                        }

                        if !handled {
                            auth_seed_refs.push(quote! { (#expr).as_ref() });
                        }
                    }
                }
            }

            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    #(#auth_bindings)*
                    let seeds: &[&[u8]] = &[#(#auth_seed_refs),*];
                    let (authority_pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                    seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                    seeds_vec.push(vec![bump]);
                    Ok((seeds_vec, authority_pda))
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        } else {
            // No authority specified - should not happen due to validation above
            let authority_arm = quote! {
                CTokenAccountVariant::#variant_name => {
                    Err(CompressibleInstructionError::MissingSeedAccount.into())
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        }
    }

    Ok(quote! {
        /// Auto-generated CTokenSeedProvider implementation
        impl ctoken_seed_system::CTokenSeedProvider for CTokenAccountVariant {
            /// Get seeds for the token account PDA (used for decompression - the owner of the compressed token)
            fn get_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, anchor_lang::prelude::Pubkey)> {
                match self {
                    #(#get_seeds_match_arms)*
                    _ => {
                        Err(CompressibleInstructionError::MissingSeedAccount.into())
                    }
                }
            }

            /// Get authority seeds for signing during compression (if authority is specified)
            fn get_authority_seeds<'a, 'info>(
                &self,
                ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, anchor_lang::prelude::Pubkey)> {
                match self {
                    #(#get_authority_seeds_match_arms)*
                    _ => {
                        Err(CompressibleInstructionError::MissingSeedAccount.into())
                    }
                }
            }
        }
    })
}

/// Generate PDA seed derivation from specification
#[inline(never)]
fn generate_pda_seed_derivation(
    spec: &TokenSeedSpec,
    _instruction_data: &[InstructionDataSpec],
    accounts_ident: &Ident,
) -> Result<TokenStream> {
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
                if let syn::Expr::Path(path_expr) = expr {
                    if let Some(ident) = path_expr.path.get_ident() {
                        let ident_str = ident.to_string();
                        if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                            seed_refs.push(quote! { #ident.as_bytes() });
                            continue;
                        }
                    }
                }

                let mut handled = false;

                match expr {
                    syn::Expr::MethodCall(mc) if mc.method == "to_le_bytes" => {
                        let binding_name =
                            syn::Ident::new(&format!("seed_binding_{}", i), expr.span());
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
                                                    // Handle optional accounts by checking for Some and unwrapping
                                                    let binding_name = syn::Ident::new(
                                                        &format!("seed_binding_{}", i),
                                                        expr.span(),
                                                    );
                                                    bindings.push(quote! {
                                                        let #binding_name = #accounts_ident.#field_name.as_ref()
                                                            .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                                                            .key();
                                                    });
                                                    seed_refs
                                                        .push(quote! { #binding_name.as_ref() });
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
                                        let binding_name = syn::Ident::new(
                                            &format!("seed_binding_{}", i),
                                            expr.span(),
                                        );
                                        bindings.push(quote! {
                                            let #binding_name = #accounts_ident.#field_name.as_ref()
                                                .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                                                .key();
                                        });
                                        seed_refs.push(quote! { #binding_name.as_ref() });
                                        handled = true;
                                    } else if segment.ident == "data" {
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
            let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
            #(
                seeds_vec.push(seeds[#indices].to_vec());
            )*
            seeds_vec.push(vec![bump]);
            (seeds_vec, pda)
        }
    })
}

/// Generate public client-side seed functions for external consumption
#[inline(never)]
fn generate_client_seed_functions(
    _account_types: &[Ident],
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
    instruction_data: &[InstructionDataSpec],
) -> Result<TokenStream> {
    let mut functions = Vec::new();

    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            let variant_name = &spec.variant;
            let function_name =
                format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());

            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let seed_count = seed_expressions.len();
            let function = quote! {
                /// Auto-generated client-side seed function
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                    let mut seed_values = Vec::with_capacity(#seed_count + 1);
                    #(
                        seed_values.push((#seed_expressions).to_vec());
                    )*
                    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
                    seed_values.push(vec![bump]);
                    (seed_values, pda)
                }
            };
            functions.push(function);
        }
    }

    // Generate CToken seed functions - FULLY GENERIC based on seed specifications
    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let variant_name = &spec.variant;
            let function_name =
                format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());

            // ALWAYS generate the regular token account seed function (for decompress)
            // This uses the token account's own seeds
            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let seed_count = seed_expressions.len();
            let function = quote! {
                /// Auto-generated client-side CToken seed function (for token account address derivation)
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                    // Pre-allocate on heap with known capacity to minimize stack usage
                    let mut seed_values = Vec::with_capacity(#seed_count + 1);
                    #(
                        seed_values.push((#seed_expressions).to_vec());
                    )*
                    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
                    seed_values.push(vec![bump]);
                    (seed_values, pda)
                }
            };
            functions.push(function);

            // Generate authority seed function (for compress signing). Required.
            if let Some(authority_seeds) = &spec.authority {
                let authority_function_name = format_ident!(
                    "get_{}_authority_seeds",
                    variant_name.to_string().to_lowercase()
                );

                let mut authority_spec = TokenSeedSpec {
                    variant: spec.variant.clone(),
                    _eq: spec._eq.clone(),
                    is_token: spec.is_token,
                    seeds: Punctuated::new(),
                    authority: None,
                };

                for auth_seed in authority_seeds {
                    authority_spec.seeds.push(auth_seed.clone());
                }

                let (auth_parameters, auth_seed_expressions) =
                    analyze_seed_spec_for_client(&authority_spec, instruction_data)?;

                let auth_seed_count = auth_seed_expressions.len();
                let authority_function = quote! {
                    /// Auto-generated authority seed function for compression signing
                    pub fn #authority_function_name(#(#auth_parameters),*) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
                        let mut seed_values = Vec::with_capacity(#auth_seed_count + 1);
                        #(
                            seed_values.push((#auth_seed_expressions).to_vec());
                        )*
                        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
                        let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
                        seed_values.push(vec![bump]);
                        (seed_values, pda)
                    }
                };
                functions.push(authority_function);
            }
        }
    }

    Ok(quote! {
        #(#functions)*
    })
}

/// Analyze seed specification and generate parameters + expressions for client functions
#[inline(never)]
fn analyze_seed_spec_for_client(
    spec: &TokenSeedSpec,
    instruction_data: &[InstructionDataSpec],
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
                        // Handle data.field, ctx.field, or ctx.accounts.field
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
                                                        parameters.push(quote! { #field_name: &anchor_lang::prelude::Pubkey });
                                                        expressions
                                                            .push(quote! { #field_name.as_ref() });
                                                    } else {
                                                        // Other nested field
                                                        parameters.push(quote! { #field_name: &anchor_lang::prelude::Pubkey });
                                                        expressions
                                                            .push(quote! { #field_name.as_ref() });
                                                    }
                                                } else {
                                                    parameters.push(quote! { #field_name: &anchor_lang::prelude::Pubkey });
                                                    expressions
                                                        .push(quote! { #field_name.as_ref() });
                                                }
                                            } else {
                                                parameters.push(quote! { #field_name: &anchor_lang::prelude::Pubkey });
                                                expressions.push(quote! { #field_name.as_ref() });
                                            }
                                        } else {
                                            // Other nested field
                                            parameters.push(quote! { #field_name: &anchor_lang::prelude::Pubkey });
                                            expressions.push(quote! { #field_name.as_ref() });
                                        }
                                    } else {
                                        parameters.push(
                                            quote! { #field_name: &anchor_lang::prelude::Pubkey },
                                        );
                                        expressions.push(quote! { #field_name.as_ref() });
                                    }
                                }
                                syn::Expr::Path(path) => {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "data" {
                                            // This is a data field - look up the type from instruction_data
                                            if let Some(data_spec) = instruction_data
                                                .iter()
                                                .find(|d| d.field_name == *field_name)
                                            {
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
                                                return Err(macro_error!(
                                                    field_name,
                                                    "data.{} used in seeds but no type specified. Add: {} = Pubkey (or u8, u16, u64)", 
                                                    field_name, field_name
                                                ));
                                            }
                                        } else {
                                            // ctx.field - all fields are Pubkeys accessed via ctx.accounts
                                            parameters.push(quote! { #field_name: &anchor_lang::prelude::Pubkey });
                                            expressions.push(quote! { #field_name.as_ref() });
                                        }
                                    } else {
                                        parameters.push(
                                            quote! { #field_name: &anchor_lang::prelude::Pubkey },
                                        );
                                        expressions.push(quote! { #field_name.as_ref() });
                                    }
                                }
                                _ => {
                                    parameters.push(
                                        quote! { #field_name: &anchor_lang::prelude::Pubkey },
                                    );
                                    expressions.push(quote! { #field_name.as_ref() });
                                }
                            }
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
                                            if let Some(data_spec) = instruction_data
                                                .iter()
                                                .find(|d| d.field_name == *field_name)
                                            {
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
                                                expressions.push(
                                                    quote! { #field_name.#method_name().as_ref() },
                                                );
                                            } else {
                                                return Err(macro_error!(
                                                    field_name,
                                                    "data.{} used in seeds but no type specified. Add: {} = Pubkey (or u8, u16, u64)", 
                                                    field_name, field_name
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
                    syn::Expr::Path(path_expr) => {
                        // Handle direct identifiers (could be const or account)
                        if let Some(ident) = path_expr.path.get_ident() {
                            let ident_str = ident.to_string();
                            // Check if it's an uppercase const
                            if ident_str
                                .chars()
                                .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                            {
                                // This is a const - use it directly, no parameter needed
                                expressions.push(quote! { #ident.as_bytes() });
                            } else {
                                // This is an account reference - add as parameter
                                parameters.push(quote! { #ident: &anchor_lang::prelude::Pubkey });
                                expressions.push(quote! { #ident.as_ref() });
                            }
                        } else {
                            // Complex path - use as-is
                            expressions.push(quote! { (#expr).as_ref() });
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
#[inline(never)]
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

/// Dependency information for compressible accounts
struct AccountDependency {
    account_name: String,
    required_seeds: Vec<String>,
}

/// Extract required account names from seed expressions and track dependencies
///
/// IMPORTANT: Preserve deterministic, insertion order based on the order of
/// appearance in the macro arguments (first PDA seeds, then token seeds) and
/// the left-to-right order within each seed tuple. This guarantees stable IDL
/// and Anchor account struct field ordering and prevents name/position
/// mismatches between client and on-chain.
///
/// Returns: (all_required_accounts, account_dependencies)
#[inline(never)]
fn extract_required_accounts_from_seeds(
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
) -> Result<(Vec<String>, Vec<AccountDependency>)> {
    // Use a Vec to preserve insertion order and perform manual dedup.
    // The number of accounts is small, so O(n^2) dedup is fine and avoids
    // bringing in external crates for ordered sets.
    let mut required_accounts: Vec<String> = Vec::new();
    let mut dependencies: Vec<AccountDependency> = Vec::new();

    // Helper to push if not present yet, preserving order.
    #[inline(always)]
    fn push_unique(list: &mut Vec<String>, value: String) {
        if !list.iter().any(|v| v == &value) {
            list.push(value);
        }
    }

    // Local wrapper that delegates to the expression walkers below.
    #[inline(never)]
    fn extract_accounts_from_seed_spec(
        spec: &TokenSeedSpec,
        ordered_accounts: &mut Vec<String>,
    ) -> Result<Vec<String>> {
        let mut spec_accounts = Vec::new();
        for seed in &spec.seeds {
            if let SeedElement::Expression(expr) = seed {
                let mut local_accounts = Vec::new();
                extract_account_from_expr(expr, &mut local_accounts);
                for acc in local_accounts {
                    push_unique(ordered_accounts, acc.clone());
                    push_unique(&mut spec_accounts, acc);
                }
            }
        }
        // Also check authority seeds for token accounts
        if let Some(authority_seeds) = &spec.authority {
            for seed in authority_seeds {
                if let SeedElement::Expression(expr) = seed {
                    let mut local_accounts = Vec::new();
                    extract_account_from_expr(expr, &mut local_accounts);
                    for acc in local_accounts {
                        push_unique(ordered_accounts, acc.clone());
                        push_unique(&mut spec_accounts, acc);
                    }
                }
            }
        }
        Ok(spec_accounts)
    }

    // Walk PDA seeds in declared order
    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            let required_seeds = extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
            dependencies.push(AccountDependency {
                account_name: spec.variant.to_string(),
                required_seeds,
            });
        }
    }

    // Then token seeds in declared order
    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let required_seeds = extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
            dependencies.push(AccountDependency {
                account_name: spec.variant.to_string(),
                required_seeds,
            });
        }
    }

    Ok((required_accounts, dependencies))
}

/// Extract account names from a seed expression, preserving insertion order.
/// Looks for ctx.accounts.FIELD_NAME pattern and extracts FIELD_NAME; also
/// supports ctx.FIELD_NAME shorthand and direct identifiers that are not
/// constants.
#[inline(never)]
fn extract_account_from_expr(expr: &syn::Expr, ordered_accounts: &mut Vec<String>) {
    // Helper to push unique values
    #[inline(always)]
    fn push_unique(list: &mut Vec<String>, value: String) {
        if !list.iter().any(|v| v == &value) {
            list.push(value);
        }
    }

    match expr {
        syn::Expr::MethodCall(method_call) => {
            // For method calls, check the receiver
            // e.g., ctx.accounts.mint.key().as_ref() -> check ctx.accounts.mint.key()
            extract_account_from_expr(&*method_call.receiver, ordered_accounts);
        }
        syn::Expr::Field(field_expr) => {
            // Check if this is ctx.accounts.FIELD_NAME or ctx.FIELD_NAME
            if let syn::Member::Named(field_name) = &field_expr.member {
                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                    if let syn::Member::Named(base_name) = &nested_field.member {
                        if base_name == "accounts" {
                            if let syn::Expr::Path(path) = &*nested_field.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        push_unique(ordered_accounts, field_name.to_string());
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
                            push_unique(ordered_accounts, field_name.to_string());
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
                // Skip "ctx", "data", and uppercase consts (like POOL_VAULT_SEED)
                if name != "ctx"
                    && name != "data"
                    && !name
                        .chars()
                        .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                {
                    push_unique(ordered_accounts, name);
                }
            }
        }
        _ => {
            // Ignore other expression types
        }
    }
}

/// Generate DecompressAccountsIdempotent struct with required accounts
#[inline(never)]
fn generate_decompress_accounts_struct(
    required_accounts: &[String],
    account_dependencies: &[AccountDependency],
    variant: InstructionVariant,
) -> Result<syn::ItemStruct> {
    let mut account_fields = vec![
        // Standard fields always present
        quote! {
            #[account(mut)]
            pub fee_payer: Signer<'info>
        },
        quote! {
            /// The global config account
            /// CHECK: load_checked.
            pub config: AccountInfo<'info>
        },
    ];

    // Add rent payer fields based on variant
    match variant {
        InstructionVariant::PdaOnly => {
            // PDA-only: only need rent_payer for PDAs
            account_fields.push(quote! {
                /// UNCHECKED: Anyone can pay to init PDAs.
                #[account(mut)]
                pub rent_payer: Signer<'info>
            });
        }
        InstructionVariant::TokenOnly => {
            // Token-only: only need ctoken_rent_sponsor for tokens
            account_fields.push(quote! {
                /// UNCHECKED: Anyone can pay to init compressed tokens.
                #[account(mut)]
                pub ctoken_rent_sponsor: Signer<'info>
            });
        }
        InstructionVariant::Mixed => {
            // Mixed: need both rent payers
            account_fields.extend(vec![
                quote! {
                    /// UNCHECKED: Anyone can pay to init PDAs.
                    #[account(mut)]
                    pub rent_payer: Signer<'info>
                },
                quote! {
                    /// UNCHECKED: Anyone can pay to init compressed tokens.
                    #[account(mut)]
                    pub ctoken_rent_sponsor: AccountInfo<'info>
                },
            ]);
        }
    }

    // Add token-specific accounts based on variant
    match variant {
        InstructionVariant::TokenOnly => {
            // Token-only: required token program accounts
            account_fields.extend(vec![
                quote! {
                    /// Compressed token program
                    /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
                    pub ctoken_program: UncheckedAccount<'info>
                },
                quote! {
                    /// CPI authority PDA of the compressed token program
                    /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
                    pub ctoken_cpi_authority: UncheckedAccount<'info>
                },
            ]);
        }
        InstructionVariant::Mixed => {
            // Mixed: required token program accounts with address constraints for constants
            // Use hardcoded well-known Pubkeys for ctoken program and cpi authority
            account_fields.extend(vec![
                quote! {
                    /// Compressed token program (auto-resolved constant)
                    /// CHECK: Enforced to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
                    #[account(address = anchor_lang::solana_program::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
                    pub ctoken_program: UncheckedAccount<'info>
                },
                quote! {
                    /// CPI authority PDA of the compressed token program (auto-resolved constant)
                    /// CHECK: Enforced to be GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy
                    #[account(address = anchor_lang::solana_program::pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
                    pub ctoken_cpi_authority: UncheckedAccount<'info>
                },
                quote! {
                    /// CHECK: CToken CompressibleConfig account (default but can be overridden)
                    pub ctoken_config: UncheckedAccount<'info>
                },
            ]);
        }
        InstructionVariant::PdaOnly => {
            // No token-specific accounts for PDA-only variant
        }
    }

    // Add required accounts as OPTIONAL unchecked accounts (skip standard fields)
    // Seed accounts are optional - only needed if their dependent compressible account is being decompressed
    let standard_fields = [
        "fee_payer",
        "rent_payer",
        "ctoken_rent_sponsor",
        "config",
        "ctoken_program",
        "ctoken_cpi_authority",
        "ctoken_config",
    ];

    for account_name in required_accounts {
        if !standard_fields.contains(&account_name.as_str()) {
            let account_ident = syn::Ident::new(account_name, proc_macro2::Span::call_site());
            account_fields.push(quote! {
                /// CHECK: Optional seed account - required only if decompressing dependent accounts.
                /// Validated by runtime checks when needed.
                pub #account_ident: Option<UncheckedAccount<'info>>
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

/// Generate PDA-only compress accounts struct (no token program accounts)
#[inline(never)]
fn generate_pda_only_compress_accounts_struct() -> Result<syn::ItemStruct> {
    Ok(syn::parse_quote! {
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

            /// CHECK: compression_authority must be the rent_authority defined when creating the PDA account.
            #[account(mut)]
            pub compression_authority: AccountInfo<'info>,
        }
    })
}

/// Generate token-only compress accounts struct (only token program accounts)
#[inline(never)]
fn generate_token_only_compress_accounts_struct() -> Result<syn::ItemStruct> {
    Ok(syn::parse_quote! {
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

            /// Token rent recipient - must match config
            /// CHECK: Token rent recipient is validated against the config
            #[account(mut)]
            pub token_rent_recipient: AccountInfo<'info>,
            /// Compressed token program
            /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
            pub compressed_token_program: UncheckedAccount<'info>,

            /// CPI authority PDA of the compressed token program
            /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
            pub compressed_token_cpi_authority: UncheckedAccount<'info>,
        }
    })
}

/// Validate that all compressed account types don't exceed the maximum size limit
#[inline(never)]
fn validate_compressed_account_sizes(account_types: &[Ident]) -> Result<TokenStream> {
    let size_checks: Vec<_> = account_types.iter().map(|account_type| {
        quote! {
            const _: () = {
                // Use COMPRESSED_INIT_SPACE, computed by the Compressible
                // derive. Considers compress_as attributes.
                const COMPRESSED_SIZE: usize = 8 + <#account_type as light_sdk::compressible::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
                if COMPRESSED_SIZE > 800 {
                    panic!(concat!(
                        "Compressed account '", stringify!(#account_type), "' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com"
                    ));
                }
            };
        }
    }).collect();

    Ok(quote! { #(#size_checks)* })
}

/// Helper to extract account name from seed expression (used internally)
/// Handles: ctx.accounts.field_name, ctx.field_name, field_name
fn extract_seed_account_name(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                // Check if this is ctx.accounts.field_name or ctx.field_name
                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                    if let syn::Member::Named(base_name) = &nested_field.member {
                        if base_name == "accounts" {
                            return Some(field_name.to_string());
                        }
                    }
                } else if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "ctx" {
                            return Some(field_name.to_string());
                        }
                    }
                }
            }
            None
        }
        syn::Expr::Path(path_expr) => {
            // Handle direct identifiers (not ctx references)
            if let Some(ident) = path_expr.path.get_ident() {
                let name = ident.to_string();
                // Skip ctx, data, and uppercase constants
                if name != "ctx"
                    && name != "data"
                    && !name
                        .chars()
                        .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                {
                    return Some(name);
                }
            }
            None
        }
        syn::Expr::MethodCall(method_call) => {
            // Recursively extract from method call receiver (e.g., ctx.accounts.mint.key())
            extract_seed_account_name(&*method_call.receiver)
        }
        _ => None,
    }
}

/// Generate error codes automatically based on instruction variant
/// This generates additional error variants that get added to the user's ErrorCode enum
#[inline(never)]
fn generate_error_codes(variant: InstructionVariant) -> Result<TokenStream> {
    let base_errors = quote! {
        #[msg("Rent recipient does not match config")]
        InvalidRentRecipient,
        #[msg("Required seed account is missing for decompression - check that all seed accounts for compressed accounts are provided")]
        MissingSeedAccount,
    };

    let variant_specific_errors = match variant {
        InstructionVariant::PdaOnly => quote! {
            #[msg("Token compression not implemented in PDA-only variant")]
            TokenCompressionNotImplemented,
        },
        InstructionVariant::TokenOnly => quote! {
            #[msg("PDA decompression not implemented in token-only variant")]
            PdaDecompressionNotImplemented,
            #[msg("PDA compression not implemented in token-only variant")]
            PdaCompressionNotImplemented,
        },
        InstructionVariant::Mixed => quote! {
            #[msg("CToken decompression not yet implemented")]
            CTokenDecompressionNotImplemented,
            #[msg("PDA decompression not implemented in token-only variant")]
            PdaDecompressionNotImplemented,
            #[msg("Token compression not implemented in PDA-only variant")]
            TokenCompressionNotImplemented,
            #[msg("PDA compression not implemented in token-only variant")]
            PdaCompressionNotImplemented,
        },
    };

    // Generate macro-specific error codes that don't conflict with user's ErrorCode
    Ok(quote! {
        /// Auto-generated error codes for compressible instructions
        /// These are separate from the user's ErrorCode enum to avoid conflicts
        #[error_code]
        pub enum CompressibleInstructionError {
            #base_errors
            #variant_specific_errors
        }
    })
}
