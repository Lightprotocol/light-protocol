//! Compressible instructions generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Item, ItemMod, LitStr, Result, Token,
};

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

#[derive(Debug, Clone, Copy)]
pub enum InstructionVariant {
    PdaOnly,
    TokenOnly,
    Mixed,
}

#[derive(Clone)]
pub struct TokenSeedSpec {
    pub variant: Ident,
    pub _eq: Token![=],
    pub is_token: Option<bool>,
    pub is_ata: bool,
    pub seeds: Punctuated<SeedElement, Token![,]>,
    pub authority: Option<Vec<SeedElement>>,
}

impl Parse for TokenSeedSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let variant = input.parse()?;
        let _eq = input.parse()?;

        let content;
        syn::parenthesized!(content in input);

        let (is_token, is_ata, seeds, authority) = if content.peek(Ident) {
            let first_ident: Ident = content.parse()?;

            match first_ident.to_string().as_str() {
                "is_token" => {
                    let _comma: Token![,] = content.parse()?;

                    if content.peek(Ident) {
                        let fork = content.fork();
                        if let Ok(second_ident) = fork.parse::<Ident>() {
                            if second_ident == "is_ata" {
                                let _: Ident = content.parse()?;
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

                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(true), false, seeds, authority)
                }
                "true" => {
                    let _comma: Token![,] = content.parse()?;
                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(true), false, seeds, authority)
                }
                "is_pda" | "false" => {
                    let _comma: Token![,] = content.parse()?;
                    let (seeds, authority) = parse_seeds_with_authority(&content)?;
                    (Some(false), false, seeds, authority)
                }
                _ => {
                    let mut seeds = Punctuated::new();
                    // Allow function-call expressions starting with an identifier, e.g. max_key(...)
                    if content.peek(syn::token::Paren) {
                        let args_tokens;
                        syn::parenthesized!(args_tokens in content);
                        let inner_ts: proc_macro2::TokenStream = args_tokens.parse()?;
                        let call_expr: syn::Expr =
                            syn::parse2(quote! { #first_ident( #inner_ts ) })?;
                        seeds.push(SeedElement::Expression(Box::new(call_expr)));
                    } else {
                        seeds.push(SeedElement::Expression(Box::new(syn::Expr::Path(
                            syn::ExprPath {
                                attrs: vec![],
                                qself: None,
                                path: syn::Path::from(first_ident),
                            },
                        ))));
                    }

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

#[allow(clippy::type_complexity)]
fn parse_seeds_with_authority(
    content: ParseStream,
) -> Result<(Punctuated<SeedElement, Token![,]>, Option<Vec<SeedElement>>)> {
    let mut seeds = Punctuated::new();
    let mut authority = None;

    while !content.is_empty() {
        if content.peek(Ident) {
            let fork = content.fork();
            if let Ok(ident) = fork.parse::<Ident>() {
                if ident == "authority" && fork.peek(Token![=]) {
                    let _: Ident = content.parse()?;
                    let _: Token![=] = content.parse()?;

                    if content.peek(syn::token::Paren) {
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
                        authority = Some(vec![content.parse::<SeedElement>()?]);
                    }

                    if content.peek(Token![,]) {
                        let _: Token![,] = content.parse()?;
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }

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
pub enum SeedElement {
    Literal(LitStr),
    Expression(Box<Expr>),
}

impl Parse for SeedElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(SeedElement::Literal(input.parse()?))
        } else {
            Ok(SeedElement::Expression(input.parse()?))
        }
    }
}

pub struct InstructionDataSpec {
    pub field_name: Ident,
    pub field_type: syn::Type,
}

impl Parse for InstructionDataSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let field_name: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let field_type: syn::Type = input.parse()?;

        Ok(InstructionDataSpec {
            field_name,
            field_type,
        })
    }
}

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
                    let field_type: syn::Type = input.parse()?;
                    instruction_data.push(InstructionDataSpec {
                        field_name: ident,
                        field_type,
                    });
                }
            } else {
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

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub fn add_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
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

    let size_validation_checks = validate_compressed_account_sizes(&account_types)?;

    let content = module.content.as_mut().unwrap();

    let ctoken_enum = if let Some(ref token_seed_specs) = token_seeds {
        if !token_seed_specs.is_empty() {
            crate::compressible::seed_providers::generate_ctoken_account_variant_enum(
                token_seed_specs,
            )?
        } else {
            crate::compressible::utils::generate_empty_ctoken_enum()
        }
    } else {
        crate::compressible::utils::generate_empty_ctoken_enum()
    };

    if let Some(ref token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            if spec.is_ata {
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
                return Err(macro_error!(
                    &spec.variant,
                    "Program-owned token account '{}' must specify authority = <seed_expr> for compression signing. For user-owned ATAs, use is_ata flag instead.",
                    spec.variant
                ));
            }
        }
    }

    let mut account_types_stream = TokenStream::new();
    for (i, account_type) in account_types.iter().enumerate() {
        if i > 0 {
            account_types_stream.extend(quote! { , });
        }
        account_types_stream.extend(quote! { #account_type });
    }
    let enum_and_traits =
        crate::compressible::variant_enum::compressed_account_variant(account_types_stream)?;

    // Generate SeedParams struct for instruction data fields
    let seed_params_struct = {
        let param_fields: Vec<_> = instruction_data
            .iter()
            .map(|spec| {
                let field_name = &spec.field_name;
                let field_type = &spec.field_type;
                quote! {
                    pub #field_name: #field_type
                }
            })
            .collect();

        quote! {
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug, Default)]
            pub struct SeedParams {
                #(#param_fields,)*
            }
        }
    };

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

    let error_codes = generate_error_codes(instruction_variant)?;

    let required_accounts = extract_required_accounts_from_seeds(&pda_seeds, &token_seeds)?;

    let decompress_accounts =
        generate_decompress_accounts_struct(&required_accounts, instruction_variant)?;

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
                impl<'info> light_sdk::compressible::PdaSeedDerivation<DecompressAccountsIdempotent<'info>, SeedParams> for #name {
                    fn derive_pda_seeds_with_accounts(
                        &self,
                        program_id: &solana_pubkey::Pubkey,
                        accounts: &DecompressAccountsIdempotent<'info>,
                        seed_params: &SeedParams,
                    ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                        #seed_derivation
                    }
                }
            })
        })
        .collect();
    let pda_seed_provider_impls = pda_seed_provider_impls?;

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
                seed_accounts: &DecompressAccountsIdempotent<'info>,
                seed_params: &SeedParams,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                light_sdk::compressible::handle_packed_pda_variant::<#name, #packed_name, DecompressAccountsIdempotent<'info>, SeedParams>(
                    accounts.rent_sponsor.as_ref(),
                    cpi_accounts,
                    address_space,
                    &solana_accounts[i],
                    i,
                    packed,
                    meta,
                    post_system_accounts,
                    compressed_pda_infos,
                    &crate::ID,
                    seed_accounts,
                    std::option::Option::Some(seed_params),
                )
            }
        }
    }).collect();

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
                                match #func_name(accounts, &cpi_accounts, address_space, solana_accounts, i, &packed, &meta, post_system_accounts, &mut compressed_pda_infos, accounts, seed_params) {
                                    std::result::Result::Ok(()) => {},
                                    std::result::Result::Err(e) => return std::result::Result::Err(e),
                                }
                            }
        }
    }).collect();

    let trait_impls: syn::ItemMod = syn::parse_quote! {
        mod __trait_impls {
            use super::*;

            impl light_sdk::compressible::HasTokenVariant for CompressedAccountData {
                fn is_packed_ctoken(&self) -> bool {
                    matches!(self.data, CompressedAccountVariant::PackedCTokenData(_))
                }
            }

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

            impl light_ctoken_sdk::CTokenSeedProvider for CTokenAccountVariant {
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

    let ctoken_trait_system: syn::ItemMod = syn::parse_quote! {
        pub mod ctoken_seed_system {
            use super::*;

            pub struct CTokenSeedContext<'a, 'info> {
                pub accounts: &'a DecompressAccountsIdempotent<'info>,
                pub remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
            }

            pub trait CTokenSeedProvider {
                fn get_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;

                fn get_authority_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;
            }
        }
    };

    let helpers_module: syn::ItemMod = {
        let helper_packed_fns = helper_packed_fns.clone();
        let call_unpacked_arms = call_unpacked_arms.clone();
        let call_packed_arms = call_packed_arms.clone();
        syn::parse_quote! {
        mod __macro_helpers {
            use super::*;
            use crate::state::*;  // Import Packed* types from state module
            #(#helper_packed_fns)*
                #[inline(never)]
                pub fn collect_pda_and_token<'a, 'b, 'info>(
                    accounts: &DecompressAccountsIdempotent<'info>,
                    cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
                    address_space: solana_pubkey::Pubkey,
                    compressed_accounts: Vec<CompressedAccountData>,
                    solana_accounts: &[solana_account_info::AccountInfo<'info>],
                    seed_params: &SeedParams,
                ) -> std::result::Result<(
                    Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                    Vec<(
                        light_ctoken_sdk::compat::PackedCTokenData<CTokenAccountVariant>,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    )>,
                ), solana_program_error::ProgramError> {
                    let post_system_offset = cpi_accounts.system_accounts_end_offset();
                    let all_infos = cpi_accounts.account_infos();
                    let post_system_accounts = &all_infos[post_system_offset..];
                    let estimated_capacity = compressed_accounts.len();
                    let mut compressed_pda_infos = Vec::with_capacity(estimated_capacity);
                    let mut compressed_token_accounts: Vec<(
                        light_ctoken_sdk::compat::PackedCTokenData<CTokenAccountVariant>,
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

    let token_variant_name = format_ident!("CTokenAccountVariant");

    let decompress_context_impl = generate_decompress_context_impl(
        instruction_variant,
        account_types.clone(),
        token_variant_name,
    )?;
    let decompress_processor_fn =
        generate_process_decompress_accounts_idempotent(instruction_variant, &instruction_data)?;
    let decompress_instruction =
        generate_decompress_instruction_entrypoint(instruction_variant, &instruction_data)?;

    let compress_accounts: syn::ItemStruct = match instruction_variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
        InstructionVariant::Mixed => syn::parse_quote! {
        #[derive(Accounts)]
        pub struct CompressAccountsIdempotent<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            /// CHECK: Checked by SDK
            pub config: AccountInfo<'info>,
            /// CHECK: Checked by SDK
            #[account(mut)]
            pub rent_sponsor: AccountInfo<'info>,
            /// CHECK: Checked by SDK
            #[account(mut)]
            pub compression_authority: AccountInfo<'info>,
        }
        },
    };

    let compress_context_impl =
        generate_compress_context_impl(instruction_variant, account_types.clone())?;
    let compress_processor_fn = generate_process_compress_accounts_idempotent(instruction_variant)?;
    let compress_instruction = generate_compress_instruction_entrypoint(instruction_variant)?;

    let processor_module: syn::ItemMod = syn::parse_quote! {
        mod __processor_functions {
            use super::*;
            #decompress_processor_fn
            #compress_processor_fn
        }
    };

    let init_config_accounts: syn::ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct InitializeCompressionConfig<'info> {
            #[account(mut)]
            pub payer: Signer<'info>,
            /// CHECK: Checked by SDK
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// CHECK: Checked by SDK
            pub program_data: AccountInfo<'info>,
            pub authority: Signer<'info>,
            pub system_program: Program<'info, System>,
        }
    };

    let update_config_accounts: syn::ItemStruct = syn::parse_quote! {
        #[derive(Accounts)]
        pub struct UpdateCompressionConfig<'info> {
            /// CHECK: Checked by SDK
            #[account(mut)]
            pub config: AccountInfo<'info>,
            /// CHECK: Checked by SDK
            pub authority: Signer<'info>,
        }
    };

    let init_config_instruction: syn::ItemFn = syn::parse_quote! {
        #[inline(never)]
        #[allow(clippy::too_many_arguments)]
    pub fn initialize_compression_config<'info>(
            ctx: Context<'_, '_, '_, 'info, InitializeCompressionConfig<'info>>,
        write_top_up: u32,
            rent_sponsor: Pubkey,
            compression_authority: Pubkey,
            rent_config: light_compressible::rent::RentConfig,
            address_space: Vec<Pubkey>,
        ) -> Result<()> {
            light_sdk::compressible::process_initialize_compression_config_checked(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.program_data.to_account_info(),
                &rent_sponsor,
                &compression_authority,
                rent_config,
                write_top_up,
                address_space,
                0,
                &ctx.accounts.payer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &crate::ID,
            )?;
            Ok(())
        }
    };

    let update_config_instruction: syn::ItemFn = syn::parse_quote! {
        #[inline(never)]
        #[allow(clippy::too_many_arguments)]
        pub fn update_compression_config<'info>(
            ctx: Context<'_, '_, '_, 'info, UpdateCompressionConfig<'info>>,
            new_rent_sponsor: Option<Pubkey>,
            new_compression_authority: Option<Pubkey>,
            new_rent_config: Option<light_compressible::rent::RentConfig>,
            new_write_top_up: Option<u32>,
            new_address_space: Option<Vec<Pubkey>>,
            new_update_authority: Option<Pubkey>,
        ) -> Result<()> {
            light_sdk::compressible::process_update_compression_config(
                ctx.accounts.config.as_ref(),
                ctx.accounts.authority.as_ref(),
                new_update_authority.as_ref(),
                new_rent_sponsor.as_ref(),
                new_compression_authority.as_ref(),
                new_rent_config,
                new_write_top_up,
                new_address_space,
                &crate::ID,
            )?;
            Ok(())
        }
    };

    // Insert SeedParams struct
    let seed_params_item: Item = syn::parse2(seed_params_struct)?;
    content.1.push(seed_params_item);

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

    if let Some(ref seeds) = token_seeds {
        if !seeds.is_empty() {
            let impl_code =
                crate::compressible::seed_providers::generate_ctoken_seed_provider_implementation(
                    seeds,
                )?;
            let ctoken_impl: syn::ItemImpl = syn::parse2(impl_code).map_err(|e| {
                syn::Error::new_spanned(
                    &seeds[0].variant,
                    format!("Failed to parse ctoken implementation: {}", e),
                )
            })?;
            content.1.push(Item::Impl(ctoken_impl));
        }
    }

    let client_seed_functions =
        crate::compressible::seed_providers::generate_client_seed_functions(
            &account_types,
            &pda_seeds,
            &token_seeds,
            &instruction_data,
        )?;

    // Add allow attribute to module itself to suppress clippy warnings
    module.attrs.push(syn::parse_quote! {
        #[allow(clippy::too_many_arguments)]
    });

    Ok(quote! {
        #size_validation_checks
        #error_codes
        #ctoken_enum
        #enum_and_traits
        #(#pda_seed_provider_impls)*
        #[allow(non_snake_case)]
        #module
        #client_seed_functions
    })
}

