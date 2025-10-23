use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Item, ItemMod, LitStr, Result, Token,
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
pub(crate) enum InstructionVariant {
    /// Only PDA seeds specified - generate PDA-only instructions
    PdaOnly,
    /// Only token seeds specified - generate token-only instructions  
    TokenOnly,
    /// Both PDA and token seeds specified - generate mixed instructions
    Mixed,
}

/// Parse seed specification for a token account variant
#[derive(Clone)]
pub(crate) struct TokenSeedSpec {
    pub variant: Ident,
    pub _eq: Token![=],
    pub is_token: Option<bool>, // Optional explicit token flag
    pub is_ata: bool,           // Flag for user-owned ATA (no seeds/authority needed)
    pub seeds: Punctuated<SeedElement, Token![,]>,
    pub authority: Option<Vec<SeedElement>>, // Optional authority seeds for CToken accounts
}

impl Parse for TokenSeedSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let variant = input.parse()?;
        let _eq = input.parse()?;

        let content;
        syn::parenthesized!(content in input);

        // Check if first element is an explicit token flag
        let (is_token, is_ata, seeds, authority) = if content.peek(Ident) {
            let first_ident: Ident = content.parse()?;

            match first_ident.to_string().as_str() {
                "is_token" => {
                    // Explicit token flag - check for is_ata
                    let _comma: Token![,] = content.parse()?;

                    // Check if next is is_ata
                    if content.peek(Ident) {
                        let fork = content.fork();
                        if let Ok(second_ident) = fork.parse::<Ident>() {
                            if second_ident == "is_ata" {
                                // Consume is_ata
                                let _: Ident = content.parse()?;
                                // ATAs have no seeds or authority
                                return Ok(TokenSeedSpec {
                                    variant,
                                    _eq,
                                    is_token: Some(true),
                                    is_ata: true,
                                    seeds: Punctuated::new(),
                                    authority: None,
                                });
                            }
                        }
                    }

                    // Regular token (not ATA) - parse seeds and authority
                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(true), false, seeds, authority)
                }
                "true" => {
                    // Explicit token flag
                    let _comma: Token![,] = content.parse()?;
                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(true), false, seeds, authority)
                }
                "is_pda" | "false" => {
                    // Explicit PDA flag
                    let _comma: Token![,] = content.parse()?;
                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(false), false, seeds, authority)
                }
                _ => {
                    // Not a flag, treat as first seed element
                    let mut seeds = Punctuated::new();
                    seeds.push(SeedElement::Expression(Box::new(syn::Expr::Path(
                        syn::ExprPath {
                            attrs: vec![],
                            qself: None,
                            path: syn::Path::from(first_ident),
                        },
                    ))));

                    if content.peek(Token![,]) {
                        let _comma: Token![,] = content.parse()?;
                        let (rest, authority) = parse_seeds_with_authority(&content)?;
                        seeds.extend(rest);
                        (None, false, seeds, authority)
                    } else {
                        (None, false, seeds, None)
                    }
                }
            }
        } else {
            // No identifier first, parse all as seeds
            let (seeds, authority) = parse_seeds_with_authority(&content)?;
            (None, false, seeds, authority)
        };

        Ok(TokenSeedSpec {
            variant,
            _eq,
            is_token,
            is_ata,
            seeds,
            authority,
        })
    }
}

// Helper function to parse seeds and extract authority if present
#[allow(clippy::type_complexity)]
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
pub(crate) enum SeedElement {
    /// String literal like "user_record"
    Literal(LitStr),
    /// Any expression: data.owner, ctx.fee_payer, data.session_id.to_le_bytes(), CONST_NAME, etc.
    Expression(Box<Expr>),
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
pub(crate) struct InstructionDataSpec {
    pub field_name: Ident,
    pub field_type: syn::Type,
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

        let mut _item_count = 0;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;

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
            _item_count += 1;
        }
        Ok(EnhancedMacroArgs {
            account_types,
            pda_seeds,
            token_seeds,
            instruction_data,
        })
    }
}

