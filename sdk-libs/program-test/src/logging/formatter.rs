//! Transaction formatting utilities for explorer-style output

use std::fmt::{self, Write};

use solana_sdk::system_program;
use tabled::{Table, Tabled};

use super::{
    config::{EnhancedLoggingConfig, LogVerbosity},
    types::{
        AccountAccess, AccountChange, EnhancedInstructionLog, EnhancedTransactionLog,
        TransactionStatus,
    },
};

/// Row for account table display
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

/// Colors for terminal output
#[derive(Debug, Clone)]
pub struct Colors {
    pub bold: String,
    pub reset: String,
    pub green: String,
    pub red: String,
    pub yellow: String,
    pub blue: String,
    pub cyan: String,
    pub gray: String,
}

impl Colors {
    pub fn new(use_colors: bool) -> Self {
        if use_colors {
            Self {
                bold: "\x1b[1m".to_string(),
                reset: "\x1b[0m".to_string(),
                green: "\x1b[32m".to_string(),
                red: "\x1b[31m".to_string(),
                yellow: "\x1b[33m".to_string(),
                blue: "\x1b[34m".to_string(),
                cyan: "\x1b[36m".to_string(),
                gray: "\x1b[90m".to_string(),
            }
        } else {
            Self {
                bold: String::new(),
                reset: String::new(),
                green: String::new(),
                red: String::new(),
                yellow: String::new(),
                blue: String::new(),
                cyan: String::new(),
                gray: String::new(),
            }
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
            let mut modified = false;

            // Split by table separators while preserving them
            let parts: Vec<&str> = line.split('|').collect();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    new_line.push('|');
                }

                // Check if this cell contains a long value
                for word in part.split_whitespace() {
                    if word.len() > 44 && word.chars().all(|c| c.is_alphanumeric()) {
                        let indent = " ".repeat(leading_chars.len() + 2); // Extra space for table formatting
                        let formatted_word = self.format_long_value_with_indent(word, 44, &indent);
                        new_line.push_str(&part.replace(word, &formatted_word));
                        modified = true;
                        break;
                    }
                }

                if !modified {
                    new_line.push_str(part);
                }
            }

            if modified {
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
                    current_line = part.to_string();
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

        // Transaction box header with number
        writeln!(output, "{}┌───────────────────────────────────────── Transaction #{} ─────────────────────────────────────────────┐{}", self.colors.gray, tx_number, self.colors.reset).expect("Failed to write box header");

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

        // Transaction box footer
        writeln!(output, "{}└─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘{}", self.colors.gray, self.colors.reset).expect("Failed to write box footer");

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
            self.write_instruction(output, instruction, 0, i + 1)?;
        }

