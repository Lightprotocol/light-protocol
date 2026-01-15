# Unified Decompress Specification v3 (Final)

## Implementation Status

| Component                        | Status | Notes                                                                           |
| -------------------------------- | ------ | ------------------------------------------------------------------------------- |
| `LightAta` type                  | DONE   | `sdk-libs/sdk/src/compressible/standard_types.rs` (re-exported from ctoken-sdk) |
| `LightMint` type                 | DONE   | `sdk-libs/sdk/src/compressible/standard_types.rs` (re-exported from ctoken-sdk) |
| Error variants                   | DONE   | `AtMostOneMintAllowed`, `MintAndTokensForbidden` in `sdk/src/error.rs`          |
| `CompressedAccountVariant` enum  | DONE   | Includes `LightAta`, `LightMint` variants in `variant_enum.rs`                  |
| `HasTokenVariant` detection      | DONE   | Detects both standard and legacy types                                          |
| Runtime validation               | DONE   | Constraint checks in `decompress_runtime.rs`                                    |
| Trait extension                  | DONE   | `collect_all_accounts` returns 5-tuple, new `process_light_*` methods           |
| Client `DecompressInput` enum    | DONE   | `compressible-client/src/lib.rs`                                                |
| Runtime processing for LightAta  | STUB   | Returns error - use `PackedCTokenData` with `is_ata` for now                    |
| Runtime processing for LightMint | STUB   | Returns error - use `CompressedMintData` for now                                |

---

## Executive Summary

Extend `decompress_accounts_idempotent` to handle **all four account types** using **standard SDK types** (`LightAta`, `LightMint`) for ATAs and Mints. Programs only declare their PDAs and program-owned CToken accounts (Vaults).

**Critical Constraint**: `compress_accounts_idempotent` does NOT support ATA/Mint compression - those are compressed by the forester invoking the ctoken program directly.

---

## 1. Account Type Taxonomy

| Type          | Declaration                                | SDK Type                      | Owner          | Signing                       | Limit |
| ------------- | ------------------------------------------ | ----------------------------- | -------------- | ----------------------------- | ----- |
| **cPDA**      | `#[compressible(Foo = (...))]`             | Program-generated             | Program        | Program PDA seeds             | Any # |
| **CToken**    | `#[compressible(Vault = (is_token, ...))]` | Program-generated             | Program        | Program PDA seeds (authority) | Any # |
| **LightAta**  | NOT declared - always available            | `light_ctoken_sdk::LightAta`  | User wallet    | Wallet signs tx               | Any # |
| **LightMint** | NOT declared - always available            | `light_ctoken_sdk::LightMint` | ctoken program | Authority signs               | Max 1 |

---

## 2. Constraints (Enforced at Runtime)

| Constraint                                 | Error                    | Rationale                                            |
| ------------------------------------------ | ------------------------ | ---------------------------------------------------- |
| Max 1 LightMint per instruction            | `AtMostOneMintAllowed`   | CMint decompression creates on-chain state           |
| LightMint + (LightAta OR CToken) forbidden | `MintAndTokensForbidden` | Both modify on-chain state, CPI context conflicts    |
| LightMint + cPDA allowed                   | -                        | PDAs use CPI context write, mint uses different path |
| Any combo of LightAta + CToken + cPDA      | -                        | All can share CPI context                            |

---

## 3. Current Architecture (What Exists)

### 3.1 Macro Declaration (`#[compressible(...)]`)

```rust
#[compressible(
    // PDAs - program-specific types
    UserRecord = ("user_record", ctx.authority, data.owner),
    GameSession = ("game_session", ctx.user, data.session_id.to_le_bytes()),

    // Program-owned CTokens (Vaults) - require authority seeds
    Vault = (is_token, "vault", ctx.cmint, authority = ("vault_authority")),

    // Data fields for seed params
    owner = Pubkey,
    session_id = u64,
)]
pub mod instructions { ... }
```

### 3.2 Generated `CompressedAccountVariant` Enum (variant_enum.rs)

```rust
pub enum CompressedAccountVariant {
    // Program-specific PDAs (from declaration)
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),
    GameSession(GameSession),
    PackedGameSession(PackedGameSession),

    // ALWAYS included (not from declaration):
    PackedCTokenData(PackedCTokenData<CTokenAccountVariant>),
    CTokenData(CTokenData<CTokenAccountVariant>),
    CompressedMint(CompressedMintData),
}
```