/// Generate full mixed PDA + compressed-token support for an Anchor program module.
///
/// This macro is a thin wrapper that wires together the lower-level derive macros
/// (`DeriveSeeds`, `DeriveCTokenSeeds`, `Compressible`, `CompressiblePack`) and the
/// runtime traits in `light_sdk::compressible`.
///
/// ### Usage (mixed PDA + token)
/// ```ignore
/// use light_sdk_macros::{Compressible, CompressiblePack, DeriveSeeds};
///
/// #[derive(Compressible, CompressiblePack, DeriveSeeds)]
/// #[seeds("user_record", owner)]
/// #[account]
/// pub struct UserRecord { /* ... */ }
///
/// #[add_compressible_instructions(
///     // PDA account types (must already implement PdaSeedProvider via DeriveSeeds)
///     UserRecord   = ("user_record", data.owner),
///
///     // Token variant (ctoken account) – must start with `is_token`
///     CTokenSigner = (is_token, "ctoken_signer", ctx.user, ctx.mint),
///
///     // Instruction data fields used in the seed expressions above
///     owner = Pubkey,
/// )]
/// #[program]
/// pub mod my_program { /* regular instructions here */ }
/// ```
///
/// ### It generates:
/// - Compile-time account size checks (max 800 bytes).
/// - `CTokenAccountVariant` and `CompressedAccountVariant` enums + all required traits.
/// - Accounts structs for compression/decompression.
/// - Instruction entrypoints: decompress, compress, config.
/// - `PdaSeedProvider` implementations for each PDA type derived from the macro seeds.
/// - Token seed providers (either via `DeriveCTokenSeeds` or the legacy generator).
/// - Client helpers for deriving **token** PDAs.
///
/// Notes:
/// - Currently the macro is designed for **mixed** flows (at least one PDA account
///   and at least one token variant). Pure‑PDA or pure‑token configurations are not
///   yet supported.
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

    // Generate the CTokenAccountVariant enum automatically from token_seeds.
    let ctoken_enum = if let Some(ref token_seed_specs) = token_seeds {
        if !token_seed_specs.is_empty() {
            crate::ctoken_seed_generation::generate_ctoken_account_variant_enum(token_seed_specs)?
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
    // (except for ATAs which are user-owned and don't need authority)
    if let Some(ref token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            if spec.is_ata {
                // ATAs must not have seeds or authority
                if !spec.seeds.is_empty() {
                    return Err(macro_error!(
                        &spec.variant,
                        "ATA variant '{}' must not have seeds - ATAs are derived from owner+mint only",
                        spec.variant
                    ));
                }
                if spec.authority.is_some() {
                    return Err(macro_error!(
                        &spec.variant,
                        "ATA variant '{}' must not have authority - ATAs are owned by user wallets",
                        spec.variant
                    ));
                }
            } else if spec.authority.is_none() {
                // Non-ATA tokens must have authority for PDA signing
                return Err(macro_error!(
                    &spec.variant,
                    "Program-owned token account '{}' must specify authority = <seed_expr> for compression signing. For user-owned ATAs, use is_ata flag instead.",
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
    let required_accounts = extract_required_accounts_from_seeds(&pda_seeds, &token_seeds)?;

    // Generate the DecompressAccountsIdempotent accounts struct with required accounts
    let decompress_accounts =
        generate_decompress_accounts_struct(&required_accounts, instruction_variant)?;

    // Generate PdaSeedProvider implementations for each PDA account type from the macro seeds.
    let pda_seed_provider_impls: Result<Vec<_>> = account_types
        .iter()
        .map(|name| {
            let name_str = name.to_string();
            let spec = if let Some(ref pda_seed_specs) = pda_seeds {
                pda_seed_specs
                    .iter()
                    .find(|s| s.variant == name_str)
                    .ok_or_else(|| {
                        macro_error!(
                            name,
                            "No seed specification for account type '{}'. All accounts must have seed specifications.",
                            name_str
                        )
                    })?
            } else {
                return Err(macro_error!(
                    name,
                    "No seed specifications provided. Use: AccountType = (\"seed\", data.field)"
                ));
            };
            let seed_derivation =
                generate_pda_seed_derivation_for_trait(spec, &instruction_data)?;
            Ok(quote! {
                impl light_sdk::compressible::PdaSeedProvider for #name {
                    fn derive_pda_seeds(
                        &self,
                        program_id: &solana_pubkey::Pubkey,
                    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                        #seed_derivation
                    }
                }
            })
        })
        .collect();
    let pda_seed_provider_impls = pda_seed_provider_impls?;

    // Generate thin helper functions that delegate to SDK
    let helper_packed_fns: Vec<_> = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        let func_name = format_ident!("handle_packed_{}", name);
        quote! {
            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn #func_name<'a, 'b, 'info>(
                accounts: &DecompressAccountsIdempotent<'info>,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
                address_space: solana_pubkey::Pubkey,
                solana_accounts: &[solana_account_info::AccountInfo<'info>],
                i: usize,
                packed: &#packed_name,
                meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                post_system_accounts: &[solana_account_info::AccountInfo<'info>],
                compressed_pda_infos: &mut Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                light_sdk::compressible::handle_packed_pda_variant::<#name, #packed_name>(
                    &accounts.rent_payer,
                    cpi_accounts,
                    address_space,
                    &solana_accounts[i],
                    i,
                    packed,
                    meta,
                    post_system_accounts,
                    compressed_pda_infos,
                    &crate::ID,
                )
            }
        }
    }).collect();

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
                                match #func_name(accounts, &cpi_accounts, address_space, solana_accounts, i, &packed, &meta, post_system_accounts, &mut compressed_pda_infos) {
                                    std::result::Result::Ok(()) => {},
                                    std::result::Result::Err(e) => return std::result::Result::Err(e),
                                }
                            }
        }
    }).collect();

    // Generate trait implementations for runtime compatibility
    let trait_impls: syn::ItemMod = syn::parse_quote! {
        /// Trait implementations for standardized runtime helpers
        mod __trait_impls {
            use super::*;

            /// Implement HasTokenVariant for CompressedAccountVariant
            impl light_sdk::compressible::HasTokenVariant for CompressedAccountData {
                fn is_packed_ctoken(&self) -> bool {
                    matches!(self.data, CompressedAccountVariant::PackedCTokenData(_))
                }
            }

            /// Implement CTokenSeedProvider for CTokenAccountVariant via local seed system
            impl light_sdk::compressible::CTokenSeedProvider for CTokenAccountVariant {
                type Accounts<'info> = DecompressAccountsIdempotent<'info>;

                fn get_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), anchor_lang::prelude::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_seeds(self, &ctx).map_err(|e: anchor_lang::error::Error| -> anchor_lang::prelude::ProgramError { e.into() })
                }

                fn get_authority_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), anchor_lang::prelude::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_authority_seeds(self, &ctx).map_err(|e: anchor_lang::error::Error| -> anchor_lang::prelude::ProgramError { e.into() })
                }
            }

            /// Also implement light_compressed_token_sdk::CTokenSeedProvider for token decompression runtime
            impl light_compressed_token_sdk::CTokenSeedProvider for CTokenAccountVariant {
                type Accounts<'info> = DecompressAccountsIdempotent<'info>;

                fn get_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_seeds(self, &ctx)
                        .map_err(|e: anchor_lang::error::Error| {
                            let program_error: anchor_lang::prelude::ProgramError = e.into();
                            let code = match program_error {
                                anchor_lang::prelude::ProgramError::Custom(code) => code,
                                _ => 0,
                            };
                            solana_program_error::ProgramError::Custom(code)
                        })
                }

                fn get_authority_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_authority_seeds(self, &ctx)
                        .map_err(|e: anchor_lang::error::Error| {
                            let program_error: anchor_lang::prelude::ProgramError = e.into();
                            let code = match program_error {
                                anchor_lang::prelude::ProgramError::Custom(code) => code,
                                _ => 0,
                            };
                            solana_program_error::ProgramError::Custom(code)
                        })
                }
            }
        }
    };

    // Generate local trait system for CToken variant seed handling
    let ctoken_trait_system: syn::ItemMod = syn::parse_quote! {
        /// Local trait-based system for CToken variant seed handling
        pub mod ctoken_seed_system {
            use super::*;

            pub struct CTokenSeedContext<'a, 'info> {
                pub accounts: &'a DecompressAccountsIdempotent<'info>,
                pub remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
            }

            pub trait CTokenSeedProvider {
                /// Get seeds for the token account PDA (used for decompression)
                fn get_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;

                /// Get authority seeds for signing during compression
                fn get_authority_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;
            }
        }
    };

    // Generate helper functions inside a private submodule to avoid Anchor treating them as instructions.
    //
    // Note: PDA seed derivation is now provided by the DeriveSeeds macro (which implements
    // `light_sdk::compressible::PdaSeedProvider` for each account type). This helper module
    // only wires those traits into the generic decompression runtime.
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
                    cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
                    address_space: solana_pubkey::Pubkey,
                    compressed_accounts: Vec<CompressedAccountData>,
                    solana_accounts: &[solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(
                    Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                    Vec<(
                        light_compressed_token_sdk::compat::PackedCTokenData<CTokenAccountVariant>,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    )>,
                ), solana_program_error::ProgramError> {
                    let post_system_offset = cpi_accounts.system_accounts_end_offset();
                    let all_infos = cpi_accounts.account_infos();
                    let post_system_accounts = &all_infos[post_system_offset..];
                    let estimated_capacity = compressed_accounts.len();
                    let mut compressed_pda_infos = Vec::with_capacity(estimated_capacity);
                    let mut compressed_token_accounts: Vec<(
                        light_compressed_token_sdk::compat::PackedCTokenData<CTokenAccountVariant>,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    )> = Vec::with_capacity(estimated_capacity);

                    for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                        let meta = compressed_data.meta;
                        match compressed_data.data {
                            #(#call_unpacked_arms)*
                            #(#call_packed_arms)*
                            CompressedAccountVariant::PackedCTokenData(mut data) => {
                                data.token_data.version = 3;
                                compressed_token_accounts.push((data, meta));
                            }
                            CompressedAccountVariant::CTokenData(_) => {
                                unreachable!();
                            }
                        }
                    }

                    std::result::Result::Ok((compressed_pda_infos, compressed_token_accounts))
                }
            }
        }
    };

    // Determine token variant name
    let token_variant_name = format_ident!("CTokenAccountVariant");

    // Generate decompress-related code using helper module
    // The helper now uses the shared derive_decompress_context implementation!
    let decompress_context_impl =
        crate::compressible_instructions_decompress::generate_decompress_context_impl(
            instruction_variant,
            account_types.clone(),
            token_variant_name,
        )?;
    let decompress_processor_fn =
        crate::compressible_instructions_decompress::generate_process_decompress_accounts_idempotent(
            instruction_variant,
        )?;
    let decompress_instruction =
        crate::compressible_instructions_decompress::generate_decompress_instruction_entrypoint(
            instruction_variant,
        )?;

    // Generate the CompressAccountsIdempotent accounts struct based on variant
    let compress_accounts: syn::ItemStruct = match instruction_variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
        InstructionVariant::Mixed => syn::parse_quote! {
        #[derive(Accounts)]
        pub struct CompressAccountsIdempotent<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            /// The global config account
            /// CHECK: Config is validated by the SDK's load_checked method
            pub config: AccountInfo<'info>,
            /// Rent sponsor - must match config
            /// CHECK: Rent sponsor is validated against the config
            #[account(mut)]
            pub rent_sponsor: AccountInfo<'info>,

            /// CHECK: compression_authority must be the rent_authority defined when creating the PDA account.
            #[account(mut)]
            pub compression_authority: AccountInfo<'info>,

            /// CHECK: token_compression_authority must be the rent_authority defined when creating the token account.
            #[account(mut)]
            pub ctoken_compression_authority: AccountInfo<'info>,

            /// Token rent sponsor - must match config
            /// CHECK: Token rent sponsor is validated against the config
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

    // Generate compress-related code using helper module
    let compress_context_impl =
        crate::compressible_instructions_compress::generate_compress_context_impl(
            instruction_variant,
            account_types.clone(),
        )?;
    let compress_processor_fn =
        crate::compressible_instructions_compress::generate_process_compress_accounts_idempotent(
            instruction_variant,
        )?;
    let compress_instruction =
        crate::compressible_instructions_compress::generate_compress_instruction_entrypoint(
            instruction_variant,
        )?;

    // Wrap processor functions in a private module to avoid Anchor scanning them
    let processor_module: syn::ItemMod = syn::parse_quote! {
        mod __processor_functions {
            use super::*;
            #decompress_processor_fn
            #compress_processor_fn
        }
    };

    // OLD INLINE VERSION (keeping as comment for reference - can delete later)

    // Generate compression config instructions
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
            rent_sponsor: Pubkey,
            address_space: Vec<Pubkey>,
        ) -> Result<()> {
            light_sdk::compressible::process_initialize_compression_config_checked(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.program_data.to_account_info(),
                &rent_sponsor,
                address_space,
                compression_delay,
                0, // one global config for now, so bump is 0.
                &ctx.accounts.payer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &crate::ID,
            )?;
            Ok(())
        }
    };

    let update_config_instruction: syn::ItemFn = syn::parse_quote! {
        /// Update compression config for the program
        #[inline(never)]
        pub fn update_compression_config<'info>(
            ctx: Context<'_, '_, '_, 'info, UpdateCompressionConfig<'info>>,
            new_compression_delay: Option<u32>,
            new_rent_sponsor: Option<Pubkey>,
            new_address_space: Option<Vec<Pubkey>>,
            new_update_authority: Option<Pubkey>,
        ) -> Result<()> {
            light_sdk::compressible::process_update_compression_config(
                ctx.accounts.config.as_ref(),
                ctx.accounts.authority.as_ref(),
                new_update_authority.as_ref(),
                new_rent_sponsor.as_ref(),
                new_address_space,
                new_compression_delay,
                &crate::ID,
            )?;
            Ok(())
        }
    };

    // Add all generated items to the module
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Mod(helpers_module));
    content.1.push(Item::Mod(ctoken_trait_system));
    content.1.push(Item::Mod(trait_impls));
    content.1.push(Item::Mod(decompress_context_impl));
    content.1.push(Item::Mod(processor_module));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(Item::Struct(compress_accounts));
    content.1.push(Item::Mod(compress_context_impl));
    content.1.push(Item::Fn(compress_instruction));
    content.1.push(Item::Struct(init_config_accounts));
    content.1.push(Item::Struct(update_config_accounts));
    content.1.push(Item::Fn(init_config_instruction));
    content.1.push(Item::Fn(update_config_instruction));

    // Generate automatic CTokenSeedProvider implementation from token seed specifications.
    // This must be added to the module content so it can access ctoken_seed_system
    if let Some(ref seeds) = token_seeds {
        if !seeds.is_empty() {
            let impl_code =
                crate::ctoken_seed_generation::generate_ctoken_seed_provider_implementation(seeds)?;
            // Parse the implementation into an Item so we can add it to the module
            let ctoken_impl: syn::ItemImpl = syn::parse2(impl_code).map_err(|e| {
                syn::Error::new_spanned(
                    &seeds[0].variant,
                    format!("Failed to parse ctoken implementation: {}", e),
                )
            })?;
            content.1.push(Item::Impl(ctoken_impl));
        }
    }

    // Generate public client-side seed functions for external consumption.
    //
    // PDA seed functions are generated from the same macro seed DSL (for clients) while
    // PdaSeedProvider impls are generated above (for the on-chain runtime).
    let client_seed_functions = crate::client_seed_functions::generate_client_seed_functions(
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

        // Auto-generated PdaSeedProvider implementations for each account type
        #(#pda_seed_provider_impls)*

        // Note: CTokenSeedProvider implementation is added to module content above

        // Suppress snake_case warnings for account type names in macro usage
        #[allow(non_snake_case)]
        #module

        // Auto-generated public seed functions for client consumption (after module to avoid Anchor scanning)
        #client_seed_functions
    })
}

