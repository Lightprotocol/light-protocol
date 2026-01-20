# Rebase Resolution Notes

## Context

Rebasing `swen/clean-decompress-base` onto `main`.

## Main Branch Changes (affecting this rebase)

### 1. Package Renames

- `ctoken-sdk` → `token-sdk`
- `light_token` → `light_token`
- `ctoken` module → `token` module in APIs
- `sdk-ctoken-test` → `sdk-light-token-test`
- `light_token_interface` → `light_token_interface`
- Type names: `CToken` → `Token` in some places

### 2. Deleted Tests/Directories in Main

Main deleted these that HEAD modified:

- `sdk-tests/csdk-anchor-derived-test/` - Deleted in main
- `sdk-tests/sdk-compressible-test/` - Deleted in main
- `sdk-libs/macros/src/compressible/GUIDE.md` - Deleted in main

### 3. User's Branch (HEAD) Changes to Preserve

- **Phase 8 refactor**: `TokenSeedProvider` trait simplified - no accounts struct needed
- Seed pubkeys embedded directly in enum variants
- `HasTokenVariant::is_packed_token()` → `is_packed_ctoken()` in some places
- Various API simplifications in decompress_runtime

## Resolution Decisions

### Content Conflicts

1. **sdk-libs/macros/src/compressible/instructions.rs**
   - Keep user's Phase 8 changes (simplified API)
   - Use main's naming (`light_token` not `light_token`)
   - Resolution: Take HEAD's code, update package names to main's convention

2. **sdk-libs/macros/src/compressible/decompress_context.rs**
   - Keep user's `RentFreeAccountData` naming
   - Use `light_token::compat::PackedCTokenData` (main's package name)
   - Fix trait method naming to match

3. **sdk-libs/macros/src/compressible/seed_providers.rs**
   - Keep user's Phase 8 implementation (simpler trait)
   - Update imports to `light_token`

4. **sdk-libs/macros/src/compressible/variant_enum.rs**
   - Keep user's variant structure with idx fields
   - Use `light_token::compat::*` imports

5. **sdk-libs/sdk/src/compressible/decompress_runtime.rs**
   - Keep user's simplified `TokenSeedProvider` trait (no accounts struct)
   - Update to `ctoken_program()` accessor name

6. **sdk-libs/token-sdk/src/compressible/decompress_runtime.rs**
   - Keep user's implementation with `TokenSeedProvider` re-export
   - Fix variable names (`token_accounts` vs `ctoken_accounts`)

7. **sdk-libs/token-sdk/src/pack.rs**
   - Keep both main's and user's Pack impls (they're compatible)
   - Use `light_token_interface` imports

8. **sdk-libs/token-sdk/src/token/create.rs, create_ata.rs**
   - Keep user's new builder pattern APIs
   - Use main's `light_token_interface` imports

9. **sdk-libs/program-test/src/compressible.rs**
   - Keep user's `compression_only` field addition
   - Use `CToken` type with zerocopy (main's approach for Token parsing)

10. **Cargo.toml**
    - Keep main's member list (sdk-light-token-test)
    - Add back sdk-compressible-test and csdk-anchor-derived-test if still needed
    - Resolution: User tests appear consolidated - use main's list

### Modify/Delete Conflicts

1. **sdk-tests/csdk-anchor-derived-test/\*** - DELETE (main removed, tests moved to csdk-anchor-full-derived-test)
2. **sdk-tests/sdk-compressible-test/\*** - DELETE (main removed, functionality consolidated)
3. **sdk-libs/macros/src/compressible/GUIDE.md** - DELETE (main removed docs)

### Test Files

- **csdk-anchor-full-derived-test** - Keep user's changes, update imports to `light_token` → `light_token`

## Confidence Level

- **High confidence**: Package renames are mechanical
- **High confidence**: Phase 8 API simplifications are the user's intended changes
- **Medium confidence**: Deleted test directories - assuming main's consolidation is correct
- **Note**: Variable naming (`token_accounts` vs `ctoken_accounts`) - using main's `token_` prefix consistently