### 3.3 Generated `CTokenAccountVariant` Enum (seed_providers.rs)

Only includes variants declared with `is_token`:

```rust
#[repr(u8)]
pub enum CTokenAccountVariant {
    Vault = 0,
}
```

Each variant implements `CTokenSeedProvider` (get_seeds, get_authority_seeds, is_ata).

### 3.4 Runtime Flow (decompress_runtime.rs)

```
process_decompress_accounts_idempotent()
    |
    +-> check_account_types() -> (has_tokens, has_pdas, has_mints)
    |
    +-> ctx.collect_all_accounts() -> (pda_infos, token_accounts, mint_accounts)
    |
    +-> if has_pdas: process PDAs via Light System CPI
    |
    +-> if has_mints: ctx.process_mints()
    |
    +-> if has_tokens: ctx.process_tokens()
```

### 3.5 Token Runtime (ctoken-sdk/decompress_runtime.rs)

`process_decompress_tokens_runtime` already handles both:

- Program-owned tokens: uses `get_seeds()`, `get_authority_seeds()` from variant
- ATAs (is_ata=true): wallet owner signs, uses `derive_ctoken_ata()`

---

## 4. Design: Standard Types (LightAta, LightMint)

### 4.1 New Types (light-ctoken-sdk)

Location: `sdk-libs/ctoken-sdk/src/compressible/standard_types.rs`

```rust
/// Standard ATA for unified decompression.
/// Wallet must sign the transaction.
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightAta {
    /// Index into packed_accounts for wallet (signer)
    pub wallet_index: u8,
    /// Index into packed_accounts for mint
    pub mint_index: u8,
    /// Index into packed_accounts for derived ATA address
    pub ata_index: u8,
    /// Token amount
    pub amount: u64,
    /// Has delegate
    pub has_delegate: bool,
    /// Delegate index (if has_delegate)
    pub delegate_index: u8,
    /// Is frozen
    pub is_frozen: bool,
}

/// Standard CMint for unified decompression.
/// CMint authority must sign (or be fee_payer).
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightMint {
    /// Index into packed_accounts for mint_seed pubkey
    pub mint_seed_index: u8,
    /// Index into packed_accounts for derived CMint PDA
    pub cmint_pda_index: u8,
    /// Has mint authority
    pub has_mint_authority: bool,
    /// Mint authority index
    pub mint_authority_index: u8,
    /// Has freeze authority
    pub has_freeze_authority: bool,
    /// Freeze authority index
    pub freeze_authority_index: u8,
    /// Decimals
    pub decimals: u8,
    /// Total supply
    pub supply: u64,
    /// Extensions (if any)
    pub extensions: Option<Vec<u8>>,
    /// Rent payment for CMint account
    pub rent_payment: u64,
    /// Write top-up
    pub write_top_up: u32,
}
```

### 4.2 Updated `CompressedAccountVariant` (variant_enum.rs)

```rust
pub enum CompressedAccountVariant {
    // Program-specific PDAs (from declaration)
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),

    // Program-owned CTokens (Vaults) - if declared
    PackedCTokenData(PackedCTokenData<CTokenAccountVariant>),
    CTokenData(CTokenData<CTokenAccountVariant>),

    // ALWAYS included - standard SDK types
    LightAta(light_ctoken_sdk::compressible::LightAta),
    LightMint(light_ctoken_sdk::compressible::LightMint),

    // Legacy (for backward compat with existing CompressedMint variant)
    CompressedMint(CompressedMintData),
}
```

### 4.3 Removed: `#[ata]` Attribute

Old (REMOVED):

```rust
// OLD - no longer supported
UserAta = (is_token, is_ata, ctx.wallet, ctx.cmint),
```

New:

- ATAs use `LightAta` standard type
- No declaration needed
- `CTokenAccountVariant` is ONLY for program-owned tokens (Vaults)

---

## 5. Implementation Details

### 5.1 Macro Changes (variant_enum.rs)

**File**: `sdk-libs/macros/src/compressible/variant_enum.rs`

Add `LightAta` and `LightMint` variants unconditionally:

