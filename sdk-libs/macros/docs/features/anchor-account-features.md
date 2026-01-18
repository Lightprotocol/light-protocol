# Anchor Account Macro Features

This document covers the 14 account constraint features available in the Anchor `#[account]` macro attribute.

## Overview

Anchor accounts are defined within a struct marked with `#[derive(Accounts)]`. Each field can have constraints applied via the `#[account(...)]` attribute.

```rust
#[derive(Accounts)]
pub struct MyInstruction<'info> {
    #[account(init, payer = user, space = 8 + 32)]
    pub data_account: Account<'info, DataAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

---

## 1. `init`

**Purpose**: Creates a new account via CPI to the System Program.

**Behavior**:
- Allocates space on-chain
- Assigns the account to the program
- Calls `System::create_account` CPI

**Required companions**: `payer`, `space`

**Example**:
```rust
#[account(init, payer = user, space = 8 + 64)]
pub my_account: Account<'info, MyData>,
```

**Generated code** (simplified):
```rust
let cpi_accounts = system_program::CreateAccount {
    from: user.to_account_info(),
    to: my_account.to_account_info(),
};
system_program::create_account(cpi_ctx, lamports, space, program_id)?;
```

---

## 2. `init_if_needed`

**Purpose**: Creates account only if it doesn't already exist.

**Behavior**:
- Checks if account data length is 0
- If zero, performs `init`
- If non-zero, deserializes existing data

**Feature flag required**: `init-if-needed`

**Example**:
```rust
#[account(init_if_needed, payer = user, space = 8 + 64)]
pub my_account: Account<'info, MyData>,
```

**Security note**: Can be dangerous with reinitialization attacks if discriminator isn't checked properly.

---

## 3. `zero`

**Purpose**: Deserializes an account that was pre-allocated (e.g., by an external process) and expects it to be zeroed.

**Behavior**:
- Expects account to already exist with allocated space
- Expects all bytes to be zero (except discriminator)
- Does NOT create the account

**Use case**: Two-phase initialization where space is allocated separately

**Example**:
```rust
#[account(zero)]
pub pre_allocated: Account<'info, MyData>,
```

---

## 4. `mut`

**Purpose**: Marks an account as mutable.

**Behavior**:
- Enables writing to account data
- Required for any account that will be modified

**Generated check**:
```rust
if !account.is_writable {
    return Err(ErrorCode::ConstraintMut.into());
}
```

**Example**:
```rust
#[account(mut)]
pub counter: Account<'info, Counter>,
```

---

## 5. `close`

**Purpose**: Closes an account and transfers lamports to a destination.

**Behavior**:
- Sets account data to zero
- Sets discriminator to `CLOSED_ACCOUNT_DISCRIMINATOR`
- Transfers all lamports to specified account

**Example**:
```rust
#[account(mut, close = user)]
pub data_account: Account<'info, MyData>,

#[account(mut)]
pub user: Signer<'info>,
```

**Generated code**:
```rust
data_account.close(user.to_account_info())?;
```

---

## 6. `realloc`

**Purpose**: Resizes an account's data allocation.

**Required companions**: `realloc::payer`, `realloc::zero`

**Behavior**:
- If growing: transfers lamports from payer for additional rent
- If shrinking: returns excess lamports to payer
- `realloc::zero = true` zeros new bytes when growing

**Example**:
```rust
#[account(
    mut,
    realloc = 8 + new_size,
    realloc::payer = user,
    realloc::zero = true
)]
pub dynamic_account: Account<'info, DynamicData>,
```

---

## 7. `signer`

**Purpose**: Asserts that an account has signed the transaction.

**Behavior**:
- Checks `account_info.is_signer == true`

**Note**: The `Signer<'info>` type automatically enforces this.

**Example**:
```rust
#[account(signer)]
pub authority: AccountInfo<'info>,
```

---

## 8. `has_one`

**Purpose**: Validates that a field in the account matches another account in the instruction.

