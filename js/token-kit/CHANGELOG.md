# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

Initial release. API may change before 1.0.

### Added

- Instruction builders for all CToken operations (transfer, mint-to, burn, approve, revoke, freeze, thaw, close)
- Transfer2 instruction builder for compressed account transfers with validity proofs
- MintAction instruction builder for compressed mint management
- Borsh-compatible codecs for all instruction data types
- `PhotonIndexer` client for fetching compressed accounts and validity proofs
- Account loading and selection utilities (`loadTokenAccountsForTransfer`, `selectAccountsForAmount`)
- `buildCompressedTransfer` high-level action builder
- PDA derivation utilities for ATAs, mints, and pools
- Compressible extension codecs for rent-free account creation
