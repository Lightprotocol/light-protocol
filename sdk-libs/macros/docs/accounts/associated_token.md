# Compressed Associated Token Account Lifecycle

## Usage

```rust
#[derive(Accounts, LightAccounts)]
```

### Field Attribute

```
#[light_account(init, associated_token::authority = ..., associated_token::mint = ...)]    # Creates ATA
#[light_account(associated_token::authority = ..., associated_token::mint = ...)]          # Mark-only (existing ATA)
```

### Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `associated_token::authority` | Yes | ATA owner field reference |
| `associated_token::mint` | Yes | Token mint field reference |
| `associated_token::bump` | No | Explicit bump, auto-derived if omitted |

Shorthand: `associated_token::authority` alone means `associated_token::authority = authority`.

### Infrastructure (auto-detected by name)

```
fee_payer                            # Pays tx fee
light_token_config                   # Token program config
light_token_rent_sponsor             # Funds rent-free creation
light_token_program                  # CToken program
system_program                       # System program
```

### Example

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateAtaParams)]
pub struct CreateAta<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub mint: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,

    #[account(mut)]
    #[light_account(init,
        associated_token::authority = owner,
        associated_token::mint = mint,
        associated_token::bump = params.ata_bump
    )]
    pub user_ata: UncheckedAccount<'info>,

    pub light_token_config: AccountInfo<'info>,
    #[account(mut)]
    pub light_token_rent_sponsor: AccountInfo<'info>,
    pub light_token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

---

## ATA Derivation

ATAs are derived using a fixed seed pattern:

```rust
Pubkey::find_program_address(
    &[
        owner.as_ref(),
        LIGHT_TOKEN_PROGRAM_ID.as_ref(),
        mint.as_ref(),
    ],
    &LIGHT_TOKEN_PROGRAM_ID,
)
```

**Key differences from regular token accounts:**
- Seeds are fixed (not user-defined)
- Derived by light-token-program (not calling program)
- No signer seeds needed for creation

---

## Runtime

State machine: **No Account -> Decompressed <-> Compressed**

### Lifecycle Comparison

| Aspect | PDA | ATA |
|--------|-----|-----|
| State tracking | `CompressionInfo` embedded | `CompressedOnly` extension |
| Derivation | User-defined seeds | Fixed (owner, program_id, mint) |
| Creation signer | Program PDA | Light Token Program |
| Compress/Decompress | Separate CPI | Transfer2 instruction |

---

## 1. Init Phase (Creation)

### Accounts Layout

```
[0] owner              (readonly)  - Wallet owner for derivation
[1] mint               (readonly)  - Token mint
[2] fee_payer          (signer)    - Pays for creation
[3] ata                (writable)  - ATA to create
[4] system_program     (readonly)
[5] compressible_config (readonly) - Light token config
[6] rent_sponsor       (writable)  - Rent sponsor
```

### Checks

| Check | Error |
|-------|-------|
| ATA derivation matches | `InvalidSeeds` |
| Idempotent (skip if exists) | - |
| Config version valid | `InvalidAccountData` |
| Rent sponsor valid | `InvalidAccountData` |

### State Changes

- **On-chain**: ATA created with `CompressedOnly` extension
- **Token state**: `Token { owner, mint, amount: 0, state: Initialized, extensions: [CompressedOnly { is_ata: 1 }] }`

---

## 2. Compress Phase

ATAs are compressed via Transfer2 instruction.

### Checks

| Check | Error |
|-------|-------|
| ATA owner matches signer | `ConstraintOwner` |
| Has CompressedOnly extension | `InvalidAccountData` |
| is_ata flag set | `InvalidAccountData` |

### State Changes

- **On-chain**: ATA closed, lamports returned to rent sponsor
- **Off-chain**: Compressed token created with `extensions: [CompressedOnly { is_ata: 1 }]`

---

## 3. Decompress Phase

ATAs are decompressed via Transfer2 instruction.

### Checks

| Check | Error |
|-------|-------|
| Compressed account proof valid | `ProofVerificationFailed` |
| CompressedOnly.is_ata == true | Skip (not ATA path) |
| ATA derivation matches | `InvalidSeeds` |

### State Changes

- **On-chain**: ATA created (if not exists) or balance updated
- **Off-chain**: Compressed token nullified

### Decompression Behavior

```rust
// ATA path: invoke() WITHOUT signer seeds
if token_account_info.data_is_empty() {
    invoke(&create_ata_instruction, remaining_accounts)?;
}
// Wallet owner signs Transfer2 (not the ATA pubkey)
token_data.owner = wallet_owner_index;
```

---

## 4. Token Data Structure

```rust
pub struct Token {
    pub mint: Pubkey,
    pub owner: Pubkey,          // Wallet owner
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: AccountState,    // Initialized/Frozen
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    pub close_authority: Option<Pubkey>,
    pub account_type: u8,       // ShaFlat = 3
    pub extensions: Option<Vec<ExtensionStruct>>,
}

pub struct CompressedOnlyExtension {
    pub delegated_amount: u64,
    pub withheld_transfer_fee: u64,
    pub is_ata: u8,             // 1 = ATA, 0 = regular
}
```

---

## 5. Verification

### ATA Decompressed

1. ATA exists at derived address
2. Token state is `Initialized` or `Frozen`
3. Owner matches wallet owner
4. Mint matches token mint
5. Compressed account nullified

### ATA Compressed

1. On-chain ATA closed (data empty)
2. Compressed token exists (query via RPC)
3. `CompressedOnly.is_ata == 1`
4. Owner/mint match original

### Derivation Check

```rust
use light_token::instruction::derive_associated_token_account;

let (expected_ata, _) = derive_associated_token_account(&owner, &mint);
assert_eq!(ata_pubkey, expected_ata);
```

---

## Source Files

| Component | Location |
|-----------|----------|
| ATA creation | `token-sdk/src/instruction/create_ata.rs` |
| Compress/Decompress | `sdk/src/interface/token.rs` |
| Derivation | `token-sdk/src/instruction/create_ata.rs:17-26` |

## Related

- [pda.md](./pda.md) - Compressed PDAs
- [token.md](./token.md) - Token accounts (vaults)
- [architecture.md](./architecture.md) - LightAccounts overview