```rust
let enum_def = quote! {
    #[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
    pub enum CompressedAccountVariant {
        #(#account_variants)*
        PackedCTokenData(light_ctoken_sdk::compat::PackedCTokenData<CTokenAccountVariant>),
        CTokenData(light_ctoken_sdk::compat::CTokenData<CTokenAccountVariant>),
        // Standard types - ALWAYS included
        LightAta(light_ctoken_sdk::compressible::LightAta),
        LightMint(light_ctoken_sdk::compressible::LightMint),
        // Legacy (kept for backward compat)
        CompressedMint(light_ctoken_sdk::compat::CompressedMintData),
    }
};
```

Update `HasTokenVariant` impl to detect `LightAta`:

```rust
impl HasTokenVariant for CompressedAccountData {
    fn is_packed_ctoken(&self) -> bool {
        matches!(self.data,
            CompressedAccountVariant::PackedCTokenData(_) |
            CompressedAccountVariant::LightAta(_)  // LightAta is a token
        )
    }

    fn is_compressed_mint(&self) -> bool {
        matches!(self.data,
            CompressedAccountVariant::CompressedMint(_) |
            CompressedAccountVariant::LightMint(_)  // LightMint is a mint
        )
    }
}
```

### 5.2 Runtime Validation (decompress_runtime.rs)

**File**: `sdk-libs/sdk/src/compressible/decompress_runtime.rs`

Add validation in `process_decompress_accounts_idempotent`:

```rust
// Enhanced check_account_types with mint count
pub fn check_account_types_with_count<T: HasTokenVariant>(
    compressed_accounts: &[T]
) -> (bool, bool, bool, usize) {
    let (mut has_tokens, mut has_pdas, mut has_mints) = (false, false, false);
    let mut mint_count = 0;

    for account in compressed_accounts {
        if account.is_packed_ctoken() {
            has_tokens = true;
        } else if account.is_compressed_mint() {
            has_mints = true;
            mint_count += 1;
        } else {
            has_pdas = true;
        }
    }
    (has_tokens, has_pdas, has_mints, mint_count)
}

// In process_decompress_accounts_idempotent:
pub fn process_decompress_accounts_idempotent<'info, Ctx>(...) -> Result<(), ProgramError>
where Ctx: DecompressContext<'info>
{
    let (has_tokens, has_pdas, has_mints, mint_count) =
        check_account_types_with_count(&compressed_accounts);

    // CONSTRAINT 1: Max 1 mint
    if mint_count > 1 {
        msg!("At most 1 LightMint allowed per instruction, found {}", mint_count);
        return Err(LightSdkError::AtMostOneMintAllowed.into());
    }

    // CONSTRAINT 2: Mint + tokens forbidden
    if has_mints && has_tokens {
        msg!("LightMint + (LightAta/CToken) combination is forbidden");
        return Err(LightSdkError::MintAndTokensForbidden.into());
    }

    // ... rest of processing
}
```

### 5.3 DecompressContext Changes (decompress_context.rs)

**File**: `sdk-libs/macros/src/compressible/decompress_context.rs`

Update `collect_all_accounts` to handle `LightAta` and `LightMint`:

```rust
fn collect_all_accounts<'b>(...) -> Result<(
    Vec<CompressedAccountInfo>,
    Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
    Vec<(Self::CompressedMintData, Self::CompressedMeta)>,
), ProgramError> {
    // ... existing PDA handling ...

    for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
        let meta = compressed_data.meta;
        match compressed_data.data {
            // Existing PDA match arms...
            #(#pda_match_arms)*

            // Program-owned tokens (Vaults)
            CompressedAccountVariant::PackedCTokenData(mut data) => {
                data.token_data.version = 3;
                compressed_token_accounts.push((data, meta));
            }

            // Standard LightAta - convert to PackedCTokenData format
            CompressedAccountVariant::LightAta(light_ata) => {
                // LightAta is handled separately in process_tokens
                // We need a marker to distinguish it from program-owned tokens
                compressed_light_atas.push((light_ata, meta));
            }

            // Standard LightMint
            CompressedAccountVariant::LightMint(light_mint) => {
                compressed_light_mints.push((light_mint, meta));
            }

            // Legacy CompressedMint
            CompressedAccountVariant::CompressedMint(data) => {
                compressed_mint_accounts.push((data, meta));
            }

            CompressedAccountVariant::CTokenData(_) => unreachable!(),
        }
    }
    // ...
}
```

### 5.4 Token Runtime: LightAta Handler (ctoken-sdk)

**File**: `sdk-libs/ctoken-sdk/src/compressible/decompress_runtime.rs`

