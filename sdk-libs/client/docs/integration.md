# Aggregator Integration Guide

Compression support for AMM pools. Minimal changes to existing infrastructure.

## Architecture

```
                         ┌─────────────────────────────────────────┐
                         │              AGGREGATOR                 │
                         │                                         │
   ┌─────────┐          │  ┌─────────────────────────────────┐   │
   │ Geyser  │──────────┼─▶│         Event Handler           │   │
   │ Stream  │          │  │                                 │   │
   └─────────┘          │  │  account_update(pubkey, data)   │   │
                         │  │          │                      │   │
                         │  │          ▼                      │   │
                         │  │  ┌───────────────────────┐      │   │
                         │  │  │ is_compressible(pk)?  │      │   │
                         │  │  └───────┬───────┬───────┘      │   │
                         │  │          │       │              │   │
                         │  │      NO  │       │ YES          │   │
                         │  │          │       │              │   │
                         │  │          ▼       ▼              │   │
                         │  │  ┌───────────────────────┐      │   │
                         │  │  │ is_closure(data)?     │      │   │
                         │  │  └───────┬───────┬───────┘      │   │
                         │  │          │       │              │   │
                         │  │      NO  │       │ YES          │   │
                         │  │          ▼       ▼              │   │
                         │  └──────────┼───────┼──────────────┘   │
                         │             │       │                   │
                         │             ▼       ▼                   │
   ┌─────────┐          │  ┌──────────────┐  ┌──────────────┐    │
   │ Photon  │◀─────────┼──│              │  │              │    │
   │ Indexer │──────────┼─▶│  hot_cache   │  │  cold_cache  │    │
   └─────────┘          │  │  (Account)   │  │ (Interface)  │    │
                         │  └──────┬───────┘  └──────┬───────┘    │
                         │         │                 │            │
                         │         └────────┬────────┘            │
                         │                  ▼                     │
                         │         ┌──────────────────┐           │
                         │         │    AMM SDK       │           │
                         │         │                  │           │
                         │         │ .quote()         │           │
                         │         │ .swap_needs_     │           │
                         │         │    loading()     │           │
                         │         │ .get_cold_swap_  │           │
                         │         │    specs()       │           │
                         │         └──────────────────┘           │
                         └─────────────────────────────────────────┘
```

## Storage Schema

### PostgreSQL

```sql
ALTER TABLE pools ADD COLUMN is_compressible BOOLEAN DEFAULT FALSE;
ALTER TABLE pools ADD COLUMN compressible_accounts TEXT[];  -- pubkeys
```

### Redis

```
# Hot accounts (unchanged)
account:{pubkey} -> Account { lamports, data, owner }

# Cold accounts (new)
cold:{pubkey} -> AccountInterface { key, account, cold_context }
```

## Event Handling

```rust
fn handle_account_update(pubkey: Pubkey, account: Option<Account>) {
    let pool = db.get_pool_by_account(pubkey)?;

    if !pool.is_compressible {
        // Existing flow - unchanged
        match account {
            Some(acc) => hot_cache.set(pubkey, acc),
            None => hot_cache.delete(pubkey),
        }
        return;
    }

    // Compressible pool
    let is_closure = account.map_or(true, |a| a.lamports == 0);
    let is_compressible_account = pool.compressible_accounts.contains(&pubkey);

    if is_closure && is_compressible_account {
        // Account went cold - fetch from Photon
        hot_cache.delete(pubkey);
        let interface = photon.get_account_interface(pubkey)?;
        cold_cache.set(pubkey, interface);
    } else if !is_closure {
        // Account is hot (maybe decompressed)
        cold_cache.delete(pubkey);
        hot_cache.set(pubkey, account.unwrap());
    }
}
```

## Quoting

```rust
fn quote(pool: &Pool, input: u64) -> QuoteResult {
    let mut amm = AmmSdk::new();

    // Load hot accounts
    let hot_accounts: Vec<_> = pool.accounts.iter()
        .filter_map(|pk| hot_cache.get(pk))
        .map(|acc| AccountInterface::hot(pk, acc))
        .collect();

    // Load cold accounts
    let cold_accounts: Vec<_> = pool.compressible_accounts.iter()
        .filter_map(|pk| cold_cache.get(pk))
        .collect();

    // Update SDK
    amm.update_with_interfaces(&hot_accounts)?;
    amm.update_with_interfaces(&cold_accounts)?;

    // Quote
    let quote = amm.quote(input)?;

    QuoteResult {
        output: quote.out_amount,
        needs_loading: amm.swap_needs_loading(),
    }
}
```

## Swap Execution

```rust
async fn build_swap_tx(pool: &Pool, params: SwapParams, indexer: &Indexer) -> Transaction {
    let amm = load_amm(pool);  // as above

    let mut instructions = vec![];

    // Prepend load instructions if needed
    if amm.swap_needs_loading() {
        let specs = amm.get_swap_specs();  // includes both hot and cold
        let load_ixs = create_load_instructions(
            &specs,
            fee_payer,
            compression_config,
            rent_sponsor,
            indexer,
        ).await?;  // internally filters for cold only
        instructions.extend(load_ixs);
    }

    // Build swap instruction
    let swap_ix = amm.get_swap_and_account_metas(&params)?;
    instructions.push(swap_ix);

    Transaction::new(&instructions, payer)
}
```

## SDK Methods Reference

| Method                                        | Purpose                                               |
| --------------------------------------------- | ----------------------------------------------------- |
| `update_with_interfaces(&[AccountInterface])` | Update SDK cache with hot/cold accounts               |
| `swap_needs_loading() -> bool`                | Check if cold accounts need decompression             |
| `get_swap_specs() -> Vec<AccountSpec>`        | Get all swap account specs (hot + cold)               |
| `get_compressible_accounts() -> Vec<Pubkey>`  | Get all compressible account pubkeys                  |
| `create_load_instructions(specs, ...)`        | Build load instructions (filters for cold internally) |

## Detection: Is Account Compressible?

On pool discovery, call:

```rust
let compressible = amm.get_compressible_accounts();
db.update_pool(pool_id, compressible_accounts: compressible);
```

## Photon API

```rust
// Returns AccountInterface for both hot and cold accounts
photon.get_account_interface(pubkey) -> AccountInterface {
    key: Pubkey,
    account: Account,           // reconstructed data
    cold: Option<ColdContext>,  // None if hot, Some if cold
}
```

## Checklist

- [ ] Add `is_compressible` and `compressible_accounts` columns to pools table
- [ ] Add cold cache (Redis or in-memory)
- [ ] Modify Geyser handler to detect closures on compressible accounts
- [ ] Integrate Photon client for cold account fetches
- [ ] Update quote flow to merge hot + cold accounts
- [ ] Update swap builder to prepend load instructions when needed
