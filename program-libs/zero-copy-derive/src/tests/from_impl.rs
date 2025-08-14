use quote::format_ident;
use syn::{parse_quote, Field};

use crate::shared::from_impl::*;

#[test]
fn test_generate_from_impl() {
    // Create a struct for testing
    let name = format_ident!("TestStruct");
    let z_struct_name = format_ident!("ZTestStruct");

    // Create some test fields
    let field_a: Field = parse_quote!(pub a: u8);
    let field_b: Field = parse_quote!(pub b: u16);
    let field_c: Field = parse_quote!(pub c: Vec<u8>);

    // Split into meta and struct fields
    let meta_fields = vec![&field_a, &field_b];
    let struct_fields = vec![&field_c];

    // Generate the implementation
    let result = generate_from_impl::<false>(&name, &z_struct_name, &meta_fields, &struct_fields);

    // Convert to string for testing
    let result_str = result.unwrap().to_string();

    // Check that the implementation contains required elements
    assert!(result_str.contains("impl < 'a > From < ZTestStruct < 'a >> for TestStruct"));

    // Check field handling
    assert!(result_str.contains("a :")); // For u8 fields
    assert!(result_str.contains("b :")); // For u16 fields
    assert!(result_str.contains("c :")); // For Vec<u8> fields
}

#[test]
fn test_generate_from_impl_mut() {
    // Create a struct for testing
    let name = format_ident!("TestStruct");
    let z_struct_name = format_ident!("ZTestStruct");

    // Create some test fields
    let field_a: Field = parse_quote!(pub a: u8);
    let field_b: Field = parse_quote!(pub b: bool);
    let field_c: Field = parse_quote!(pub c: Option<u32>);

    // Split into meta and struct fields
    let meta_fields = vec![&field_a, &field_b];
    let struct_fields = vec![&field_c];

    // Generate the implementation for mutable version
    let result = generate_from_impl::<true>(&name, &z_struct_name, &meta_fields, &struct_fields);

    // Convert to string for testing
    let result_str = result.unwrap().to_string();

    // Check that the implementation contains required elements
    assert!(result_str.contains("impl < 'a > From < ZTestStructMut < 'a >> for TestStruct"));

    // Check field handling
    assert!(result_str.contains("a :")); // For u8 fields
    assert!(result_str.contains("b :")); // For bool fields
    assert!(result_str.contains("c :")); // For Option fields
}
