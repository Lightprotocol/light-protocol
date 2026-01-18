# SDK-Libs Cleanup Spec

Exhaustive list of items to rename across `sdk-libs/`.

---

## 1. CTOKEN Renames → "light-token" or "token"

### 1.1 Directory Names
| Current | Proposed |
|---------|----------|
| `token-sdk/src/compressed_token/ctoken_instruction.rs` | `token_instruction.rs` |

### 1.2 Module Names
| Current | Proposed |
|---------|----------|
| `pub mod ctoken_instruction;` | `pub mod token_instruction;` |

### 1.3 Structs/Enums
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/compressed_token/v2/account2.rs` | `CTokenAccount2` | `LightTokenAccount` or `TokenAccount2` |
| `token-sdk/src/compressed_token/v1/account.rs` | `CTokenAccount` | `LightTokenAccount` or `TokenAccountV1` |
| `token-sdk/src/pack.rs` | `CTokenDataWithVariant<V>` | `LightTokenDataWithVariant<V>` |
| `token-types/src/account_infos/transfer.rs` | `CTokenProgram` (enum variant) | `LightTokenProgram` |
| `compressible-client/src/decompress_atas.rs` | `CTokenAccount2` usage | `LightTokenAccount` |

### 1.4 Traits
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/compressed_token/ctoken_instruction.rs` | `CTokenInstruction` | `LightTokenInstruction` |
| `sdk/src/compressible/traits.rs` | `IntoCTokenVariant<V, T>` | `IntoLightTokenVariant<V, T>` |

### 1.5 Type Aliases
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/pack.rs:301` | `CompressibleTokenDataWithVariant<V> = CTokenDataWithVariant<V>` | Remove alias, use consistent name |
| `token-sdk/src/pack.rs:303` | `CTokenData<V> = CTokenDataWithVariant<V>` | `LightTokenData<V>` |
| `token-sdk/src/pack.rs:304` | `PackedCTokenData<V> = PackedTokenDataWithVariant<V>` | `PackedLightTokenData<V>` |

### 1.6 Functions
| File | Current | Proposed |
|------|---------|----------|
| `sdk/src/compressible/traits.rs` | `into_ctoken_variant()` | `into_light_token_variant()` |
| `macros/src/rentfree/program/variant_enum.rs` | `into_ctoken_variant()` (generated) | `into_light_token_variant()` |

### 1.7 Constants
| File | Current | Proposed |
|------|---------|----------|
| `token-types/src/constants.rs:4` | `PROGRAM_ID: [u8; 32] = pubkey_array!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m")` | Keep (on-chain address) |
| `token-sdk/src/token/mod.rs:214` | `LIGHT_TOKEN_PROGRAM_ID: Pubkey = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m")` | Keep name, address is on-chain |

### 1.8 Field Names / Variables
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/compressed_token/v2/mint_action/cpi_accounts.rs:62` | `ctoken_accounts: &'a [A]` | `light_token_accounts` |
| `token-sdk/tests/mint_action_cpi_accounts_tests.rs:149,260` | `parsed.ctoken_accounts` | `parsed.light_token_accounts` |
| `token-sdk/src/token/decompress_mint.rs:341` | `ctoken_cpi_authority` | `light_token_cpi_authority` |
| `compressible-client/src/lib.rs:254` | `ctoken_variant` param | `light_token_variant` |
| `compressible-client/src/lib.rs:227-242` | `from_ctoken()` method | `from_light_token()` |

### 1.9 Generated Code / Macros
| File | Current | Proposed |
|------|---------|----------|
| `macros/src/rentfree/program/variant_enum.rs:79-80` | `PackedCTokenData`, `CTokenData` variants | `PackedLightTokenData`, `LightTokenData` |
| `macros/src/rentfree/program/variant_enum.rs:534` | `IntoCTokenVariant` impl | `IntoLightTokenVariant` |
| `macros/src/rentfree/traits/decompress_context.rs:120` | `PackedCTokenData` | `PackedLightTokenData` |
| `macros/src/rentfree/traits/decompress_context.rs:175,179` | `RentFreeAccountVariant::PackedCTokenData`, `CTokenData` | Rename variants |
| `macros/src/rentfree/accounts/parse.rs:25-28` | `CTokenConfig`, `CTokenRentSponsor`, `CTokenProgram`, `CTokenCpiAuthority` | `LightTokenConfig`, etc. |