pub fn generate_decompress_context_impl(
    _variant: InstructionVariant,
    pda_type_idents: Vec<Ident>,
    token_variant_ident: Ident,
) -> Result<syn::ItemMod> {
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    let trait_impl =
        crate::compressible::decompress_context::generate_decompress_context_trait_impl(
            pda_type_idents,
            token_variant_ident,
            lifetime,
        )?;

    Ok(syn::parse_quote! {
        mod __decompress_context_impl {
            use super::*;

            #trait_impl
        }
    })
}

pub fn generate_process_decompress_accounts_idempotent(
    _variant: InstructionVariant,
    instruction_data: &[InstructionDataSpec],
) -> Result<syn::ItemFn> {
    // If we have seed parameters, accept them as a single struct
    let (params, seed_params_arg) = if !instruction_data.is_empty() {
        (
            quote! { seed_data: SeedParams, },
            quote! { std::option::Option::Some(&seed_data) },
        )
    } else {
        (quote! {}, quote! { std::option::Option::None })
    };

    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn process_decompress_accounts_idempotent<'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
            #params
        ) -> Result<()> {
            light_sdk::compressible::process_decompress_accounts_idempotent(
                accounts,
                remaining_accounts,
                compressed_accounts,
                proof,
                system_accounts_offset,
                LIGHT_CPI_SIGNER,
                &crate::ID,
                #seed_params_arg,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}

pub fn generate_decompress_instruction_entrypoint(
    _variant: InstructionVariant,
    instruction_data: &[InstructionDataSpec],
) -> Result<syn::ItemFn> {
    // If we have seed parameters, pass them as a single struct
    let (params, args) = if !instruction_data.is_empty() {
        (quote! { seed_data: SeedParams, }, quote! { seed_data, })
    } else {
        (quote! {}, quote! {})
    };

    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
            #params
        ) -> Result<()> {
            __processor_functions::process_decompress_accounts_idempotent(
                &ctx.accounts,
                &ctx.remaining_accounts,
                proof,
                compressed_accounts,
                system_accounts_offset,
                #args
            )
        }
    })
}

