# Implementation Details

Details that remain the same as the current implementation.

---

## Discriminator

Uses existing `#[derive(LightDiscriminator)]` implementation (Anchor-style).

---

## Error Handling

Uses `anchor_lang::error::Error` and `ProgramError`:
- `LightPreInit` returns `Result<()>` with Anchor error type
- `LightFinalize` returns `Result<()>` with Anchor error type
- SDK functions use `.map_err()` for error conversion

---

## Seed Verification (Decompress)

At decompress time:
1. Seeds are reconstructed from packed variant + remaining_accounts
2. PDA is derived using `Pubkey::find_program_address(&seeds, program_id)`
3. Derived PDA must match the account being decompressed
4. Verification failure returns error

---

## Client Seed Helpers

The `PackedLightAccountVariant` provides:
- `seed_refs_with_bump()` - Returns seed slice with bump for CPI signing
- Seeds can be reconstructed by unpacking indices from remaining_accounts

---

## Size Validation

Compressed accounts must fit within 800 bytes. Validated at:
- Compile-time: `const _: () = assert!(T::INIT_SPACE <= 800);`
- Runtime: SDK functions validate before Merkle tree insertion

---

## Nested Field Access in Seeds

Supports one level of nesting:
- `params.owner` - extracts `owner` into Seeds struct
- `params.config.owner` - extracts `owner` (terminal field)
- Deeper nesting follows same pattern (terminal field extracted)

Expression suffixes stripped:
- `params.owner.as_ref()` → `owner`
- `authority.key().as_ref()` → `authority`

---

## Zero-Copy Pod Accounts

Requirements:
- Data struct must implement `Pod` from `bytemuck`
- Data struct must also implement `BorshSerialize` and `BorshDeserialize`
- Use `AccountLoader<'info, T>` instead of `Account<'info, T>`
- Add `zero_copy` to light_account attribute: `#[light_account(init, zero_copy)]`

Serialization:
- Pack/unpack uses borsh (same as regular accounts)
- On-chain account uses zero-copy Pod layout

---

## Multiple PDAs in Same Accounts Struct

Supported. Each PDA field:
1. Gets its own `CompressedAccountInfo`
2. Collected into `compressed_accounts` vec in order of declaration
3. Initialized in declaration order
4. `num_pdas` counter tracks position for mint offset

---

## `#[compress_as(field = value)]` Attribute

Overrides field values in compressed/hashed representation.

Allowed values:
- Literals: `0`, `None`, `false`, `""`
- Constants: `DEFAULT_VALUE`
- Not allowed: `self.field` references, function calls

Auto-skipped fields:
- `compression_info` always excluded from hash
- Fields with `#[skip]` excluded

---

## `#[skip]` Attribute

Excludes field from:
- Hash computation
- Pack/unpack (field not in packed struct)
- Size calculation

Cannot skip required fields. `compression_info` is auto-handled.

---

## Constants in Seeds

Uppercase identifiers treated as constants.

Supported:
- Local constants: `SEED`
- Qualified paths: `crate::seeds::VAULT_SEED`
- Module paths: `seeds::MY_SEED`

Constants go directly into `seed_refs()`, not into Seeds struct.
