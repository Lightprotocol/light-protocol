//! Anchor-style crate context parser for `#[light_program]`.
//!
//! This module recursively reads all module files at macro expansion time,
//! allowing `#[light_program]` to discover all `#[derive(LightAccounts)]` structs
//! across the entire crate.
//!
//! Based on Anchor's `CrateContext::parse()` pattern from `anchor-syn/src/parser/context.rs`.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use syn::{Item, ItemStruct};

// // =============================================================================

// =============================================================================
// CRATE CONTEXT
// =============================================================================

/// Context containing all parsed modules in the crate.
pub struct CrateContext {
    modules: BTreeMap<String, ParsedModule>,
}

impl CrateContext {
    /// Parse all modules starting from the crate root (lib.rs or main.rs).
    ///
    /// Uses `CARGO_MANIFEST_DIR` environment variable to locate the crate root.
    pub fn parse_from_manifest() -> syn::Result<Self> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "CARGO_MANIFEST_DIR not set - cannot parse crate context",
            )
        })?;

        let src_dir = PathBuf::from(&manifest_dir).join("src");

        // Try lib.rs first, then main.rs
        let root_file = if src_dir.join("lib.rs").exists() {
            src_dir.join("lib.rs")
        } else if src_dir.join("main.rs").exists() {
            src_dir.join("main.rs")
        } else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Could not find lib.rs or main.rs in {:?}", src_dir),
            ));
        };

        Self::parse(&root_file)
    }

    /// Parse all modules starting from a specific root file.
    pub fn parse(root: &Path) -> syn::Result<Self> {
        let modules = ParsedModule::parse_recursive(root, "crate")?;
        Ok(CrateContext { modules })
    }

    /// Iterate over all struct items in all parsed modules.
    pub fn structs(&self) -> impl Iterator<Item = &ItemStruct> {
        self.modules.values().flat_map(|module| module.structs())
    }

    /// Find structs that have a specific derive attribute (e.g., "LightAccounts").
    pub fn structs_with_derive(&self, derive_name: &str) -> Vec<&ItemStruct> {
        self.structs()
            .filter(|s| has_derive_attribute(&s.attrs, derive_name))
            .collect()
    }

    /// Get the field names of a struct by its type.
    ///
    /// The type can be a simple identifier (e.g., "SinglePubkeyRecord") or
    /// a qualified path. Returns None if the struct is not found.
    pub fn get_struct_fields(&self, type_name: &syn::Type) -> Option<Vec<String>> {
        // Extract the struct name from the type path
        let struct_name = match type_name {
            syn::Type::Path(type_path) => type_path.path.segments.last()?.ident.to_string(),
            _ => return None,
        };

        // Find the struct by name
        for item_struct in self.structs() {
            if item_struct.ident == struct_name {
                // Extract field names from the struct
                if let syn::Fields::Named(named_fields) = &item_struct.fields {
                    let field_names: Vec<String> = named_fields
                        .named
                        .iter()
                        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
                        .collect();
                    return Some(field_names);
                }
            }
        }
        None
    }
}

/// A parsed module containing its items.
pub struct ParsedModule {
    /// All items in the module
    items: Vec<Item>,
}

impl ParsedModule {
    /// Recursively parse all modules starting from a root file.
    fn parse_recursive(
        root: &Path,
        module_path: &str,
    ) -> syn::Result<BTreeMap<String, ParsedModule>> {
        let mut modules = BTreeMap::new();

        // Read and parse the root file
        let content = std::fs::read_to_string(root).map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Failed to read {:?}: {}", root, e),
            )
        })?;

        let file: syn::File = syn::parse_str(&content).map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("Failed to parse {:?}: {}", root, e),
            )
        })?;

        let root_dir = root.parent().unwrap_or(Path::new("."));
        let root_name = root.file_stem().and_then(|s| s.to_str()).unwrap_or("root");

        // Create the root module
        let root_module = ParsedModule {
            items: file.items.clone(),
        };
        modules.insert(module_path.to_string(), root_module);

        // Process each item for nested modules
        for item in &file.items {
            if let Item::Mod(item_mod) = item {
                let mod_name = item_mod.ident.to_string();
                let child_path = format!("{}::{}", module_path, mod_name);

                if let Some((_, items)) = &item_mod.content {
                    // Inline module: mod foo { ... }
                    let inline_module = ParsedModule {
                        items: items.clone(),
                    };
                    modules.insert(child_path, inline_module);
                } else {
                    // External module: mod foo; - need to find the file
                    if let Some(mod_file) = find_module_file(root_dir, root_name, &mod_name) {
                        // Recursively parse the external module
                        let child_modules = Self::parse_recursive(&mod_file, &child_path)?;
                        modules.extend(child_modules);
                    }
                    // If file not found, silently skip (might be a cfg'd out module)
                }
            }
        }

        Ok(modules)
    }

    /// Get all struct items in this module.
    fn structs(&self) -> impl Iterator<Item = &ItemStruct> {
        self.items.iter().filter_map(|item| {
            if let Item::Struct(s) = item {
                Some(s)
            } else {
                None
            }
        })
    }
}

/// Find the file for an external module declaration.
///
/// Tries multiple paths following Rust module resolution:
/// - sibling_dir/mod_name.rs
/// - sibling_dir/mod_name/mod.rs
/// - parent_mod/mod_name.rs (if parent is a mod.rs)
/// - parent_mod/mod_name/mod.rs (if parent is a mod.rs)
fn find_module_file(parent_dir: &Path, parent_name: &str, mod_name: &str) -> Option<PathBuf> {
    // Standard paths relative to parent directory
    let paths = vec![
        // sibling file: parent_dir/mod_name.rs
        parent_dir.join(format!("{}.rs", mod_name)),
        // directory module: parent_dir/mod_name/mod.rs
        parent_dir.join(mod_name).join("mod.rs"),
    ];

    // If parent is mod.rs or lib.rs, also check parent_name directory
    if parent_name == "mod" || parent_name == "lib" {
        for path in &paths {
            if path.exists() {
                return Some(path.clone());
            }
        }
    } else {
        // Parent is a regular file like foo.rs, check foo/mod_name.rs
        let parent_mod_dir = parent_dir.join(parent_name);
        let extra_paths = [
            parent_mod_dir.join(format!("{}.rs", mod_name)),
            parent_mod_dir.join(mod_name).join("mod.rs"),
        ];

        for path in paths.iter().chain(extra_paths.iter()) {
            if path.exists() {
                return Some(path.clone());
            }
        }
    }

    // Check standard paths
    for path in &paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

/// Check if a struct has a specific derive attribute.
pub(crate) fn has_derive_attribute(attrs: &[syn::Attribute], derive_name: &str) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }

        // Parse the derive contents
        if let Ok(nested) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
        ) {
            for path in nested {
                // Check simple ident: #[derive(LightAccounts)]
                if let Some(ident) = path.get_ident() {
                    if ident == derive_name {
                        return true;
                    }
                }
                // Check path: #[derive(light_sdk::LightAccounts)]
                if let Some(segment) = path.segments.last() {
                    if segment.ident == derive_name {
                        return true;
                    }
                }
            }
        }
    }
    false
}