pub fn generate_compress_context_impl(
    _variant: InstructionVariant,
    account_types: Vec<Ident>,
) -> Result<syn::ItemMod> {
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    let compress_arms: Vec<_> = account_types.iter().map(|name| {
        quote! {
            d if d == #name::LIGHT_DISCRIMINATOR => {
                drop(data);
                let data_borrow = account_info.try_borrow_data().map_err(|e| {
                    let err: anchor_lang::error::Error = e.into();
                    let program_error: anchor_lang::prelude::ProgramError = err.into();
                    let code = match program_error {
                        anchor_lang::prelude::ProgramError::Custom(code) => code,
                        _ => 0,
                    };
                    solana_program_error::ProgramError::Custom(code)
                })?;
                let mut account_data = #name::try_deserialize(&mut &data_borrow[..]).map_err(|e| {
                    let err: anchor_lang::error::Error = e.into();
                    let program_error: anchor_lang::prelude::ProgramError = err.into();
                    let code = match program_error {
                        anchor_lang::prelude::ProgramError::Custom(code) => code,
                        _ => 0,
                    };
                    solana_program_error::ProgramError::Custom(code)
                })?;
                drop(data_borrow);

                let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<#name>(
                    program_id,
                    account_info,
                    &mut account_data,
                    meta,
                    cpi_accounts,
                    &compression_config.address_space,
                )?;
                // Compute rent-based close distribution and transfer lamports:
                // - Completed epochs to rent sponsor
                // - Partial epoch (unused) to fee payer (user refund)
                #[cfg(target_os = "solana")]
                let current_slot = anchor_lang::solana_program::sysvar::clock::Clock::get()
                    .map_err(|_| anchor_lang::prelude::ProgramError::UnsupportedSysvar)?
                    .slot;
                #[cfg(not(target_os = "solana"))]
                let current_slot = 0;
                let bytes = account_info.data_len() as u64;
                let current_lamports = account_info.lamports();
                let rent_exemption = anchor_lang::solana_program::sysvar::rent::Rent::get()
                    .map_err(|_| anchor_lang::prelude::ProgramError::UnsupportedSysvar)?
                    .minimum_balance(bytes as usize);
                let ci_ref = account_data.compression_info();
                let state = light_compressible::rent::AccountRentState {
                    num_bytes: bytes,
                    current_slot,
                    current_lamports,
                    last_claimed_slot: ci_ref.last_claimed_slot(),
                };
                let dist = state.calculate_close_distribution(&ci_ref.rent_config, rent_exemption);
                // Transfer partial epoch back to fee payer (user)
                if dist.to_user > 0 {
                    let fee_payer_info = self.fee_payer.to_account_info();
                    let mut src = account_info.try_borrow_mut_lamports().map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err.into();
                        program_error
                    })?;
                    let mut dst = fee_payer_info.try_borrow_mut_lamports().map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err.into();
                        program_error
                    })?;
                    **src = src.checked_sub(dist.to_user).ok_or(anchor_lang::prelude::ProgramError::InsufficientFunds)?;
                    **dst = dst.checked_add(dist.to_user).ok_or(anchor_lang::prelude::ProgramError::Custom(0))?;
                }
                // Transfer completed epochs (and base) to rent sponsor
                if dist.to_rent_sponsor > 0 {
                    let rent_sponsor_info = self.rent_sponsor.to_account_info();
                    let mut src = account_info.try_borrow_mut_lamports().map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err.into();
                        program_error
                    })?;
                    let mut dst = rent_sponsor_info.try_borrow_mut_lamports().map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err.into();
                        program_error
                    })?;
                    **src = src.checked_sub(dist.to_rent_sponsor).ok_or(anchor_lang::prelude::ProgramError::InsufficientFunds)?;
                    **dst = dst.checked_add(dist.to_rent_sponsor).ok_or(anchor_lang::prelude::ProgramError::Custom(0))?;
                }
                Ok(Some(compressed_info))
            }
        }
    }).collect();

    Ok(syn::parse_quote! {
        mod __compress_context_impl {
            use super::*;
            use light_sdk::LightDiscriminator;
            use light_sdk::compressible::HasCompressionInfo;

            impl<#lifetime> light_sdk::compressible::CompressContext<#lifetime> for CompressAccountsIdempotent<#lifetime> {
                fn fee_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &*self.fee_payer
                }

                fn config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.config
                }

                fn rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.rent_sponsor
                }

                fn compression_authority(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.compression_authority
                }

                fn compress_pda_account(
                    &self,
                    account_info: &solana_account_info::AccountInfo<#lifetime>,
                    meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'_, #lifetime>,
                    compression_config: &light_sdk::compressible::CompressibleConfig,
                    program_id: &solana_pubkey::Pubkey,
                ) -> std::result::Result<Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>, solana_program_error::ProgramError> {
                    let data = account_info.try_borrow_data().map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err.into();
                        let code = match program_error {
                            anchor_lang::prelude::ProgramError::Custom(code) => code,
                            _ => 0,
                        };
                        solana_program_error::ProgramError::Custom(code)
                    })?;
                    let discriminator = &data[0..8];

                    match discriminator {
                        #(#compress_arms)*
                        _ => {
                            let err: anchor_lang::error::Error = anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into();
                            let program_error: anchor_lang::prelude::ProgramError = err.into();
                            let code = match program_error {
                                anchor_lang::prelude::ProgramError::Custom(code) => code,
                                _ => 0,
                            };
                            Err(solana_program_error::ProgramError::Custom(code))
                        }
                    }
                }
            }
        }
    })
}

