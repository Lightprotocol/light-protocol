use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Ident};

use crate::hasher::input_validator::{
    detect_field_type, get_field_attribute, FieldAttribute, FieldType,
};

pub(crate) struct FieldProcessingContext {
    pub flatten_field_exists: bool,
    pub hash_to_field_size_imported: bool,
    pub added_flattned_field: bool,
    pub hash_to_field_size_code: Vec<TokenStream>,
    pub data_hasher_assignments: Vec<TokenStream>,
    pub flattened_fields_added: Vec<TokenStream>,
    pub code: Vec<TokenStream>,
}

impl FieldProcessingContext {
    pub fn new(flatten_field_exists: bool) -> Self {
        Self {
            flatten_field_exists,
            hash_to_field_size_imported: false,
            added_flattned_field: false,
            hash_to_field_size_code: Vec::new(),
            data_hasher_assignments: Vec::new(),
            flattened_fields_added: Vec::new(),
            code: Vec::new(),
        }
    }

    pub fn add_hash_import(&mut self) {
        if !self.hash_to_field_size_imported {
            self.hash_to_field_size_code.push(quote! {
                use ::light_hasher::hash_to_field_size::HashToFieldSize;
            });
            self.hash_to_field_size_imported = true;
        }
    }
}

/// Process a single field from the struct and generate the corresponding code for trait implementations
pub(crate) fn process_field(field: &Field, index: usize, context: &mut FieldProcessingContext) {
    let field_name = &field.ident;
    let attribute = get_field_attribute(field);

    match attribute {
        FieldAttribute::None => {
            process_regular_field(field_name, index, context);
        }
        FieldAttribute::Hash => {
            process_hash_field(field, field_name, index, context);
        }
        FieldAttribute::Skip => {
            // Skip this field
        }
        FieldAttribute::Flatten => {
            process_flatten_field(field, index, context);
        }
    }
}