        Ok(())
    }

    /// Write single instruction with proper indentation and hierarchy
    fn write_instruction(
        &self,
        output: &mut String,
        instruction: &EnhancedInstructionLog,
        depth: usize,
        number: usize,
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
                if let Some(ref parsed) = instruction.parsed_data {
                    self.write_parsed_instruction_data(
                        output,
                        parsed,
                        &instruction.data,
                        depth + 1,
                    )?;
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

            // Create a table for better account formatting
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

                let account_name = self.get_account_name(&account.pubkey);
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

        // Write inner instructions recursively
        for (i, inner) in instruction.inner_instructions.iter().enumerate() {
            if depth < self.config.max_inner_instruction_depth {
                self.write_instruction(output, inner, depth + 1, i + 1)?;
            }
        }

        Ok(())
    }

    /// Write parsed instruction data
    fn write_parsed_instruction_data(
        &self,
        output: &mut String,
        parsed: &super::types::ParsedInstructionData,
        instruction_data: &[u8],
        depth: usize,
    ) -> fmt::Result {
        let indent = self.get_tree_indent(depth);

        match parsed {
            super::types::ParsedInstructionData::LightSystemProgram {
                instruction_type,
                compressed_accounts,
                proof_info,
                address_params,
                fee_info,
                input_account_data,
                output_account_data,
            } => {
                writeln!(
                    output,
                    "{}{}Light System: {}{}{}",
                    indent,
                    self.colors.gray,
                    self.colors.yellow,
                    instruction_type,
                    self.colors.reset
                )?;

                if let Some(compressed_accounts) = compressed_accounts {
                    writeln!(
                        output,
                        "{}{}Accounts: {}in: {}, out: {}{}",
                        indent,
                        self.colors.gray,
                        self.colors.cyan,
                        compressed_accounts.input_accounts,
                        compressed_accounts.output_accounts,
                        self.colors.reset
                    )?;
                }

                if let Some(proof_info) = proof_info {
                    if proof_info.has_validity_proof {
                        writeln!(
                            output,
                            "{}{}Proof: {}{} proof{}",
                            indent,
                            self.colors.gray,
                            self.colors.cyan,
                            proof_info.proof_type,
                            self.colors.reset
                        )?;
                    }
                }

                // Display input account data
                if let Some(ref input_accounts) = input_account_data {
                    writeln!(
                        output,
                        "{}{}Input Accounts ({}):{}",
                        indent,
                        self.colors.gray,
                        input_accounts.len(),
                        self.colors.reset
                    )?;
                    for (i, acc_data) in input_accounts.iter().enumerate() {
                        writeln!(
                            output,
                            "{}  {}[{}]{}",
                            indent, self.colors.gray, i, self.colors.reset
                        )?;
                        writeln!(
                            output,
                            "{}      {}owner: {}{}{}",
                            indent,
                            self.colors.gray,
                            self.colors.yellow,
                            acc_data
                                .owner
                                .map(|o| o.to_string())
                                .unwrap_or("None".to_string()),
                            self.colors.reset
                        )?;
                        if let Some(ref address) = acc_data.address {
                            writeln!(
                                output,
                                "{}      {}address: {}{:?}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                address,
                                self.colors.reset
                            )?;
                        }
                        writeln!(
                            output,
                            "{}      {}lamports: {}{}{}",
                            indent,
                            self.colors.gray,
                            self.colors.cyan,
                            acc_data.lamports,
                            self.colors.reset
                        )?;
                        if !acc_data.data_hash.is_empty() {
                            writeln!(
                                output,
                                "{}      {}data_hash: {}{:?}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                acc_data.data_hash,
                                self.colors.reset
                            )?;
                        }
                        if !acc_data.discriminator.is_empty() {
                            writeln!(
                                output,
                                "{}      {}discriminator: {}{:?}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                acc_data.discriminator,
                                self.colors.reset
                            )?;
                        }
                        if let Some(tree_idx) = acc_data.merkle_tree_index {
                            if let Some(tree_pubkey) = acc_data.merkle_tree_pubkey {
                                writeln!(
                                    output,
                                    "{}      {}merkle_tree_pubkey (index {}{}{}): {}{}{}",
                                    indent,
                                    self.colors.gray,
                                    self.colors.cyan,
                                    tree_idx,
                                    self.colors.gray,
                                    self.colors.yellow,
                                    tree_pubkey,
                                    self.colors.reset
                                )?;
                            } else {
                                writeln!(
                                    output,
                                    "{}      {}merkle_tree_index: {}{}{}",
                                    indent,
                                    self.colors.gray,
                                    self.colors.cyan,
                                    tree_idx,
                                    self.colors.reset
                                )?;
                            }
                        } else if let Some(tree_pubkey) = acc_data.merkle_tree_pubkey {
                            writeln!(
                                output,
                                "{}      {}merkle_tree_pubkey: {}{}{}",
                                indent,
                                self.colors.gray,
                                self.colors.yellow,
                                tree_pubkey,
                                self.colors.reset
                            )?;
                        }
                        if let Some(queue_idx) = acc_data.queue_index {
                            if let Some(queue_pubkey) = acc_data.queue_pubkey {
                                writeln!(
                                    output,
                                    "{}      {}queue_pubkey (index {}{}{}): {}{}{}",
                                    indent,
                                    self.colors.gray,
                                    self.colors.cyan,
                                    queue_idx,
                                    self.colors.gray,
                                    self.colors.yellow,
                                    queue_pubkey,
                                    self.colors.reset
                                )?;
                            } else {
                                writeln!(
                                    output,
                                    "{}      {}queue_index: {}{}{}",
                                    indent,
                                    self.colors.gray,
                                    self.colors.cyan,
                                    queue_idx,
                                    self.colors.reset
                                )?;
                            }
                        } else if let Some(queue_pubkey) = acc_data.queue_pubkey {
                            writeln!(
                                output,
                                "{}      {}queue_pubkey: {}{}{}",
                                indent,
                                self.colors.gray,
                                self.colors.yellow,
                                queue_pubkey,
                                self.colors.reset
                            )?;
                        }
                        // Display leaf index after queue_pubkey
                        if let Some(leaf_idx) = acc_data.leaf_index {
                            writeln!(
                                output,
                                "{}      {}leaf_index: {}{}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                leaf_idx,
                                self.colors.reset
                            )?;
                        }
                        // Display root index after leaf index
                        if let Some(root_idx) = acc_data.root_index {
                            writeln!(
                                output,
                                "{}      {}root_index: {}{}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                root_idx,
                                self.colors.reset
                            )?;
                        }
                    }
                }

                // Display output account data
                if let Some(ref output_data) = output_account_data {
                    writeln!(
                        output,
                        "{}{}Output Accounts ({}):{}",
                        indent,
                        self.colors.gray,
                        output_data.len(),
                        self.colors.reset
                    )?;
                    for (i, acc_data) in output_data.iter().enumerate() {
                        writeln!(
                            output,
                            "{}  {}[{}]{}",
                            indent, self.colors.gray, i, self.colors.reset
                        )?;
                        writeln!(
                            output,
                            "{}      {}owner: {}{}{}",
                            indent,
                            self.colors.gray,
                            self.colors.yellow,
                            acc_data
                                .owner
                                .map(|o| o.to_string())
                                .unwrap_or("None".to_string()),
                            self.colors.reset
                        )?;
                        if let Some(ref address) = acc_data.address {
                            writeln!(
                                output,
                                "{}      {}address: {}{:?}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                address,
                                self.colors.reset
                            )?;
                        }
                        writeln!(
                            output,
                            "{}      {}lamports: {}{}{}",
                            indent,
                            self.colors.gray,
                            self.colors.cyan,
                            acc_data.lamports,
                            self.colors.reset
                        )?;
                        if !acc_data.data_hash.is_empty() {
                            writeln!(
                                output,
                                "{}      {}data_hash: {}{:?}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                acc_data.data_hash,
                                self.colors.reset
                            )?;
                        }
                        if !acc_data.discriminator.is_empty() {
                            writeln!(
                                output,
                                "{}      {}discriminator: {}{:?}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                acc_data.discriminator,
                                self.colors.reset
                            )?;
                        }
                        if let Some(ref data) = acc_data.data {
                            writeln!(
                                output,
                                "{}      {}data ({} bytes): {}{:?}{}",
                                indent,
                                self.colors.gray,
                                data.len(),
                                self.colors.cyan,
                                data,
                                self.colors.reset
                            )?;
                        }
                        if let Some(tree_idx) = acc_data.merkle_tree_index {
                            if let Some(tree_pubkey) = acc_data.merkle_tree_pubkey {
                                writeln!(
                                    output,
                                    "{}      {}merkle_tree_pubkey (index {}{}{}): {}{}{}",
                                    indent,
                                    self.colors.gray,
                                    self.colors.cyan,
                                    tree_idx,
                                    self.colors.gray,
                                    self.colors.yellow,
                                    tree_pubkey,
                                    self.colors.reset
                                )?;
                            } else {
                                writeln!(
                                    output,
                                    "{}      {}merkle_tree_index: {}{}{}",
                                    indent,
                                    self.colors.gray,
                                    self.colors.cyan,
                                    tree_idx,
                                    self.colors.reset
                                )?;
                            }
                        } else if let Some(tree_pubkey) = acc_data.merkle_tree_pubkey {
                            writeln!(
                                output,
                                "{}      {}merkle_tree_pubkey: {}{}{}",
                                indent,
                                self.colors.gray,
                                self.colors.yellow,
                                tree_pubkey,
                                self.colors.reset
                            )?;
                        }
                    }
                }

                // Display address parameters with actual values
                if let Some(address_params) = address_params {
                    writeln!(
                        output,
                        "{}{}New Addresses ({}):{}",
                        indent,
                        self.colors.gray,
                        address_params.len(),
                        self.colors.reset
                    )?;
                    for (i, addr_param) in address_params.iter().enumerate() {
                        writeln!(
                            output,
                            "{}  {}[{}] {}seed: {}{:?}{}",
                            indent,
                            self.colors.gray,
                            i,
                            self.colors.gray,
                            self.colors.cyan,
                            addr_param.seed,
                            self.colors.reset
                        )?;

                        // Check if v2 by comparing tree and queue pubkeys
                        let is_v2 = addr_param.address_merkle_tree_pubkey
                            == addr_param.address_queue_pubkey;

                        // Display address tree
                        if let Some(tree_pubkey) = addr_param.address_merkle_tree_pubkey {
                            writeln!(
                                output,
                                "{}      {}tree[{}]: {}{}{}",
                                indent,
                                self.colors.gray,
                                addr_param.merkle_tree_index.unwrap_or(0),
                                self.colors.yellow,
                                tree_pubkey,
                                self.colors.reset
                            )?;
                        }

                        // Only display queue for v1 trees (when different from tree)
                        if !is_v2 {
                            if let Some(queue_pubkey) = addr_param.address_queue_pubkey {
                                writeln!(
                                    output,
                                    "{}      {}queue[{}]: {}{}{}",
                                    indent,
                                    self.colors.gray,
                                    addr_param.address_queue_index.unwrap_or(0),
                                    self.colors.yellow,
                                    queue_pubkey,
                                    self.colors.reset
                                )?;
                            }
                        }

                        if let Some(ref derived_addr) = addr_param.derived_address {
                            writeln!(
                                output,
                                "{}      {}address: {}{:?}{}",
                                indent,
                                self.colors.gray,
                                self.colors.cyan,
                                derived_addr,
                                self.colors.reset
                            )?;
                        }
                        let assignment_str = match addr_param.assigned_account_index {
                            super::types::AddressAssignment::AssignedIndex(idx) => {
                                format!("{}", idx)
                            }
                            super::types::AddressAssignment::None => "none".to_string(),
                            super::types::AddressAssignment::V1 => "n/a (v1)".to_string(),
                        };
                        writeln!(
                            output,
                            "{}      {}assigned: {}{}{}",
                            indent,
                            self.colors.gray,
                            self.colors.yellow,
                            assignment_str,
                            self.colors.reset
                        )?;
                    }
                }

                if let Some(fee_info) = fee_info {
                    if let Some(relay_fee) = fee_info.relay_fee {
                        writeln!(
                            output,
                            "{}{}Relay Fee: {}{} lamports{}",
                            indent,
                            self.colors.gray,
                            self.colors.yellow,
                            relay_fee,
                            self.colors.reset
                        )?;
                    }
                    if let Some(compression_fee) = fee_info.compression_fee {
                        writeln!(
                            output,
                            "{}{}Compression Fee: {}{} lamports{}",
                            indent,
                            self.colors.gray,
                            self.colors.yellow,
                            compression_fee,
                            self.colors.reset
                        )?;
                    }
                }
            }
            super::types::ParsedInstructionData::ComputeBudget {
                instruction_type,
                value,
            } => {
                write!(
                    output,
                    "{}{}Compute Budget: {}{}{}",
                    indent,
                    self.colors.gray,
                    self.colors.yellow,
                    instruction_type,
                    self.colors.reset
                )?;

                if let Some(val) = value {
                    writeln!(output, " ({})", val)?;
                } else {
                    writeln!(output)?;
                }
            }
            super::types::ParsedInstructionData::System {
                instruction_type,
                lamports,
                space: _,
                new_account: _,
            } => {
                write!(
                    output,
                    "{}{}System: {}{}{}",
                    indent,
                    self.colors.gray,
                    self.colors.yellow,
                    instruction_type,
                    self.colors.reset
                )?;

                if let Some(amount) = lamports {
                    writeln!(output, " ({} lamports)", amount)?;
                } else {
                    writeln!(output)?;
                }
            }
            super::types::ParsedInstructionData::Unknown {
                program_name,
                data_preview: _,
            } => {
                writeln!(
                    output,
                    "{}{}Program: {}{}{}",
                    indent, self.colors.gray, self.colors.yellow, program_name, self.colors.reset
                )?;

                // Show raw instruction data for unknown programs with chunking
                // Skip instruction data for account compression program unless explicitly configured
                let should_show_data = if program_name == "Account Compression" {
                    self.config.show_compression_instruction_data
                } else {
                    true
                };

                if !instruction_data.is_empty() && should_show_data {
                    writeln!(
                        output,
                        "{}{}Raw instruction data ({} bytes): {}[",
                        indent,
                        self.colors.gray,
                        instruction_data.len(),
                        self.colors.cyan
                    )?;

                    // Chunk the data into 32-byte groups for better readability
                    for (i, chunk) in instruction_data.chunks(32).enumerate() {
                        write!(output, "{}  ", indent)?;
                        for (j, byte) in chunk.iter().enumerate() {
                            if j > 0 {
                                write!(output, ", ")?;
                            }
                            write!(output, "{}", byte)?;
                        }
                        if i < instruction_data.chunks(32).len() - 1 {
                            writeln!(output, ",")?;
                        } else {
                            writeln!(output, "]{}", self.colors.reset)?;
                        }
                    }
                }
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
            TransactionStatus::Success => &self.colors.green,
            TransactionStatus::Failed(_) => &self.colors.red,
            TransactionStatus::Unknown => &self.colors.yellow,
        }
    }

    /// Get human-readable name for known accounts using constants and test accounts
    fn get_account_name(&self, pubkey: &solana_sdk::pubkey::Pubkey) -> String {
        let pubkey_bytes = pubkey.to_bytes();

        // Light Protocol Programs and Accounts from constants
        if pubkey_bytes == light_sdk_types::constants::LIGHT_SYSTEM_PROGRAM_ID {
            return "light system program".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::ACCOUNT_COMPRESSION_PROGRAM_ID {
            return "account compression program".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::REGISTERED_PROGRAM_PDA {
            return "registered program pda".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::ACCOUNT_COMPRESSION_AUTHORITY_PDA {
            return "account compression authority".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::NOOP_PROGRAM_ID {
            return "noop program".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::C_TOKEN_PROGRAM_ID {
            return "compressed token program".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::ADDRESS_TREE_V1 {
            return "address tree v1".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::ADDRESS_QUEUE_V1 {
            return "address queue v1".to_string();
        }
        if pubkey_bytes == light_sdk_types::constants::SOL_POOL_PDA {
            return "sol pool pda".to_string();
        }

        // String-based matches for test accounts and other addresses
        match pubkey.to_string().as_str() {
            "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy" => "test program".to_string(),

            // Test accounts from test_accounts.rs - Local Test Validator
            "smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT" => "v1 state merkle tree".to_string(),
            "nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148" => "v1 nullifier queue".to_string(),
            "cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4" => "v1 cpi context".to_string(),
            "amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2" => "v1 address merkle tree".to_string(),
            "aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F" => "v1 address queue".to_string(),

            // V2 State Trees and Queues (5 tree triples)
            "bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU" => "v2 state merkle tree 1".to_string(),
            "oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto" => "v2 state output queue 1".to_string(),
            "cpi15BoVPKgEPw5o8wc2T816GE7b378nMXnhH3Xbq4y" => "v2 cpi context 1".to_string(),
            "bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi" => "v2 state merkle tree 2".to_string(),
            "oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg" => "v2 state output queue 2".to_string(),
            "cpi2yGapXUR3As5SjnHBAVvmApNiLsbeZpF3euWnW6B" => "v2 cpi context 2".to_string(),
            "bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb" => "v2 state merkle tree 3".to_string(),
            "oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ" => "v2 state output queue 3".to_string(),
            "cpi3mbwMpSX8FAGMZVP85AwxqCaQMfEk9Em1v8QK9Rf" => "v2 cpi context 3".to_string(),
            "bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8" => "v2 state merkle tree 4".to_string(),
            "oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq" => "v2 state output queue 4".to_string(),
            "cpi4yyPDc4bCgHAnsenunGA8Y77j3XEDyjgfyCKgcoc" => "v2 cpi context 4".to_string(),
            "bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2" => "v2 state merkle tree 5".to_string(),
            "oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P" => "v2 state output queue 5".to_string(),
            "cpi5ZTjdgYpZ1Xr7B1cMLLUE81oTtJbNNAyKary2nV6" => "v2 cpi context 5".to_string(),

            // V2 Address Trees (test accounts)
            "amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx" => "v2 address merkle tree".to_string(),

            // CPI Authority (commonly used in tests)
            "HZH7qSLcpAeDqCopVU4e5XkhT9j3JFsQiq8CmruY3aru" => "cpi authority pda".to_string(),

            // Solana Native Programs
            id if id == system_program::ID.to_string() => "system program".to_string(),
            "ComputeBudget111111111111111111111111111111" => "compute budget program".to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "token program".to_string(),
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => {
                "associated token program".to_string()
            }

            _ => {
                // Check if it's a PDA or regular account
                if pubkey.is_on_curve() {
                    "user account".to_string()
                } else {
                    "pda account".to_string()
                }
            }
        }
    }
}