pub fn generate_process_compress_accounts_idempotent(
    _variant: InstructionVariant,
) -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn process_compress_accounts_idempotent<'info>(
            accounts: &CompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_sdk::compressible::compress_runtime::process_compress_pda_accounts_idempotent(
                accounts,
                remaining_accounts,
                compressed_accounts,
                system_accounts_offset,
                LIGHT_CPI_SIGNER,
                &crate::ID,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}

pub fn generate_compress_instruction_entrypoint(
    _variant: InstructionVariant,
) -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        #[allow(clippy::too_many_arguments)]
        pub fn compress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, CompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            __processor_functions::process_compress_accounts_idempotent(
                &ctx.accounts,
                &ctx.remaining_accounts,
                compressed_accounts,
                system_accounts_offset,
            )
        }
    })
}

#[inline(never)]
fn generate_pda_seed_derivation_for_trait(
    spec: &TokenSeedSpec,
    _instruction_data: &[InstructionDataSpec],
) -> Result<TokenStream> {
    let mut bindings: Vec<TokenStream> = Vec::new();
    let mut seed_refs = Vec::new();

    // Recursively rewrite expressions:
    // - `data.<field>` -> `seed_params.<field>` (from instruction params, not struct fields!)
    // - `ctx.accounts.<account>` -> `accounts.<account>`
    // - `ctx.<field>` -> `accounts.<field>`
    // While preserving function/method calls and references.
    fn map_pda_expr_to_params(expr: &syn::Expr) -> syn::Expr {
        match expr {
            syn::Expr::Field(field_expr) => {
                if let syn::Member::Named(field_name) = &field_expr.member {
                    // Handle nested field access: ctx.accounts.field_name -> accounts.field_name.as_ref().ok_or(...)?.key()
                    if let syn::Expr::Field(nested_field) = &*field_expr.base {
                        if let syn::Member::Named(base_name) = &nested_field.member {
                            if base_name == "accounts" {
                                if let syn::Expr::Path(path) = &*nested_field.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "ctx" {
                                            return syn::parse_quote! { accounts.#field_name.as_ref().ok_or_else(|| -> solana_program_error::ProgramError {
                                                let err: anchor_lang::error::Error = CompressibleInstructionError::MissingSeedAccount.into();
                                                err.into()
                                            })?.key() };
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Handle direct field access
                    if let syn::Expr::Path(path) = &*field_expr.base {
                        if let Some(segment) = path.path.segments.first() {
                            if segment.ident == "data" {
                                // data.field -> seed_params.field (from instruction params!)
                                return syn::parse_quote! { seed_params.#field_name };
                            } else if segment.ident == "ctx" {
                                // ctx.field -> accounts.field.as_ref().ok_or(...)?.key() (error if optional account is missing)
                                return syn::parse_quote! { accounts.#field_name.as_ref().ok_or_else(|| -> solana_program_error::ProgramError {
                                    let err: anchor_lang::error::Error = CompressibleInstructionError::MissingSeedAccount.into();
                                    err.into()
                                })?.key() };
                            }
                        }
                    }
                }
                expr.clone()
            }
            syn::Expr::MethodCall(method_call) => {
                // Special case: ctx.accounts.account_name.key() -> accounts.account_name.key()
                // This is already handled by the Field case transforming ctx.accounts.X -> accounts.X
                let mut new_method_call = method_call.clone();
                new_method_call.receiver = Box::new(map_pda_expr_to_params(&method_call.receiver));
                new_method_call.args = method_call
                    .args
                    .iter()
                    .map(map_pda_expr_to_params)
                    .collect();
                syn::Expr::MethodCall(new_method_call)
            }
            syn::Expr::Call(call_expr) => {
                // Map function args recursively. We do not transform the function path.
                let mut new_call_expr = call_expr.clone();
                new_call_expr.args = call_expr.args.iter().map(map_pda_expr_to_params).collect();
                syn::Expr::Call(new_call_expr)
            }
            syn::Expr::Reference(ref_expr) => {
                let mut new_ref_expr = ref_expr.clone();
                new_ref_expr.expr = Box::new(map_pda_expr_to_params(&ref_expr.expr));
                syn::Expr::Reference(new_ref_expr)
            }
            _ => {
                // For other expressions (constants, literals, paths), leave as-is
                expr.clone()
            }
        }
    }

    for (i, seed) in spec.seeds.iter().enumerate() {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                seed_refs.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                if let syn::Expr::Path(path_expr) = &**expr {
                    if let Some(ident) = path_expr.path.get_ident() {
                        let ident_str = ident.to_string();
                        if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                            seed_refs.push(quote! { #ident.as_bytes() });
                            continue;
                        }
                    }
                }

                // Generic solution: rewrite any `data.*` occurrences recursively to `self.*`,
                // then bind the result to a local to ensure lifetimes are valid,
                // and use `.as_ref()` to convert into a seed `&[u8]`.
                let binding_name =
                    syn::Ident::new(&format!("seed_{}", i), proc_macro2::Span::call_site());
                let mapped_expr = map_pda_expr_to_params(expr);
                bindings.push(quote! {
                    let #binding_name = #mapped_expr;
                });
                seed_refs.push(quote! { (#binding_name).as_ref() });
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
        Ok((seeds_vec, pda))
    })
}

#[inline(never)]
fn extract_required_accounts_from_seeds(
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
) -> Result<Vec<String>> {
    let mut required_accounts: Vec<String> = Vec::new();

    #[inline(always)]
    fn push_unique(list: &mut Vec<String>, value: String) {
        if !list.iter().any(|v| v == &value) {
            list.push(value);
        }
    }

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

    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            let _required_seeds = extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
        }
    }

    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let _required_seeds = extract_accounts_from_seed_spec(spec, &mut required_accounts)?;
        }
    }

    Ok(required_accounts)
}