Add handler for `LightAta` in `process_decompress_tokens_runtime`:

```rust
// Process LightAta accounts
for (light_ata, meta) in light_atas.into_iter() {
    let wallet_info = &packed_accounts[light_ata.wallet_index as usize];
    let mint_info = &packed_accounts[light_ata.mint_index as usize];
    let ata_account = &packed_accounts[light_ata.ata_index as usize];

    // Validate wallet is signer
    if !wallet_info.is_signer {
        msg!("Wallet must be signer for LightAta decompression");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive ATA and verify
    let (derived_ata, bump) = derive_ctoken_ata(wallet_info.key, mint_info.key);
    if derived_ata != *ata_account.key {
        msg!("LightAta address mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Create ATA idempotently
    CreateAssociatedCTokenAccountCpi {
        payer: fee_payer.clone(),
        associated_token_account: ata_account.clone(),
        owner: wallet_info.clone(),
        mint: mint_info.clone(),
        // ... compression_only: true, idempotent: true
    }.invoke()?;

    // Build decompress indices...
}
```

### 5.5 Mint Runtime: LightMint Handler (ctoken-sdk)

**File**: `sdk-libs/ctoken-sdk/src/compressible/decompress_runtime.rs`

Add handler for `LightMint` in `process_decompress_mints_runtime`:

```rust
// Process LightMint accounts
for (light_mint, meta) in light_mints.into_iter() {
    let mint_seed_info = &packed_accounts[light_mint.mint_seed_index as usize];
    let cmint_info = &packed_accounts[light_mint.cmint_pda_index as usize];

    // Derive CMint PDA and verify
    let (derived_cmint, _) = find_cmint_address(mint_seed_info.key);
    if derived_cmint != *cmint_info.key {
        msg!("LightMint CMint PDA mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Build CompressedMintWithContext from LightMint data
    let compressed_mint_with_context = CompressedMintWithContext {
        mint_seed_pubkey: *mint_seed_info.key,
        base: CompressedMintBase {
            mint_authority: if light_mint.has_mint_authority {
                Some(packed_accounts[light_mint.mint_authority_index as usize].key.into())
            } else { None },
            freeze_authority: if light_mint.has_freeze_authority {
                Some(packed_accounts[light_mint.freeze_authority_index as usize].key.into())
            } else { None },
            decimals: light_mint.decimals,
            supply: light_mint.supply,
        },
        extensions: light_mint.extensions.clone(),
        // ... tree context from meta
    };

    // Invoke DecompressCMint CPI...
}
```

---

## 6. Client-Side Implementation

**File**: `sdk-libs/compressible-client/src/lib.rs`

### 6.1 New Input Enum

```rust
/// Input for decompress_accounts_idempotent supporting all account types.
pub enum DecompressInput<T> {
    /// Program-specific PDA or program-owned CToken
    Standard {
        compressed_account: CompressedAccount,
        data: T,
    },
    /// Standard ATA (user-owned token account)
    LightAta {
        /// Compressed token account from indexer
        compressed_account: CompressedTokenAccount,
        /// Wallet that owns this ATA (must sign transaction)
        wallet: Pubkey,
    },
    /// Standard CMint (compressed mint)
    LightMint {
        /// Compressed mint account from indexer
        compressed_account: CompressedAccount,
        /// Mint seed pubkey (for CMint PDA derivation)
        mint_seed_pubkey: Pubkey,
    },
}
```

### 6.2 Client-Side Validation

```rust
fn validate_inputs<T>(inputs: &[DecompressInput<T>]) -> Result<(), CompressibleClientError> {
    let mut mint_count = 0;
    let mut has_tokens = false;

    for input in inputs {
        match input {
            DecompressInput::LightMint { .. } => {
                mint_count += 1;
            }
            DecompressInput::LightAta { .. } => {
                has_tokens = true;
            }
            DecompressInput::Standard { compressed_account, .. } => {
                if compressed_account.owner == C_TOKEN_PROGRAM_ID.into() {
                    has_tokens = true;
                }
            }
        }
    }

    // Constraint 1: Max 1 mint
    if mint_count > 1 {
        return Err(CompressibleClientError::AtMostOneMintAllowed);
    }

    // Constraint 2: Mint + tokens forbidden
    if mint_count > 0 && has_tokens {
        return Err(CompressibleClientError::MintAndTokensForbidden);
    }

    Ok(())
}
```