**Behavior**:
- Reads a `Pubkey` field from the account
- Compares it to another account's key

**Example**:
```rust
#[account(has_one = authority)]
pub config: Account<'info, Config>,

pub authority: Signer<'info>,
```

**Generated check**:
```rust
if config.authority != authority.key() {
    return Err(ErrorCode::ConstraintHasOne.into());
}
```

---

## 9. `owner`

**Purpose**: Validates the owner program of an account.

**Example**:
```rust
#[account(owner = token::ID)]
pub token_account: AccountInfo<'info>,
```

**Generated check**:
```rust
if *account.owner != expected_owner {
    return Err(ErrorCode::ConstraintOwner.into());
}
```

---

## 10. `address`

**Purpose**: Validates that an account's address matches an expected value.

**Example**:
```rust
#[account(address = MY_CONSTANT_PUBKEY)]
pub specific_account: AccountInfo<'info>,
```

**Generated check**:
```rust
if account.key() != expected_address {
    return Err(ErrorCode::ConstraintAddress.into());
}
```

---

## 11. `executable`

**Purpose**: Validates that an account is an executable program.

**Example**:
```rust
#[account(executable)]
pub some_program: AccountInfo<'info>,
```

**Generated check**:
```rust
if !account.executable {
    return Err(ErrorCode::ConstraintExecutable.into());
}
```

---

## 12. `seeds` + `bump`

**Purpose**: Derives and validates a PDA (Program Derived Address).

**Behavior**:
- Derives PDA from provided seeds
- Validates account address matches derived PDA
- Can be combined with `init` to create PDAs

**Example**:
```rust
#[account(
    init,
    seeds = [b"config", user.key().as_ref()],
    bump,
    payer = user,
    space = 8 + 64
)]
pub user_config: Account<'info, UserConfig>,
```

**With explicit bump**:
```rust
#[account(
    seeds = [b"config", user.key().as_ref()],
    bump = user_config.bump
)]
pub user_config: Account<'info, UserConfig>,
```

---

## 13. `rent_exempt`

**Purpose**: Controls rent exemption behavior.

**Options**:
- `rent_exempt = enforce`: Account must be rent-exempt
- `rent_exempt = skip`: Skip rent exemption check

**Example**:
```rust
#[account(rent_exempt = enforce)]
pub my_account: Account<'info, MyData>,
```

---

## 14. `constraint`

**Purpose**: Arbitrary boolean constraint with custom error.

**Behavior**:
- Evaluates a boolean expression
- Fails with custom error if false

**Example**:
```rust
#[account(
    constraint = counter.count < 100 @ MyError::CounterOverflow
)]
pub counter: Account<'info, Counter>,
```

**Generated check**:
```rust
if !(counter.count < 100) {
    return Err(MyError::CounterOverflow.into());
}
```

---

## Constraint Execution Order

When multiple constraints are specified, they execute in this order:

1. **Signer checks** (`signer`)
2. **Owner checks** (`owner`)
3. **Address checks** (`address`)
4. **Executable checks** (`executable`)
5. **Seeds derivation** (`seeds`, `bump`)
6. **Account creation** (`init`, `init_if_needed`, `zero`)
7. **Mutability checks** (`mut`)
8. **Rent exemption** (`rent_exempt`)
9. **Has-one relationships** (`has_one`)
10. **Custom constraints** (`constraint`)
11. **Reallocation** (`realloc`)
12. **Account closing** (`close`) - happens in exit handler

---

## Combined Example

```rust
#[derive(Accounts)]
#[instruction(new_authority: Pubkey)]
pub struct TransferAuthority<'info> {
    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump,
        has_one = authority,
        constraint = new_authority != Pubkey::default() @ MyError::InvalidAuthority
    )]
    pub config: Account<'info, Config>,

    pub authority: Signer<'info>,
}
```

This validates:
1. Account is writable
2. PDA matches expected derivation
3. `config.authority == authority.key()`
4. New authority is not the default pubkey