fn process_regular_field(
    field_name: &Option<Ident>,
    index: usize,
    context: &mut FieldProcessingContext,
) {
    if context.flatten_field_exists {
        context.data_hasher_assignments.push(quote! {
            field_array[#index + num_flattned_fields] = self.#field_name.to_byte_array()?;
        });
    } else {
        context.data_hasher_assignments.push(quote! {
            self.#field_name.to_byte_array()?
        });
    }
}

/// HashToFieldSize:
/// 1. General case: self.#field_name.hash_to_field_size()?
/// 2. Vec<u8> -> hashv_to_bn254_field_size_be(&[self.#field_name.as_slice()])
/// 3. Option<Vec<u8>> -> if let Some(#field_name) = self.#field_name { hashv_to_bn254_field_size_be(&[self.#field_name.as_slice()]) } else { [0u8;32] }
fn process_hash_field(
    field: &Field,
    field_name: &Option<Ident>,
    index: usize,
    context: &mut FieldProcessingContext,
) {
    context.add_hash_import();

    let field_type = detect_field_type(field);

    match field_type {
        FieldType::Default => process_hash_default(field_name, index, context),
        FieldType::VecU8 => process_vec_u8_field(field_name, index, context),
        FieldType::OptionVecU8 => process_option_vec_u8_field(field_name, index, context),
        FieldType::Option => process_option_field(field_name, index, context),
        FieldType::Pubkey => process_pubkey_field(field_name, index, context),
        // u8 arrays share vec u8 implementations.
        FieldType::U8Array => process_vec_u8_field(field_name, index, context),
        FieldType::OptionU8Array => process_option_vec_u8_field(field_name, index, context),
    }
}

fn process_pubkey_field(
    field_name: &Option<Ident>,
    index: usize,
    context: &mut FieldProcessingContext,
) {
    if context.flatten_field_exists {
        context.data_hasher_assignments.push(quote! {
            field_array[#index + num_flattned_fields] = ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_ref()).as_slice();
            slices[#index + num_flattned_fields] = field_array[#index + num_flattned_fields].as_slice();
        });
    } else {
        context.data_hasher_assignments.push(quote! {
            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_ref())
        });
    }
}

fn process_vec_u8_field(
    field_name: &Option<Ident>,
    index: usize,
    context: &mut FieldProcessingContext,
) {
    if context.flatten_field_exists {
        context.data_hasher_assignments.push(quote! {
            field_array[#index + num_flattned_fields] = ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_slice()).as_slice();
            slices[#index + num_flattned_fields] = field_array[#index + num_flattned_fields].as_slice();
        });
    } else {
        context.data_hasher_assignments.push(quote! {
            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.#field_name.as_slice())
        });
    }
}

fn process_option_vec_u8_field(
    field_name: &Option<Ident>,
    index: usize,
    context: &mut FieldProcessingContext,
) {
    if context.flatten_field_exists {
        context.data_hasher_assignments.push(quote! {
            field_array[#index + num_flattned_fields] = if let Some(#field_name) = &self.#field_name {
                ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(#field_name.as_slice())
            } else {
                [0u8;32]
            };
            slices[#index + num_flattned_fields] = field_array[#index + num_flattned_fields].as_slice();
        });
    } else {
        context.data_hasher_assignments.push(quote! {
            {
                if let Some(#field_name) = &self.#field_name {
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(#field_name.as_slice())
                } else {
                    [0u8;32]
                }
            }
        });
    }
}

fn process_option_field(
    field_name: &Option<Ident>,
    index: usize,
    context: &mut FieldProcessingContext,
) {
    if context.flatten_field_exists {
        context.data_hasher_assignments.push(quote! {
            field_array[#index + num_flattned_fields] = if let Some(#field_name) = &self.#field_name {
               let result = #field_name.hash_to_field_size()?;
                // Security check to ensure that hash_to_field_size
                // does not produce a collision with None.
                // This cannot happen in light_hasher hash_to_field_size implementations,
                // but third parties could implement hash_to_field_size insecurely.
                if result == [0u8; 32] {
                    return Err(::light_hasher::errors::HasherError::OptionHashToFieldSizeZero);
                }
                result
            } else {
                [0u8;32]
            };
            slices[#index + num_flattned_fields] = field_array[#index + num_flattned_fields].as_slice();
        });
    } else {
        context.data_hasher_assignments.push(quote! {
            if let Some(#field_name) = &self.#field_name {
                let result = #field_name.hash_to_field_size()?;
                // Security check to ensure that hash_to_field_size
                // does not produce a collision with None.
                // This cannot happen in light_hasher hash_to_field_size implementations,
                // but third parties could implement hash_to_field_size insecurely.
                if result == [0u8; 32] {
                    return Err(::light_hasher::errors::HasherError::OptionHashToFieldSizeZero);
                }
                result
            } else {
                [0u8;32]
            }
        });
    }
}

fn process_hash_default(
    field_name: &Option<Ident>,
    index: usize,
    context: &mut FieldProcessingContext,
) {
    if context.flatten_field_exists {
        context.data_hasher_assignments.push(quote! {
            field_array[#index + num_flattned_fields] = self.#field_name.hash_to_field_size()?;
        });
    } else {
        context.data_hasher_assignments.push(quote! {
            self.#field_name.hash_to_field_size()?
        });
    }
}

fn process_flatten_field(field: &Field, index: usize, context: &mut FieldProcessingContext) {
    let field_type = &field.ty;
    let field_name = &field.ident;

    if !context.added_flattned_field {
        context.added_flattned_field = true;
        context.flattened_fields_added.push(quote! {
            #field_type::NUM_FIELDS as usize
        });
    } else {
        context.flattened_fields_added.push(quote! {
            + #field_type::NUM_FIELDS as usize
        });
    }

    context.code.push(quote! {
        {
            for (j, element) in <#field_type as ::light_hasher::to_byte_array::ToByteArray>::to_byte_arrays::<{#field_type::NUM_FIELDS}>(&self.#field_name)?.iter().enumerate() {
                field_array[#index + j + num_flattned_fields] = *element;
                num_flattned_fields += 1;
            }
        }
    });
}
