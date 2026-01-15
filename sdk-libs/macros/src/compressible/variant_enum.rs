use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Result, Token,
};

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

pub fn compressed_account_variant(input: TokenStream) -> Result<TokenStream> {
    let type_list = syn::parse2::<AccountTypeList>(input)?;
    let account_types: Vec<&Ident> = type_list.types.iter().collect();

    if account_types.is_empty() {
        return Err(syn::Error::new_spanned(
            &type_list.types,
            "At least one account type must be specified",
        ));
    }

    let account_variants = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            #name(#name),
            #packed_name(#packed_name),
        }
    });

    let enum_def = quote! {
        #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
        pub enum CompressedAccountVariant {
            #(#account_variants)*
            /// Program-owned CToken accounts (Vaults)
            PackedCTokenData(light_ctoken_sdk::compat::PackedCTokenData<CTokenAccountVariant>),
            CTokenData(light_ctoken_sdk::compat::CTokenData<CTokenAccountVariant>),
            /// Standard ATA for unified decompression (always available)
            LightAta(light_sdk::compressible::LightAta),
            /// Standard CMint for unified decompression (always available)
            LightMint(light_sdk::compressible::LightMint),
        }
    };

    let first_type = account_types[0];
    let default_impl = quote! {
        impl Default for CompressedAccountVariant {
            fn default() -> Self {
                Self::#first_type(#first_type::default())
            }
        }
    };

    let hash_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#name(data) => <#name as light_hasher::DataHasher>::hash::<H>(data),
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
        }
    });

    let data_hasher_impl = quote! {
        impl light_hasher::DataHasher for CompressedAccountVariant {
            fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], light_hasher::HasherError> {
                match self {
                    #(#hash_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                    Self::LightAta(_) => unreachable!(),
                    Self::LightMint(_) => unreachable!(),
                }
            }
        }
    };

    let light_discriminator_impl = quote! {
        impl light_sdk::LightDiscriminator for CompressedAccountVariant {
            const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
            const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        }
    };

    let compression_info_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#name(data) => <#name as light_sdk::compressible::HasCompressionInfo>::compression_info(data),
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
        }
    });

    let compression_info_mut_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#name(data) => <#name as light_sdk::compressible::HasCompressionInfo>::compression_info_mut(data),
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
        }
    });

    let compression_info_mut_opt_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#name(data) => <#name as light_sdk::compressible::HasCompressionInfo>::compression_info_mut_opt(data),
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
        }
    });

    let set_compression_info_none_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#name(data) => <#name as light_sdk::compressible::HasCompressionInfo>::set_compression_info_none(data),
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
        }
    });

    let has_compression_info_impl = quote! {
        impl light_sdk::compressible::HasCompressionInfo for CompressedAccountVariant {
            fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                    Self::LightAta(_) => unreachable!(),
                    Self::LightMint(_) => unreachable!(),
                }
            }

            fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
                match self {
                    #(#compression_info_mut_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                    Self::LightAta(_) => unreachable!(),
                    Self::LightMint(_) => unreachable!(),
                }
            }

            fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
                match self {
                    #(#compression_info_mut_opt_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                    Self::LightAta(_) => unreachable!(),
                    Self::LightMint(_) => unreachable!(),
                }
            }

            fn set_compression_info_none(&mut self) {
                match self {
                    #(#set_compression_info_none_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                    Self::LightAta(_) => unreachable!(),
                    Self::LightMint(_) => unreachable!(),
                }
            }
        }
    };

    let size_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#name(data) => <#name as light_sdk::account::Size>::size(data),
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
        }
    });

    let size_impl = quote! {
        impl light_sdk::account::Size for CompressedAccountVariant {
            fn size(&self) -> usize {
                match self {
                    #(#size_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(_) => unreachable!(),
                    Self::LightAta(_) => unreachable!(),
                    Self::LightMint(_) => unreachable!(),
                }
            }
        }
    };

    let pack_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#packed_name(_) => unreachable!(),
            CompressedAccountVariant::#name(data) => CompressedAccountVariant::#packed_name(<#name as light_sdk::compressible::Pack>::pack(data, remaining_accounts)),
        }
    });

    let pack_impl = quote! {
        impl light_sdk::compressible::Pack for CompressedAccountVariant {
            type Packed = Self;

            fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                match self {
                    #(#pack_match_arms)*
                    Self::PackedCTokenData(_) => unreachable!(),
                    Self::CTokenData(data) => {
                        // Use ctoken-sdk's Pack trait for CTokenData (program-owned tokens only)
                        Self::PackedCTokenData(light_ctoken_sdk::pack::Pack::pack(data, remaining_accounts))
                    }
                    // LightAta and LightMint are already packed (they come from client pre-packed)
                    Self::LightAta(data) => Self::LightAta(data.clone()),
                    Self::LightMint(data) => Self::LightMint(data.clone()),
                }
            }
        }
    };

    let unpack_match_arms = account_types.iter().map(|name| {
        let packed_name = quote::format_ident!("Packed{}", name);
        quote! {
            CompressedAccountVariant::#packed_name(data) => Ok(CompressedAccountVariant::#name(<#packed_name as light_sdk::compressible::Unpack>::unpack(data, remaining_accounts)?)),
            CompressedAccountVariant::#name(_) => unreachable!(),
        }
    });

    let unpack_impl = quote! {
        impl light_sdk::compressible::Unpack for CompressedAccountVariant {
            type Unpacked = Self;

            fn unpack(
                &self,
                remaining_accounts: &[anchor_lang::prelude::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
                match self {
                    #(#unpack_match_arms)*
                    Self::PackedCTokenData(_data) => Ok(self.clone()),
                    Self::CTokenData(_data) => unreachable!(),
                    // LightAta and LightMint don't need unpacking - indices are used directly
                    Self::LightAta(_data) => Ok(self.clone()),
                    Self::LightMint(_data) => Ok(self.clone()),
                }
            }
        }
    };

    let standard_variant_impl = quote! {
        impl light_sdk::compressible::StandardCompressedVariant for CompressedAccountVariant {
            fn pack_light_ata(
                light_ata: light_sdk::compressible::LightAta,
            ) -> Self::Packed {
                CompressedAccountVariant::LightAta(light_ata)
            }

            fn pack_light_mint(
                light_mint: light_sdk::compressible::LightMint,
            ) -> Self::Packed {
                CompressedAccountVariant::LightMint(light_mint)
            }
        }
    };

    let compressed_account_data_struct = quote! {
        #[derive(Clone, Debug, anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)]
        pub struct CompressedAccountData {
            pub meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            pub data: CompressedAccountVariant,
            // /// Indices into remaining_accounts for seed account references (starting from seed_accounts_offset)
            // pub seed_indices: Vec<u8>,
            // /// Indices into remaining_accounts for authority seed references (for CTokens only)
            // pub authority_indices: Vec<u8>,
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
        #standard_variant_impl
        #compressed_account_data_struct
    };

    Ok(expanded)
}
