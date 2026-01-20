//! Instruction decoder registry for Light Protocol and common Solana programs

use std::collections::HashMap;

use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::{DecodedInstruction, InstructionDecoder};

// ============================================================================
// Trait-based Decoder Registry
// ============================================================================

/// Registry of instruction decoders
pub struct DecoderRegistry {
    decoders: HashMap<Pubkey, Box<dyn InstructionDecoder>>,
}

impl std::fmt::Debug for DecoderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecoderRegistry")
            .field("decoder_count", &self.decoders.len())
            .field("program_ids", &self.decoders.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl DecoderRegistry {
    /// Create a new registry with built-in decoders
    pub fn new() -> Self {
        let mut registry = Self {
            decoders: HashMap::new(),
        };

        // Register generic Solana program decoders (always available)
        registry.register(Box::new(crate::programs::ComputeBudgetInstructionDecoder));
        registry.register(Box::new(crate::programs::SplTokenInstructionDecoder));
        registry.register(Box::new(crate::programs::Token2022InstructionDecoder));
        registry.register(Box::new(crate::programs::SystemInstructionDecoder));

        // Register Light Protocol decoders (requires light-protocol feature)
        #[cfg(feature = "light-protocol")]
        {
            registry.register(Box::new(crate::programs::LightSystemInstructionDecoder));
            registry.register(Box::new(
                crate::programs::AccountCompressionInstructionDecoder,
            ));
            registry.register(Box::new(crate::programs::CTokenInstructionDecoder));
            registry.register(Box::new(crate::programs::RegistryInstructionDecoder));
        }

        registry
    }

    /// Register a custom decoder
    pub fn register(&mut self, decoder: Box<dyn InstructionDecoder>) {
        self.decoders.insert(decoder.program_id(), decoder);
    }

    /// Register multiple decoders from a Vec
    pub fn register_all(&mut self, decoders: Vec<Box<dyn InstructionDecoder>>) {
        for decoder in decoders {
            self.register(decoder);
        }
    }

    /// Decode an instruction using registered decoders
    pub fn decode(
        &self,
        program_id: &Pubkey,
        data: &[u8],
        accounts: &[AccountMeta],
    ) -> Option<(DecodedInstruction, &dyn InstructionDecoder)> {
        self.decoders.get(program_id).and_then(|decoder| {
            decoder
                .decode(data, accounts)
                .map(|d| (d, decoder.as_ref()))
        })
    }

    /// Get a decoder by program ID
    pub fn get_decoder(&self, program_id: &Pubkey) -> Option<&dyn InstructionDecoder> {
        self.decoders.get(program_id).map(|d| d.as_ref())
    }

    /// Check if a decoder exists for a program ID
    pub fn has_decoder(&self, program_id: &Pubkey) -> bool {
        self.decoders.contains_key(program_id)
    }
}

impl Default for DecoderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
