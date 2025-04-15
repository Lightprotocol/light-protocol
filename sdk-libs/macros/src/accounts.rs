use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Fields, ItemStruct, Result};

pub(crate) fn process_light_system_accounts(input: ItemStruct) -> Result<TokenStream> {
    let mut output = input.clone();

    let fields =
        match output.fields {
            Fields::Named(ref mut fields) => fields,
            _ => return Err(Error::new_spanned(
                input,
                "`light_system_accounts` attribute can only be used with structs that have named fields.",
            )),
        };

    let fields_to_add = [
        ("light_system_program", "AccountInfo<'info>"),
        ("system_program", "Program<'info, System>"),
        ("account_compression_program", "AccountInfo<'info>"),
    ];
    let fields_to_add_check = [
        ("registered_program_pda", "AccountInfo<'info>"),
        ("noop_program", "AccountInfo<'info>"),
        ("account_compression_authority", "AccountInfo<'info>"),
    ];
    let existing_field_names: Vec<_> = fields
        .named
        .iter()
        .map(|f| f.ident.as_ref().unwrap().to_string())
        .collect();

    // TODO: Eventually we want to provide flexibility to override.
    // Until then, we error if the fields are manually defined.
    for (field_name, field_type) in fields_to_add.iter().chain(fields_to_add_check.iter()) {
        if existing_field_names.contains(&field_name.to_string()) {
            return Err(syn::Error::new_spanned(
                &output,
                format!("Field `{}` already exists in the struct.", field_name),
            ));
        }

        let new_field = syn::Field {
            attrs: vec![],
            vis: syn::Visibility::Public(syn::token::Pub {
                span: proc_macro2::Span::call_site(),
            }),
            mutability: syn::FieldMutability::None,
            ident: Some(syn::Ident::new(field_name, proc_macro2::Span::call_site())),
            colon_token: Some(syn::Token![:](proc_macro2::Span::call_site())),
            ty: syn::parse_str(field_type)?,
        };
        fields.named.push(new_field);
    }

    let expanded = quote! {
        #output
    };

    Ok(expanded)
}

mod light_account_kw {
    // Action
    syn::custom_keyword!(init);
    syn::custom_keyword!(close);
    // Constraint
    syn::custom_keyword!(constraint);
    // Seeds
    syn::custom_keyword!(seeds);
}

#[cfg(test)]
mod tests {
    use syn::{parse_quote, ItemStruct};

    use super::*;

    #[test]
    fn test_process_light_system_accounts_adds_fields_correctly() {
        let input: ItemStruct = parse_quote! {
            struct TestStruct {
                #[light_account(mut)]
                foo: u64,
                existing_field: u32,
            }
        };

        let output = process_light_system_accounts(input).unwrap();
        let output_string = output.to_string();

        println!("{output_string}");

        assert!(output_string.contains("light_system_program"));
        assert!(output_string.contains("system_program"));
        assert!(output_string.contains("account_compression_program"));
        assert!(output_string.contains("registered_program_pda"));
        assert!(output_string.contains("noop_program"));
        assert!(output_string.contains("account_compression_authority"));
    }

    #[test]
    fn test_process_light_system_accounts_fails_on_existing_field() {
        let input: ItemStruct = parse_quote! {
            struct TestStruct {
                existing_field: u32,
                system_program: Program<'info, System>,
            }
        };

        let result = process_light_system_accounts(input);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Field `system_program` already exists in the struct."));
    }
}
