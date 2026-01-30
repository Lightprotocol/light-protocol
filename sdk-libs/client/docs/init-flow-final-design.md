# Init Flow: Final Battle-Tested Design

## Design Principles

1. **SYNC derivation, ASYNC proof** - Clear separation
2. **Typed structs over generics** - Compile-time safety
3. **Raw access always available** - Aggregators need control
4. **Convenience is optional** - Don't force abstraction
5. **Debuggable by default** - Names everywhere

---

## Core API

### 1. Client-Side Types (in light_client::interface)

```rust
/// Address with tree info for proof fetching.
/// Re-export from indexer for convenience.
pub use crate::indexer::AddressWithTree;

/// Address proof input with debug name.
#[derive(Debug, Clone)]
pub struct AddressProofInput {
    pub address: [u8; 32],
    pub tree: Pubkey,
    pub name: &'static str,
}

impl AddressProofInput {
    pub fn to_address_with_tree(&self) -> AddressWithTree {
        AddressWithTree { address: self.address, tree: self.tree }
    }
}

/// Trait for SDK-generated init account structs.
pub trait InitAccounts: Sized {
    /// Get proof inputs (addresses that need non-inclusion proofs).
    fn proof_inputs(&self) -> &[AddressProofInput];
    
    /// Get addresses ready for get_validity_proof.
    fn addresses_with_trees(&self) -> Vec<AddressWithTree> {
        self.proof_inputs().iter().map(|p| p.to_address_with_tree()).collect()
    }
}
```

### 2. New Proof Function (in light_client::interface)

```rust
/// Get proof using pre-derived addresses.
/// 
/// Use this with addresses from `InitAccounts::addresses_with_trees()`.
/// For aggregators who want to batch addresses from multiple operations.
pub async fn get_proof_for_addresses<R: Rpc + Indexer>(
    rpc: &R,
    program_id: &Pubkey,
    addresses: Vec<AddressWithTree>,
) -> Result<CreateAccountsProofResult, CreateAccountsProofError> {
    if addresses.is_empty() {
        return empty_proof_result(rpc).await;
    }
    
    let validity_proof = rpc
        .get_validity_proof(vec![], addresses, None)
        .await?
        .value;
    
    let state_tree_info = rpc
        .get_random_state_tree_info()
        .map_err(CreateAccountsProofError::Rpc)?;
    
    let has_mints = addresses.iter().any(|a| a.tree == MINT_ADDRESS_TREE_PUBKEY);
    let cpi_context = if has_mints { state_tree_info.cpi_context } else { None };
    
    let packed = if has_mints {
        pack_proof_for_mints(program_id, validity_proof.clone(), &state_tree_info, cpi_context)?
    } else {
        pack_proof(program_id, validity_proof.clone(), &state_tree_info, cpi_context)?
    };
    
    Ok(CreateAccountsProofResult {
        create_accounts_proof: CreateAccountsProof {
            proof: validity_proof.proof,
            address_tree_info: packed.packed_tree_infos.address_trees.first().copied()
                .ok_or(CreateAccountsProofError::EmptyInputs)?,
            output_state_tree_index: packed.output_tree_index,
            state_tree_index: packed.state_tree_index,
        },
        remaining_accounts: packed.remaining_accounts,
    })
}
```

---

## SDK Implementation Pattern

Each program SDK defines its own typed structs:

### AMM SDK Example

