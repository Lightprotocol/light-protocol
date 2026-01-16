//! File scanning for #[rentfree_program] macro.
//!
//! This module reads external Rust source files to extract seed information
//! from Accounts structs that contain #[rentfree] fields.

use std::path::{Path, PathBuf};
use syn::{Item, ItemMod, ItemStruct};

use crate::compressible::anchor_seeds::{
    extract_from_accounts_struct, ExtractedAccountsInfo, ExtractedSeedSpec, ExtractedTokenSpec,
};

/// Result of scanning a module and its external files
#[derive(Debug, Default)]
pub struct ScannedModuleInfo {
    pub pda_specs: Vec<ExtractedSeedSpec>,
    pub token_specs: Vec<ExtractedTokenSpec>,
    pub errors: Vec<String>,
    /// Names of Accounts structs that have rentfree fields (for auto-wrapping handlers)
    pub rentfree_struct_names: std::collections::HashSet<String>,
}

/// Scan the entire src/ directory for Accounts structs with #[rentfree] fields.
///
/// This function scans all .rs files in the crate's src/ directory
/// and extracts seed information from Accounts structs.
pub fn scan_module_for_compressible(
    _module: &ItemMod,
    base_path: &Path,
) -> syn::Result<ScannedModuleInfo> {
    let mut result = ScannedModuleInfo::default();

    // Scan all .rs files in the src directory
    scan_directory_recursive(base_path, &mut result);

    Ok(result)
}

/// Recursively scan a directory for .rs files
fn scan_directory_recursive(dir: &Path, result: &mut ScannedModuleInfo) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            result
                .errors
                .push(format!("Failed to read directory {:?}: {}", dir, e));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            scan_directory_recursive(&path, result);
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            scan_rust_file(&path, result);
        }
    }
}

/// Scan a single Rust file for Accounts structs
fn scan_rust_file(path: &Path, result: &mut ScannedModuleInfo) {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            result
                .errors
                .push(format!("Failed to read {:?}: {}", path, e));
            return;
        }
    };

    let parsed: syn::File = match syn::parse_str(&contents) {
        Ok(f) => f,
        Err(e) => {
            // Not all files may be valid on their own (e.g., test files with main)
            // Just skip them silently
            let _ = e;
            return;
        }
    };

    for item in parsed.items {
        match item {
            Item::Struct(item_struct) => {
                if let Ok(Some((info, struct_name))) = try_extract_from_struct(&item_struct) {
                    result.pda_specs.extend(info.pda_fields);
                    result.token_specs.extend(info.token_fields);
                    result.rentfree_struct_names.insert(struct_name);
                }
            }
            Item::Mod(inner_mod) if inner_mod.content.is_some() => {
                // Inline module - recursively scan
                scan_inline_module(&inner_mod, result);
            }
            _ => {}
        }
    }
}

/// Scan an inline module for Accounts structs
fn scan_inline_module(module: &ItemMod, result: &mut ScannedModuleInfo) {
    let content = match &module.content {
        Some((_, items)) => items,
        None => return,
    };

    for item in content {
        match item {
            Item::Struct(item_struct) => {
                if let Ok(Some((info, struct_name))) = try_extract_from_struct(item_struct) {
                    result.pda_specs.extend(info.pda_fields);
                    result.token_specs.extend(info.token_fields);
                    result.rentfree_struct_names.insert(struct_name);
                }
            }
            Item::Mod(inner_mod) if inner_mod.content.is_some() => {
                scan_inline_module(inner_mod, result);
            }
            _ => {}
        }
    }
}

/// Try to extract rentfree info from a struct.
/// Returns (ExtractedAccountsInfo, struct_name) if the struct has rentfree fields.
fn try_extract_from_struct(
    item_struct: &ItemStruct,
) -> syn::Result<Option<(ExtractedAccountsInfo, String)>> {
    // Check if it has #[derive(Accounts)]
    let has_accounts_derive = item_struct.attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            if let Ok(meta) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            ) {
                return meta.iter().any(|p| p.is_ident("Accounts"));
            }
        }
        false
    });

    if !has_accounts_derive {
        return Ok(None);
    }

    let info = extract_from_accounts_struct(item_struct)?;
    match info {
        Some(extracted) => {
            let struct_name = extracted.struct_name.to_string();
            Ok(Some((extracted, struct_name)))
        }
        None => Ok(None),
    }
}

/// Resolve the base path for the crate source
///
/// This attempts to find the src/ directory by looking at CARGO_MANIFEST_DIR
/// or falling back to current directory.
pub fn resolve_crate_src_path() -> PathBuf {
    // Try CARGO_MANIFEST_DIR first (set during cargo build)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let src_path = PathBuf::from(&manifest_dir).join("src");
        if src_path.exists() {
            return src_path;
        }
        // Fallback to manifest dir itself
        return PathBuf::from(manifest_dir);
    }

    // Fallback to current directory
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("src")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path() {
        let path = resolve_crate_src_path();
        println!("Resolved path: {:?}", path);
    }
}
