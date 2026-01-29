//! Unit tests for crate context parsing utilities.
//!
//! Tests for `light_pdas/parsing/crate_context.rs`.

use syn::ItemStruct;

use crate::light_pdas::parsing::crate_context::has_derive_attribute;

#[test]
fn test_has_derive_attribute() {
    let code = quote::quote! {
        #[derive(Accounts, LightAccounts)]
        pub struct CreateUser<'info> {
            pub fee_payer: Signer<'info>,
        }
    };
    let item: ItemStruct = syn::parse2(code).unwrap();
    assert!(has_derive_attribute(&item.attrs, "LightAccounts"));
    assert!(has_derive_attribute(&item.attrs, "Accounts"));
    assert!(!has_derive_attribute(&item.attrs, "Clone"));
}

#[test]
fn test_has_derive_attribute_qualified() {
    let code = quote::quote! {
        #[derive(light_sdk::LightAccounts)]
        pub struct CreateUser<'info> {
            pub fee_payer: Signer<'info>,
        }
    };
    let item: ItemStruct = syn::parse2(code).unwrap();
    assert!(has_derive_attribute(&item.attrs, "LightAccounts"));
}
