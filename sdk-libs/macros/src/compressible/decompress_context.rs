//! Derive macro for DecompressContext trait.
//!
//! This generates the trait implementation automatically from struct fields and attributes.
//! Can be used standalone or via add_compressible_instructions.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Ident, Result, Token,
};

/// Parse the pda_types attribute: #[pda_types(MyAccount)]
struct PdaTypesAttr {
    types: Punctuated<Ident, Token![,]>,
}

impl Parse for PdaTypesAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(PdaTypesAttr {
            types: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Parse the token_variant attribute: #[token_variant(CTokenAccountVariant)]
struct TokenVariantAttr {
    variant: Ident,
}

impl Parse for TokenVariantAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(TokenVariantAttr {
            variant: input.parse()?,
        })
    }
}

/// Internal function to generate DecompressContext trait implementation.
///
/// This is used by both:
/// - #[derive(DecompressContext)] macro (from attributes)
/// - #[add_compressible_instructions] macro (from parsed args)
///
/// Generates:
/// - Account accessor methods
/// - collect_pda_and_token method with variant matching
/// - process_tokens method that delegates to runtime
pub fn generate_decompress_context_trait_impl(
    pda_type_idents: Vec<Ident>,
    token_variant_ident: Ident,
    lifetime: syn::Lifetime,
) -> Result<TokenStream> {
    // Generate match arms for PDA collection
    let pda_match_arms: Vec<_> = pda_type_idents
        .iter()
        .map(|pda_type| {
            let packed_name = format_ident!("Packed{}", pda_type);
            quote! {
                CompressedAccountVariant::#packed_name(packed) => {
                    match light_sdk::compressible::handle_packed_pda_variant::<#pda_type, #packed_name>(
                        &*self.rent_payer,
                        cpi_accounts,
                        address_space,
                        &solana_accounts[i],
                        i,
                        &packed,
                        &meta,
                        post_system_accounts,
                        &mut compressed_pda_infos,
                        &program_id,
                    ) {
                        std::result::Result::Ok(()) => {},
                        std::result::Result::Err(e) => return std::result::Result::Err(e),
                    }
                }
                CompressedAccountVariant::#pda_type(_) => {
                    unreachable!("Unpacked variants should not be present during decompression");
                }
            }
        })
        .collect();

    Ok(quote! {
        impl<#lifetime> light_sdk::compressible::DecompressContext<#lifetime> for DecompressAccountsIdempotent<#lifetime> {
            type CompressedData = CompressedAccountData;
            type PackedTokenData = light_compressed_token_sdk::compat::PackedCTokenData<#token_variant_ident>;
            type CompressedMeta = light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

            // Account accessors
            fn fee_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &*self.fee_payer
            }

            fn config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &self.config
            }

            fn rent_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &*self.rent_payer
            }

            fn ctoken_rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &self.ctoken_rent_sponsor
            }

            fn ctoken_program(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &*self.ctoken_program
            }

            fn ctoken_cpi_authority(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &*self.ctoken_cpi_authority
            }

            fn ctoken_config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &*self.ctoken_config
            }

            // Program-specific collection logic
            fn collect_pda_and_token<'b>(
                &self,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                address_space: solana_pubkey::Pubkey,
                compressed_accounts: Vec<Self::CompressedData>,
                solana_accounts: &[solana_account_info::AccountInfo<#lifetime>],
            ) -> std::result::Result<(
                Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
            ), solana_program_error::ProgramError> {
                let post_system_offset = cpi_accounts.system_accounts_end_offset();
                let all_infos = cpi_accounts.account_infos();
                let post_system_accounts = &all_infos[post_system_offset..];
                let program_id = &crate::ID;

                let mut compressed_pda_infos = Vec::with_capacity(compressed_accounts.len());
                let mut compressed_token_accounts = Vec::with_capacity(compressed_accounts.len());

                for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                    let meta = compressed_data.meta;
                    match compressed_data.data {
                        #(#pda_match_arms)*
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

            // Token processing - delegates to runtime
            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn process_tokens<'b>(
                &self,
                remaining_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                fee_payer: &solana_account_info::AccountInfo<#lifetime>,
                ctoken_program: &solana_account_info::AccountInfo<#lifetime>,
                ctoken_rent_sponsor: &solana_account_info::AccountInfo<#lifetime>,
                ctoken_cpi_authority: &solana_account_info::AccountInfo<#lifetime>,
                ctoken_config: &solana_account_info::AccountInfo<#lifetime>,
                config: &solana_account_info::AccountInfo<#lifetime>,
                ctoken_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
                proof: light_sdk::instruction::ValidityProof,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                post_system_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                has_pdas: bool,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                light_compressed_token_sdk::decompress_runtime::process_decompress_tokens_runtime(
                    self,
                    remaining_accounts,
                    fee_payer,
                    ctoken_program,
                    ctoken_rent_sponsor,
                    ctoken_cpi_authority,
                    ctoken_config,
                    config,
                    ctoken_accounts,
                    proof,
                    cpi_accounts,
                    post_system_accounts,
                    has_pdas,
                    &crate::ID,
                )
            }
        }
    })
}

/// Derive DecompressContext trait implementation from attributes.
///
/// This is the public derive macro entry point.
pub fn derive_decompress_context(input: DeriveInput) -> Result<TokenStream> {
    // Extract pda_types attribute
    let pda_types_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("pda_types"))
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &input,
                "DecompressContext derive requires #[pda_types(Type1, Type2, ...)] attribute",
            )
        })?;

    let pda_types: PdaTypesAttr = pda_types_attr.parse_args()?;
    let pda_type_idents: Vec<Ident> = pda_types.types.iter().cloned().collect();

    // Extract token_variant attribute
    let token_variant_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("token_variant"))
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &input,
                "DecompressContext derive requires #[token_variant(CTokenAccountVariant)] attribute",
            )
        })?;

    let token_variant: TokenVariantAttr = token_variant_attr.parse_args()?;
    let token_variant_ident = token_variant.variant;

    // Extract lifetime from struct
    let lifetime = if let Some(lt) = input.generics.lifetimes().next() {
        lt.lifetime.clone()
    } else {
        return Err(syn::Error::new_spanned(
            &input,
            "DecompressContext requires a lifetime parameter (e.g., <'info>)",
        ));
    };

    // Call shared implementation
    generate_decompress_context_trait_impl(pda_type_idents, token_variant_ident, lifetime)
}
