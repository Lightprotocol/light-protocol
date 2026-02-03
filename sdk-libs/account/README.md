<!-- cargo-rdme start -->

# Light Accounts

Rent-free Light Accounts and Light Token Accounts for Anchor programs.

## How It Works

**Light Accounts (PDAs)**
1. Create a Solana PDA normally (Anchor `init`)
2. Add `#[light_account(init)]` - becomes a Light Account
3. Use it as normal Solana account
3. When rent runs out, account compresses (cold state)
4. State preserved on-chain, client loads when needed (hot state)
5. When account is hot, use it as normal Solana account

**Light Token Accounts (associated token accounts, Vaults)**
- Use `#[light_account(init, associated_token, ...)]` for associated token accounts
- Use `#[light_account(init, token, ...)]` for program-owned vaults
- Cold/hot lifecycle

**Light Mints**
- Created via `CreateMintsCpi`
- Cold/hot lifecycle

## Quick Start

### 1. Program Setup

```rust
use light_account::{derive_light_cpi_signer, light_program, CpiSigner};

declare_id!("Your11111111111111111111111111111111111111");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("Your11111111111111111111111111111111111111");

#[light_program]
#[program]
pub mod my_program {
    // ...
}
```

### 2. State Definition

```rust
use light_account::{CompressionInfo, LightAccount};

#[derive(Default, LightAccount)]
#[account]
pub struct UserRecord {
    pub compression_info: CompressionInfo,  // Required field
    pub owner: Pubkey,
    pub data: u64,
}
```

### 3. Accounts Struct

```rust
use light_account::{CreateAccountsProof, LightAccounts};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: Rent sponsor
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(init, payer = fee_payer, space = 8 + UserRecord::INIT_SPACE, seeds = [b"record", params.owner.as_ref()], bump)]
    #[light_account(init)]
    pub record: Account<'info, UserRecord>,

    pub system_program: Program<'info, System>,
}
```

## Account Types

### 1. Light Account (PDA)

```rust
#[account(init, payer = fee_payer, space = 8 + MyRecord::INIT_SPACE, seeds = [...], bump)]
#[light_account(init)]
pub record: Account<'info, MyRecord>,
```

### 2. Light Account (zero-copy)

```rust
#[account(init, payer = fee_payer, space = 8 + size_of::<MyZcRecord>(), seeds = [...], bump)]
#[light_account(init, zero_copy)]
pub record: AccountLoader<'info, MyZcRecord>,
```

### 3. Light Token Account (vault)

**With `init` (Anchor-created):**
```rust
#[account(mut, seeds = [b"vault", mint.key().as_ref()], bump)]
#[light_account(init, token::seeds = [b"vault", self.mint.key()], token::owner_seeds = [b"vault_authority"])]
pub vault: UncheckedAccount<'info>,
```

**Without `init` (manual creation via `CreateTokenAccountCpi`):**
```rust
#[account(mut, seeds = [b"vault", mint.key().as_ref()], bump)]
#[light_account(token::seeds = [b"vault", self.mint.key()], token::owner_seeds = [b"vault_authority"])]
pub vault: UncheckedAccount<'info>,
```

### 4. Light Token Account (associated token account)

**With `init` (Anchor-created):**
```rust
#[account(mut)]
#[light_account(init, associated_token::authority = owner, associated_token::mint = mint, associated_token::bump = params.bump)]
pub token_account: UncheckedAccount<'info>,
```

**Without `init` (manual creation via `CreateTokenAtaCpi`):**
```rust
#[account(mut)]
#[light_account(associated_token::authority = owner, associated_token::mint = mint)]
pub token_account: UncheckedAccount<'info>,
```

### 5. Light Mint

```rust
#[account(mut)]
#[light_account(init,
    mint::signer = mint_signer,           // PDA that signs mint creation
    mint::authority = mint_authority,     // Mint authority
    mint::decimals = 9,                   // Token decimals
    mint::seeds = &[SEED, self.key.as_ref()],  // Seeds for mint PDA
    mint::bump = params.bump,             // Bump seed
    // Optional: PDA authority
    mint::authority_seeds = &[b"authority"],
    mint::authority_bump = params.auth_bump,
    // Optional: Token metadata
    mint::name = params.name,
    mint::symbol = params.symbol,
    mint::uri = params.uri,
    mint::update_authority = update_auth,
    mint::additional_metadata = params.metadata
)]
pub mint: UncheckedAccount<'info>,
```

## Required Derives

| Derive | Use |
|--------|-----|
| `LightAccount` | State structs (must have `compression_info: CompressionInfo`) |
| `LightAccounts` | Accounts structs with `#[light_account(...)]` fields |

## Required Macros

| Macro | Use |
|-------|-----|
| `#[light_program]` | Program module (before `#[program]`) |
| `derive_light_cpi_signer!` | CPI signer PDA constant |
| `derive_light_rent_sponsor_pda!` | Rent sponsor PDA (optional) |

<!-- cargo-rdme end -->