/// Generate PDA seed derivation for PdaSeedProvider trait implementation.
///
/// This generates seed derivation code that uses `&self` (the unpacked account data)
/// instead of extracting from an accounts struct.
#[inline(never)]
fn generate_pda_seed_derivation_for_trait(
    spec: &TokenSeedSpec,
    _instruction_data: &[InstructionDataSpec],
) -> Result<TokenStream> {
    let mut bindings = Vec::new();
    let mut seed_refs = Vec::new();

    for (i, seed) in spec.seeds.iter().enumerate() {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                seed_refs.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                // Check for uppercase consts
                if let syn::Expr::Path(path_expr) = &**expr {
                    if let Some(ident) = path_expr.path.get_ident() {
                        let ident_str = ident.to_string();
                        if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                            seed_refs.push(quote! { #ident.as_bytes() });
                            continue;
                        }
                    }
                }

                // Handle data.field -> self.field
                match &**expr {
                    syn::Expr::MethodCall(mc) if mc.method == "to_le_bytes" => {
                        // Check if it's data.field.to_le_bytes()
                        if let syn::Expr::Field(field_expr) = &*mc.receiver {
                            if let syn::Expr::Path(path) = &*field_expr.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "data" {
                                        // Rewrite data.field.to_le_bytes() to self.field.to_le_bytes()
                                        if let syn::Member::Named(field_name) = &field_expr.member {
                                            let binding_name = syn::Ident::new(
                                                &format!("seed_{}", i),
                                                proc_macro2::Span::call_site(),
                                            );
                                            bindings.push(quote! {
                                                let #binding_name = self.#field_name.to_le_bytes();
                                            });
                                            seed_refs.push(quote! { #binding_name.as_ref() });
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    syn::Expr::Field(field_expr) => {
                        if let syn::Expr::Path(path) = &*field_expr.base {
                            if let Some(segment) = path.path.segments.first() {
                                if segment.ident == "data" {
                                    // Rewrite data.field to self.field
                                    if let syn::Member::Named(field_name) = &field_expr.member {
                                        seed_refs.push(quote! { self.#field_name.as_ref() });
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                // Fallback: use expression as-is (for consts, etc.)
                seed_refs.push(quote! { (#expr).as_ref() });
            }
        }
    }

    let indices: Vec<usize> = (0..seed_refs.len()).collect();

    Ok(quote! {
        #(#bindings)*
        let seeds: &[&[u8]] = &[#(#seed_refs,)*];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
        #(
            seeds_vec.push(seeds[#indices].to_vec());
        )*
        seeds_vec.push(vec![bump]);
        (seeds_vec, pda)
    })
}

/// Extract required account names from seed expressions and track dependencies
///
/// Returns: (all_required_accounts, account_dependencies)
fn extract_required_accounts_from_seeds(
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
) -> Result<Vec<String>> {
    // Use a Vec to preserve insertion order and perform manual dedup.
    // The number of accounts is small, so O(n^2) dedup is fine and avoids
    // bringing in external crates for ordered sets.
    let mut required_accounts: Vec<String> = Vec::new();

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

    // TODO: check if we can remove.
    // Walk PDA seeds in declared order
    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            let _required_seeds = extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
        }
    }

    // Then token seeds in declared order
    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let _required_seeds = extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
        }
    }

    Ok(required_accounts)
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
            extract_account_from_expr(&method_call.receiver, ordered_accounts);
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
        syn::Expr::Call(call_expr) => {
            // Recursively extract accounts from all function arguments
            // This handles max_key(&base_mint.key(), &quote_mint.key())
            for arg in &call_expr.args {
                extract_account_from_expr(arg, ordered_accounts);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            // Unwrap references and continue extracting
            extract_account_from_expr(&ref_expr.expr, ordered_accounts);
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
            unreachable!()
        }
        InstructionVariant::TokenOnly => {
            unreachable!()
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
            unreachable!()
        }
        InstructionVariant::Mixed => {
            // Mixed: required token program accounts with address constraints for constants
            // Use hardcoded well-known Pubkeys for ctoken program and cpi authority
            account_fields.extend(vec![
                quote! {
                    /// Compressed token program (auto-resolved constant)
                    /// CHECK: Enforced to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
                    #[account(address = solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
                    pub ctoken_program: UncheckedAccount<'info>
                },
                quote! {
                    /// CPI authority PDA of the compressed token program (auto-resolved constant)
                    /// CHECK: Enforced to be GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy
                    #[account(address = solana_pubkey::pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
                    pub ctoken_cpi_authority: UncheckedAccount<'info>
                },
                quote! {
                    /// CHECK: CToken CompressibleConfig account (default but can be overridden)
                    pub ctoken_config: UncheckedAccount<'info>
                },
            ]);
        }
        InstructionVariant::PdaOnly => {
            unreachable!()
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

    syn::parse2(struct_def)
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

/// Generate error codes automatically based on instruction variant
/// This generates additional error variants that get added to the user's ErrorCode enum
#[inline(never)]
fn generate_error_codes(variant: InstructionVariant) -> Result<TokenStream> {
    let base_errors = quote! {
            #[msg("Rent sponsor does not match config")]
            InvalidRentSponsor,
        #[msg("Required seed account is missing for decompression - check that all seed accounts for compressed accounts are provided")]
        MissingSeedAccount,
        #[msg("ATA variants use SPL ATA derivation, not seed-based PDA derivation")]
        AtaDoesNotUseSeedDerivation,
    };

    let variant_specific_errors = match variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
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