```rust
//! csdk_anchor_full_derived_test_sdk/src/init_pool.rs

use light_client::interface::{AddressProofInput, InitAccounts};
use solana_pubkey::Pubkey;

/// All accounts for InitializePool with bumps.
#[derive(Debug, Clone)]
pub struct InitPoolAccounts {
    // Accounts that need address proofs
    pub pool_state: Pubkey,
    pub pool_state_bump: u8,
    pub observation_state: Pubkey,
    pub observation_state_bump: u8,
    pub lp_mint_signer: Pubkey,
    pub lp_mint_signer_bump: u8,
    
    // Derived accounts (no proof needed but required for instruction)
    pub lp_mint: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_0_vault_bump: u8,
    pub token_1_vault: Pubkey,
    pub token_1_vault_bump: u8,
    pub creator_lp_token: Pubkey,
    pub creator_lp_token_bump: u8,
    pub authority: Pubkey,
    pub authority_bump: u8,
    
    // Pre-computed proof inputs (set during derivation)
    proof_inputs: Vec<AddressProofInput>,
}

impl InitAccounts for InitPoolAccounts {
    fn proof_inputs(&self) -> &[AddressProofInput] {
        &self.proof_inputs
    }
}

impl InitPoolAccounts {
    /// Derive all accounts for InitializePool.
    /// 
    /// # Arguments
    /// * `address_tree` - From `rpc.get_address_tree_v2().tree`
    pub fn derive(
        amm_config: &Pubkey,
        token_0_mint: &Pubkey,
        token_1_mint: &Pubkey,
        creator: &Pubkey,
        address_tree: &Pubkey,
    ) -> Self {
        // Derive PDAs
        let (pool_state, pool_state_bump) = Pubkey::find_program_address(
            &[POOL_SEED.as_bytes(), amm_config.as_ref(), 
              token_0_mint.as_ref(), token_1_mint.as_ref()],
            &PROGRAM_ID,
        );
        let (observation_state, observation_state_bump) = Pubkey::find_program_address(
            &[OBSERVATION_SEED.as_bytes(), pool_state.as_ref()],
            &PROGRAM_ID,
        );
        let (authority, authority_bump) = Pubkey::find_program_address(
            &[AUTH_SEED.as_bytes()],
            &PROGRAM_ID,
        );
        let (lp_mint_signer, lp_mint_signer_bump) = Pubkey::find_program_address(
            &[POOL_LP_MINT_SIGNER_SEED, pool_state.as_ref()],
            &PROGRAM_ID,
        );
        let (lp_mint, _) = find_mint_address(&lp_mint_signer);
        let (token_0_vault, token_0_vault_bump) = Pubkey::find_program_address(
            &[POOL_VAULT_SEED.as_bytes(), pool_state.as_ref(), token_0_mint.as_ref()],
            &PROGRAM_ID,
        );
        let (token_1_vault, token_1_vault_bump) = Pubkey::find_program_address(
            &[POOL_VAULT_SEED.as_bytes(), pool_state.as_ref(), token_1_mint.as_ref()],
            &PROGRAM_ID,
        );
        let (creator_lp_token, creator_lp_token_bump) = 
            get_associated_token_address_and_bump(creator, &lp_mint);

        // Derive compressed addresses for accounts needing proofs
        let pool_address = derive_address(
            &pool_state.to_bytes(),
            &address_tree.to_bytes(),
            &PROGRAM_ID.to_bytes(),
        );
        let observation_address = derive_address(
            &observation_state.to_bytes(),
            &address_tree.to_bytes(),
            &PROGRAM_ID.to_bytes(),
        );
        let mint_address = derive_mint_compressed_address(
            &lp_mint_signer,
            &MINT_ADDRESS_TREE_PUBKEY,
        );

        let proof_inputs = vec![
            AddressProofInput {
                address: pool_address,
                tree: *address_tree,
                name: "pool_state",
            },
            AddressProofInput {
                address: observation_address,
                tree: *address_tree,
                name: "observation_state",
            },
            AddressProofInput {
                address: mint_address,
                tree: MINT_ADDRESS_TREE_PUBKEY,
                name: "lp_mint",
            },
        ];

        Self {
            pool_state, pool_state_bump,
            observation_state, observation_state_bump,
            lp_mint_signer, lp_mint_signer_bump,
            lp_mint,
            token_0_vault, token_0_vault_bump,
            token_1_vault, token_1_vault_bump,
            creator_lp_token, creator_lp_token_bump,
            authority, authority_bump,
            proof_inputs,
        }
    }

    /// Build Anchor accounts struct.
    pub fn to_anchor_accounts(
        &self,
        creator: &Pubkey,
        amm_config: &Pubkey,
        token_0_mint: &Pubkey,
        token_1_mint: &Pubkey,
        config_pda: &Pubkey,
    ) -> InitializePool {
        InitializePool {
            creator: *creator,
            amm_config: *amm_config,
            authority: self.authority,
            pool_state: self.pool_state,
            token_0_mint: *token_0_mint,
            token_1_mint: *token_1_mint,
            lp_mint_signer: self.lp_mint_signer,
            lp_mint: self.lp_mint,
            creator_lp_token: self.creator_lp_token,
            token_0_vault: self.token_0_vault,
            token_1_vault: self.token_1_vault,
            observation_state: self.observation_state,
            token_program: LIGHT_TOKEN_PROGRAM_ID,
            // ... other static accounts ...
            compression_config: *config_pda,
        }
    }

    /// Build instruction params with proof.
    pub fn to_params(
        &self,
        proof: CreateAccountsProof,
        init_amount_0: u64,
        init_amount_1: u64,
    ) -> InitializeParams {
        InitializeParams {
            init_amount_0,
            init_amount_1,
            open_time: 0,
            create_accounts_proof: proof,
            lp_mint_signer_bump: self.lp_mint_signer_bump,
            creator_lp_token_bump: self.creator_lp_token_bump,
            authority_bump: self.authority_bump,
        }
    }
    
    /// Log proof inputs for debugging.
    pub fn log_proof_inputs(&self) {
        for input in &self.proof_inputs {
            log::debug!("{}: {:?} (tree: {})", input.name, input.address, input.tree);
        }
    }
}
```

---

## Client Usage

### Simple Client

```rust
// 1. Get address tree
let address_tree = rpc.get_address_tree_v2().tree;

// 2. Derive accounts (SYNC)
let accounts = InitPoolAccounts::derive(&config, &mint_0, &mint_1, &creator, &address_tree);

// 3. Get proof (ASYNC)
let proof_result = get_proof_for_addresses(
    &rpc,
    &PROGRAM_ID,
    accounts.addresses_with_trees(),
).await?;

// 4. Build instruction (SYNC)
let anchor_accounts = accounts.to_anchor_accounts(&creator, &config, &mint_0, &mint_1, &config_pda);
let params = accounts.to_params(proof_result.create_accounts_proof, 1000, 1000);

let ix = Instruction {
    program_id: PROGRAM_ID,
    accounts: [anchor_accounts.to_account_metas(None), proof_result.remaining_accounts].concat(),
    data: instruction::InitializePool { params }.data(),
};
```

