use std::{env, fs, path::PathBuf};

fn main() {
    let spec_path = PathBuf::from("../../external/photon/src/openapi/specs/api.yaml");

    // Re-run if spec changes
    println!("cargo::rerun-if-changed={}", spec_path.display());

    // Read and parse the OpenAPI spec
    let spec_content = fs::read_to_string(&spec_path)
        .expect("Failed to read OpenAPI spec");

    let mut spec: serde_yaml::Value = serde_yaml::from_str(&spec_content)
        .expect("Failed to parse OpenAPI spec");

    // Add operationIds to each path's operation
    if let Some(paths) = spec.get_mut("paths").and_then(|p| p.as_mapping_mut()) {
        for (path, methods) in paths.iter_mut() {
            let path_str = path.as_str().unwrap_or("");
            // Convert path like "/getCompressedAccount" to operation id
            let base_id = path_str.trim_start_matches('/');

            if let Some(methods_map) = methods.as_mapping_mut() {
                for (method, operation) in methods_map.iter_mut() {
                    // Skip summary field
                    if method.as_str() == Some("summary") {
                        continue;
                    }

                    if let Some(op_map) = operation.as_mapping_mut() {
                        let method_str = method.as_str().unwrap_or("get");
                        // Create operation ID from path
                        let operation_id = format!("{}_{}", method_str, to_snake_case(base_id));

                        op_map.insert(
                            serde_yaml::Value::String("operationId".to_string()),
                            serde_yaml::Value::String(operation_id),
                        );
                    }
                }
            }
        }
    }

    // Write modified spec to OUT_DIR
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = PathBuf::from(&out_dir).join("api.yaml");
    let modified_spec = serde_yaml::to_string(&spec)
        .expect("Failed to serialize modified spec");
    fs::write(&out_path, &modified_spec)
        .expect("Failed to write modified spec");

    // Parse the modified spec for progenitor
    let spec: openapiv3::OpenAPI = serde_yaml::from_str(&modified_spec)
        .expect("Failed to parse modified spec as OpenAPI");

    // Generate the client code using progenitor
    let mut settings = progenitor::GenerationSettings::default();
    settings.with_interface(progenitor::InterfaceStyle::Builder);

    let mut generator = progenitor::Generator::new(&settings);
    let tokens = generator.generate_tokens(&spec)
        .expect("Failed to generate client code");

    // Format the generated code
    let ast: syn::File = syn::parse2(tokens)
        .expect("Failed to parse generated code");
    let content = prettyplease::unparse(&ast);

    let dest_path = PathBuf::from(&out_dir).join("codegen.rs");
    fs::write(&dest_path, content)
        .expect("Failed to write generated code");
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;

    for c in s.chars() {
        if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.extend(c.to_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = c.is_lowercase();
        }
    }

    result
}
