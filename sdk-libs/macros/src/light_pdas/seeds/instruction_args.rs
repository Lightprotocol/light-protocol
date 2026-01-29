//! Instruction argument set for seed classification.
//!
//! This module provides `InstructionArgSet` for tracking instruction argument names.
//! The parsing logic has been consolidated into `parsing/instruction_arg.rs`.

use std::collections::HashSet;

/// Set of instruction argument names for Format 2 detection.
///
/// Anchor supports two formats for `#[instruction(...)]`:
/// - Format 1: `#[instruction(params: SomeStruct)]` - users write `params.field`
/// - Format 2: `#[instruction(owner: Pubkey, amount: u64)]` - users write bare `owner`
///
/// This struct holds the names from Format 2 so we can recognize them in seed expressions.
#[derive(Clone, Debug, Default)]
pub struct InstructionArgSet {
    /// Names of instruction args (e.g., {"owner", "amount", "bump"})
    pub names: HashSet<String>,
}

impl InstructionArgSet {
    /// Create an empty arg set (used when no #[instruction] attribute present)
    pub fn empty() -> Self {
        Self {
            names: HashSet::new(),
        }
    }

    /// Create from a list of argument names
    pub fn from_names(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            names: names.into_iter().collect(),
        }
    }

    /// Check if a name is a known instruction argument
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_arg_set_empty() {
        let args = InstructionArgSet::empty();
        assert!(!args.contains("owner"));
        assert!(args.names.is_empty());
    }

    #[test]
    fn test_instruction_arg_set_from_names() {
        let args = InstructionArgSet::from_names(vec!["owner".to_string(), "amount".to_string()]);
        assert!(args.contains("owner"));
        assert!(args.contains("amount"));
        assert!(!args.contains("other"));
    }
}
