//! Shared utility functions for compressible macro generation.

use syn::{GenericArgument, PathArguments, Type};

/// Determines if a type is a Copy type (primitives, Pubkey, and Options of Copy types).
///
/// This is used to decide whether to use `.clone()` or direct copy during field assignments.
#[inline(never)]
pub(crate) fn is_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                matches!(
                    type_name.as_str(),
                    "u8" | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                        | "Pubkey"
                ) || (type_name == "Option" && has_copy_inner_type(&segment.arguments))
            } else {
                false
            }
        }
        Type::Array(_) => true,
        _ => false,
    }
}

/// Checks if a type argument contains a Copy type (for generic types like Option<T>).
#[inline(never)]
pub(crate) fn has_copy_inner_type(args: &PathArguments) -> bool {
    match args {
        PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| {
            if let GenericArgument::Type(ty) = arg {
                is_copy_type(ty)
            } else {
                false
            }
        }),
        _ => false,
    }
}

/// Determines if a type is specifically a Pubkey type.
#[inline(never)]
pub(crate) fn is_pubkey_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            segment.ident == "Pubkey"
        } else {
            false
        }
    } else {
        false
    }
}
