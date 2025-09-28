use quote::ToTokens;
use syn::{Error, Field, Fields, ItemStruct, Result};

/// Different types of attributes that can be applied to fields
#[derive(Debug, PartialEq)]
pub(crate) enum FieldAttribute {
    Hash,
    Skip,
    Flatten,
    None,
}

/// Different types of fields that need different handling
#[derive(Debug, PartialEq)]
pub(crate) enum FieldType {
    VecU8,
    OptionVecU8,
    Option,
    Pubkey,
    U8Array,
    OptionU8Array,
    Default,
}

/// Validates an ItemStruct for use with the LightHasher derive macro
pub(crate) fn validate_input(input: &ItemStruct) -> Result<()> {
    // Check that we have a struct with named fields
    match &input.fields {
        Fields::Named(_) => (),
        _ => {
            return Err(Error::new_spanned(
                input,
                "Only structs with named fields are supported",
            ))
        }
    };

    // Check the field count
    let field_count = input.fields.iter().count();
    if field_count >= 13 {
        return Err(Error::new_spanned(
            input,
            "Structs with 13 or more fields are not supported.",
        ));
    }

    // Check for flatten attribute support
    let flatten_field_exists = input
        .fields
        .iter()
        .any(|field| get_field_attribute(field) == FieldAttribute::Flatten);

    if flatten_field_exists {
        return Err(Error::new_spanned(
            input,
            "Flatten attribute is not supported.",
        ));
    }

    Ok(())
}

/// SHA256-specific validation - much more relaxed constraints
pub(crate) fn validate_input_sha(input: &ItemStruct) -> Result<()> {
    // Check that we have a struct with named fields
    match &input.fields {
        Fields::Named(_) => (),
        _ => {
            return Err(Error::new_spanned(
                input,
                "Only structs with named fields are supported",
            ))
        }
    };

    // For SHA256, we don't limit field count or require specific attributes
    // Just ensure flatten is not used (not implemented for SHA256 path)
    let flatten_field_exists = input
        .fields
        .iter()
        .any(|field| get_field_attribute(field) == FieldAttribute::Flatten);

    if flatten_field_exists {
        return Err(Error::new_spanned(
            input,
            "Flatten attribute is not supported in SHA256 hasher.",
        ));
    }

    Ok(())
}

/// Gets the primary attribute for a field (only one attribute can be active)
pub(crate) fn get_field_attribute(field: &Field) -> FieldAttribute {
    if field.attrs.iter().any(|attr| attr.path().is_ident("hash")) {
        FieldAttribute::Hash
    } else if field.attrs.iter().any(|attr| attr.path().is_ident("skip")) {
        FieldAttribute::Skip
    } else if field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("flatten"))
    {
        FieldAttribute::Flatten
    } else {
        FieldAttribute::None
    }
}

/// Detects the type of a field for specialized processing
pub(crate) fn detect_field_type(field: &Field) -> FieldType {
    let type_str = field.ty.to_token_stream().to_string();

    if type_str == "Vec < u8 >" {
        FieldType::VecU8
    } else if type_str.starts_with("Option < Vec < u8 > >") {
        FieldType::OptionVecU8
    } else if type_str.starts_with("Option < ") {
        FieldType::Option
    } else if type_str.starts_with("Pubkey") {
        FieldType::Pubkey
    } else if type_str.starts_with("[ u8 ;") {
        FieldType::U8Array
    } else if type_str.starts_with("Option < [ u8 ;") {
        FieldType::OptionU8Array
    } else {
        FieldType::Default
    }
}