### 1.10 Doc Comments (update terminology)
| File | Pattern |
|------|---------|
| `token-sdk/src/lib.rs:1-28` | "cToken SDK", "cToken Accounts", "cMints" → "Light Token SDK", "Light Token Accounts", "Light Mints" |
| `token-sdk/src/token/mod.rs:1-37` | "ctoken" references → "light token" |
| `token-sdk/src/token/transfer_to_spl.rs:17,55` | "ctoken" → "light token" |
| `token-sdk/src/token/transfer_from_spl.rs:16,55` | "ctoken" → "light token" |
| `token-sdk/src/token/transfer.rs:8,34` | "ctoken" → "light token" |
| `token-sdk/src/token/transfer_checked.rs:8,39` | "ctoken" → "light token" |
| `token-sdk/src/token/mint_to.rs:8,38` | "ctoken" → "light token" |
| `token-sdk/src/token/mint_to_checked.rs:8,41` | "ctoken" → "light token" |
| `token-sdk/src/token/burn.rs:8,38` | "ctoken" → "light token" |
| `token-sdk/src/token/burn_checked.rs:8,41` | "ctoken" → "light token" |
| `token-sdk/src/token/create_ata.rs:28,135,254` | "ctoken" → "light token" |
| `token-sdk/src/token/create.rs:14,87,166` | "ctoken" → "light token" |
| `token-sdk/src/token/close.rs:9,64,72` | "ctoken" → "light token" |
| `token-sdk/src/token/decompress.rs:24,66` | "cToken" → "Light Token" |
| `token-sdk/src/token/decompress_mint.rs:265-267` | "ctoken's config/rent sponsor" → "light token's" |
| `token-sdk/src/token/compressible.rs:7,30,78,84-85` | "ctoken" → "light token" |
| `token-sdk/src/spl_interface.rs:39` | "ctoken" → "light token" |
| `token-sdk/src/pack.rs:1,15` | "c-tokens", "ctoken-interface" → "light-tokens", "light-token-interface" |
| `token-sdk/src/error.rs:30` | "Ctoken::" → "LightToken::" |
| `token-types/Cargo.toml:5` | "ctoken and compressed token types" → "Light token types" |
| `token-client/src/instructions/mint_action.rs:50` | `MintToCToken` variant | `MintToLightToken` |
| `token-client/src/actions/transfer.rs:69` | "CTokenTransfer discriminator" → "LightTokenTransfer" |

