use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, Fields, Ident};

/// Generate the zero-copy enum definition with type aliases for pattern matching
/// The `MUT` parameter controls whether to generate mutable or immutable variants
pub fn generate_z_enum<const MUT: bool>(
    z_enum_name: &Ident,
    enum_data: &DataEnum,
) -> syn::Result<TokenStream> {
    // Add Mut suffix when MUT is true
    let z_enum_name = if MUT {
        format_ident!("{}Mut", z_enum_name)
    } else {
        z_enum_name.clone()
    };

    // Collect type aliases for complex variants
    let mut type_aliases = Vec::new();
    let mut has_lifetime_dependent_variants = false;

    let variants = enum_data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Unit => {
                Ok(quote! { #variant_name })
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Single unnamed field: TokenMetadata(TokenMetadataInstructionData)
                let field_type = &fields.unnamed.first()
                    .ok_or_else(|| syn::Error::new_spanned(
                        fields,
                        "Internal error: expected exactly one unnamed field but found none"
                    ))?
                    .ty;

                // This variant uses lifetime
                has_lifetime_dependent_variants = true;

                // Create a type alias for this variant to enable pattern matching
                let alias_name = if MUT {
                    format_ident!("{}TypeMut", variant_name)
                } else {
                    format_ident!("{}Type", variant_name)
                };

                // Generate appropriate type based on MUT
                type_aliases.push(if MUT {
                    quote! {
                        pub type #alias_name<'a> = <#field_type as ::light_zero_copy::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut;
                    }
                } else {
                    quote! {
                        pub type #alias_name<'a> = <#field_type as ::light_zero_copy::traits::ZeroCopyAt<'a>>::ZeroCopyAt;
                    }
                });

                Ok(quote! { #variant_name(#alias_name<'a>) })
            }
            Fields::Named(_) => {
                // Named fields - not commonly used in enums but we can support it
                Err(syn::Error::new_spanned(
                    variant,
                    "Named fields in enum variants are not supported yet",
                ))
            }
            Fields::Unnamed(fields) if fields.unnamed.len() > 1 => {
                // Multiple unnamed fields - not common but we can support it
                Err(syn::Error::new_spanned(
                    variant,
                    "Multiple unnamed fields in enum variants are not supported yet",
                ))
            }
            _ => {
                Err(syn::Error::new_spanned(
                    variant,
                    "Unsupported enum variant format",
                ))
            }
        }
    }).collect::<Result<Vec<_>, _>>()?;

    // For mutable enums, we don't derive Clone (can't clone mutable references)
    let derive_attrs = if MUT {
        quote! { #[derive(Debug, PartialEq)] }
    } else {
        quote! { #[derive(Debug, Clone, PartialEq)] }
    };

    // Conditionally add lifetime parameter only if needed
    let enum_declaration = if has_lifetime_dependent_variants {
        quote! {
            #derive_attrs
            pub enum #z_enum_name<'a> {
                #(#variants,)*
            }
        }
    } else {
        quote! {
            #derive_attrs
            pub enum #z_enum_name {
                #(#variants,)*
            }
        }
    };

    Ok(quote! {
        // Generate type aliases for complex variants
        #(#type_aliases)*

        #enum_declaration
    })
}

/// Generate the deserialize implementation for the enum
/// The `MUT` parameter controls whether to generate mutable or immutable deserialization
pub fn generate_enum_deserialize_impl<const MUT: bool>(
    original_name: &Ident,
    z_enum_name: &Ident,
    enum_data: &DataEnum,
) -> syn::Result<TokenStream> {
    // Add Mut suffix when MUT is true
    let z_enum_name = if MUT {
        format_ident!("{}Mut", z_enum_name)
    } else {
        z_enum_name.clone()
    };

    // Choose trait and method based on MUT
    let (trait_name, mutability, method_name, associated_type) = if MUT {
        (
            quote!(::light_zero_copy::traits::ZeroCopyAtMut),
            quote!(mut),
            quote!(zero_copy_at_mut),
            quote!(ZeroCopyAtMut),
        )
    } else {
        (
            quote!(::light_zero_copy::traits::ZeroCopyAt),
            quote!(),
            quote!(zero_copy_at),
            quote!(ZeroCopyAt),
        )
    };

    // Check if any variants need lifetime parameters
    let mut has_lifetime_dependent_variants = false;

    // Generate match arms for each variant
    let match_arms_result: Result<Vec<TokenStream>, syn::Error> = enum_data.variants.iter().enumerate().map(|(index, variant)| {
        let variant_name = &variant.ident;
        let discriminant = u8::try_from(index)
            .map_err(|_| syn::Error::new_spanned(
                variant,
                format!("Enum variant index {} exceeds u8 maximum (255)", index)
            ))?;

        match &variant.fields {
            Fields::Unit => {
                // Unit variant
                Ok(quote! {
                    #discriminant => {
                        Ok((#z_enum_name::#variant_name, remaining_data))
                    }
                })
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Single unnamed field needs lifetime
                has_lifetime_dependent_variants = true;

                let field_type = &fields.unnamed.first()
                    .ok_or_else(|| syn::Error::new_spanned(
                        fields,
                        "Internal error: expected exactly one unnamed field but found none"
                    ))?
                    .ty;

                // Use appropriate trait method based on MUT
                let deserialize_call = if MUT {
                    quote! {
                        <#field_type as ::light_zero_copy::traits::ZeroCopyAtMut>::zero_copy_at_mut(remaining_data)?
                    }
                } else {
                    quote! {
                        <#field_type as ::light_zero_copy::traits::ZeroCopyAt>::zero_copy_at(remaining_data)?
                    }
                };

                Ok(quote! {
                    #discriminant => {
                        let (value, remaining_bytes) = #deserialize_call;
                        Ok((#z_enum_name::#variant_name(value), remaining_bytes))
                    }
                })
            }
            _ => {
                // Other cases already handled in generate_z_enum
                Ok(quote! {
                    #discriminant => {
                        Err(::light_zero_copy::errors::ZeroCopyError::InvalidConversion)
                    }
                })
            }
        }
    }).collect();
    let match_arms = match_arms_result?;

    // Conditional type annotation based on whether lifetime is needed
    let type_annotation = if has_lifetime_dependent_variants {
        quote! { #z_enum_name<'a> }
    } else {
        quote! { #z_enum_name }
    };

    Ok(quote! {
        impl<'a> #trait_name<'a> for #original_name {
            type #associated_type = #type_annotation;

            fn #method_name(
                data: &'a #mutability [u8],
            ) -> Result<(Self::#associated_type, &'a #mutability [u8]), ::light_zero_copy::errors::ZeroCopyError> {
                // Read discriminant (first 1 byte for borsh enum)
                // Note: Discriminant is ALWAYS immutable for safety, even in mutable deserialization
                if data.is_empty() {
                    return Err(::light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        data.len(),
                    ));
                }

                let discriminant = data[0];
                let remaining_data = &#mutability data[1..];

                match discriminant {
                    #(#match_arms)*
                    _ => Err(::light_zero_copy::errors::ZeroCopyError::InvalidConversion),
                }
            }
        }
    })
}

/// Generate the ZeroCopyStructInner implementation for the enum
/// The `MUT` parameter controls whether to generate mutable or immutable struct inner trait
pub fn generate_enum_zero_copy_struct_inner<const MUT: bool>(
    original_name: &Ident,
    z_enum_name: &Ident,
    enum_data: &DataEnum,
) -> syn::Result<TokenStream> {
    // Add Mut suffix when MUT is true
    let z_enum_name = if MUT {
        format_ident!("{}Mut", z_enum_name)
    } else {
        z_enum_name.clone()
    };

    // Check if any variants need lifetime parameters
    let has_lifetime_dependent_variants = enum_data.variants.iter().any(
        |variant| matches!(&variant.fields, Fields::Unnamed(fields) if fields.unnamed.len() == 1),
    );

    // Conditional type annotation based on whether lifetime is needed
    let type_annotation = if has_lifetime_dependent_variants {
        quote! { #z_enum_name<'static> }
    } else {
        quote! { #z_enum_name }
    };

    // Generate appropriate trait impl based on MUT
    Ok(if MUT {
        quote! {
            impl ::light_zero_copy::traits::ZeroCopyStructInnerMut for #original_name {
                type ZeroCopyInnerMut = #type_annotation;
            }
        }
    } else {
        quote! {
            impl ::light_zero_copy::traits::ZeroCopyStructInner for #original_name {
                type ZeroCopyInner = #type_annotation;
            }
        }
    })
}
