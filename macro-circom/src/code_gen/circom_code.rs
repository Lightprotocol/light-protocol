use super::utxo_type;
use crate::{
    code_gen::{circom_main_code::DISCLAIMER_STRING, connecting_hash_circom_template},
    errors::MacroCircomError::{self, LightTransactionUndefined},
    Instance, Utxo,
};

pub fn generate_psp_circom_code(
    input: &str,
    checked_utxos: &Vec<Utxo>,
    instance: &mut Instance,
    utxo_types: &Vec<utxo_type::UtxoType>,
) -> Result<(String, String), MacroCircomError> {
    let mut found_bracket = false;
    let mut found_bracket_once = false;
    let mut remaining_lines = Vec::new();
    let mut found_instance = false;
    let mut verifier_name = String::new();
    let mut checked_utxos = checked_utxos.clone();
    let mut skip_closign_brackets = 0;

    for line in input.lines() {
        let line = line.trim();
        // skip macro code
        if line.starts_with("#[instance")
            || line.starts_with("utxoType")
            || line.starts_with("inUtxo")
            || line.starts_with("outUtxo")
        {
            skip_closign_brackets = 1;
            continue;
        }
        if line.starts_with("dataChecks") || line.starts_with("checks") {
            skip_closign_brackets += 1;
            continue;
        }

        if skip_closign_brackets > 0 {
            if line.starts_with('}') {
                skip_closign_brackets -= 1;
            }
            continue;
        }

        if line.starts_with("#[entrypoint]") {
            if found_instance {
                panic!("A light transaction can only have one #[entrypoint].");
            };
            found_instance = true;
            verifier_name = String::from("verifierTwo");
            found_bracket = true;
            found_bracket_once = true;
            continue;
        }

        if found_bracket_once {
            if let Some(index) = checked_utxos.iter().position(|utxo| {
                line.starts_with(&"utxo".to_string()) && line.contains(&utxo.name.to_string())
            }) {
                if checked_utxos[index].is_declared {
                    panic!("A utxo can only be declared once.");
                }
                remaining_lines.push(checked_utxos[index].declare_code.clone());
                checked_utxos[index].is_declared = true;
                continue;
            }

            if let Some(index) = checked_utxos.iter().position(|utxo| {
                line.contains(&format!("{}.check();", utxo.name)) && utxo.checks.is_some()
            }) {
                if !checked_utxos[index].is_declared {
                    panic!("A utxo needs to be declared before it can be checked.");
                }
                if checked_utxos[index].is_checked {
                    panic!("A utxo can only be checked once.");
                }
                remaining_lines.push(checked_utxos[index].check_code.clone());
                checked_utxos[index].is_checked = true;
                continue;
            }
            if line.contains(".check()") {
                continue;
            }
        }
        if !found_bracket {
            remaining_lines.push(line.to_string());
        }
        if found_bracket && line.starts_with("template") {
            instance.template_name = extract_template_name(line);
            let to_insert = &format!("{} levels, nIns, nOuts, feeAsset, indexFeeAsset, indexPublicAsset, nAssets, nInAssets, nOutAssets", if instance.template_constants.is_none() || instance.template_constants.as_ref().unwrap().is_empty() { "" } else { "," });
            remaining_lines.push(insert_string_before_parenthesis(line, to_insert));
            remaining_lines
                .push(connecting_hash_circom_template::CONNECTING_HASH_VERIFIER_TWO.to_string());
            found_bracket = false;
        }
    }

    if !found_instance {
        return Err(LightTransactionUndefined);
    }
    remaining_lines.insert(0, DISCLAIMER_STRING.to_string());
    for utxo_type in utxo_types.iter() {
        remaining_lines.push(utxo_type.code.clone());
    }

    for utxo in checked_utxos.iter() {
        if !utxo.is_declared {
            return Err(MacroCircomError::CheckUtxoNotDeclared(utxo.name.clone()));
        }
        if !utxo.is_checked && utxo.checks.is_some() {
            return Err(MacroCircomError::CheckUtxoNotChecked(utxo.name.clone()));
        }
    }
    let remaining_lines = squash_empty_lines(&remaining_lines.join("\n"));
    let remaining_lines = format_custom_data(&remaining_lines);

    Ok((verifier_name, remaining_lines))
}

fn extract_template_name(input: &str) -> Option<String> {
    let start = input.find("template ")? + "template ".len();
    let end = input.find('(')?;

    Some(input[start..end].trim().to_string())
}

fn insert_string_before_parenthesis(input: &str, to_insert: &str) -> String {
    let closing_parenthesis_index = input.find(')').unwrap();
    let mut result = input[0..closing_parenthesis_index].to_string();
    result.push_str(to_insert);
    result.push_str(&input[closing_parenthesis_index..]);
    result
}

// Function to squash multiple empty lines into a single empty line
pub fn squash_empty_lines(input: &str) -> String {
    let mut result = String::new();
    let mut prev_line_empty = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_line_empty {
                result.push('\n');
            }
            prev_line_empty = true;
        } else {
            result.push_str(line);
            result.push('\n');
            prev_line_empty = false;
        }
    }

    result
}

