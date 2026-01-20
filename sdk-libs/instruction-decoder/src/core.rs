//! Core types for instruction decoding.

use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

/// A decoded instruction field for display.
#[derive(Debug, Clone)]
pub struct DecodedField {
    /// Field name
    pub name: String,
    /// Field value as string
    pub value: String,
    /// Optional nested fields (for complex types)
    pub children: Vec<DecodedField>,
}

impl DecodedField {
    /// Create a simple field with name and value.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            children: Vec::new(),
        }
    }

    /// Create a field with nested children.
    pub fn with_children(name: impl Into<String>, children: Vec<DecodedField>) -> Self {
        Self {
            name: name.into(),
            value: String::new(),
            children,
        }
    }
}

/// Result of decoding an instruction.
#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    /// Human-readable instruction name (e.g., "Transfer", "MintTo")
    pub name: String,
    /// Decoded fields to display
    pub fields: Vec<DecodedField>,
    /// Account names in order (index corresponds to account position)
    pub account_names: Vec<String>,
}

impl DecodedInstruction {
    /// Create a decoded instruction with fields and account names.
    pub fn with_fields_and_accounts(
        name: impl Into<String>,
        fields: Vec<DecodedField>,
        account_names: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            fields,
            account_names,
        }
    }
}

/// Trait for instruction decoders - each program implements this.
pub trait InstructionDecoder: Send + Sync {
    /// Program ID this decoder handles.
    fn program_id(&self) -> Pubkey;

    /// Human-readable program name (e.g., "Compressed Token Program").
    fn program_name(&self) -> &'static str;

    /// Decode instruction data into a structured representation.
    /// Returns None if decoding fails or instruction is unknown.
    fn decode(&self, data: &[u8], accounts: &[AccountMeta]) -> Option<DecodedInstruction>;
}