### 1.11 CMint References
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/token/decompress_mint.rs:240-274` | `DecompressCMintWithCpiContext`, `cmint_pda`, `_cmint_bump` | `DecompressLightMintWithCpiContext`, `light_mint_pda` |
| `token-sdk/src/token/decompress_mint.rs:318-395` | `DecompressCMintCpiWithContext`, `cmint: AccountInfo` | `DecompressLightMintCpiWithContext`, `light_mint` |
| `token-sdk/src/compressed_token/v2/create_compressed_mint/account_metas.rs:6,46` | "cMint" → "Light Mint" |
| `compressible-client/src/decompress_mint.rs` (entire file) | `cmint`, `CMint` references | `light_mint`, `LightMint` |
| `compressible-client/src/load_accounts.rs:63-64,134` | `cmint` field | `light_mint` |
| `compressible-client/src/create_accounts_proof.rs:44,61` | "CMint" → "Light Mint" |
| `compressible-client/src/account_interface_ext.rs:55-96` | `cmint` variables | `light_mint` |
| `macros/src/rentfree/accounts/light_mint.rs:34,374,391` | "CMint", `_cmint_bump` | "LightMint", `_light_mint_bump` |
| `macros/src/rentfree/accounts/derive.rs:10,16-17` | "CMint" → "LightMint" |
| `macros/src/rentfree/accounts/builder.rs:183` | "CMint" → "LightMint" |

---

## 2. COMPRESSIBLE Renames → Weave into "light-"

### 2.1 Crate/Package Names
| Current | Proposed |
|---------|----------|
| `light-compressible-client` (compressible-client/) | `light-client-rentfree` or merge into `light-client` |
| `light-compressible` (program-libs/compressible/) | `light-rentfree` |

### 2.2 Directory/Module Names
| Current | Proposed |
|---------|----------|
| `sdk/src/compressible/` | `sdk/src/rentfree/` |
| `token-sdk/src/compressible/` | `token-sdk/src/rentfree/` |
| `token-sdk/src/token/compressible.rs` | `token-sdk/src/token/rentfree.rs` |
| `program-test/src/compressible.rs` | `program-test/src/rentfree.rs` |
| `program-test/src/accounts/compressible_config.rs` | `program-test/src/accounts/rentfree_config.rs` |
| `program-test/src/forester/compress_and_close_forester.rs` | Keep (compress is action verb) |
| `token-client/src/actions/create_compressible_token_account.rs` | `create_rentfree_token_account.rs` |
| `macros/src/rentfree/traits/light_compressible.rs` | `macros/src/rentfree/traits/rentfree_account.rs` |
| `macros/docs/traits/compressible.md` | `macros/docs/traits/rentfree.md` |
| `macros/docs/traits/compressible_pack.md` | `macros/docs/traits/rentfree_pack.md` |
| `macros/docs/traits/light_compressible.md` | `macros/docs/traits/rentfree_account.md` |
| `compressible-client/src/get_compressible_account.rs` | `get_rentfree_account.rs` |

### 2.3 Structs/Enums
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/token/compressible.rs:7-30` | `CompressibleParams` | `RentFreeParams` |
| `token-sdk/src/token/compressible.rs:78` | `CompressibleParamsCpi` | `RentFreeParamsCpi` |
| `token-sdk/src/token/create_ata.rs:4` | `CompressibleExtensionInstructionData` | `RentFreeExtensionInstructionData` |
| `token-sdk/src/token/create_associated_token_account.rs:6,24` | `CompressibleExtensionInstructionData`, `CreateCompressibleAssociatedTokenAccountInputs` | Rename |
| `sdk/src/compressible/config.rs` | `CompressibleConfig` | `RentFreeConfig` |
| `compressible-client/src/lib.rs:63-68` | `InitializeCompressionConfigData` | `InitializeRentFreeConfigData` |
| `compressible-client/src/lib.rs:70-75` | `UpdateCompressionConfigData` | `UpdateRentFreeConfigData` |
| `compressible-client/src/lib.rs:78-83` | `DecompressMultipleAccountsIdempotentData` | Keep or rename to match pattern |
| `compressible-client/src/lib.rs:85-90` | `CompressAccountsIdempotentData` | Keep or rename |
| `compressible-client/src/initialize_config.rs` | `InitializeRentFreeConfig` (already correct) | Keep |
| `compressible-client/src/get_compressible_account.rs` (filename) | | `get_rentfree_account.rs` |
| `token-client/src/actions/create_compressible_token_account.rs` | Functions in file | Rename file and functions |
| `program-test/src/accounts/compressible_config.rs` | `CompressibleConfigTestContext` | `RentFreeConfigTestContext` |

### 2.4 Functions
| File | Current | Proposed |
|------|---------|----------|
| `sdk/src/lib.rs:166-169` | `process_initialize_compression_config_*`, `process_update_compression_config` | `process_initialize_rentfree_config_*`, `process_update_rentfree_config` |
| `sdk/src/compressible/config.rs` | All `compression_config` functions | Rename to `rentfree_config` |
| `token-sdk/src/token/create_ata.rs:83` | `with_compressible()` | `with_rentfree()` |
| `token-sdk/src/token/create.rs:47` | `with_compressible()` | `with_rentfree()` |
| `token-sdk/src/token/create_associated_token_account.rs:45-80` | `create_compressible_associated_token_account*()` functions | `create_rentfree_associated_token_account*()` |
| `token-sdk/src/token/create_associated_token_account.rs:247-272` | `create_compressible_associated_token_account2*()` | Rename |
| `macros/src/rentfree/traits/light_compressible.rs:56` | `derive_rentfree_account()` (already correct) | Keep |

