use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

/// Info about ctx.* seeds for a PDA type
#[derive(Clone, Debug)]
pub struct PdaCtxSeedInfo {
    pub type_name: Ident,
    /// Field names from ctx.accounts.XXX references in seeds
    pub ctx_seed_fields: Vec<Ident>,
}

impl PdaCtxSeedInfo {
    pub fn new(type_name: Ident, ctx_seed_fields: Vec<Ident>) -> Self {
        Self {
            type_name,
            ctx_seed_fields,
        }
    }
}

/// Enhanced function that generates variants with ctx.* seed fields
pub fn compressed_account_variant_with_ctx_seeds(
    account_types: &[&Ident],
    pda_ctx_seeds: &[PdaCtxSeedInfo],
) -> Result<TokenStream> {
    if account_types.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "At least one account type must be specified",
        ));
    }

    // Build a map from type name to ctx seed fields
    let ctx_seeds_map: std::collections::HashMap<String, &[Ident]> = pda_ctx_seeds
        .iter()
        .map(|info| (info.type_name.to_string(), info.ctx_seed_fields.as_slice()))
        .collect();

    // Phase 2: Generate struct variants with ctx.* seed fields
    let account_variants = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        let ctx_fields = ctx_seeds_map.get(&name.to_string()).copied().unwrap_or(&[]);

        // Unpacked variant: Pubkey fields for ctx.* seeds
        // Note: Use bare Pubkey which is in scope via `use anchor_lang::prelude::*`
        let unpacked_ctx_fields = ctx_fields.iter().map(|field| {
            quote! { #field: Pubkey }
        });

        // Packed variant: u8 index fields for ctx.* seeds
        let packed_ctx_fields = ctx_fields.iter().map(|field| {
            let idx_field = format_ident!("{}_idx", field);
            quote! { #idx_field: u8 }
        });

        quote! {
            #name { data: #name, #(#unpacked_ctx_fields,)* },
            #packed_name { data: #packed_name, #(#packed_ctx_fields,)* },
        }
    });

    // Phase 8: PackedCTokenData uses PackedTokenAccountVariant (with idx fields)
    //          CTokenData uses TokenAccountVariant (with Pubkey fields)
    let enum_def = quote! {
        #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
        pub enum RentFreeAccountVariant {
            #(#account_variants)*
            PackedCTokenData(light_token_sdk::compat::PackedCTokenData<PackedTokenAccountVariant>),
            CTokenData(light_token_sdk::compat::CTokenData<TokenAccountVariant>),
        }
    };

    let first_type = account_types[0];
    let first_ctx_fields = ctx_seeds_map
        .get(&first_type.to_string())
        .copied()
        .unwrap_or(&[]);
    let first_default_ctx_fields = first_ctx_fields.iter().map(|field| {
        quote! { #field: Pubkey::default() }
    });
    let default_impl = quote! {
        impl Default for RentFreeAccountVariant {
            fn default() -> Self {
                Self::#first_type { data: #first_type::default(), #(#first_default_ctx_fields,)* }
            }
        }
    };

    let hash_match_arms = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        quote! {
            RentFreeAccountVariant::#name { data, .. } => <#name as light_hasher::DataHasher>::hash::<H>(data),
            RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
        }
    });

    let data_hasher_impl = quote! {
        impl light_hasher::DataHasher for RentFreeAccountVariant {
            fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], light_hasher::HasherError> {
                match self {
                    #(#hash_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }
        }
    };

    let light_discriminator_impl = quote! {
        impl light_sdk::LightDiscriminator for RentFreeAccountVariant {
            const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        }
    };

    let compression_info_match_arms = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        quote! {
            RentFreeAccountVariant::#name { data, .. } => <#name as light_sdk::compressible::HasCompressionInfo>::compression_info(data),
            RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
        }
    });

    let compression_info_mut_match_arms = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        quote! {
            RentFreeAccountVariant::#name { data, .. } => <#name as light_sdk::compressible::HasCompressionInfo>::compression_info_mut(data),
            RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
        }
    });

    let compression_info_mut_opt_match_arms = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        quote! {
            RentFreeAccountVariant::#name { data, .. } => <#name as light_sdk::compressible::HasCompressionInfo>::compression_info_mut_opt(data),
            RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
        }
    });

    let set_compression_info_none_match_arms = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        quote! {
            RentFreeAccountVariant::#name { data, .. } => <#name as light_sdk::compressible::HasCompressionInfo>::set_compression_info_none(data),
            RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
        }
    });

    let has_compression_info_impl = quote! {
        impl light_sdk::compressible::HasCompressionInfo for RentFreeAccountVariant {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_mut_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                match self {
                    #(#compression_info_mut_opt_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }

            fn set_compression_info_none(&mut self) {
                match self {
                    #(#set_compression_info_none_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }
        }
    };

    let size_match_arms = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        quote! {
            RentFreeAccountVariant::#name { data, .. } => <#name as light_sdk::account::Size>::size(data),
            RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
        }
    });

    let size_impl = quote! {
        impl light_sdk::account::Size for RentFreeAccountVariant {
            fn size(&self) -> usize {
                match self {
                    #(#size_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                }
            }
        }
    };

    // Phase 2: Pack/Unpack with ctx seed fields
    let pack_match_arms: Vec<_> = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        let ctx_fields = ctx_seeds_map.get(&name.to_string()).copied().unwrap_or(&[]);

        if ctx_fields.is_empty() {
            // No ctx seeds - simple pack
            quote! {
                RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
                RentFreeAccountVariant::#name { data, .. } => RentFreeAccountVariant::#packed_name {
                    data: <#name as light_sdk::compressible::Pack>::pack(data, remaining_accounts),
                },
            }
        } else {
            // Has ctx seeds - pack data and ctx seed pubkeys
            let field_names: Vec<_> = ctx_fields.iter().collect();
            let idx_field_names: Vec<_> = ctx_fields.iter().map(|f| format_ident!("{}_idx", f)).collect();
            let pack_ctx_seeds: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                // Dereference because we're matching on &self, so field is &Pubkey
                quote! { let #idx_field = remaining_accounts.insert_or_get(*#field); }
            }).collect();

            quote! {
                RentFreeAccountVariant::#packed_name { .. } => unreachable!(),
                RentFreeAccountVariant::#name { data, #(#field_names,)* .. } => {
                    #(#pack_ctx_seeds)*
                    RentFreeAccountVariant::#packed_name {
                        data: <#name as light_sdk::compressible::Pack>::pack(data, remaining_accounts),
                        #(#idx_field_names,)*
                    }
                },
            }
        }
    }).collect();

    let pack_impl = quote! {
        impl light_sdk::compressible::Pack for RentFreeAccountVariant {
            type Packed = Self;

            fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                match self {
                    #(#pack_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(data) => {
                        // Use ctoken-sdk's Pack trait for CTokenData
                        Self::PackedCTokenData(light_token_sdk::pack::Pack::pack(data, remaining_accounts))
                    }
                }
            }
        }
    };

    let unpack_match_arms: Vec<_> = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        let ctx_fields = ctx_seeds_map.get(&name.to_string()).copied().unwrap_or(&[]);

        if ctx_fields.is_empty() {
            // No ctx seeds - simple unpack
            quote! {
                RentFreeAccountVariant::#packed_name { data, .. } => Ok(RentFreeAccountVariant::#name {
                    data: <#packed_name as light_sdk::compressible::Unpack>::unpack(data, remaining_accounts)?,
                }),
                RentFreeAccountVariant::#name { .. } => unreachable!(),
            }
        } else {
            // Has ctx seeds - unpack data and resolve ctx seed pubkeys from indices
            let idx_field_names: Vec<_> = ctx_fields.iter().map(|f| format_ident!("{}_idx", f)).collect();
            let field_names: Vec<_> = ctx_fields.iter().collect();
            let unpack_ctx_seeds: Vec<_> = ctx_fields.iter().map(|field| {
                let idx_field = format_ident!("{}_idx", field);
                quote! {
                    let #field = *remaining_accounts
                        .get(*#idx_field as usize)
                        .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                        .key;
                }
            }).collect();

            quote! {
                RentFreeAccountVariant::#packed_name { data, #(#idx_field_names,)* .. } => {
                    #(#unpack_ctx_seeds)*
                    Ok(RentFreeAccountVariant::#name {
                        data: <#packed_name as light_sdk::compressible::Unpack>::unpack(data, remaining_accounts)?,
                        #(#field_names,)*
                    })
                },
                RentFreeAccountVariant::#name { .. } => unreachable!(),
            }
        }
    }).collect();

    let unpack_impl = quote! {
        impl light_sdk::compressible::Unpack for RentFreeAccountVariant {
            type Unpacked = Self;

            fn unpack(
                &self,
                remaining_accounts: &[anchor_lang::prelude::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
                match self {
                    #(#unpack_match_arms)*
                    Self::PackedCTokenData(_data) => Ok(self.clone()),
                    Self::CTokenData(_data) => unreachable!(),
                }
            }
        }
    };

    let rentfree_account_data_struct = quote! {
        #[derive(Clone, Debug, anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)]
        pub struct RentFreeAccountData {
            pub meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            pub data: RentFreeAccountVariant,
        }
    };

    let expanded = quote! {
        #enum_def
        #default_impl
        #data_hasher_impl
        #light_discriminator_impl
        #has_compression_info_impl
        #size_impl
        #pack_impl
        #unpack_impl
        #rentfree_account_data_struct
    };

    Ok(expanded)
}
