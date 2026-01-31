//! Transaction formatting utilities for explorer-style output

use std::{
    collections::HashMap,
    fmt::{self, Write},
};

use solana_pubkey::Pubkey;
use tabled::{Table, Tabled};

use crate::{
    config::{EnhancedLoggingConfig, LogVerbosity},
    types::{
        AccountAccess, AccountChange, AccountStateSnapshot, EnhancedInstructionLog,
        EnhancedTransactionLog, TransactionStatus,
    },
};

/// Format a number with thousands separators (e.g., 1000000 -> "1,000,000")
fn format_with_thousands_separator(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Format a signed number with thousands separators, preserving the sign
fn format_signed_with_thousands_separator(n: i64) -> String {
    if n >= 0 {
        format_with_thousands_separator(n as u64)
    } else {
        format!("-{}", format_with_thousands_separator(n.unsigned_abs()))
    }
}

/// Known test accounts and programs mapped to human-readable names
static KNOWN_ACCOUNTS: &[(&str, &str)] = &[
    // Test program
    (
        "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy",
        "test program",
    ),
    // V1 test accounts
    (
        "smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT",
        "v1 state merkle tree",
    ),
    (
        "nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148",
        "v1 nullifier queue",
    ),
    (
        "cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4",
        "v1 cpi context",
    ),
    (
        "amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2",
        "v1 address merkle tree",
    ),
    (
        "aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F",
        "v1 address queue",
    ),
    // V2 state trees (5 triples)
    (
        "bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU",
        "v2 state merkle tree 1",
    ),
    (
        "oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto",
        "v2 state output queue 1",
    ),
    (
        "cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y",
        "v2 cpi context 1",
    ),
    (
        "bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi",
        "v2 state merkle tree 2",
    ),
    (
        "oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg",
        "v2 state output queue 2",
    ),
    (
        "cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B",
        "v2 cpi context 2",
    ),
    (
        "bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb",
        "v2 state merkle tree 3",
    ),
    (
        "oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ",
        "v2 state output queue 3",
    ),
    (
        "cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf",
        "v2 cpi context 3",
    ),
    (
        "bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8",
        "v2 state merkle tree 4",
    ),
    (
        "oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq",
        "v2 state output queue 4",
    ),
    (
        "cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc",
        "v2 cpi context 4",
    ),
    (
        "bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2",
        "v2 state merkle tree 5",
    ),
    (
        "oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P",
        "v2 state output queue 5",
    ),
    (
        "cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6",
        "v2 cpi context 5",
    ),
    // V2 address tree
    (
        "amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx",
        "v2 address merkle tree",
    ),
    // CPI authorities
    (
        "HZH7qSLcpAeDqCopVU4e5XkhT9j3JFsQiq8CmruY3aru",
        "light system cpi authority",
    ),
    (
        "GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy",
        "light token cpi authority",
    ),
    // Rent sponsor
    (
        "r18WwUxfG8kQ69bQPAB2jV6zGNKy3GosFGctjQoV4ti",
        "rent sponsor",
    ),
    // Compressible config PDA
    (
        "ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg",
        "compressible config",
    ),
    // Registered program PDA
    (
        "35hkDgaAKwMCaxRz2ocSZ6NaUrtKkyNqU6c4RV3tYJRh",
        "registered program pda",
    ),
    // Config counter PDA
    (
        "8gH9tmziWsS8Wc4fnoN5ax3jsSumNYoRDuSBvmH2GMH8",
        "config counter pda",
    ),
    // Registered registry program PDA
    (
        "DumMsyvkaGJG4QnQ1BhTgvoRMXsgGxfpKDUCr22Xqu4w",
        "registered registry program pda",
    ),
    // Account compression authority PDA
    (
        "HwXnGK3tPkkVY6P439H2p68AxpeuWXd5PcrAxFpbmfbA",
        "account compression authority pda",
    ),
    // Sol pool PDA
    (
        "CHK57ywWSDncAoRu1F8QgwYJeXuAJyyBYT4LixLXvMZ1",
        "sol pool pda",
    ),
    // SPL Noop program
    (
        "noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV",
        "noop program",
    ),
    // Solana native programs
    ("11111111111111111111111111111111", "system program"),
    (
        "ComputeBudget111111111111111111111111111111",
        "compute budget program",
    ),
    (
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "token program",
    ),
    (
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
        "associated token program",
    ),
];

/// Row for account table display (4 columns - used for inner instructions)
#[derive(Tabled)]
struct AccountRow {
    #[tabled(rename = "#")]
    symbol: String,
    #[tabled(rename = "Account")]
    pubkey: String,
    #[tabled(rename = "Type")]
    access: String,
    #[tabled(rename = "Name")]
    name: String,
}

/// Row for outer instruction account table display (8 columns - includes account state)
#[derive(Tabled)]
struct OuterAccountRow {
    #[tabled(rename = "#")]
    symbol: String,
    #[tabled(rename = "Account")]
    pubkey: String,
    #[tabled(rename = "Type")]
    access: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Owner")]
    owner: String,
    #[tabled(rename = "Data Len")]
    data_len: String,
    #[tabled(rename = "Lamports")]
    lamports: String,
    #[tabled(rename = "Change")]
    lamports_change: String,
}

/// Colors for terminal output
#[derive(Debug, Clone, Default)]
pub struct Colors {
    pub bold: &'static str,
    pub reset: &'static str,
    pub green: &'static str,
    pub red: &'static str,
    pub yellow: &'static str,
    pub blue: &'static str,
    pub cyan: &'static str,
    pub gray: &'static str,
}

impl Colors {
    pub fn new(use_colors: bool) -> Self {
        if use_colors {
            Self {
                bold: "\x1b[1m",
                reset: "\x1b[0m",
                green: "\x1b[32m",
                red: "\x1b[31m",
                yellow: "\x1b[33m",
                blue: "\x1b[34m",
                cyan: "\x1b[36m",
                gray: "\x1b[90m",
            }
        } else {
            Self::default()
        }
    }
}

/// Transaction formatter with configurable output
pub struct TransactionFormatter {
    config: EnhancedLoggingConfig,
    colors: Colors,
}

impl TransactionFormatter {
    pub fn new(config: &EnhancedLoggingConfig) -> Self {
        Self {
            config: config.clone(),
            colors: Colors::new(config.use_colors),
        }
    }

    /// Apply line breaks to long values in the complete output
    fn apply_line_breaks(&self, text: &str) -> String {
        let mut result = String::new();

        for line in text.lines() {
            // Look for patterns that need line breaking
            if let Some(formatted_line) = self.format_line_if_needed(line) {
                result.push_str(&formatted_line);
            } else {
                result.push_str(line);
            }
            result.push('\n');
        }

        result
    }

    /// Format a line if it contains long values that need breaking
    fn format_line_if_needed(&self, line: &str) -> Option<String> {
        // Extract leading whitespace/indentation and table characters
        let leading_chars = line
            .chars()
            .take_while(|&c| c.is_whitespace() || "│├└┌┬┴┐┤─".contains(c))
            .collect::<String>();

        // Match patterns like "address: [0, 1, 2, 3, ...]" or "Raw instruction data (N bytes): [...]"
        if line.contains(": [") && line.contains("]") {
            // Handle byte arrays
            if let Some(start) = line.find(": [") {
                if let Some(end_pos) = line[start..].find(']') {
                    let end = start + end_pos;
                    let prefix = &line[..start + 2]; // Include ": "
                    let array_part = &line[start + 2..end + 1]; // The "[...]" part
                    let suffix = &line[end + 1..];

                    // For raw instruction data, use a shorter line length to better fit in terminal
                    let max_width = if line.contains("Raw instruction data") {
                        80 // Wider for raw instruction data to fit more numbers per line
                    } else {
                        50 // Keep existing width for other arrays
                    };

                    // Always format if it's raw instruction data or if it exceeds max_width
                    if line.contains("Raw instruction data") || array_part.len() > max_width {
                        let formatted_array = self.format_long_value_with_indent(
                            array_part,
                            max_width,
                            &leading_chars,
                        );
                        return Some(format!("{}{}{}", prefix, formatted_array, suffix));
                    }
                }
            }
        }

        // Handle long base58 strings (44+ characters) in table cells
        if line.contains('|') && !line.trim_start().starts_with('|') {
            // This is a table content line, not a border
            let mut new_line = String::new();
            let mut any_modified = false;

            // Split by table separators while preserving them
            let parts: Vec<&str> = line.split('|').collect();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    new_line.push('|');
                }

                // Check if this cell contains a long value
                let mut cell_modified = false;
                for word in part.split_whitespace() {
                    if word.len() > 44 && word.chars().all(|c| c.is_alphanumeric()) {
                        let indent = " ".repeat(leading_chars.len() + 2); // Extra space for table formatting
                        let formatted_word = self.format_long_value_with_indent(word, 44, &indent);
                        new_line.push_str(&part.replace(word, &formatted_word));
                        cell_modified = true;
                        any_modified = true;
                        break;
                    }
                }

                if !cell_modified {
                    new_line.push_str(part);
                }
            }

            if any_modified {
                return Some(new_line);
            }
        }

        None
    }

    /// Format long value with proper indentation for continuation lines
    fn format_long_value_with_indent(&self, value: &str, max_width: usize, indent: &str) -> String {
        if value.len() <= max_width {
            return value.to_string();
        }

        let mut result = String::new();

        // Handle byte arrays specially by breaking at natural comma boundaries when possible
        if value.starts_with('[') && value.ends_with(']') {
            // This is a byte array - try to break at comma boundaries for better readability
            let inner = &value[1..value.len() - 1]; // Remove [ and ]
            let parts: Vec<&str> = inner.split(", ").collect();

            result.push('[');
            let mut current_line = String::new();
            let mut first_line = true;

            for (i, part) in parts.iter().enumerate() {
                let addition = if i == 0 {
                    part.to_string()
                } else {
                    format!(", {}", part)
                };

                // Check if adding this part would exceed the line width
                if current_line.len() + addition.len() > max_width && !current_line.is_empty() {
                    // Add current line to result and start new line
                    if first_line {
                        result.push_str(&current_line);
                        first_line = false;
                    } else {
                        result.push_str(&format!("\n{}{}", indent, current_line));
                    }
                    // Use addition to preserve the ", " separator for non-first items
                    current_line = addition;
                } else {
                    current_line.push_str(&addition);
                }
            }

            // Add the last line
            if !current_line.is_empty() {
                if first_line {
                    result.push_str(&current_line);
                } else {
                    result.push_str(&format!("\n{}{}", indent, current_line));
                }
            }

            result.push(']');
        } else {
            // Fall back to character-based breaking for non-array values
            let chars = value.chars().collect::<Vec<char>>();
            let mut pos = 0;

            while pos < chars.len() {
                let end = (pos + max_width).min(chars.len());
                let chunk: String = chars[pos..end].iter().collect();

                if pos == 0 {
                    result.push_str(&chunk);
                } else {
                    result.push_str(&format!("\n{}{}", indent, chunk));
                }

                pos = end;
            }
        }

        result
    }

    /// Format complete transaction log
    pub fn format(&self, log: &EnhancedTransactionLog, tx_number: usize) -> String {
        let mut output = String::new();

        // Transaction box header with number (wide enough for signature + slot + status)
        writeln!(output, "{}┌──────────────────────────────────────────────────────────── Transaction #{} ─────────────────────────────────────────────────────────────┐{}", self.colors.gray, tx_number, self.colors.reset).expect("Failed to write box header");

        // Transaction header
        self.write_transaction_header(&mut output, log)
            .expect("Failed to write header");

        // Instructions section
        if !log.instructions.is_empty() {
            self.write_instructions_section(&mut output, log)
                .expect("Failed to write instructions");
        }

        // Account changes section
        if self.config.show_account_changes && !log.account_changes.is_empty() {
            self.write_account_changes_section(&mut output, log)
                .expect("Failed to write account changes");
        }

        // Light Protocol events section
        if !log.light_events.is_empty() {
            self.write_light_events_section(&mut output, log)
                .expect("Failed to write Light Protocol events");
        }

        // Program logs section (LiteSVM pretty logs)
        if !log.program_logs_pretty.trim().is_empty() {
            self.write_program_logs_section(&mut output, log)
                .expect("Failed to write program logs");
        }

        // Transaction box footer (matches header width)
        writeln!(output, "{}└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘{}", self.colors.gray, self.colors.reset).expect("Failed to write box footer");

        // Apply line breaks for long values in the complete output
        self.apply_line_breaks(&output)
    }

    /// Write transaction header with status, fee, and compute units
    fn write_transaction_header(
        &self,
        output: &mut String,
        log: &EnhancedTransactionLog,
    ) -> fmt::Result {
        writeln!(
            output,
            "{}│{} {}Transaction: {}{} | Slot: {} | Status: {}{}",
            self.colors.gray,
            self.colors.reset,
            self.colors.bold,
            self.colors.cyan,
            log.signature,
            log.slot,
            self.status_color(&log.status),
            log.status.text(),
        )?;

        writeln!(
            output,
            "{}│{} Fee: {}{:.6} SOL | Compute Used: {}{}/{} CU{}",
            self.colors.gray,
            self.colors.reset,
            self.colors.yellow,
            log.fee as f64 / 1_000_000_000.0,
            self.colors.blue,
            log.compute_used,
            log.compute_total,
            self.colors.reset
        )?;

        writeln!(output, "{}│{}", self.colors.gray, self.colors.reset)?;
        Ok(())
    }

    /// Write instructions hierarchy
    fn write_instructions_section(
        &self,
        output: &mut String,
        log: &EnhancedTransactionLog,
    ) -> fmt::Result {
        writeln!(
            output,
            "{}│{} {}Instructions ({}):{}",
            self.colors.gray,
            self.colors.reset,
            self.colors.bold,
            log.instructions.len(),
            self.colors.reset
        )?;
        writeln!(output, "{}│{}", self.colors.gray, self.colors.reset)?;

        for (i, instruction) in log.instructions.iter().enumerate() {
            self.write_instruction(output, instruction, 0, i + 1, log.account_states.as_ref())?;
        }

        Ok(())
    }

    /// Write single instruction with proper indentation and hierarchy
    ///
    /// For outer instructions (depth=0), if account_states is provided, displays
    /// an 8-column table with Owner, Data Len, Lamports, and Change columns.
    /// For inner instructions, displays a 4-column table.
    fn write_instruction(
        &self,
        output: &mut String,
        instruction: &EnhancedInstructionLog,
        depth: usize,
        number: usize,
        account_states: Option<&HashMap<Pubkey, AccountStateSnapshot>>,
    ) -> fmt::Result {
        let indent = self.get_tree_indent(depth);
        let prefix = if depth == 0 { "├─" } else { "└─" };

        // Instruction header
        let inner_count = if instruction.inner_instructions.is_empty() {
            String::new()
        } else {
            format!(".{}", instruction.inner_instructions.len())
        };

        write!(
            output,
            "{}{} {}#{}{} {}{} ({}{}{})",
            indent,
            prefix,
            self.colors.bold,
            number,
            inner_count,
            self.colors.blue,
            instruction.program_id,
            self.colors.cyan,
            instruction.program_name,
            self.colors.reset
        )?;

        // Add instruction name if parsed
        if let Some(ref name) = instruction.instruction_name {
            write!(
                output,
                " - {}{}{}",
                self.colors.yellow, name, self.colors.reset
            )?;
        }

        // Add compute units if available and requested
        if self.config.show_compute_units {
            if let Some(compute) = instruction.compute_consumed {
                write!(
                    output,
                    " {}({}{}CU{})",
                    self.colors.gray, self.colors.blue, compute, self.colors.gray
                )?;
            }
        }

        writeln!(output, "{}", self.colors.reset)?;

        // Show instruction details based on verbosity
        match self.config.verbosity {
            LogVerbosity::Detailed | LogVerbosity::Full => {
                // Display decoded instruction fields from custom decoder
                if let Some(ref decoded) = instruction.decoded_instruction {
                    if !decoded.fields.is_empty() {
                        let indent = self.get_tree_indent(depth + 1);
                        for field in &decoded.fields {
                            self.write_decoded_field(field, output, &indent, 0)?;
                        }
                    }
                } else if !instruction.data.is_empty() {
                    // Show raw instruction data for unparseable instructions with chunking
                    // Skip instruction data for account compression program unless explicitly configured
                    let should_show_data = if instruction.program_name == "Account Compression" {
                        self.config.show_compression_instruction_data
                    } else {
                        true
                    };

                    if should_show_data {
                        let indent = self.get_tree_indent(depth + 1);
                        writeln!(
                            output,
                            "{}{}Raw instruction data ({} bytes): {}[",
                            indent,
                            self.colors.gray,
                            instruction.data.len(),
                            self.colors.cyan
                        )?;

                        // Chunk the data into 32-byte groups for better readability
                        for (i, chunk) in instruction.data.chunks(32).enumerate() {
                            write!(output, "{}  ", indent)?;
                            for (j, byte) in chunk.iter().enumerate() {
                                if j > 0 {
                                    write!(output, ", ")?;
                                }
                                write!(output, "{}", byte)?;
                            }
                            if i < instruction.data.chunks(32).len() - 1 {
                                writeln!(output, ",")?;
                            } else {
                                writeln!(output, "]{}", self.colors.reset)?;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Show accounts if verbose
        if self.config.verbosity == LogVerbosity::Full && !instruction.accounts.is_empty() {
            let accounts_indent = self.get_tree_indent(depth + 1);
            writeln!(
                output,
                "{}{}Accounts ({}):{}",
                accounts_indent,
                self.colors.gray,
                instruction.accounts.len(),
                self.colors.reset
            )?;

            // For outer instructions (depth=0) with account states, use 8-column table
            // For inner instructions, use 4-column table
            if let (0, Some(states)) = (depth, account_states) {
                let mut outer_rows: Vec<OuterAccountRow> = Vec::new();

                for (idx, account) in instruction.accounts.iter().enumerate() {
                    let access = if account.is_signer && account.is_writable {
                        AccountAccess::SignerWritable
                    } else if account.is_signer {
                        AccountAccess::Signer
                    } else if account.is_writable {
                        AccountAccess::Writable
                    } else {
                        AccountAccess::Readonly
                    };

                    // Try to get account name from decoded instruction first, then fall back to lookup
                    // Empty names from resolver indicate "use KNOWN_ACCOUNTS lookup"
                    let account_name = instruction
                        .decoded_instruction
                        .as_ref()
                        .and_then(|decoded| decoded.account_names.get(idx).cloned())
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| self.get_account_name(&account.pubkey));

                    // Get account state if available
                    let (data_len, lamports, lamports_change, owner_str) = if let Some(state) =
                        states.get(&account.pubkey)
                    {
                        let change = (state.lamports_after as i128 - state.lamports_before as i128)
                            .clamp(i64::MIN as i128, i64::MAX as i128)
                            as i64;
                        let change_str = if change > 0 {
                            format!("+{}", format_signed_with_thousands_separator(change))
                        } else if change < 0 {
                            format_signed_with_thousands_separator(change)
                        } else {
                            "0".to_string()
                        };
                        let owner_pubkey_str = state.owner.to_string();
                        let owner_short = if owner_pubkey_str.len() >= 5 {
                            owner_pubkey_str[..5].to_string()
                        } else {
                            owner_pubkey_str
                        };
                        (
                            format_with_thousands_separator(state.data_len_before as u64),
                            format_with_thousands_separator(state.lamports_before),
                            change_str,
                            owner_short,
                        )
                    } else {
                        (
                            "-".to_string(),
                            "-".to_string(),
                            "-".to_string(),
                            "-".to_string(),
                        )
                    };

                    outer_rows.push(OuterAccountRow {
                        symbol: access.symbol(idx + 1),
                        pubkey: account.pubkey.to_string(),
                        access: access.text().to_string(),
                        name: account_name,
                        owner: owner_str,
                        data_len,
                        lamports,
                        lamports_change,
                    });
                }

                if !outer_rows.is_empty() {
                    let table = Table::new(outer_rows)
                        .to_string()
                        .lines()
                        .map(|line| format!("{}{}", accounts_indent, line))
                        .collect::<Vec<_>>()
                        .join("\n");
                    writeln!(output, "{}", table)?;
                }
            } else {
                // Inner instructions or no account states - use 4-column table
                let mut account_rows: Vec<AccountRow> = Vec::new();

                for (idx, account) in instruction.accounts.iter().enumerate() {
                    let access = if account.is_signer && account.is_writable {
                        AccountAccess::SignerWritable
                    } else if account.is_signer {
                        AccountAccess::Signer
                    } else if account.is_writable {
                        AccountAccess::Writable
                    } else {
                        AccountAccess::Readonly
                    };

                    // Try to get account name from decoded instruction first, then fall back to lookup
                    // Empty names from resolver indicate "use KNOWN_ACCOUNTS lookup"
                    let account_name = instruction
                        .decoded_instruction
                        .as_ref()
                        .and_then(|decoded| decoded.account_names.get(idx).cloned())
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| self.get_account_name(&account.pubkey));
                    account_rows.push(AccountRow {
                        symbol: access.symbol(idx + 1),
                        pubkey: account.pubkey.to_string(),
                        access: access.text().to_string(),
                        name: account_name,
                    });
                }

                if !account_rows.is_empty() {
                    let table = Table::new(account_rows)
                        .to_string()
                        .lines()
                        .map(|line| format!("{}{}", accounts_indent, line))
                        .collect::<Vec<_>>()
                        .join("\n");
                    writeln!(output, "{}", table)?;
                }
            }
        }

        // Write inner instructions recursively (inner instructions don't get account states)
        for (i, inner) in instruction.inner_instructions.iter().enumerate() {
            if depth < self.config.max_cpi_depth {
                self.write_instruction(output, inner, depth + 1, i + 1, None)?;
            }
        }

        Ok(())
    }

    /// Collapse simple multiline enum variants onto one line
    /// Converts `Some(\n    2,\n)` to `Some(2)`
    fn collapse_simple_enums(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '(' {
                // Collect content until matching )
                let mut paren_content = String::new();
                let mut paren_depth = 1;

                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    if next_c == '(' {
                        paren_depth += 1;
                        paren_content.push(next_c);
                    } else if next_c == ')' {
                        paren_depth -= 1;
                        if paren_depth == 0 {
                            break;
                        }
                        paren_content.push(next_c);
                    } else {
                        paren_content.push(next_c);
                    }
                }

                // Check if content is simple (just whitespace and a single value)
                let trimmed = paren_content.trim().trim_end_matches(',');
                let is_simple = (!trimmed.contains('(')
                    && !trimmed.contains('{')
                    && !trimmed.contains('[')
                    && !trimmed.contains('\n'))
                    || (trimmed.parse::<i64>().is_ok())
                    || (trimmed == "true" || trimmed == "false")
                    || trimmed.is_empty();

                if is_simple && paren_content.contains('\n') {
                    // Collapse to single line
                    result.push('(');
                    result.push_str(trimmed);
                    result.push(')');
                } else {
                    // Keep original
                    result.push('(');
                    result.push_str(&paren_content);
                    result.push(')');
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Truncate byte arrays in a string to show first N and last N elements
    /// Handles both single-line `[1, 2, 3, ...]` and multiline arrays from pretty Debug
    fn truncate_byte_arrays(input: &str, show_start: usize, show_end: usize) -> String {
        let min_elements_to_truncate = show_start + show_end + 4;

        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '[' {
                // Potential start of an array - collect until matching ]
                let mut array_content = String::new();
                let mut bracket_depth = 1;
                let mut is_byte_array = true;

                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    if next_c == '[' {
                        bracket_depth += 1;
                        is_byte_array = false; // Nested arrays aren't simple byte arrays
                        array_content.push(next_c);
                    } else if next_c == ']' {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            break;
                        }
                        array_content.push(next_c);
                    } else {
                        // Check if content looks like a byte array (numbers, commas, whitespace)
                        if !next_c.is_ascii_digit() && next_c != ',' && !next_c.is_whitespace() {
                            is_byte_array = false;
                        }
                        array_content.push(next_c);
                    }
                }

                if is_byte_array && !array_content.is_empty() {
                    // Parse elements (split by comma, trim whitespace)
                    let elements: Vec<&str> = array_content
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if elements.len() >= min_elements_to_truncate {
                        // Truncate: show first N and last N
                        let start_elements: Vec<&str> =
                            elements.iter().take(show_start).copied().collect();
                        let end_elements: Vec<&str> = elements
                            .iter()
                            .skip(elements.len().saturating_sub(show_end))
                            .copied()
                            .collect();

                        result.push('[');
                        result.push_str(&start_elements.join(", "));
                        result.push_str(", ...");
                        result.push_str(&format!("({} bytes)", elements.len()));
                        result.push_str("..., ");
                        result.push_str(&end_elements.join(", "));
                        result.push(']');
                    } else {
                        // Keep original
                        result.push('[');
                        result.push_str(&array_content);
                        result.push(']');
                    }
                } else {
                    // Not a byte array - recursively process the content to handle nested byte arrays
                    let processed_content =
                        Self::truncate_byte_arrays(&array_content, show_start, show_end);
                    result.push('[');
                    result.push_str(&processed_content);
                    result.push(']');
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Write a single decoded field (called recursively for nested fields)
    fn write_decoded_field(
        &self,
        field: &crate::DecodedField,
        output: &mut String,
        indent: &str,
        depth: usize,
    ) -> fmt::Result {
        let field_indent = format!("{}  {}", indent, "  ".repeat(depth));
        if field.children.is_empty() {
            // Apply formatting transformations if enabled
            let display_value = if let Some((first, last)) = self.config.truncate_byte_arrays {
                let collapsed = self.collapse_simple_enums(&field.value);
                Self::truncate_byte_arrays(&collapsed, first, last)
            } else {
                field.value.clone()
            };

            // Handle multiline values by indenting each subsequent line
            if display_value.contains('\n') {
                let continuation_indent = format!("{}  ", field_indent);
                let indented_value = display_value
                    .lines()
                    .enumerate()
                    .map(|(i, line)| {
                        if i == 0 {
                            line.to_string()
                        } else {
                            format!("{}{}", continuation_indent, line)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                // Skip "name: " prefix if field name is empty
                if field.name.is_empty() {
                    writeln!(
                        output,
                        "{}{}{}{}",
                        field_indent, self.colors.cyan, indented_value, self.colors.reset
                    )?;
                } else {
                    writeln!(
                        output,
                        "{}{}{}: {}{}{}",
                        field_indent,
                        self.colors.gray,
                        field.name,
                        self.colors.cyan,
                        indented_value,
                        self.colors.reset
                    )?;
                }
            } else {
                // Skip "name: " prefix if field name is empty
                if field.name.is_empty() {
                    writeln!(
                        output,
                        "{}{}{}{}",
                        field_indent, self.colors.cyan, display_value, self.colors.reset
                    )?;
                } else {
                    writeln!(
                        output,
                        "{}{}{}: {}{}{}",
                        field_indent,
                        self.colors.gray,
                        field.name,
                        self.colors.cyan,
                        display_value,
                        self.colors.reset
                    )?;
                }
            }
        } else {
            // Skip "name:" if field name is empty
            if !field.name.is_empty() {
                writeln!(
                    output,
                    "{}{}{}:{}",
                    field_indent, self.colors.gray, field.name, self.colors.reset
                )?;
            }
            // Depth guard to prevent stack overflow from deeply nested fields
            if depth < self.config.max_cpi_depth {
                for child in &field.children {
                    self.write_decoded_field(child, output, indent, depth + 1)?;
                }
            } else {
                writeln!(
                    output,
                    "{}  {}<max depth reached>{}",
                    field_indent, self.colors.gray, self.colors.reset
                )?;
            }
        }
        Ok(())
    }

    /// Write account changes section
    fn write_account_changes_section(
        &self,
        output: &mut String,
        log: &EnhancedTransactionLog,
    ) -> fmt::Result {
        writeln!(output)?;
        writeln!(
            output,
            "{}Account Changes ({}):{}\n",
            self.colors.bold,
            log.account_changes.len(),
            self.colors.reset
        )?;

        for change in &log.account_changes {
            self.write_account_change(output, change)?;
        }

        Ok(())
    }

    /// Write single account change
    fn write_account_change(&self, output: &mut String, change: &AccountChange) -> fmt::Result {
        writeln!(
            output,
            "│ {}{} {} ({}) - {}{}{}",
            change.access.symbol(change.account_index),
            self.colors.cyan,
            change.pubkey,
            change.access.text(),
            self.colors.yellow,
            change.account_type,
            self.colors.reset
        )?;

        if change.lamports_before != change.lamports_after {
            writeln!(
                output,
                "│   {}Lamports: {} → {}{}",
                self.colors.gray, change.lamports_before, change.lamports_after, self.colors.reset
            )?;
        }

        Ok(())
    }

    /// Write Light Protocol events section
    fn write_light_events_section(
        &self,
        output: &mut String,
        log: &EnhancedTransactionLog,
    ) -> fmt::Result {
        writeln!(output)?;
        writeln!(
            output,
            "{}Light Protocol Events ({}):{}\n",
            self.colors.bold,
            log.light_events.len(),
            self.colors.reset
        )?;

        for event in &log.light_events {
            writeln!(
                output,
                "│ {}Event: {}{}{}",
                self.colors.blue, self.colors.yellow, event.event_type, self.colors.reset
            )?;

            if !event.compressed_accounts.is_empty() {
                writeln!(
                    output,
                    "│   {}Compressed Accounts: {}{}",
                    self.colors.gray,
                    event.compressed_accounts.len(),
                    self.colors.reset
                )?;
            }

            if !event.merkle_tree_changes.is_empty() {
                writeln!(
                    output,
                    "│   {}Merkle Tree Changes: {}{}",
                    self.colors.gray,
                    event.merkle_tree_changes.len(),
                    self.colors.reset
                )?;
            }
        }

        Ok(())
    }

    /// Write program logs section using LiteSVM's pretty logs
    fn write_program_logs_section(
        &self,
        output: &mut String,
        log: &EnhancedTransactionLog,
    ) -> fmt::Result {
        writeln!(output)?;
        writeln!(
            output,
            "{}│{} {}Program Logs:{}",
            self.colors.gray, self.colors.reset, self.colors.bold, self.colors.reset
        )?;
        writeln!(output, "{}│{}", self.colors.gray, self.colors.reset)?;

        // Display LiteSVM's pretty formatted logs with proper indentation
        for line in log.program_logs_pretty.lines() {
            if !line.trim().is_empty() {
                writeln!(
                    output,
                    "{}│{} {}",
                    self.colors.gray, self.colors.reset, line
                )?;
            }
        }

        Ok(())
    }

    /// Get tree-style indentation for given depth
    fn get_tree_indent(&self, depth: usize) -> String {
        let border = format!("{}│{} ", self.colors.gray, self.colors.reset);
        if depth == 0 {
            border
        } else {
            format!("{}{}", border, "│  ".repeat(depth))
        }
    }

    /// Get color for transaction status
    fn status_color(&self, status: &TransactionStatus) -> &str {
        match status {
            TransactionStatus::Success => self.colors.green,
            TransactionStatus::Failed(_) => self.colors.red,
            TransactionStatus::Unknown => self.colors.yellow,
        }
    }

    /// Get human-readable name for known accounts using constants and test accounts
    fn get_account_name(&self, pubkey: &Pubkey) -> String {
        #[cfg(feature = "light-protocol")]
        {
            use light_sdk_types::constants;

            let pubkey_bytes = pubkey.to_bytes();

            // Light Protocol Programs and Accounts from constants
            let light_accounts: &[([u8; 32], &str)] = &[
                (constants::LIGHT_SYSTEM_PROGRAM_ID, "light system program"),
                (
                    constants::ACCOUNT_COMPRESSION_PROGRAM_ID,
                    "account compression program",
                ),
                (constants::REGISTERED_PROGRAM_PDA, "registered program pda"),
                (
                    constants::ACCOUNT_COMPRESSION_AUTHORITY_PDA,
                    "account compression authority",
                ),
                (constants::NOOP_PROGRAM_ID, "noop program"),
                (constants::LIGHT_TOKEN_PROGRAM_ID, "light token program"),
                (constants::ADDRESS_TREE_V1, "address tree v1"),
                (constants::ADDRESS_QUEUE_V1, "address queue v1"),
                (constants::SOL_POOL_PDA, "sol pool pda"),
            ];

            for (id, name) in light_accounts {
                if pubkey_bytes == *id {
                    return name.to_string();
                }
            }
        }

        // String-based matches for test accounts and other addresses
        let pubkey_str = pubkey.to_string();
        for (addr, name) in KNOWN_ACCOUNTS {
            if pubkey_str == *addr {
                return name.to_string();
            }
        }

        // Classify based on curve: on-curve = wallet, off-curve = pda (or program, but we can't tell without executable flag)
        if pubkey.is_on_curve() {
            "unknown wallet".to_string()
        } else {
            "unknown pda".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_with_thousands_separator() {
        assert_eq!(format_with_thousands_separator(0), "0");
        assert_eq!(format_with_thousands_separator(1), "1");
        assert_eq!(format_with_thousands_separator(12), "12");
        assert_eq!(format_with_thousands_separator(123), "123");
        assert_eq!(format_with_thousands_separator(1234), "1,234");
        assert_eq!(format_with_thousands_separator(12345), "12,345");
        assert_eq!(format_with_thousands_separator(123456), "123,456");
        assert_eq!(format_with_thousands_separator(1234567), "1,234,567");
        assert_eq!(format_with_thousands_separator(1000000000), "1,000,000,000");
    }

    #[test]
    fn test_format_signed_with_thousands_separator() {
        assert_eq!(format_signed_with_thousands_separator(0), "0");
        assert_eq!(format_signed_with_thousands_separator(1234), "1,234");
        assert_eq!(format_signed_with_thousands_separator(-1234), "-1,234");
        assert_eq!(
            format_signed_with_thousands_separator(-1000000),
            "-1,000,000"
        );
    }
}