### 2.5 Constants
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/token/mod.rs:219` | `COMPRESSIBLE_CONFIG_V1` | `RENTFREE_CONFIG_V1` or `LIGHT_TOKEN_RENTFREE_CONFIG` |
| `sdk/src/compressible/config.rs` | `COMPRESSIBLE_CONFIG_SEED` | `RENTFREE_CONFIG_SEED` |

### 2.6 Trait Names
| File | Current | Proposed |
|------|---------|----------|
| `sdk/src/compressible/compression_info.rs` | `HasCompressionInfo` | `HasRentFreeInfo` or keep (compression is accurate) |
| `sdk/src/compressible/compression_info.rs:244` | `CompressedAccountData<T>` | Keep (describes compressed state) |
| `macros/src/lib.rs:124` | `HasCompressionInfo` derive | Keep or rename |
| `macros/src/lib.rs:164` | `CompressAs` derive | Keep |
| `macros/src/lib.rs:257` | `Compressible` derive | `RentFree` derive |
| `macros/src/lib.rs:284` | `CompressiblePack` derive | `RentFreePack` |
| `macros/src/lib.rs:334` | `RentFreeAccount` derive (already correct) | Keep |

### 2.7 Field Names
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/token/create_ata.rs:47` | `compressible: CompressibleParams` | `rentfree: RentFreeParams` |
| `token-sdk/src/token/decompress_mint.rs:141-142,265-266,327-328` | `compressible_config` | `rentfree_config` |
| `token-sdk/src/token/create_mint.rs:230-231` | `compressible_config` | `rentfree_config` |
| Various CPI structs | `compressible_config` fields | `rentfree_config` |

### 2.8 Instruction Discriminators
| File | Current | Proposed |
|------|---------|----------|
| `compressible-client/src/lib.rs:285-296` | `INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR`, `UPDATE_COMPRESSION_CONFIG_DISCRIMINATOR`, `DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR`, `COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR` | Rename to `RENTFREE_*` pattern |

### 2.9 Feature Flags
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/Cargo.toml:12` | `compressible = ["cpi-context"]` | `rentfree = ["cpi-context"]` |
| `token-sdk/tests/pack_test.rs:1` | `#![cfg(feature = "compressible")]` | `#![cfg(feature = "rentfree")]` |

---

## 3. COMPRESSED_ Prefix (Legacy - Document for Awareness)

These use `compressed_` prefix and represent legacy naming. Lower priority to rename but documenting for completeness.

### 3.1 Type Names (Widely Used)
| File | Name | Notes |
|------|------|-------|
| `sdk-types/src/instruction/account_meta.rs` | `CompressedAccountMeta`, `CompressedAccountMetaNoLamportsNoAddress`, `CompressedAccountMetaNoAddress`, `CompressedAccountMetaTrait`, `CompressedAccountMetaBurn`, `CompressedAccountMetaReadOnly` | Core SDK types |
| `sdk/src/compressible/compression_info.rs:244` | `CompressedAccountData<T>` | |
| `token-types/src/instruction/transfer.rs:2` | `CompressedProof`, `CompressedCpiContext` | From light-compressed-account |
| `token-sdk/src/lib.rs:78` | Re-exports `CompressedProof`, `ValidityProof` | |

### 3.2 Function Names
| File | Name |
|------|------|
| `token-sdk/src/token/create_mint.rs:367` | `derive_mint_compressed_address()` |
| `token-sdk/src/compressed_token/v2/update_compressed_mint/*.rs` | `update_compressed_mint()`, `update_compressed_mint_cpi()`, `create_update_compressed_mint_cpi_write()`, `get_update_compressed_mint_instruction_account_metas()` |
| `token-sdk/src/compressed_token/v2/mint_to_compressed/*.rs` | `create_mint_to_compressed_instruction()`, `get_mint_to_compressed_instruction_account_metas()` |
| `token-sdk/src/compressed_token/v2/create_compressed_mint/*.rs` | Module and functions |