#[inline(never)]
fn extract_account_from_expr(expr: &syn::Expr, ordered_accounts: &mut Vec<String>) {
    #[inline(always)]
    fn push_unique(list: &mut Vec<String>, value: String) {
        if !list.iter().any(|v| v == &value) {
            list.push(value);
        }
    }

    match expr {
        syn::Expr::MethodCall(method_call) => {
            extract_account_from_expr(&method_call.receiver, ordered_accounts);
        }
        syn::Expr::Field(field_expr) => {
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
                            push_unique(ordered_accounts, field_name.to_string());
                        }
                    }
                }
            }
        }
        syn::Expr::Path(path_expr) => {
            if let Some(ident) = path_expr.path.get_ident() {
                let name = ident.to_string();
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
            for arg in &call_expr.args {
                extract_account_from_expr(arg, ordered_accounts);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            extract_account_from_expr(&ref_expr.expr, ordered_accounts);
        }
        _ => {}
    }
}

#[inline(never)]
fn generate_decompress_accounts_struct(
    required_accounts: &[String],
    variant: InstructionVariant,
) -> Result<syn::ItemStruct> {
    let mut account_fields = vec![
        quote! {
            #[account(mut)]
            pub fee_payer: Signer<'info>
        },
        quote! {
            /// CHECK: Checked by SDK
            pub config: AccountInfo<'info>
        },
    ];

    match variant {
        InstructionVariant::PdaOnly => {
            unreachable!()
        }
        InstructionVariant::TokenOnly => {
            unreachable!()
        }
        InstructionVariant::Mixed => {
            account_fields.extend(vec![
                quote! {
                    /// CHECK: anyone can pay
                    #[account(mut)]
                    pub rent_sponsor: UncheckedAccount<'info>
                },
                quote! {
                    /// CHECK: optional - only needed if decompressing tokens
                    #[account(mut)]
                    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>
                },
            ]);
        }
    }

    match variant {
        InstructionVariant::TokenOnly => {
            unreachable!()
        }
        InstructionVariant::Mixed => {
            account_fields.extend(vec![
                quote! {
                    /// CHECK:
                    #[account(address = solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
                    pub ctoken_program: Option<UncheckedAccount<'info>>
                },
                quote! {
                    /// CHECK:
                    #[account(address = solana_pubkey::pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
                    pub ctoken_cpi_authority: Option<UncheckedAccount<'info>>
                },
                quote! {
                    /// CHECK: Checked by SDK
                    pub ctoken_config: Option<UncheckedAccount<'info>>
                },
            ]);
        }
        InstructionVariant::PdaOnly => {
            unreachable!()
        }
    }

    let standard_fields = [
        "fee_payer",
        "rent_sponsor",
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
                /// CHECK: optional seed account
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

#[inline(never)]
fn validate_compressed_account_sizes(account_types: &[Ident]) -> Result<TokenStream> {
    let size_checks: Vec<_> = account_types.iter().map(|account_type| {
        quote! {
            const _: () = {
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

#[inline(never)]
fn generate_error_codes(variant: InstructionVariant) -> Result<TokenStream> {
    let base_errors = quote! {
            #[msg("Rent sponsor mismatch")]
            InvalidRentSponsor,
        #[msg("Missing seed account")]
        MissingSeedAccount,
        #[msg("ATA uses SPL ATA derivation")]
        AtaDoesNotUseSeedDerivation,
    };

    let variant_specific_errors = match variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
        InstructionVariant::Mixed => quote! {
            #[msg("Not implemented")]
            CTokenDecompressionNotImplemented,
            #[msg("Not implemented")]
            PdaDecompressionNotImplemented,
            #[msg("Not implemented")]
            TokenCompressionNotImplemented,
            #[msg("Not implemented")]
            PdaCompressionNotImplemented,
        },
    };

    Ok(quote! {
        #[error_code]
        pub enum CompressibleInstructionError {
            #base_errors
            #variant_specific_errors
        }
    })
}