### Jupiter Integration

```rust
impl LightAmm for LightProtocolAmm {
    /// Jupiter's standardized interface.
    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        // Return accounts Jupiter needs to fetch for quotes
        vec![self.pool_state, self.observation_state]
    }
    
    /// For init operations, Jupiter needs to derive first.
    fn derive_init_accounts(&self, address_tree: &Pubkey) -> Box<dyn InitAccounts> {
        Box::new(InitPoolAccounts::derive(
            &self.config, &self.mint_0, &self.mint_1, &self.creator, address_tree
        ))
    }
}

// Jupiter's aggregation loop
async fn aggregate_inits(operations: &[InitOp]) -> Result<Vec<Instruction>> {
    let tree = cache.get_address_tree().await;
    
    // Derive all (parallel, SYNC)
    let accounts: Vec<_> = operations
        .par_iter()
        .map(|op| op.amm.derive_init_accounts(&tree))
        .collect();
    
    // Batch all addresses
    let all_addresses: Vec<_> = accounts
        .iter()
        .flat_map(|a| a.addresses_with_trees())
        .collect();
    
    // Single batch proof (efficient)
    let proof = jupiter_prover.batch(all_addresses).await?;
    
    // Build instructions
    // ... split proof result back to individual operations ...
}
```

### DFlow Integration

```rust
// DFlow processes orders through their pipeline
async fn process_order(order: &InitOrder) -> Result<Transaction> {
    // 1. Derive (SYNC - fast, cacheable)
    let accounts = InitPoolAccounts::derive(
        &order.config, &order.mint_0, &order.mint_1, 
        &order.creator, &order.address_tree
    );
    
    // 2. Log for compliance/audit
    accounts.log_proof_inputs();
    
    // 3. Generate proof with DFlow's infrastructure
    let proof = dflow_prover::generate(accounts.addresses_with_trees()).await?;
    
    // 4. Build transaction
    let ix = build_init_ix(&accounts, proof);
    Ok(Transaction::new(&[ix], &order.signers))
}
```

---

## Comparison: Before vs After

### Before (Current Test Code)

```rust
// 50+ lines of PDA derivation
let pdas = derive_amm_pdas(&program_id, &config, &mint_0, &mint_1, &creator);

// Client must know which accounts need proofs
let proof = get_create_accounts_proof(&rpc, &program_id, vec![
    CreateAccountsProofInput::pda(pdas.pool_state),        // Must know this needs proof
    CreateAccountsProofInput::pda(pdas.observation_state), // Must know this too
    CreateAccountsProofInput::mint(pdas.lp_mint_signer),   // And this is a mint
    // Don't include token_0_vault - that's a token account!
    // Don't include creator_lp_token - that's an ATA!
]).await?;

// Manual instruction building - repeat all pubkeys
let accounts = InitializePool {
    creator: creator.pubkey(),
    amm_config: config,
    pool_state: pdas.pool_state,      // Manual
    observation_state: pdas.observation_state,  // Manual
    // ... 15 more fields ...
};
```

### After (Final Design)

```rust
// Get tree once (cache it)
let tree = rpc.get_address_tree_v2().tree;

// Derive everything (SYNC)
let accounts = InitPoolAccounts::derive(&config, &mint_0, &mint_1, &creator, &tree);

// Get proof - SDK knows what needs proofs
let proof = get_proof_for_addresses(&rpc, &PROGRAM_ID, accounts.addresses_with_trees()).await?;

// Type-safe instruction building
let anchor_accounts = accounts.to_anchor_accounts(&creator, &config, &mint_0, &mint_1, &cfg);
let params = accounts.to_params(proof.create_accounts_proof, 1000, 1000);
let ix = build_ix(anchor_accounts, params, proof.remaining_accounts);
```

---

## Checklist: What This Design Achieves

| Requirement | Status |
|-------------|--------|
| SDK handles proof input selection | Yes - in derive() |
| Typed PDA struct with bumps | Yes - InitPoolAccounts |
| No hidden RPC in derivation | Yes - address_tree is param |
| Debug names for addresses | Yes - AddressProofInput.name |
| Batchable for aggregators | Yes - addresses_with_trees() |
| Type-safe instruction building | Yes - to_anchor_accounts() |
| Simple client path | Yes - 4 steps |
| Advanced aggregator path | Yes - raw access |
| Jupiter-like flat API | Yes - InitAccounts trait |

---

## Files to Create/Modify

1. `sdk-libs/client/src/interface/init_accounts.rs` - Core types
2. `sdk-libs/client/src/interface/mod.rs` - Export new types
3. `sdk-tests/csdk-anchor-full-derived-test-sdk/src/init_pool.rs` - Example impl
4. `sdk-tests/csdk-anchor-full-derived-test/tests/amm_test.rs` - Updated test
