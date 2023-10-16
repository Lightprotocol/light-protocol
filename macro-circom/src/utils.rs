use std::{
    env,
    fs::{self, File},
    io::{self, prelude::*},
    process::{Command, Stdio},
    thread::spawn,
};

use quote::ToTokens;

/// Asserts that two Rust code strings are equivalent by parsing them with `syn` and comparing the token streams.
pub fn assert_syn_eq(output: &str, expected_output: &str) {
    let parsed_output: syn::File = syn::parse_str(output).expect("Failed to parse expected output");
    let parsed_expected: syn::File =
        syn::parse_str(expected_output).expect("Failed to parse expected output");

    let output_tokens = parsed_output.into_token_stream().to_string();
    let expected_tokens = parsed_expected.into_token_stream().to_string();

    assert_eq!(output_tokens, expected_tokens);
}

pub fn describe_error(
    input: &str,
    error: lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token<'_>, &'_ str>,
) -> String {
    match error {
        lalrpop_util::ParseError::InvalidToken { location } => {
            let start = location.saturating_sub(10);
            let end = std::cmp::min(location + 10, input.len());
            format!(
                "Invalid token near: `{}`. Full context: `{}`",
                &input[location..location + 1],
                &input[start..end]
            )
        }
        lalrpop_util::ParseError::UnrecognizedToken {
            token: (start, token, end),
            expected,
        } => {
            let context_start = start.saturating_sub(50);
            let context_end = std::cmp::min(end + 10, input.len());
            format!(
                "Unrecognized token `{}` at position {}:{} within context `{}`. Expected one of: {:?}",
                token,
                start,
                end,
                &input[context_start..context_end],
                expected
            )
        }
        lalrpop_util::ParseError::UnrecognizedEof { location, expected } => {
            let context_start = location.saturating_sub(10);
            let context_end = std::cmp::min(location + 10, input.len());
            format!(
                "Unrecognized end of file at position {}. Context: `{}`. Expected one of: {:?}",
                location,
                &input[context_start..context_end],
                expected
            )
        }
        lalrpop_util::ParseError::ExtraToken {
            token: (start, token, end),
        } => {
            let context_start = start.saturating_sub(10);
            let context_end = std::cmp::min(end + 10, input.len());
            format!(
                "Extra token `{}` at position {}:{} within context `{}`.",
                token,
                start,
                end,
                &input[context_start..context_end]
            )
        }
        lalrpop_util::ParseError::User { error } => {
            format!("User-defined error: {}", error)
        }
    }
}

pub fn open_file(file_path: &str) -> Result<String, std::io::Error> {
    // Open the file
    let mut file = File::open(file_path).expect("Unable to open the file");

    // Read the file's content
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn create_file(path: &str, code: &str) -> Result<(), std::io::Error> {
    println!("Writing circom to file: {}", path);
    let mut output_file = fs::File::create(path)?;
    write!(&mut output_file, "{}", code)
}

pub fn write_rust_code_to_file(path: String, code: String) {
    let code = rustfmt(code).unwrap();
    println!("Writing rust to file: {}", path);
    let mut output_file_idl = fs::File::create(path).unwrap();
    output_file_idl.write(&code).unwrap();
}

#[allow(dead_code)]
pub fn rustfmt(code: String) -> Result<Vec<u8>, anyhow::Error> {
    let mut cmd = match env::var_os("RUSTFMT") {
        Some(r) => Command::new(r),
        None => Command::new("rustfmt"),
    };

    let mut cmd = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = cmd.stdin.take().unwrap();
    let mut stdout = cmd.stdout.take().unwrap();

    let stdin_handle = spawn(move || {
        stdin.write_all(code.as_bytes()).unwrap();
        // Manually flush and close the stdin handle
        stdin.flush().unwrap();
        drop(stdin);
    });

    let mut formatted_code = vec![];
    io::copy(&mut stdout, &mut formatted_code)?;

    let _ = cmd.wait();
    stdin_handle.join().unwrap();

    Ok(formatted_code)
}

#[allow(dead_code)]
pub fn remove_formatting(input: &str) -> String {
    let res: Vec<String> = input
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|ch| ch.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .collect();
    res.join("")
}

pub fn build_value(value: String, add: Option<String>) -> String {
    match add {
        Some(add) => vec![value, add].join("."),
        None => value.to_string(),
    }
}
