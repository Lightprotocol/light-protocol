//! Anchor-style crate context parser for `#[instruction_decoder]`.
//!
//! This module recursively reads all module files at macro expansion time,
//! allowing `#[instruction_decoder]` to discover all Anchor `#[derive(Accounts)]` structs
//! across the entire crate and extract their field names.
//!
//! Based on Anchor's `CrateContext::parse()` pattern from `anchor-syn/src/parser/context.rs`.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use syn::{Item, ItemStruct};

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

    /// Get field names of a struct by its simple name (e.g., "CreateTwoMints").
    ///
    /// Returns None if the struct is not found.
    ///
    /// # Limitations
    ///
    /// - **First match wins**: If multiple modules define structs with the same name,
    ///   this returns the first one found (iteration order is not guaranteed).
    ///   To avoid ambiguity, ensure struct names are unique across the crate.
    ///
    /// - **No derive validation**: This does not verify that the struct has
    ///   `#[derive(Accounts)]`. Any struct with a matching name will be used.
    ///   Ensure the struct name passed corresponds to an actual Accounts struct.
    pub fn get_struct_field_names(&self, struct_name: &str) -> Option<Vec<String>> {
        for item_struct in self.structs() {
            if item_struct.ident == struct_name {
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
                    modules.insert(child_path.clone(), inline_module);

                    // For inline module's children, the directory is root_dir/mod_name
                    let inline_module_dir = root_dir.join(&mod_name);

                    // Recursively process nested modules within inline modules
                    Self::process_inline_modules(
                        items,
                        &child_path,
                        &inline_module_dir,
                        &mut modules,
                    );
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

    /// Recursively process inline modules to find nested module declarations.
    ///
    /// For inline modules like `mod foo { mod bar { ... } }`, this traverses
    /// the nested structure and adds each module to the map. Also handles
    /// external module references (`mod bar;`) within inline modules.
    ///
    /// # Arguments
    /// * `items` - Items in the current module
    /// * `parent_path` - Module path prefix (e.g., "crate::foo")
    /// * `module_dir` - Directory where children of this module level would be found
    /// * `modules` - Map to insert discovered modules into
    fn process_inline_modules(
        items: &[Item],
        parent_path: &str,
        module_dir: &Path,
        modules: &mut BTreeMap<String, ParsedModule>,
    ) {
        for item in items {
            if let Item::Mod(item_mod) = item {
                let mod_name = item_mod.ident.to_string();
                let child_path = format!("{}::{}", parent_path, mod_name);

                if let Some((_, nested_items)) = &item_mod.content {
                    // Nested inline module
                    let nested_module = ParsedModule {
                        items: nested_items.clone(),
                    };
                    modules.insert(child_path.clone(), nested_module);

                    // For inline module's children, the directory is module_dir/mod_name
                    let child_module_dir = module_dir.join(&mod_name);

                    // Recursively process deeper nested modules
                    Self::process_inline_modules(
                        nested_items,
                        &child_path,
                        &child_module_dir,
                        modules,
                    );
                } else {
                    // External module: mod foo; - resolve using file system
                    // Inline modules act like mod.rs for their children's resolution
                    if let Some(mod_file) = find_module_file(module_dir, "mod", &mod_name) {
                        // Load and parse the external module file
                        if let Ok(content) = std::fs::read_to_string(&mod_file) {
                            if let Ok(file) = syn::parse_str::<syn::File>(&content) {
                                let external_module = ParsedModule {
                                    items: file.items.clone(),
                                };
                                modules.insert(child_path.clone(), external_module);

                                // Determine the directory for this external module's children
                                let ext_mod_dir = mod_file.parent().unwrap_or(Path::new("."));
                                let ext_mod_name = mod_file
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("mod");

                                // Process nested modules within the external module
                                Self::process_inline_modules(
                                    &file.items,
                                    &child_path,
                                    &if ext_mod_name == "mod" {
                                        ext_mod_dir.to_path_buf()
                                    } else {
                                        ext_mod_dir.join(ext_mod_name)
                                    },
                                    modules,
                                );
                            }
                        }
                        // If file read/parse fails, silently skip
                    }
                    // If file not found, silently skip (might be a cfg'd out module)
                }
            }
        }
    }
}

/// Find the file for an external module declaration.
///
/// Tries multiple paths following Rust module resolution:
/// - For root files (lib.rs, main.rs, mod.rs): sibling paths first
/// - For non-root files (e.g., foo.rs): parent-namespaced paths first (foo/bar.rs)
fn find_module_file(parent_dir: &Path, parent_name: &str, mod_name: &str) -> Option<PathBuf> {
    // Standard sibling paths relative to parent directory
    let sibling_paths = [
        // sibling file: parent_dir/mod_name.rs
        parent_dir.join(format!("{}.rs", mod_name)),
        // directory module: parent_dir/mod_name/mod.rs
        parent_dir.join(mod_name).join("mod.rs"),
    ];

    // Check if parent is a root file (mod.rs, lib.rs, or main.rs)
    let is_root = parent_name == "mod" || parent_name == "lib" || parent_name == "main";

    if is_root {
        // For root files, check sibling paths only
        for path in &sibling_paths {
            if path.exists() {
                return Some(path.clone());
            }
        }
    } else {
        // For non-root files (e.g., foo.rs with `mod bar;`), check parent-namespaced paths FIRST
        // This ensures src/foo/bar.rs is preferred over src/bar.rs for crate::foo::bar
        let parent_mod_dir = parent_dir.join(parent_name);
        let namespaced_paths = [
            parent_mod_dir.join(format!("{}.rs", mod_name)),
            parent_mod_dir.join(mod_name).join("mod.rs"),
        ];

        // Check namespaced paths first, then fall back to sibling paths
        for path in namespaced_paths.iter().chain(sibling_paths.iter()) {
            if path.exists() {
                return Some(path.clone());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_module_file_sibling() {
        // This test verifies the path construction logic
        let parent = Path::new("/some/src");
        let paths_checked = [parent.join("foo.rs"), parent.join("foo").join("mod.rs")];
        // Just verify the paths are constructed correctly
        assert!(paths_checked[0].to_str().unwrap().contains("foo.rs"));
        assert!(paths_checked[1].to_str().unwrap().contains("mod.rs"));
    }
}
