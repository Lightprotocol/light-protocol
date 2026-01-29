<!-- cargo-rdme start -->

# light-instruction-decoder

Instruction decoder and transaction logger for Light Protocol programs.

This crate provides:
- Core types for instruction decoding (`DecodedField`, `DecodedInstruction`, `InstructionDecoder` trait)
- Decoder registry for managing multiple program decoders
- Built-in decoders for Light Protocol programs (System, Compressed Token, etc.)
- Transaction logging configuration and formatting utilities

| Export | Description |
|--------|-------------|
| [`InstructionDecoder`] | Trait for decoding program instructions |
| [`DecoderRegistry`] | Registry for multiple program decoders |
| [`EnhancedLoggingConfig`] | Transaction logging configuration |
| [`TransactionFormatter`] | Format transaction logs with ANSI colors |
| [`instruction_decoder`] | Derive macro for decoder implementations |

Note: Most functionality is only available off-chain (not on Solana targets).

<!-- cargo-rdme end -->