### 3.3 Module/Directory Names
| Current | Notes |
|---------|-------|
| `token-sdk/src/compressed_token/` | Main module for compressed token operations |
| `token-sdk/src/compressed_token/v2/create_compressed_mint/` | |
| `token-sdk/src/compressed_token/v2/update_compressed_mint/` | |
| `token-sdk/src/compressed_token/v2/mint_to_compressed/` | |
| `token-types/src/account_infos/create_compressed_mint.rs` | |
| `token-types/src/account_infos/mint_to_compressed.rs` | |
| `token-types/src/instruction/update_compressed_mint.rs` | |

### 3.4 Constants
| File | Name |
|------|------|
| `token-sdk/src/compressed_token/v2/update_compressed_mint/instruction.rs:21` | `UPDATE_COMPRESSED_MINT_DISCRIMINATOR` |
| `token-sdk/src/compressed_token/v2/mint_to_compressed/instruction.rs:14` | `MINT_TO_COMPRESSED_DISCRIMINATOR` |

### 3.5 Struct Names
| File | Name |
|------|------|
| `token-sdk/src/compressed_token/v2/mint_to_compressed/instruction.rs:28` | `MintToCompressedInputs` |
| `token-types/src/account_infos/mint_to_compressed.rs:7` | `DecompressedMintConfig` |
| `token-sdk/src/compressed_token/v2/mint_to_compressed/mod.rs:8` | `MintToCompressedMetaConfig` |
| `client/src/indexer/types.rs:4,645` | `CompressedAccount`, `CompressedAccountData` |

### 3.6 Field Names
| Pattern | Example Files |
|---------|---------------|
| `compressed_token_program` | `token-sdk/src/utils.rs:90`, `token-sdk/src/compressed_token/v2/transfer2/cpi_accounts.rs` |
| `compressed_token_cpi_authority` | `token-sdk/src/compressed_token/v2/transfer2/cpi_accounts.rs:25,62-63` |
| `compressed_accounts` | Multiple files in compressible-client |
| `compressed_mint_*` | `token-sdk/src/compressed_token/v2/update_compressed_mint/instruction.rs:26,97` |
| `compress_or_decompress_*` | `token-types/src/instruction/transfer.rs:77`, `token-sdk/src/compressed_token/v1/transfer/*` |
| `output_compressed_accounts` | `token-sdk/src/compressed_token/v2/transfer2/instruction.rs:95` |

---

## 4. Additional Cleanup Items

### 4.1 Inconsistent Naming
| Current | Issue | Proposed |
|---------|-------|----------|
| `LIGHT_TOKEN_PROGRAM_ID` vs `COMPRESSED_TOKEN_PROGRAM_ID` | Both exist | Standardize to `LIGHT_TOKEN_PROGRAM_ID` |
| `token-sdk/src/token/create_associated_token_account.rs:224,234,237` | Uses `COMPRESSED_TOKEN_PROGRAM_ID` | Use `LIGHT_TOKEN_PROGRAM_ID` |

### 4.2 Test File Renames
| Current | Proposed |
|---------|----------|
| `token-sdk/tests/pack_test.rs` | Update feature flag |
| `token-sdk/tests/mint_action_cpi_accounts_tests.rs` | Update variable names |

### 4.3 Error Messages
| File | Current | Proposed |
|------|---------|----------|
| `token-sdk/src/error.rs:30` | "Ctoken::transfer, compress, or decompress..." | "LightToken::transfer..." |
| `macros/src/rentfree/program/compress.rs:220` | `CTokenDecompressionNotImplemented` | `LightTokenDecompressionNotImplemented` |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| ctoken renames | ~60 items |
| compressible renames | ~40 items |
| compressed_ (legacy) | ~50 items |
| **Total** | **~150 items** |

---

## Migration Priority

1. **High Priority**: Public API (trait names, struct names, function names)
2. **Medium Priority**: Internal module names, file names
3. **Low Priority**: Doc comments, legacy `compressed_` patterns, test files