### 6.3 Updated `decompress_accounts_idempotent`

```rust
/// Builds decompress_accounts_idempotent instruction handling all account types.
#[allow(clippy::too_many_arguments)]
pub fn decompress_accounts_idempotent<T>(
    program_id: &Pubkey,
    discriminator: &[u8],
    decompressed_account_addresses: &[Pubkey],
    inputs: &[DecompressInput<T>],
    program_account_metas: &[AccountMeta],
    validity_proof_with_context: ValidityProofWithContext,
) -> Result<Instruction, CompressibleClientError>
where
    T: Pack + Clone + std::fmt::Debug,
{
    if inputs.is_empty() {
        return Err(CompressibleClientError::EmptyInputs);
    }

    // Validate constraints
    validate_inputs(inputs)?;

    // Detect account types for CPI context decision
    let (has_pdas, has_mints, has_tokens) = detect_account_types(inputs);

    let mut remaining_accounts = PackedAccounts::default();

    // Setup CPI context if needed (2+ different types)
    let type_count = has_pdas as u8 + has_mints as u8 + has_tokens as u8;
    if type_count >= 2 {
        let cpi_context = get_cpi_context_from_inputs(inputs)?;
        let system_config = SystemAccountMetaConfig::new_with_cpi_context(*program_id, cpi_context);
        remaining_accounts.add_system_accounts_v2(system_config)?;
    } else {
        let system_config = SystemAccountMetaConfig::new(*program_id);
        remaining_accounts.add_system_accounts_v2(system_config)?;
    }

    // Pack output queue
    let first_tree_info = get_first_tree_info(inputs)?;
    let output_queue = get_output_queue(&first_tree_info);
    let output_state_tree_index = remaining_accounts.insert_or_get(output_queue);

    // Pack tree infos from validity proof
    let packed_tree_infos = validity_proof_with_context.pack_tree_infos(&mut remaining_accounts);
    let packed_tree_infos_slice = &packed_tree_infos.state_trees.as_ref().unwrap().packed_tree_infos;

    // Pack each input
    let mut compressed_accounts = Vec::with_capacity(inputs.len());

    for (i, input) in inputs.iter().enumerate() {
        let tree_info = packed_tree_infos_slice.get(i)
            .ok_or(CompressibleClientError::TreeInfoMismatch)?;

        match input {
            DecompressInput::Standard { compressed_account, data } => {
                remaining_accounts.insert_or_get(compressed_account.tree_info.queue);
                let packed_data = data.pack(&mut remaining_accounts);
                compressed_accounts.push(CompressedAccountData {
                    meta: CompressedAccountMetaNoLamportsNoAddress {
                        tree_info: *tree_info,
                        output_state_tree_index,
                    },
                    data: CompressedAccountVariant::from_packed(packed_data),
                });
            }

            DecompressInput::LightAta { compressed_account, wallet } => {
                remaining_accounts.insert_or_get(compressed_account.tree_info.queue);

                // Insert wallet as signer
                let wallet_index = remaining_accounts.insert_or_get_config(*wallet, true, false);

                // Get mint from token data
                let mint = compressed_account.token.mint;
                let mint_index = remaining_accounts.insert_or_get_read_only(mint);

                // Derive and insert ATA address
                let (ata, _) = get_associated_ctoken_address_and_bump(wallet, &mint);
                let ata_index = remaining_accounts.insert_or_get(ata);

                let light_ata = LightAta {
                    wallet_index,
                    mint_index,
                    ata_index,
                    amount: compressed_account.token.amount,
                    has_delegate: compressed_account.token.delegate.is_some(),
                    delegate_index: compressed_account.token.delegate.map(|d|
                        remaining_accounts.insert_or_get_read_only(d)
                    ).unwrap_or(0),
                    is_frozen: compressed_account.token.state == AccountState::Frozen,
                };

                compressed_accounts.push(CompressedAccountData {
                    meta: CompressedAccountMetaNoLamportsNoAddress {
                        tree_info: *tree_info,
                        output_state_tree_index,
                    },
                    data: CompressedAccountVariant::LightAta(light_ata),
                });
            }

            DecompressInput::LightMint { compressed_account, mint_seed_pubkey } => {
                remaining_accounts.insert_or_get(compressed_account.tree_info.queue);

                // Insert mint_seed
                let mint_seed_index = remaining_accounts.insert_or_get_read_only(*mint_seed_pubkey);

                // Derive and insert CMint PDA
                let (cmint_pda, _) = find_cmint_address(mint_seed_pubkey);
                let cmint_pda_index = remaining_accounts.insert_or_get(cmint_pda);

                // Parse mint data from compressed account
                let mint_data: CompressedMint = borsh::BorshDeserialize::deserialize(
                    &mut &compressed_account.data.as_ref().unwrap().data[..]
                )?;

                let has_mint_authority = mint_data.base.mint_authority.is_some();
                let mint_authority_index = mint_data.base.mint_authority.map(|auth|
                    remaining_accounts.insert_or_get_read_only(auth.into())
                ).unwrap_or(0);

                let has_freeze_authority = mint_data.base.freeze_authority.is_some();
                let freeze_authority_index = mint_data.base.freeze_authority.map(|auth|
                    remaining_accounts.insert_or_get_read_only(auth.into())
                ).unwrap_or(0);

                let light_mint = LightMint {
                    mint_seed_index,
                    cmint_pda_index,
                    has_mint_authority,
                    mint_authority_index,
                    has_freeze_authority,
                    freeze_authority_index,
                    decimals: mint_data.base.decimals,
                    supply: mint_data.base.supply,
                    extensions: mint_data.extensions.clone(),
                    rent_payment: DEFAULT_RENT_PAYMENT,
                    write_top_up: DEFAULT_WRITE_TOP_UP,
                };

                compressed_accounts.push(CompressedAccountData {
                    meta: CompressedAccountMetaNoLamportsNoAddress {
                        tree_info: *tree_info,
                        output_state_tree_index,
                    },
                    data: CompressedAccountVariant::LightMint(light_mint),
                });
            }
        }
    }

    // Build instruction
    let mut accounts = program_account_metas.to_vec();
    let (system_accounts, system_accounts_offset, _) = remaining_accounts.to_account_metas();
    accounts.extend(system_accounts);

    for address in decompressed_account_addresses {
        accounts.push(AccountMeta::new(*address, false));
    }

    let instruction_data = DecompressMultipleAccountsIdempotentData {
        proof: validity_proof_with_context.proof,
        compressed_accounts,
        system_accounts_offset: system_accounts_offset as u8,
    };

    let serialized_data = instruction_data.try_to_vec()?;
    let mut data = Vec::with_capacity(discriminator.len() + serialized_data.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(&serialized_data);

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
```

