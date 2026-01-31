//! Randomized fuzz-style tests for seed parsing.
//!
//! These tests run as part of `cargo test -p light-sdk-macros` and exercise
//! the actual parsing functions with random inputs.

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use syn::parse_str;

    use crate::light_pdas::seeds::{
        anchor_extraction::extract_anchor_seeds, classification::classify_seed_expr,
        InstructionArgSet,
    };

    /// Generate a random seed expression string
    fn generate_random_seed_expr(rng: &mut StdRng) -> String {
        match rng.gen_range(0..=20) {
            // Byte string literals
            0 => "b\"seed\"".to_string(),
            1 => "b\"user\"".to_string(),
            2 => format!("b\"{}\"", random_string(rng, 1, 20)),

            // Byte string with slice
            3 => "b\"seed\"[..]".to_string(),
            4 => format!("b\"{}\"[..]", random_string(rng, 1, 10)),

            // Constants (uppercase)
            5 => "SEED_CONSTANT".to_string(),
            6 => "VAULT_PREFIX".to_string(),
            7 => "crate::SEED_PREFIX".to_string(),
            8 => "module::nested::CONSTANT".to_string(),

            // Account key access
            9 => "fee_payer.key().as_ref()".to_string(),
            10 => "authority.key().as_ref()".to_string(),
            11 => format!("field_{}.key().as_ref()", rng.gen_range(0..10)),

            // Instruction arg field access
            12 => "params.owner.as_ref()".to_string(),
            13 => "data.owner.as_ref()".to_string(),
            14 => "args.value.as_ref()".to_string(),

            // Nested field access
            15 => "params.nested.field.as_ref()".to_string(),
            16 => "data.inner.key.as_ref()".to_string(),
            17 => "params.deep.nested.value.as_ref()".to_string(),

            // to_le_bytes conversion
            18 => "params.amount.to_le_bytes().as_ref()".to_string(),
            19 => "amount.to_le_bytes().as_ref()".to_string(),

            // Array indexing
            _ => format!("params.arrays[{}]", rng.gen_range(0..10)),
        }
    }

    /// Generate random instruction args
    fn generate_random_instruction_args(rng: &mut StdRng) -> InstructionArgSet {
        let possible_args = ["params", "data", "args", "input", "owner", "amount", "bump"];
        let count = rng.gen_range(0..=3);
        let names: Vec<String> = (0..count)
            .map(|_| possible_args[rng.gen_range(0..possible_args.len())].to_string())
            .collect();
        InstructionArgSet::from_names(names)
    }

    /// Generate random string
    fn random_string(rng: &mut StdRng, min_len: usize, max_len: usize) -> String {
        let len = rng.gen_range(min_len..=max_len);
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz_0123456789".chars().collect();
        (0..len)
            .map(|_| chars[rng.gen_range(0..chars.len())])
            .collect()
    }

    /// Fuzz test for classify_seed_expr - runs many random inputs
    #[test]
    fn fuzz_classify_seed_expr() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);

        for iteration in 0..10_000 {
            let expr_str = generate_random_seed_expr(&mut rng);
            let args = generate_random_instruction_args(&mut rng);

            // Try to parse as expression
            if let Ok(expr) = parse_str::<syn::Expr>(&expr_str) {
                // This should not panic - errors are fine, panics are not
                let result = classify_seed_expr(&expr, &args);

                // Verify result is consistent
                if result.is_ok() {
                    let result2 = classify_seed_expr(&expr, &args);
                    assert!(
                        result2.is_ok(),
                        "classify_seed_expr not deterministic at iteration {}",
                        iteration
                    );
                }
            }
        }
    }

    /// Fuzz test with malformed/edge-case expressions
    #[test]
    fn fuzz_classify_seed_expr_edge_cases() {
        let mut rng = StdRng::seed_from_u64(0xCAFEBABE);

        let edge_cases = vec![
            // Empty and minimal
            "",
            "a",
            "_",
            // Deeply nested
            "a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p",
            "a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p.as_ref()",
            // Method chains
            "x.key().as_ref().as_bytes()",
            "x.to_le_bytes().to_be_bytes()",
            // References
            "&x",
            "&&x",
            "&x.key()",
            "&params.owner.as_ref()",
            // Indexing edge cases
            "arr[0]",
            "arr[999999]",
            "params.arr[0][1][2]",
            "b\"seed\"[0..2]",
            "b\"seed\"[..]",
            "b\"seed\"[1..]",
            "b\"seed\"[..1]",
            // Function calls
            "max_key(&a.key(), &b.key())",
            "some_fn()",
            "some_fn(a, b, c, d, e)",
            // Mixed case identifiers (constant detection)
            "CONSTANT",
            "constant",
            "Constant",
            "CONSTANT_WITH_UNDERSCORE",
            "NOT_A_constant",
            "_UNDERSCORE_START",
            // Unicode (should fail gracefully)
            // Numeric literals (unsupported)
            "123",
            "0x1234",
            // Tuples (unsupported)
            "(a, b)",
            // Closures (unsupported)
            "|x| x",
            // Blocks (unsupported)
            "{ x }",
        ];

        for expr_str in &edge_cases {
            if let Ok(expr) = parse_str::<syn::Expr>(expr_str) {
                let args = generate_random_instruction_args(&mut rng);
                // Should not panic
                let _ = classify_seed_expr(&expr, &args);
            }
        }
    }

    /// Fuzz test for extract_anchor_seeds with random attributes
    #[test]
    fn fuzz_extract_anchor_seeds() {
        let mut rng = StdRng::seed_from_u64(0xBEEFCAFE);

        for _ in 0..10_000 {
            let seeds: Vec<String> = (0..rng.gen_range(1..=5))
                .map(|_| generate_random_seed_expr(&mut rng))
                .collect();
            let seeds_str = seeds.join(", ");

            // Create a struct with the attribute to parse
            let struct_str = format!(
                "struct Test {{ #[account(seeds = [{}], bump)] field: u8 }}",
                seeds_str
            );

            // Parse the struct and extract the attribute
            if let Ok(item) = syn::parse_str::<syn::ItemStruct>(&struct_str) {
                if let Some(field) = item.fields.iter().next() {
                    let args = generate_random_instruction_args(&mut rng);
                    // Should not panic
                    let _ = extract_anchor_seeds(&field.attrs, &args);
                }
            }
        }
    }

    /// Fuzz test with truly random byte strings - chaos monkey style
    #[test]
    fn fuzz_classify_seed_expr_random_bytes() {
        let mut rng = StdRng::seed_from_u64(0xCA0505);

        for _ in 0..10_000 {
            // Generate random length (1-100 bytes)
            let len = rng.gen_range(1..=100);

            // Generate completely random bytes
            let random_bytes: Vec<u8> = (0..len).map(|_| rng.gen::<u8>()).collect();

            // Try to interpret as UTF-8 string
            if let Ok(random_str) = String::from_utf8(random_bytes.clone()) {
                // Try to parse as expression
                if let Ok(expr) = parse_str::<syn::Expr>(&random_str) {
                    let args = generate_random_instruction_args(&mut rng);
                    // Should not panic - errors are fine
                    let _ = classify_seed_expr(&expr, &args);
                }
            }

            // Also try with printable ASCII subset for higher parse success rate
            let printable_bytes: Vec<u8> = (0..len)
                .map(|_| {
                    // ASCII printable range: 32-126, plus some Rust-relevant chars
                    rng.gen_range(32..=126) as u8
                })
                .collect();

            if let Ok(printable_str) = String::from_utf8(printable_bytes) {
                if let Ok(expr) = parse_str::<syn::Expr>(&printable_str) {
                    let args = generate_random_instruction_args(&mut rng);
                    let _ = classify_seed_expr(&expr, &args);
                }
            }
        }
    }

    /// Property test: valid expressions should produce consistent results
    #[test]
    fn property_classify_seed_expr_deterministic() {
        let valid_exprs = [
            "b\"seed\"",
            "CONSTANT",
            "params.owner.as_ref()",
            "authority.key().as_ref()",
            "params.amount.to_le_bytes().as_ref()",
            "b\"test\"[..]",
        ];

        let args = InstructionArgSet::from_names(vec!["params".to_string()]);

        for expr_str in &valid_exprs {
            let expr: syn::Expr = syn::parse_str(expr_str).unwrap();

            let result1 = classify_seed_expr(&expr, &args);
            let result2 = classify_seed_expr(&expr, &args);

            // Both should succeed or both should fail
            assert_eq!(
                result1.is_ok(),
                result2.is_ok(),
                "Non-deterministic for: {}",
                expr_str
            );
        }
    }
}
