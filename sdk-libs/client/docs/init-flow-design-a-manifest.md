# Init Flow Design A: Account Manifest Pattern

## Philosophy

**Jupiter-inspired**: Flat data structures, explicit account lists, no hidden magic.
**Transparency**: Every account is visible with its classification.
**Aggregator-friendly**: Easy to audit, log, or transform account lists.

---

## Core Type: AccountManifest

A simple struct listing all accounts an init instruction will touch:

```rust
/// Classification of how an account participates in init.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitAccountRole {
    /// PDA that needs address proof (compressed address derivation)
    AddressedPda,
    /// Mint that needs address proof (uses MINT_ADDRESS_TREE)
    AddressedMint,
    /// Token account - NO address proof needed (uses ATA derivation)
    TokenAccount,
    /// ATA - NO address proof needed
    Ata,
    /// Signer account (creator, authority, etc.)
    Signer,
    /// Static account (program, system, rent, etc.)
    Static,
}

/// Single account entry in the manifest.
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    pub pubkey: Pubkey,
    pub role: InitAccountRole,
    /// Human-readable name for debugging/logging
    pub name: &'static str,
}

impl ManifestEntry {
    /// Does this account need an address proof?
    #[inline]
    pub fn needs_address_proof(&self) -> bool {
        matches!(self.role, InitAccountRole::AddressedPda | InitAccountRole::AddressedMint)
    }
}

/// Complete account manifest for an init instruction.
#[derive(Debug, Clone)]
pub struct AccountManifest {
    pub entries: Vec<ManifestEntry>,
}
```

---

## SDK Contract

Each SDK provides a single function per init instruction:

```rust
impl AmmSdk {
    /// Returns complete account manifest for InitializePool.
    /// 
    /// SYNC - no RPC. Pure derivation.
    /// 
    /// All PDAs are derived. All roles are classified.
    /// Client can inspect, filter, log, or transform.
    pub fn init_pool_manifest(
        amm_config: &Pubkey,
        token_0_mint: &Pubkey,
        token_1_mint: &Pubkey,
        creator: &Pubkey,
    ) -> AccountManifest {
        let (pool_state, _) = derive_pool_state(amm_config, token_0_mint, token_1_mint);
        let (observation_state, _) = derive_observation_state(&pool_state);
        let (authority, _) = derive_authority();
        let (lp_mint_signer, _) = derive_lp_mint_signer(&pool_state);
        let (lp_mint, _) = find_mint_address(&lp_mint_signer);
        let (token_0_vault, _) = derive_token_vault(&pool_state, token_0_mint);
        let (token_1_vault, _) = derive_token_vault(&pool_state, token_1_mint);
        let (creator_lp_token, _) = get_associated_token_address_and_bump(creator, &lp_mint);

        AccountManifest {
            entries: vec![
                ManifestEntry { pubkey: pool_state, role: InitAccountRole::AddressedPda, name: "pool_state" },
                ManifestEntry { pubkey: observation_state, role: InitAccountRole::AddressedPda, name: "observation_state" },
                ManifestEntry { pubkey: lp_mint_signer, role: InitAccountRole::AddressedMint, name: "lp_mint_signer" },
                ManifestEntry { pubkey: lp_mint, role: InitAccountRole::Static, name: "lp_mint" },
                ManifestEntry { pubkey: token_0_vault, role: InitAccountRole::TokenAccount, name: "token_0_vault" },
                ManifestEntry { pubkey: token_1_vault, role: InitAccountRole::TokenAccount, name: "token_1_vault" },
                ManifestEntry { pubkey: creator_lp_token, role: InitAccountRole::Ata, name: "creator_lp_token" },
                ManifestEntry { pubkey: *creator, role: InitAccountRole::Signer, name: "creator" },
                ManifestEntry { pubkey: authority, role: InitAccountRole::Static, name: "authority" },
            ],
        }
    }
}
```

---

## Client Flow

```rust
// 1. Get manifest (SYNC)
let manifest = AmmSdk::init_pool_manifest(&config, &mint_0, &mint_1, &creator.pubkey());

// 2. Extract accounts needing proofs (simple filter)
let proof_inputs: Vec<CreateAccountsProofInput> = manifest
    .entries
    .iter()
    .filter(|e| e.needs_address_proof())
    .map(|e| match e.role {
        InitAccountRole::AddressedPda => CreateAccountsProofInput::pda(e.pubkey),
        InitAccountRole::AddressedMint => CreateAccountsProofInput::mint(e.pubkey),
        _ => unreachable!(),
    })
    .collect();

// 3. Get proof (ASYNC - only RPC call)
let proof_result = get_create_accounts_proof(&rpc, &program_id, proof_inputs).await?;

// 4. Build instruction using manifest pubkeys
let ix = build_init_pool_ix(&manifest, &proof_result, init_params);
```

---

## Helper: Auto-Filter

For clients that don't want manual filtering:

```rust
impl AccountManifest {
    /// Extract proof inputs for accounts that need address proofs.
    pub fn to_proof_inputs(&self) -> Vec<CreateAccountsProofInput> {
        self.entries
            .iter()
            .filter_map(|e| match e.role {
                InitAccountRole::AddressedPda => Some(CreateAccountsProofInput::pda(e.pubkey)),
                InitAccountRole::AddressedMint => Some(CreateAccountsProofInput::mint(e.pubkey)),
                _ => None,
            })
            .collect()
    }
    
    /// Get pubkey by name.
    pub fn get(&self, name: &str) -> Option<Pubkey> {
        self.entries.iter().find(|e| e.name == name).map(|e| e.pubkey)
    }
}
```

---

## Aggregator Usage (Jupiter/DFlow)

```rust
// Jupiter integration - they want to see everything
let manifest = AmmSdk::init_pool_manifest(&config, &mint_0, &mint_1, &creator);

// Log for debugging/audit
for entry in &manifest.entries {
    log::info!("{}: {} ({:?})", entry.name, entry.pubkey, entry.role);
}

// They control proof fetching
let proof_inputs = manifest.to_proof_inputs();
if !proof_inputs.is_empty() {
    let proof = get_create_accounts_proof(&rpc, &program_id, proof_inputs).await?;
    // ...
}
```

---

## Trade-offs

### Pros
- **Fully transparent**: Every account is visible and classified
- **Debuggable**: Names + roles make logging trivial
- **Flexible**: Aggregators can transform/filter as needed
- **No hidden state**: Pure function, no SDK instance needed
- **Jupiter-like**: Matches their `get_accounts_to_update()` pattern

### Cons
- Manual mapping from manifest to instruction accounts (but explicit)
- Client still needs to understand PDA vs Mint vs Token distinction (but it's visible)
- Requires SDK to define roles correctly

---

## Comparison with Current Design

| Aspect | Current (v1 spec) | Design A (Manifest) |
|--------|-------------------|---------------------|
| Account visibility | Hidden in trait method | Fully exposed |
| Role classification | Implicit | Explicit enum |
| Debugging | Hard | Easy (names + roles) |
| Aggregator fit | Medium | High |
| Code verbosity | Lower | Slightly higher |
| Magic | Some | None |

---

## Open Questions

1. Should `AccountManifest` include bumps for each PDA?
2. Should we provide a `manifest.to_account_metas()` helper?
3. Should `name` be an enum instead of `&'static str`?
