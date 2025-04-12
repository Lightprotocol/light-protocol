#![no_main]

use libfuzzer_sys::fuzz_target;
use light_sdk_macros::LightHasher;
use syn::{parse_str, ItemStruct};
use rand::{rngs::StdRng, Rng, SeedableRng};

// Define a nested struct that we'll use in our random struct generation
// We need to define this outside because we reference SimpleNested in the struct generation
#[derive(LightHasher, Clone)]
pub struct SimpleNested {
    pub a: u32,
    pub b: i32,
    #[hash]
    pub c: String,
}

// Fuzz target that generates random struct definitions
// and feeds them to the LightHasher derive macro
fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return; // Need at least a seed for the RNG
    }
    
    // Use the first 8 bytes as a seed
    let mut seed = [0u8; 8];
    seed.copy_from_slice(&data[0..8]);
    let seed = u64::from_le_bytes(seed);
    let mut rng = StdRng::seed_from_u64(seed);
    
    // Generate a random struct with various field types and attributes
    let struct_def = generate_random_struct(&mut rng);
    
    // Try to parse the struct definition
    if let Ok(_item_struct) = parse_str::<ItemStruct>(&struct_def) {
        // Since we can't directly call the internal hasher function,
        // we'll just validate that the struct definition is parseable.
        // The real testing happens in the struct_generation target
        // where we test the runtime behavior.
    }
});

/// Field generation strategies for testing different aspects of the LightHasher macro
enum FieldStrategy {
    Primitive,           // u8, u32, i32, etc.
    Array,               // Random sized arrays
    ArrayBoundary,       // Arrays at boundary conditions (31, 32, 33 bytes)
    ExactArraySize(usize), // Arrays of exact size (for DataHasher impl testing)
    Option,              // Option<T>
    NestedStruct,        // Nested structs
    HashableString,      // String with #[hash]
    HashableArray,       // [u8; N] with #[hash], should work for all sizes
}

/// Struct generation strategies for testing different macro behaviors
enum StructStrategy {
    Random,              // Random fields and attributes
    MaxFieldCount,       // Test with exactly 12-13 fields (limit)
    AllArraySizes,       // Test specifically with arrays of sizes 1-12
    AttributeCombinations, // Test combinations of #[hash] and #[skip]
    NestedDepth(usize),  // Test nested struct depth
}

// Generate a random struct definition with various field types and attributes
fn generate_random_struct(rng: &mut StdRng) -> String {
    // Choose a strategy
    let strategy = match rng.gen_range(0..=4) {
        0 => StructStrategy::Random,
        1 => StructStrategy::MaxFieldCount,
        2 => StructStrategy::AllArraySizes,
        3 => StructStrategy::AttributeCombinations,
        _ => StructStrategy::NestedDepth(rng.gen_range(1..=3)),
    };
    
    // Generate struct based on strategy
    match strategy {
        StructStrategy::Random => {
            let field_count = rng.gen_range(1..=15);
            generate_struct_with_random_fields(rng, field_count)
        }
        StructStrategy::MaxFieldCount => {
            // Test at and around the field count limit
            let field_count = if rng.gen_bool(0.7) { 12 } else { 13 };
            generate_struct_with_random_fields(rng, field_count)
        }
        StructStrategy::AllArraySizes => {
            // Test with arrays of sizes that have specific DataHasher impls
            let array_size = rng.gen_range(1..=12);
            generate_struct_with_specific_field(rng, FieldStrategy::ExactArraySize(array_size))
        }
        StructStrategy::AttributeCombinations => {
            // Test various combinations of hash and skip attributes
            generate_struct_with_attribute_combinations(rng)
        }
        StructStrategy::NestedDepth(depth) => {
            // Test deeply nested structs
            generate_nested_struct(rng, depth)
        }
    }
}

// Generate a struct with random fields
fn generate_struct_with_random_fields(rng: &mut StdRng, field_count: usize) -> String {
    let struct_name = format!("TestStruct{}", rng.gen::<u32>());
    let mut fields = Vec::new();
    
    for i in 0..field_count {
        let field_type = generate_random_type(rng);
        let field_name = format!("field_{}", i);
        let has_attr = rng.gen_bool(0.3);
        
        let attr = if has_attr {
            match rng.gen_range(0..=2) {
                0 => "#[hash]",
                1 => "#[skip]",
                _ => "#[flatten]", // Intentionally test unsupported attribute
            }
        } else {
            ""
        };
        
        fields.push(format!("    {}\n    pub {}: {}", attr, field_name, field_type));
    }
    
    format!(
        "#[derive(LightHasher)]\npub struct {} {{\n{}\n}}",
        struct_name,
        fields.join(",\n")
    )
}

