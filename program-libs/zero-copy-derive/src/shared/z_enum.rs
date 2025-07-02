use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, Fields, Ident};

/// Generate the zero-copy enum definition with type aliases for pattern matching
pub fn generate_z_enum(z_enum_name: &Ident, enum_data: &DataEnum) -> syn::Result<TokenStream> {
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
                let alias_name = format_ident!("{}Type", variant_name);
                type_aliases.push(quote! {
                    pub type #alias_name<'a> = <#field_type as ::light_zero_copy::traits::ZeroCopyAt<'a>>::ZeroCopyAt;
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

    // Conditionally add lifetime parameter only if needed
    let enum_declaration = if has_lifetime_dependent_variants {
        quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub enum #z_enum_name<'a> {
                #(#variants,)*
            }
        }
    } else {
        quote! {
            #[derive(Debug, Clone, PartialEq)]
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
pub fn generate_enum_deserialize_impl(
    original_name: &Ident,
    z_enum_name: &Ident,
    enum_data: &DataEnum,
) -> syn::Result<TokenStream> {
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
                Ok(quote! {
                    #discriminant => {
                        let (value, remaining_bytes) =
                            <#field_type as ::light_zero_copy::traits::ZeroCopyAt>::zero_copy_at(remaining_data)?;
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
        impl<'a> ::light_zero_copy::traits::ZeroCopyAt<'a> for #original_name {
            type ZeroCopyAt = #type_annotation;

            fn zero_copy_at(
                data: &'a [u8],
            ) -> Result<(Self::ZeroCopyAt, &'a [u8]), ::light_zero_copy::errors::ZeroCopyError> {
                // Read discriminant (first 1 byte for borsh enum)
                if data.is_empty() {
                    return Err(::light_zero_copy::errors::ZeroCopyError::ArraySize(
                        1,
                        data.len(),
                    ));
                }

                let discriminant = data[0];
                let remaining_data = &data[1..];

                match discriminant {
                    #(#match_arms)*
                    _ => Err(::light_zero_copy::errors::ZeroCopyError::InvalidConversion),
                }
            }
        }
    })
}

/// Generate the ZeroCopyStructInner implementation for the enum
pub fn generate_enum_zero_copy_struct_inner(
    original_name: &Ident,
    z_enum_name: &Ident,
    enum_data: &DataEnum,
) -> syn::Result<TokenStream> {
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

    Ok(quote! {
        impl ::light_zero_copy::traits::ZeroCopyStructInner for #original_name {
            type ZeroCopyInner = #type_annotation;
        }
    })
}
