# ProgramPackedAccounts

**Path:** `program-libs/account-checks/src/packed_accounts.rs`

## Description

ProgramPackedAccounts provides index-based access to dynamically sized account arrays. This utility is designed for instructions that work with variable numbers of accounts (mint, owner, delegate, merkle tree, queue accounts) where accounts are referenced by index rather than position.

## Structure

```rust
pub struct ProgramPackedAccounts<'info, A: AccountInfoTrait> {
    pub accounts: &'info [A],
}
```

## Methods

### `get`
```rust
fn get(&self, index: usize, name: &str) -> Result<&A, AccountError>
```
- Retrieves account at specified index with bounds checking
- **Parameters:**
  - `index`: Zero-based index into account array
  - `name`: Descriptive name for error messages
- **Error:** `NotEnoughAccountKeys` (12020) with location tracking

### `get_u8`
```rust
fn get_u8(&self, index: u8, name: &str) -> Result<&A, AccountError>
```
- Convenience method for u8 indices (common in instruction data)
- Internally calls `get(index as usize, name)`
- **Error:** `NotEnoughAccountKeys` (12020)

## Usage Patterns

### Dynamic Account Access
```rust
use light_account_checks::{AccountInfoTrait, ProgramPackedAccounts};

fn process_with_dynamic_accounts<A: AccountInfoTrait>(
    accounts: &[A],
    mint_index: u8,
    owner_index: u8,
) -> Result<(), AccountError> {
    let packed = ProgramPackedAccounts { accounts };

    // Access accounts by index from instruction data
    let mint = packed.get_u8(mint_index, "mint")?;
    let owner = packed.get_u8(owner_index, "owner")?;

    // Validate retrieved accounts
    check_owner(&token_program_id, mint)?;
    check_signer(owner)?;

    Ok(())
}
```

### Multiple Optional Accounts
```rust
struct TransferInstruction {
    mint_indices: Vec<u8>,
    amounts: Vec<u64>,
}

fn process_multi_transfer<A: AccountInfoTrait>(
    accounts: &[A],
    instruction: TransferInstruction,
) -> Result<(), AccountError> {
    let packed = ProgramPackedAccounts { accounts };

    for (i, &mint_index) in instruction.mint_indices.iter().enumerate() {
        let mint = packed.get_u8(mint_index, &format!("mint_{}", i))?;

        // Process transfer for this mint
        process_single_transfer(mint, instruction.amounts[i])?;
    }

    Ok(())
}
```

### Merkle Tree and Queue Access
```rust
fn access_merkle_accounts<A: AccountInfoTrait>(
    accounts: &[A],
    tree_index: u8,
    queue_index: u8,
) -> Result<(), AccountError> {
    let packed = ProgramPackedAccounts { accounts };

    let merkle_tree = packed.get_u8(tree_index, "merkle_tree")?;
    let queue = packed.get_u8(queue_index, "queue")?;

    // Validate merkle tree account
    check_account_info::<MerkleTree, A>(&program_id, merkle_tree)?;

    // Validate queue account
    check_account_info::<Queue, A>(&program_id, queue)?;

    Ok(())
}
```

## Error Messages

ProgramPackedAccounts provides detailed error messages with `#[track_caller]`:

```
ERROR: Not enough accounts. Requested 'mint' at index 5 but only 3 accounts available. src/processor.rs:42:18

ERROR: Not enough accounts. Requested 'merkle_tree' at index 10 but only 8 accounts available. src/processor.rs:55:23
```

## Comparison with AccountIterator

| Feature | ProgramPackedAccounts | AccountIterator |
|---------|----------------------|-----------------|
| **Access Pattern** | Random by index | Sequential |
| **Use Case** | Dynamic account sets | Fixed account order |
| **Index Source** | Instruction data | Implicit position |
| **Validation** | Manual after retrieval | Built-in methods |
| **State** | Stateless | Tracks position |

### When to Use Each

**Use ProgramPackedAccounts when:**
- Account indices come from instruction data
- Accounts can be accessed in any order
- Number of accounts varies per instruction
- Indices might skip positions

**Use AccountIterator when:**
- Accounts have fixed order
- Sequential processing is natural
- Built-in validation is helpful
- Error context from position is sufficient

## Integration Example

Combining with AccountIterator for hybrid access:

```rust
fn process_complex_instruction<A: AccountInfoTrait>(
    accounts: &[A],
    dynamic_indices: Vec<u8>,
) -> Result<(), AccountError> {
    let mut iter = AccountIterator::new(accounts);

    // Fixed accounts at start
    let authority = iter.next_signer("authority")?;
    let payer = iter.next_signer_mut("payer")?;
    let config = iter.next_non_mut("config")?;

    // Remaining accounts accessed dynamically
    let remaining = iter.remaining()?;
    let packed = ProgramPackedAccounts { accounts: remaining };

    // Access by indices from instruction data
    for (i, &index) in dynamic_indices.iter().enumerate() {
        let account = packed.get(index as usize, &format!("dynamic_{}", i))?;
        // Process dynamic account
    }

    Ok(())
}
```

## Future Enhancements

The TODO comment suggests adding validation methods:
```rust
// TODO: add get_checked_account from PackedAccounts.
```

This would enable:
```rust
fn get_checked<T: Discriminator>(
    &self,
    index: usize,
    name: &str,
    program_id: &[u8; 32],
) -> Result<&A, AccountError> {
    let account = self.get(index, name)?;
    check_account_info::<T, A>(program_id, account)?;
    Ok(account)
}
```

## Best Practices

1. **Use descriptive names in errors:**
   ```rust
   packed.get_u8(index, "token_mint")  // Good
   packed.get_u8(index, "account")     // Less helpful
   ```

2. **Validate after retrieval:**
   ```rust
   let account = packed.get(index, "mint")?;
   check_owner(&spl_token_id, account)?;  // Always validate
   ```

3. **Handle index bounds explicitly:**
   ```rust
   if index >= accounts.len() {
       return Err(CustomError::InvalidIndex);
   }
   let account = packed.get(index, "account")?;
   ```

4. **Consider caching for repeated access:**
   ```rust
   let mint = packed.get_u8(mint_index, "mint")?;
   // Store reference if accessed multiple times
   ```

## See Also
- [ACCOUNT_ITERATOR.md](ACCOUNT_ITERATOR.md) - Sequential account processing
- [ERRORS.md](ERRORS.md) - NotEnoughAccountKeys error details
- [ACCOUNT_INFO_TRAIT.md](ACCOUNT_INFO_TRAIT.md) - AccountInfoTrait abstraction