---

## 7. Client Usage Example

```rust
use light_compressible_client::compressible_instruction::{
    decompress_accounts_idempotent, DecompressInput,
};

// Decompress a mix of PDAs and ATAs
let inputs = vec![
    // Program-specific PDA
    DecompressInput::Standard {
        compressed_account: user_record_compressed,
        data: user_record_data,
    },
    // Standard ATA (wallet must sign)
    DecompressInput::LightAta {
        compressed_account: user_ata_compressed,
        wallet: user_wallet,  // Add to tx signers!
    },
];

let ix = decompress_accounts_idempotent(
    &program_id,
    &DECOMPRESS_DISCRIMINATOR,
    &[user_record_pda, user_ata_address],
    &inputs,
    &program_account_metas,
    validity_proof,
)?;

// Decompress a CMint (separate instruction - can't mix with tokens)
let mint_inputs = vec![
    DecompressInput::LightMint {
        compressed_account: cmint_compressed,
        mint_seed_pubkey: mint_seed,
    },
];

let mint_ix = decompress_accounts_idempotent(
    &program_id,
    &DECOMPRESS_DISCRIMINATOR,
    &[cmint_pda],
    &mint_inputs,
    &program_account_metas,
    validity_proof,
)?;
```

---

## 8. Processing Order (Fixed)

```
1. PDAs      -> Light System CPI (write to CPI context if 2+ types)
2. LightMint -> DecompressCMint CPI (write to CPI context if tokens follow)
3. LightAta + CToken -> Transfer2 CPI (consume CPI context if prior writes)
```

Tokens (LightAta + CToken) are ALWAYS processed last because they consume the CPI context.

---

## 9. Files to Modify

### 9.1 Standard Types (light-ctoken-sdk)

| File                                       | Change                            |
| ------------------------------------------ | --------------------------------- |
| `src/compressible/standard_types.rs` (NEW) | Add `LightAta`, `LightMint` types |
| `src/compressible/mod.rs`                  | Export new types                  |

