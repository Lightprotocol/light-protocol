# Init Flow Simplification Spec (v2)

## Problem Statement

When initializing Light Protocol accounts, clients must:
1. Manually derive all PDAs
2. Know which accounts need address proofs (only PDAs and Mints - NOT token accounts or ATAs)
3. Call `get_create_accounts_proof` with exactly the right subset

This is error-prone. Clients shouldn't need to know protocol internals.

---

## Design Goals

1. **Sync SDK** - All SDK methods are synchronous (no RPC calls)
2. **Instruction-based** - Use `Instruction` enum like existing trait methods
3. **Fast exit** - If no address proofs needed, client can skip RPC entirely
4. **Minimal indirection** - Reuse existing `CreateAccountsProofInput` type
5. **Consistent** - Follow existing `LightProgramInterface` patterns

---

## Proposed Solution

### Extend `LightProgramInterface` Trait

Add one method to the existing trait:

```rust
pub trait LightProgramInterface: Sized {
    // ... existing methods ...

    /// Returns inputs needed for `get_create_accounts_proof` for an init instruction.
    /// 
    /// Returns `Vec<CreateAccountsProofInput>` containing ONLY accounts that need 
    /// address proofs (PDAs, Mints). Token accounts and ATAs are excluded.
    /// 
    /// Returns empty vec if instruction creates no new addressed accounts
    /// (client can skip proof RPC call entirely).
    #[must_use]
    fn get_create_accounts_inputs(&self, ix: &Self::Instruction) -> Vec<CreateAccountsProofInput>;
}
```

### Client Flow

```rust
// 1. SDK returns proof inputs (SYNC - no RPC)
let inputs = sdk.get_create_accounts_inputs(&AmmInstruction::InitializePool { 
    amm_config,
    token_0_mint,
    token_1_mint,
    creator,
});

// 2. Fast exit if no proofs needed
let proof_result = if inputs.is_empty() {
    // No address proofs needed - skip RPC
    CreateAccountsProofResult::empty()
} else {
    // Client does RPC call
    get_create_accounts_proof(&rpc, &program_id, inputs).await?
};

// 3. Build instruction with proof
```

### Helper for Fast Exit

```rust
impl CreateAccountsProofResult {
    /// Empty result for instructions that don't create new addressed accounts.
    pub fn empty() -> Self {
        Self {
            create_accounts_proof: CreateAccountsProof::default(),
            remaining_accounts: vec![],
        }
    }
}

/// Convenience wrapper that handles empty case.
pub async fn get_create_accounts_proof_if_needed<R: Rpc + Indexer>(
    rpc: &R,
    program_id: &Pubkey,
    inputs: Vec<CreateAccountsProofInput>,
) -> Result<CreateAccountsProofResult, CreateAccountsProofError> {
    if inputs.is_empty() {
        return Ok(CreateAccountsProofResult::empty());
    }
    get_create_accounts_proof(rpc, program_id, inputs).await
}
```

---

## Example Implementation (AmmSdk)

### Instruction Enum with Init Params

```rust
#[derive(Debug, Clone)]
pub enum AmmInstruction {
    /// Initialize a new pool
    InitializePool {
        amm_config: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        creator: Pubkey,
    },
    Swap,
    Deposit,
    Withdraw,
}
```

### Trait Implementation

```rust
impl LightProgramInterface for AmmSdk {
    type Instruction = AmmInstruction;
    // ... other types ...

    fn get_create_accounts_inputs(&self, ix: &Self::Instruction) -> Vec<CreateAccountsProofInput> {
        match ix {
            AmmInstruction::InitializePool { 
                amm_config, 
                token_0_mint, 
                token_1_mint, 
                .. 
            } => {
                // Derive PDAs that need address proofs
                let (pool_state, _) = Pubkey::find_program_address(
                    &[POOL_SEED.as_bytes(), amm_config.as_ref(), 
                      token_0_mint.as_ref(), token_1_mint.as_ref()],
                    &PROGRAM_ID,
                );
                
                let (observation_state, _) = Pubkey::find_program_address(
                    &[OBSERVATION_SEED.as_bytes(), pool_state.as_ref()],
                    &PROGRAM_ID,
                );
                
                let (lp_mint_signer, _) = Pubkey::find_program_address(
                    &[POOL_LP_MINT_SIGNER_SEED, pool_state.as_ref()],
                    &PROGRAM_ID,
                );
                
                // Return ONLY accounts needing address proofs
                // Token vaults and ATAs are NOT included
                vec![
                    CreateAccountsProofInput::pda(pool_state),
                    CreateAccountsProofInput::pda(observation_state),
                    CreateAccountsProofInput::mint(lp_mint_signer),
                ]
            }
            // Non-init instructions don't create new addressed accounts
            AmmInstruction::Swap | 
            AmmInstruction::Deposit | 
            AmmInstruction::Withdraw => vec![],
        }
    }
    
    // ... other methods ...
}
```

### SDK Helper for Full PDA Derivation (Optional)

SDKs can still provide a helper for clients that need all PDAs + bumps:

```rust
impl AmmSdk {
    /// Derive all PDAs for InitializePool (sync, no RPC).
    /// Returns addresses AND bumps for instruction building.
    pub fn derive_init_pool_pdas(
        amm_config: &Pubkey,
        token_0_mint: &Pubkey,
        token_1_mint: &Pubkey,
        creator: &Pubkey,
    ) -> AmmPdas {
        // ... full derivation with bumps ...
    }
}
```

---

## Client Usage

### Before (Current)

```rust
// Client must know which accounts need proofs
let proof = get_create_accounts_proof(&rpc, &program_id, vec![
    CreateAccountsProofInput::pda(pdas.pool_state),
    CreateAccountsProofInput::pda(pdas.observation_state),
    CreateAccountsProofInput::mint(pdas.lp_mint_signer),
    // Must NOT include vaults, ATAs!
]).await?;
```

### After (Proposed)

```rust
// SDK tells client exactly what's needed (SYNC)
let inputs = sdk.get_create_accounts_inputs(&AmmInstruction::InitializePool {
    amm_config: config,
    token_0_mint: mint_0,
    token_1_mint: mint_1,
    creator: creator.pubkey(),
});

// Client does RPC (or skips if empty)
let proof = get_create_accounts_proof_if_needed(&rpc, &program_id, inputs).await?;
```

---

## Design Rationale

### Why Extend Existing Trait?

- Consistent with `get_accounts_to_update`, `get_specs_for_instruction`
- No new trait to implement
- Instruction enum already exists

### Why Use `CreateAccountsProofInput` Directly?

- No new types needed
- Client already uses this for `get_create_accounts_proof`
- SDK just filters/derives correctly

### Why Return Empty Vec (Not Option)?

- Simpler API
- Empty vec naturally flows to "skip RPC" logic
- Consistent with other methods that return `Vec`

### Why Sync Only?

- SDK is pure derivation logic
- RPC calls belong in client code
- Easier to test, compose, debug

---

## Summary

Single addition to `LightProgramInterface`:

```rust
fn get_create_accounts_inputs(&self, ix: &Self::Instruction) -> Vec<CreateAccountsProofInput>;
```

Benefits:
- Client doesn't need to know which accounts need proofs
- SDK is sync (no RPC)
- Fast exit when `inputs.is_empty()`
- Reuses existing types
- Follows existing trait patterns