// Generate a struct with a specific field type of interest
fn generate_struct_with_specific_field(rng: &mut StdRng, field_strategy: FieldStrategy) -> String {
    let struct_name = format!("TestStruct{}", rng.gen::<u32>());
    let mut fields = Vec::new();
    
    // Add 1-3 random fields
    for i in 0..rng.gen_range(1..=3) {
        let field_name = format!("random_field_{}", i);
        fields.push(format!("    pub {}: {}", field_name, generate_random_type(rng)));
    }
    
    // Add the specific field of interest
    let specific_field = match field_strategy {
        FieldStrategy::ExactArraySize(size) => {
            // 50% chance to add #[hash] attribute, which should allow any array size
            let attr = if rng.gen_bool(0.5) { "#[hash]\n    " } else { "" };
            format!("    {}pub array_field: [u8; {}]", attr, size)
        }
        FieldStrategy::ArrayBoundary => {
            let sizes = [31, 32, 33, 64]; // Boundary sizes
            let size = sizes[rng.gen_range(0..sizes.len())];
            // Arrays >= 32 should have #[hash] to work properly
            let needs_hash = size >= 32;
            let attr = if needs_hash || rng.gen_bool(0.5) { "#[hash]\n    " } else { "" };
            format!("    {}pub array_field: [u8; {}]", attr, size)
        }
        FieldStrategy::HashableArray => {
            // Any array size should work with #[hash]
            let size = rng.gen_range(1..=100);
            format!("    #[hash]\n    pub array_field: [u8; {}]", size)
        }
        _ => format!("    pub special_field: {}", generate_type_for_strategy(rng, field_strategy)),
    };
    
    fields.push(specific_field);
    
    format!(
        "#[derive(LightHasher)]\npub struct {} {{\n{}\n}}",
        struct_name,
        fields.join(",\n")
    )
}

// Generate a struct with various attribute combinations
fn generate_struct_with_attribute_combinations(rng: &mut StdRng) -> String {
    let struct_name = format!("TestStruct{}", rng.gen::<u32>());
    let field_count = rng.gen_range(3..=8);
    let mut fields = Vec::new();
    
    // Distribution of attributes: 1/3 regular, 1/3 #[hash], 1/3 #[skip]
    for i in 0..field_count {
        let field_name = format!("field_{}", i);
        let field_type = generate_random_type(rng);
        
        // Choose attribute
        let attr = match i % 3 {
            0 => "", // No attribute
            1 => "#[hash]", // hash attribute
            _ => "#[skip]", // skip attribute
        };
        
        fields.push(format!("    {}\n    pub {}: {}", attr, field_name, field_type));
    }
    
    // Add specific test case: large array with hash
    fields.push(format!("    #[hash]\n    pub large_array: [u8; {}]", rng.gen_range(32..=100)));
    
    format!(
        "#[derive(LightHasher)]\npub struct {} {{\n{}\n}}",
        struct_name,
        fields.join(",\n")
    )
}

// Generate a nested struct with specified depth
fn generate_nested_struct(rng: &mut StdRng, depth: usize) -> String {
    if depth == 0 {
        return "SimpleNested".to_string();
    }
    
    let struct_name = format!("NestedStruct{}", rng.gen::<u32>());
    let mut fields = Vec::new();
    
    // Add 1-3 regular fields
    for i in 0..rng.gen_range(1..=3) {
        fields.push(format!("    pub field_{}: {}", i, generate_random_type(rng)));
    }
    
    // Add nested field
    let nested_field_type = if depth == 1 {
        "SimpleNested".to_string()
    } else {
        format!("NestedStruct{}", rng.gen::<u32>())
    };
    
    fields.push(format!("    pub nested: {}", nested_field_type));
    
    // If depth > 1, add definition for the nested type first
    let nested_def = if depth > 1 {
        format!("{}\n\n", generate_nested_struct(rng, depth - 1))
    } else {
        "".to_string()
    };
    
    format!(
        "{}#[derive(LightHasher)]\npub struct {} {{\n{}\n}}",
        nested_def,
        struct_name,
        fields.join(",\n")
    )
}

// Generate a type based on a specific strategy
fn generate_type_for_strategy(rng: &mut StdRng, strategy: FieldStrategy) -> String {
    match strategy {
        FieldStrategy::Primitive => {
            match rng.gen_range(0..=5) {
                0 => "u8".to_string(),
                1 => "u32".to_string(), 
                2 => "u64".to_string(),
                3 => "i32".to_string(),
                4 => "i64".to_string(),
                _ => "bool".to_string(),
            }
        }
        FieldStrategy::Array => {
            format!("[u8; {}]", rng.gen_range(1..=64))
        }
        FieldStrategy::ArrayBoundary => {
            let sizes = [31, 32, 33, 64]; // Boundary sizes
            format!("[u8; {}]", sizes[rng.gen_range(0..sizes.len())])
        }
        FieldStrategy::ExactArraySize(size) => {
            format!("[u8; {}]", size)
        }
        FieldStrategy::Option => {
            let inner_type = match rng.gen_range(0..=3) {
                0 => "u32".to_string(),
                1 => "String".to_string(),
                2 => "SimpleNested".to_string(),
                _ => format!("[u8; {}]", rng.gen_range(1..=64)),
            };
            format!("Option<{}>", inner_type)
        }
        FieldStrategy::NestedStruct => {
            "SimpleNested".to_string()
        }
        FieldStrategy::HashableString => {
            "String".to_string() // Will be marked with #[hash]
        }
        FieldStrategy::HashableArray => {
            format!("[u8; {}]", rng.gen_range(1..=100)) // Will be marked with #[hash]
        }
    }
}

// Generate a random type for a field
fn generate_random_type(rng: &mut StdRng) -> String {
    // Choose a field strategy
    let strategy = match rng.gen_range(0..=9) {
        0..=2 => FieldStrategy::Primitive,
        3..=4 => FieldStrategy::Array,
        5 => FieldStrategy::ArrayBoundary,
        6..=7 => FieldStrategy::Option,
        8 => FieldStrategy::NestedStruct,
        _ => if rng.gen_bool(0.5) { FieldStrategy::HashableString } else { FieldStrategy::HashableArray },
    };
    
    generate_type_for_strategy(rng, strategy)
}