<!-- cargo-rdme start -->

# Light Accounts Pinocchio

Rent-free Light Accounts and Light Token Accounts for Pinocchio programs.

## How It Works

**Light Accounts (PDAs)**
1. Create a Solana PDA normally
2. Register it with `#[derive(LightProgramPinocchio)]` - becomes a Light Account
3. Use it as normal Solana account
4. When rent runs out, account compresses (cold state)
5. State preserved on-chain, client loads when needed (hot state)

**Light Token Accounts (associated token accounts, Vaults)**
- Use `#[light_account(associated_token)]` for associated token accounts
- Use `#[light_account(token::seeds = [...], token::owner_seeds = [...])]` for vaults
- Cold/hot lifecycle

**Light Mints**
- Created via `invoke_create_mints`
- Cold/hot lifecycle

## Quick Start

### 1. Program Setup

```rust
use light_account_pinocchio::{derive_light_cpi_signer, CpiSigner, LightProgramPinocchio};
use pinocchio_pubkey::pubkey;

pub const ID: Pubkey = pubkey!("Your11111111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("Your11111111111111111111111111111111111111");
```

### 2. State Definition

```rust
use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CompressionInfo, LightDiscriminator, LightHasherSha};

#[derive(BorshSerialize, BorshDeserialize, LightDiscriminator, LightHasherSha)]
pub struct MyRecord {
    pub compression_info: CompressionInfo,  // Required first or last field
    pub owner: [u8; 32],
    pub data: u64,
}
```

### 3. Program Accounts Enum

```rust
#[derive(LightProgramPinocchio)]
pub enum ProgramAccounts {
    #[light_account(pda::seeds = [b"record", ctx.owner])]
    MyRecord(MyRecord),
}
```

## Account Types

### 1. Light Account (PDA)

```rust
#[light_account(pda::seeds = [b"record", ctx.owner])]
MyRecord(MyRecord),
```

### 2. Light Account (zero-copy)

```rust
#[light_account(pda::seeds = [b"record", ctx.owner], pda::zero_copy)]
ZeroCopyRecord(ZeroCopyRecord),
```

### 3. Light Token Account (vault)

```rust
#[light_account(token::seeds = [b"vault", ctx.mint], token::owner_seeds = [b"vault_auth"])]
Vault,
```

### 4. Light Token Account (associated token account)

```rust
#[light_account(associated_token)]
Ata,
```

## Required Derives

| Derive | Use |
|--------|-----|
| `LightDiscriminator` | State structs (8-byte discriminator) |
| `LightHasherSha` | State structs (compression hashing) |
| `LightProgramPinocchio` | Program accounts enum |

## Required Macros

| Macro | Use |
|-------|-----|
| `derive_light_cpi_signer!` | CPI signer PDA constant |
| `pinocchio_pubkey::pubkey!` | Program ID as `Pubkey` |

For a complete example, see `sdk-tests/pinocchio-light-program-test`.

<!-- cargo-rdme end -->
