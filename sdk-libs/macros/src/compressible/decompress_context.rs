//! DecompressContext trait generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Ident, Result, Token,
};

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

pub fn generate_decompress_context_trait_impl(
    pda_type_idents: Vec<Ident>,
    token_variant_ident: Ident,
    lifetime: syn::Lifetime,
) -> Result<TokenStream> {
    let pda_match_arms: Vec<_> = pda_type_idents
        .iter()
        .map(|pda_type| {
            let packed_name = format_ident!("Packed{}", pda_type);
            quote! {
                CompressedAccountVariant::#packed_name(packed) => {
                    match light_sdk::compressible::handle_packed_pda_variant::<#pda_type, #packed_name, _, _>(
                        &*self.rent_sponsor,
                        cpi_accounts,
                        address_space,
                        &solana_accounts[i],
                        i,
                        &packed,
                        &meta,
                        post_system_accounts,
                        &mut compressed_pda_infos,
                        &program_id,
                        self,  // Pass the context itself as seed_accounts
                        seed_params,
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
            type PackedTokenData = light_token_sdk::compat::PackedCTokenData<#token_variant_ident>;
            type CompressedMeta = light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
            type SeedParams = SeedParams;

            fn fee_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &*self.fee_payer
            }

            fn config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &self.config
            }

            fn rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                &self.rent_sponsor
            }

            fn token_rent_sponsor(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_rent_sponsor.as_ref()
            }

            fn token_program(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_program.as_ref().map(|a| &**a)
            }

            fn token_cpi_authority(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_cpi_authority.as_ref().map(|a| &**a)
            }

            fn token_config(&self) -> std::option::Option<&solana_account_info::AccountInfo<#lifetime>> {
                self.ctoken_config.as_ref().map(|a| &**a)
            }

            fn collect_pda_and_token<'b>(
                &self,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                address_space: solana_pubkey::Pubkey,
                compressed_accounts: Vec<Self::CompressedData>,
                solana_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                seed_params: std::option::Option<&Self::SeedParams>,
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

            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn process_tokens<'b>(
                &self,
                remaining_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                fee_payer: &solana_account_info::AccountInfo<#lifetime>,
                token_program: &solana_account_info::AccountInfo<#lifetime>,
                token_rent_sponsor: &solana_account_info::AccountInfo<#lifetime>,
                token_cpi_authority: &solana_account_info::AccountInfo<#lifetime>,
                token_config: &solana_account_info::AccountInfo<#lifetime>,
                config: &solana_account_info::AccountInfo<#lifetime>,
                token_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
                proof: light_sdk::instruction::ValidityProof,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, #lifetime>,
                post_system_accounts: &[solana_account_info::AccountInfo<#lifetime>],
                has_pdas: bool,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                light_token_sdk::compressible::process_decompress_tokens_runtime(
                    self,
                    remaining_accounts,
                    fee_payer,
                    token_program,
                    token_rent_sponsor,
                    token_cpi_authority,
                    token_config,
                    config,
                    token_accounts,
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

pub fn derive_decompress_context(input: DeriveInput) -> Result<TokenStream> {
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

    let lifetime = if let Some(lt) = input.generics.lifetimes().next() {
        lt.lifetime.clone()
    } else {
        return Err(syn::Error::new_spanned(
            &input,
            "DecompressContext requires a lifetime parameter (e.g., <'info>)",
        ));
    };

    generate_decompress_context_trait_impl(pda_type_idents, token_variant_ident, lifetime)
}