pub fn format_custom_data(input: &str) -> String {
    let mut result = String::new();
    let mut indent_level = 0;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.contains('{') {
            result.push_str(&"\t".repeat(indent_level));
            result.push_str(trimmed);
            result.push('\n');
            indent_level += 1;
        } else if trimmed.contains('}') {
            indent_level -= 1;
            result.push_str(&"\t".repeat(indent_level));
            result.push_str(trimmed);
            result.push('\n');
        } else {
            result.push_str(&"\t".repeat(indent_level));
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result
}
#[cfg(test)]
mod light_transaction_tests {
    use std::{process::Command, vec};

    use super::*;
    use crate::utils::{create_file, open_file};
    #[allow(unused_imports)]
    use crate::{
        code_gen::{
            check_utxo_code::{assign_utxo_type, generate_check_utxo_code},
            utxo_type::{self, generate_utxo_type_code, UtxoType},
        },
        preprocess, Instance,
    };

    #[test]
    fn test_extract_template_name() {
        let input = "template AppTransaction(";
        let expected = Some("AppTransaction".to_string());
        assert_eq!(expected, extract_template_name(input));

        let input = "template  AnotherTemplate \n(";
        let expected = Some("AnotherTemplate".to_string());
        assert_eq!(expected, extract_template_name(input));

        let input = "invalid format(";
        let expected: Option<String> = None;
        assert_eq!(expected, extract_template_name(input));

        let input = "template MissingParenthesis";
        let expected: Option<String> = None;
        assert_eq!(expected, extract_template_name(input));
    }

    #[test]
    fn test_parse_light_transaction_light_transaction_undefined() {
        let mut utxo_types = vec![utxo_type::UtxoType {
            name: "typeName".to_string(),
            code: "".to_string(),
            fields: vec![String::from("x"), String::from("y")],
        }];
        let input = String::from("no #[entrypoint!] keyword");
        let mut instance = Instance {
            name: String::from("name"),
            template_name: None,
            template_constants: None,
            public_inputs: vec![],
        };

        let result =
            generate_psp_circom_code(&input, &Vec::<Utxo>::new(), &mut instance, &mut utxo_types);
        assert_eq!(result, Err(LightTransactionUndefined));
    }

    #[test]
    #[should_panic]
    fn test_parse_light_transaction_double_declaration() {
        let utxo_types = vec![utxo_type::UtxoType {
            name: "typeName".to_string(),
            code: "".to_string(),
            fields: vec![String::from("x"), String::from("y")],
        }];
        let input = String::from("#[entrypoint] { ... } \n #[entrypoint] { ... }");
        let mut instance = Instance {
            name: String::from("name"),
            template_name: None,
            template_constants: None,
            public_inputs: vec![],
        };

        let _ = generate_psp_circom_code(&input, &Vec::<Utxo>::new(), &mut instance, &utxo_types);
    }

    #[test]
    fn test_parse_light_transaction_functional() {
        let file_path = "./test-files/test-data/test_data.light";
        let input = open_file(file_path).unwrap();
        let input = match preprocess(&input, 0usize, false) {
            Ok(res) => res,
            Err(_) => {
                println!("Preprocessing error.");
                panic!("Preprocessing error");
            }
        };
        let mut instance = Instance {
            name: String::from("name"),
            template_name: None,
            template_constants: None,
            public_inputs: vec![],
        };

        let mut utxo_types = vec![utxo_type::UtxoType {
            name: "typeName".to_string(),
            code: "".to_string(),
            fields: vec![String::from("x"), String::from("y")],
        }];
        generate_utxo_type_code(&mut utxo_types).unwrap();
        let mut checked_utxo = vec![Utxo::default()];
        checked_utxo[0].name = "checkedProgramUtxo".to_string();
        checked_utxo[0].declare_code = "signal input x;\n signal input y;\n".to_string();
        checked_utxo[0].type_name = "typeName".to_string();
        checked_utxo[0].type_struct = Some(utxo_types[0].clone());
        checked_utxo[0].no_utxos = 1.to_string();
        generate_check_utxo_code(&mut checked_utxo).unwrap();

        let (verifier_name, code) =
            generate_psp_circom_code(&input, &checked_utxo, &mut instance, &utxo_types).unwrap();

        println!("{}", verifier_name);
        println!("{}", code);
        let name = "../target/test_data.circom";
        create_file(name, &code).unwrap();

        let name = "../target/test_data_main.circom";

        let main_file_code = "/**
        * This file is auto-generated by the Light cli.
        * DO NOT EDIT MANUALLY.
        * THE FILE WILL BE OVERWRITTEN EVERY TIME THE LIGHT CLI BUILD IS RUN.
        */
        pragma circom 2.1.4;
        include \"./test_data.circom\";
        component main {public [publicZ, publicTransactionHash, publicProgramId]} =  TestData( 18, 4, 4, 184598798020101492503359154328231866914977581098629757339001774613643340069, 0, 1, 3, 2, 2);";
        create_file(name, &main_file_code).unwrap();

        let command_output = Command::new("circom")
            .args(&[
                "-l",
                "../circuit-lib/circuit-lib.circom/node_modules/circomlib/circuits/",
                "-l",
                "../circuit-lib/circuit-lib.circom/src/merkle-tree/",
                "../target/test_data_main.circom",
                "-l",
                "../circuit-lib/circuit-lib.circom/src/transaction-utils/",
                "-l",
                "../circuit-lib/circuit-lib.circom/src/transaction/",
            ])
            .output()
            .expect("Failed to execute command");
        if !command_output.status.success() {
            let stderr = String::from_utf8_lossy(&command_output.stderr);
            println!("Command output (stderr):\n{}", stderr);
            std::process::exit(1);
        }
    }
}
