use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    ConstParam, Error, Field, Fields, FieldsNamed, GenericParam, ItemStruct, LifetimeParam, Meta,
    Result, Token, TypeParam,
};

pub(crate) struct AlignedSizedArgs {
    /// Include Anchor discriminator (8 bytes).
    anchor: bool,
}

impl Parse for AlignedSizedArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut anchor = false;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "anchor" => anchor = true,
                _ => return Err(input.error("Unsupported attribute")),
            }

            // If there's a comma, consume it, otherwise break out of the loop
            if input.peek(syn::token::Comma) {
                let _ = input.parse::<syn::token::Comma>();
            } else {
                break;
            }
        }

        Ok(Self { anchor })
    }
}

/// Provides an impelentation of `LEN` constant for the givent struct.
pub(crate) fn aligned_sized(args: AlignedSizedArgs, strct: ItemStruct) -> Result<TokenStream> {
    let name = strct.clone().ident;

    // Expressions which define the size of each field. They can be:
    //
    // * `core::mem::size_of<T>()` calls - that's what we pick for fields without
    //   attributes.
    // * Integer literals - either provided via `size` field attribute or
    //   defined by us (Anchor discriminator).
    let mut field_size_getters = if args.anchor {
        // Add Anchor discriminator.
        let mut v = Vec::with_capacity(strct.fields.len() + 1);
        v.push(quote! { 8 });
        v
    } else {
        Vec::with_capacity(strct.fields.len())
    };

    let mut fields = Punctuated::new();

    // Iterate over all fields. Try to find the `size` attribute in them. If
    // not found, construct a `core::mem::size_of::<T>()` expression
    for field in strct.fields.iter() {
        // Attributes to reassign to the field.
        // We want to exclude our `#[size]` attribure here, because it might
        // annoy the compiler. To be safe, we need to remove it right after
        // consuming it.
        let mut attrs = Vec::with_capacity(field.attrs.len());

        // Iterate over attributes.
        for attr in field.attrs.iter() {
            // Check the type of attribute. We look for meta name attribute
            // with `size` key, e.g. `#[size = 128]`.
            //
            // For all other types, return an error.
            match attr.clone().meta {
                Meta::NameValue(name_value) => {
                    if name_value.path.is_ident("size") {
                        let value = name_value.value;
                        field_size_getters.push(quote! { #value });

                        // Go to the next attribute. Do not include this one.
                        continue;
                    }
                    // Include all other attributes.
                    attrs.push(attr.to_owned());
                }
                // Include all other attributes.
                _ => attrs.push(attr.to_owned()),
            }
        }

        let ty = field.clone().ty;
        field_size_getters.push(quote! { ::core::mem::size_of::<#ty>() });

        let field =
            Field {
                attrs,
                vis: field.vis.clone(),
                mutability: field.mutability.clone(),
                ident: field.ident.clone(),
                colon_token: field.colon_token,
                ty: field.ty.clone(),
            };
        fields.push(field);
    }

    // Replace fields to make sure that the updated struct definition doesn't
    // have fields with `size` attribute.
    let brace_token = match strct.fields {
        Fields::Named(fields_named) => fields_named.brace_token,
        _ => {
            return Err(Error::new(
                Span::call_site(),
                "Only structs with named fields are supported",
            ))
        }
    };
    let fields = Fields::Named(FieldsNamed {
        brace_token,
        named: fields,
    });
    let strct = ItemStruct {
        attrs: strct.attrs.clone(),
        vis: strct.vis.clone(),
        struct_token: strct.struct_token,
        ident: strct.ident.clone(),
        generics: strct.generics.clone(),
        // Exactly here.
        fields,
        semi_token: strct.semi_token,
    };

    #[allow(clippy::redundant_clone)]
    let impl_generics = strct.generics.clone();
    // Generics listed after struct ident need to contain only idents, bounds
    // and const generic types are not expected anymore. Sadly, there seems to
    // be no quick way to do that cleanup in non-manual way.
    let strct_generics: Punctuated<GenericParam, Token![,]> =
        strct
            .generics
            .params
            .clone()
            .into_iter()
            .map(|param: GenericParam| match param {
                GenericParam::Const(ConstParam { ident, .. })
                | GenericParam::Type(TypeParam { ident, .. }) => GenericParam::Type(TypeParam {
                    attrs: vec![],
                    ident,
                    colon_token: None,
                    bounds: Default::default(),
                    eq_token: None,
                    default: None,
                }),
                GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => {
                    GenericParam::Lifetime(LifetimeParam {
                        attrs: vec![],
                        lifetime,
                        colon_token: None,
                        bounds: Default::default(),
                    })
                }
            })
            .collect();

    // Define a constant with the size of the struct (sum of all fields).
    Ok(quote! {
        #strct

        impl #impl_generics #name <#strct_generics> {
            pub const LEN: usize = #(#field_size_getters)+*;
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn sized_struct() {
        let test_struct: ItemStruct = parse_quote! {
            struct TestStruct {
                foo: u32,
                bar: u32,
                ayy: u16,
                lmao: u16,
                #[size = 128]
                kaboom: Vec<u8>,
            }
        };

        let res_no_args = aligned_sized(parse_quote! {}, test_struct.clone())
            .unwrap()
            .to_string();
        assert!(res_no_args.contains(":: core :: mem :: size_of :: < u32 > ()"));
        assert!(res_no_args.contains(":: core :: mem :: size_of :: < u16 > ()"));
        assert!(res_no_args.contains(" 128usize "));

        let res_anchor = aligned_sized(parse_quote! { anchor }, test_struct)
            .unwrap()
            .to_string();
        assert!(res_anchor.contains(" 8 "));
        assert!(res_anchor.contains(":: core :: mem :: size_of :: < u32 > ()"));
        assert!(res_anchor.contains(":: core :: mem :: size_of :: < u16 > ()"));
        assert!(res_anchor.contains(" 128usize "));
    }
}
