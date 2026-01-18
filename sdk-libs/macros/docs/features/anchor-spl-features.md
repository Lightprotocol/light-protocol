# Anchor SPL Features

This document covers the 12 SPL-specific features available in Anchor through the `anchor-spl` crate for working with tokens and mints.

## Overview

Anchor SPL provides typed wrappers and constraints for SPL Token and Token-2022 programs. These features enable type-safe token account and mint initialization with validation.

```rust
use anchor_spl::token::{Token, TokenAccount, Mint};
use anchor_spl::token_interface::{TokenInterface, TokenAccount as InterfaceTokenAccount};
```

---

## Token Account Constraints

### 1. `token::mint`

**Purpose**: Validates that a token account's mint matches the specified mint.

**Behavior**:
- Reads the `mint` field from the token account
- Compares against the specified mint account's key

**Example**:
```rust
#[account(token::mint = usdc_mint)]
pub user_token_account: Account<'info, TokenAccount>,

pub usdc_mint: Account<'info, Mint>,
```

**Generated check**:
```rust
if user_token_account.mint != usdc_mint.key() {
    return Err(ErrorCode::ConstraintTokenMint.into());
}
```

---

### 2. `token::authority`

**Purpose**: Validates that a token account's authority (owner) matches the specified account.

**Behavior**:
- Reads the `owner` field from the token account
- Compares against the specified authority's key

**Example**:
```rust
#[account(token::authority = user)]
pub user_token_account: Account<'info, TokenAccount>,

pub user: Signer<'info>,
```

**Generated check**:
```rust
if user_token_account.owner != user.key() {
    return Err(ErrorCode::ConstraintTokenOwner.into());
}
```

---

### 3. `token::token_program`

**Purpose**: Specifies which token program owns this account (SPL Token or Token-2022).

**Use case**: When working with Token-2022 accounts or when the program could be either.

**Example**:
```rust
#[account(
    init,
    payer = user,
    token::mint = mint,
    token::authority = user,
    token::token_program = token_program
)]
pub token_account: InterfaceAccount<'info, TokenAccount>,

pub token_program: Interface<'info, TokenInterface>,
```

---

## Mint Constraints

### 4. `mint::authority`

**Purpose**: Sets or validates the mint authority.

**Behavior when initializing**: Sets the mint authority to the specified account
**Behavior when validating**: Checks mint's `mint_authority` matches

**Example** (initialization):
```rust
#[account(
    init,
    payer = user,
    mint::decimals = 6,
    mint::authority = authority
)]
pub new_mint: Account<'info, Mint>,

pub authority: Signer<'info>,
```

---

### 5. `mint::decimals`

**Purpose**: Sets or validates the mint's decimal places.

**Behavior when initializing**: Sets decimals to specified value
**Behavior when validating**: Checks mint's `decimals` matches

**Example**:
```rust
#[account(
    init,
    payer = user,
    mint::decimals = 9,
    mint::authority = authority
)]
pub new_mint: Account<'info, Mint>,
```

---

### 6. `mint::freeze_authority`

**Purpose**: Sets or validates the freeze authority for a mint.

**Behavior**: Optional authority that can freeze token accounts

**Example**:
```rust
#[account(
    init,
    payer = user,
    mint::decimals = 6,
    mint::authority = authority,
    mint::freeze_authority = freeze_authority
)]
pub new_mint: Account<'info, Mint>,

pub freeze_authority: AccountInfo<'info>,
```

---

## Associated Token Account Constraints

### 7. `associated_token::mint`

**Purpose**: Specifies the mint for an associated token account derivation.

**Behavior**: Used in PDA derivation: `[user.key, token_program.key, mint.key]`

**Example**:
```rust
#[account(
    init,
    payer = user,
    associated_token::mint = mint,
    associated_token::authority = user
)]
pub user_ata: Account<'info, TokenAccount>,
```

---

### 8. `associated_token::authority`

**Purpose**: Specifies the authority (wallet) for an associated token account.

**Behavior**: The wallet whose ATA this is

**Example**:
```rust
#[account(
    associated_token::mint = mint,
    associated_token::authority = wallet
)]
pub wallet_ata: Account<'info, TokenAccount>,

pub wallet: SystemAccount<'info>,
pub mint: Account<'info, Mint>,
```