### 9.2 Macros (sdk-libs/macros)

| File                                 | Change                                                |
| ------------------------------------ | ----------------------------------------------------- |
| `src/compressible/variant_enum.rs`   | Always add `LightAta`, `LightMint` variants           |
| `src/compressible/variant_enum.rs`   | Update `HasTokenVariant` to detect LightAta/LightMint |
| `src/compressible/seed_providers.rs` | Remove `#[ata]` attribute handling                    |

### 9.3 SDK Runtime (sdk-libs/sdk)

| File                                     | Change                                               |
| ---------------------------------------- | ---------------------------------------------------- |
| `src/compressible/decompress_runtime.rs` | Add `check_account_types_with_count`                 |
| `src/compressible/decompress_runtime.rs` | Add constraint validation                            |
| `src/error.rs`                           | Add `AtMostOneMintAllowed`, `MintAndTokensForbidden` |

### 9.4 CToken SDK Runtime (sdk-libs/ctoken-sdk)

| File                                     | Change                              |
| ---------------------------------------- | ----------------------------------- |
| `src/compressible/decompress_runtime.rs` | Handle `LightAta` in process_tokens |
| `src/compressible/decompress_runtime.rs` | Handle `LightMint` in process_mints |

### 9.5 Client (sdk-libs/compressible-client)

| File         | Change                                  |
| ------------ | --------------------------------------- |
| `src/lib.rs` | Add `DecompressInput` enum              |
| `src/lib.rs` | Add `validate_inputs` function          |
| `src/lib.rs` | Update `decompress_accounts_idempotent` |

---

## 10. NOT In Scope

- `compress_accounts_idempotent` does NOT support ATA/Mint compression
- ATAs and Mints are compressed by the forester invoking ctoken program directly
- No changes needed to compression flow

---

## 11. Visual Flow Diagram

```
                    +------------------------+
                    | Client builds inputs   |
                    | DecompressInput enum   |
                    +------------------------+
                              |
                    +---------v---------+
                    | validate_inputs   |
                    | - max 1 mint      |
                    | - mint+tokens !=  |
                    +-------------------+
                              |
                    +---------v---------+
                    | pack_inputs       |
                    | - Standard -> T   |
                    | - LightAta        |
                    | - LightMint       |
                    +-------------------+
                              |
                    +---------v---------+
                    | TX to program     |
                    +-------------------+
                              |
       +----------------------+----------------------+
       |                                             |
+------v------+                               +------v------+
| On-chain    |                               | collect_all |
| validation  |                               | _accounts   |
| - same checks|                              +-------------+
+-------------+                                      |
                          +-------------+------------+-------------+
                          |             |            |             |
                    +-----v-----+ +-----v-----+ +----v----+ +------v------+
                    | PDAs      | | LightMint | | CToken  | | LightAta    |
                    +-----------+ +-----------+ +---------+ +-------------+
                          |             |            |             |
                    +-----v-----+ +-----v-----+      +------v------+
                    | Light Sys | | Decompress|      | Transfer2   |
                    | CPI write | | CMint CPI |      | CPI consume |
                    +-----------+ +-----------+      +-------------+
                          |             |                   |
                          +------+------+-------------------+
                                 |
                          +------v------+
                          | Done        |
                          +-------------+
```

---

## 12. Summary

| Old Design                                  | New Design                                   |
| ------------------------------------------- | -------------------------------------------- |
| Declare `UserAta = (is_token, is_ata, ...)` | Use `LightAta` (standard, always available)  |
| Declare CMint variants                      | Use `LightMint` (standard, always available) |
| Client manually packs ATAs                  | Client uses `DecompressInput::LightAta`      |
| Client manually packs Mints                 | Client uses `DecompressInput::LightMint`     |
| No constraint validation                    | Client + on-chain validation                 |

**Benefits:**

1. **Simpler macro declarations** - No ATA/Mint variants to declare
2. **Standard client API** - `DecompressInput` enum handles all types uniformly
3. **Explicit constraints** - Clear errors for forbidden combinations
4. **Type safety** - Can't confuse ATAs with Vaults

**Constraints enforced:**

1. `AtMostOneMintAllowed` - Max 1 LightMint per instruction
2. `MintAndTokensForbidden` - LightMint + tokens combination blocked

**Processing order (fixed):** PDAs -> LightMint -> CToken/LightAta
