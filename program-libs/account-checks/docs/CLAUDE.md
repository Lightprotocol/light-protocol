# Account Checks Documentation

This directory contains detailed documentation for the `light-account-checks` crate components.

## Core Components

### [ACCOUNT_INFO_TRAIT.md](ACCOUNT_INFO_TRAIT.md)
AccountInfoTrait abstraction layer that unifies account handling across Solana and Pinocchio runtimes. Covers the trait definition, implementations, and usage patterns for runtime-agnostic account processing.

### [ACCOUNT_CHECKS.md](ACCOUNT_CHECKS.md)
Comprehensive validation functions from the `checks` module. Documents all check functions including ownership validation, permission checks, discriminator handling, and PDA verification with code examples.

### [ACCOUNT_ITERATOR.md](ACCOUNT_ITERATOR.md)
Enhanced account iterator with detailed error reporting. Shows how to sequentially process accounts with automatic validation and location-based error messages for debugging.

## Type System

### [DISCRIMINATOR.md](DISCRIMINATOR.md)
Account type identification using 8-byte discriminators. Explains the Discriminator trait, constant arrays for compile-time verification, and integration with account initialization.

### [ERRORS.md](ERRORS.md)
Complete error type documentation with numeric codes (12006-12021 range), common causes, and resolution strategies. Includes conversion mappings for both Solana and Pinocchio runtimes.

## Utilities

### [PACKED_ACCOUNTS.md](PACKED_ACCOUNTS.md)
Index-based dynamic account access for handling variable account sets. Used for accessing mint, owner, and delegate accounts by index with bounds checking.

## Navigation Tips

- Each document focuses on a single module or concept
- Code examples demonstrate both Solana and Pinocchio usage where applicable
- Error codes reference actual values that appear in transaction logs
- Cross-references link related concepts across documents