---

### 9. `associated_token::token_program`

**Purpose**: Specifies the token program for ATA derivation.

**Use case**: When working with Token-2022 associated token accounts

**Example**:
```rust
#[account(
    init,
    payer = user,
    associated_token::mint = mint,
    associated_token::authority = user,
    associated_token::token_program = token_program
)]
pub user_ata: InterfaceAccount<'info, TokenAccount>,

pub token_program: Interface<'info, TokenInterface>,
pub associated_token_program: Program<'info, AssociatedToken>,
```

---

## Interface Types

### 10. `InterfaceAccount<'info, T>`

**Purpose**: Account wrapper that works with both SPL Token and Token-2022.

**Behavior**:
- Accepts accounts from either token program
- Validates based on the interface, not specific program

**Example**:
```rust
use anchor_spl::token_interface::TokenAccount;

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub from: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub to: InterfaceAccount<'info, TokenAccount>,
}
```

---

### 11. `TokenInterface`

**Purpose**: Interface type that accepts either SPL Token or Token-2022 program.

**Example**:
```rust
use anchor_spl::token_interface::TokenInterface;

#[derive(Accounts)]
pub struct TokenOp<'info> {
    pub token_program: Interface<'info, TokenInterface>,
}
```

**Generated validation**:
```rust
// Accepts either:
// - spl_token::ID
// - spl_token_2022::ID
```

---

## Token-2022 Extensions

### 12. Token-2022 Extension Support

**Purpose**: Support for Token-2022 extension features.

Anchor SPL supports these Token-2022 extensions through the interface types:

| Extension | Description |
|-----------|-------------|
| Transfer Fee | Automatic fee on transfers |
| Interest-Bearing | Accruing interest on balance |
| Non-Transferable | Soul-bound tokens |
| Permanent Delegate | Irrevocable delegate authority |
| Transfer Hook | Custom transfer logic |
| Metadata | On-chain token metadata |
| Confidential Transfers | ZK-based private transfers |
| Default Account State | Default frozen/unfrozen |
| CPI Guard | Prevents CPI-based attacks |
| Immutable Owner | Cannot change token account owner |
| Memo Required | Requires memo on transfers |
| Close Authority | Required authority to close |

**Example with extensions**:
```rust
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{
    Mint as InterfaceMint,
    TokenAccount as InterfaceTokenAccount,
};

#[derive(Accounts)]
pub struct Token2022Op<'info> {
    #[account(
        mut,
        token::mint = mint,
        token::authority = owner,
        token::token_program = token_program
    )]
    pub token_account: InterfaceAccount<'info, InterfaceTokenAccount>,

    pub mint: InterfaceAccount<'info, InterfaceMint>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}
```

---

## Complete Initialization Example

```rust
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The mint to create
    #[account(
        init,
        payer = payer,
        mint::decimals = 6,
        mint::authority = payer,
        mint::freeze_authority = payer,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// The payer's associated token account
    #[account(
        init,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = payer,
        associated_token::token_program = token_program,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
```

---

## Constraint Combinations

### Token Account with All Constraints

```rust
#[account(
    mut,
    token::mint = mint,
    token::authority = owner,
    token::token_program = token_program,
    constraint = token_account.amount >= required_amount @ MyError::InsufficientBalance
)]
pub token_account: InterfaceAccount<'info, TokenAccount>,
```

### Mint with All Constraints

```rust
#[account(
    init,
    payer = payer,
    mint::decimals = decimals,
    mint::authority = mint_authority,
    mint::freeze_authority = freeze_authority,
    mint::token_program = token_program,
)]
pub mint: InterfaceAccount<'info, Mint>,
```

---

## CPI Helpers

Anchor SPL also provides CPI helpers for token operations:

```rust
use anchor_spl::token_interface::{transfer_checked, TransferChecked};

// Transfer tokens
transfer_checked(
    CpiContext::new(
        token_program.to_account_info(),
        TransferChecked {
            from: from_account.to_account_info(),
            mint: mint.to_account_info(),
            to: to_account.to_account_info(),
            authority: authority.to_account_info(),
        },
    ),
    amount,
    decimals,
)?;
```
