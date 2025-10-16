# Accounts

## CompressibleConfig

### Description
Configuration account that defines compressible ctoken solana account behavior in the Light Protocol. This account is owned by the Light Registry program and stores rent parameters, authority keys, and address space configuration for compressible CToken accounts.

### State Layout
**Path:** `program-libs/compressible/src/config.rs`

**Size:** 256 bytes (including 8-byte discriminator)

```rust
#[repr(C)]
pub struct CompressibleConfig {
    pub version: u16,                    // 2 bytes - Config version for future upgrades
    pub state: u8,                        // 1 byte - State: 0=Inactive, 1=Active, 2=Deprecated
    pub bump: u8,                         // 1 byte - PDA bump seed
    pub update_authority: Pubkey,        // 32 bytes - Can update config state
    pub withdrawal_authority: Pubkey,    // 32 bytes - Can withdraw from rent recipient pool
    pub rent_sponsor: Pubkey,          // 32 bytes - CToken program PDA receiving rent
    pub compression_authority: Pubkey,          // 32 bytes - Registry PDA that can claim/compress
    pub rent_sponsor_bump: u8,         // 1 byte - Bump for rent_sponsor PDA
    pub compression_authority_bump: u8,         // 1 byte - Bump for compression_authority PDA
    pub rent_config: RentConfig,         // 8 bytes - Rent curve parameters
    pub address_space: [Pubkey; 4],      // 128 bytes - Allowed address trees
    pub _place_holder: [u8; 32],         // 32 bytes - Reserved for future use
}

pub struct RentConfig {
    pub base_rent: u16,                   // 2 bytes - Minimum rent per epoch
    pub compression_cost: u16, // 2 bytes - Compression cost + incentive
    pub lamports_per_byte_per_epoch: u8,               // 1 byte - Rent per byte per epoch
    _place_holder_bytes: [u8; 3],       // 3 bytes - Padding for alignment
}
```

### Discriminator
`[180, 4, 231, 26, 220, 144, 55, 168]` - 8-byte discriminator for account validation

### Ownership
**Owner:** Light Registry Program (`Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX`)

### PDA Derivation
**Seeds:** `[b"compressible_config", version.to_le_bytes()]`

**Bump:** Stored in account at `bump` field

**Common PDAs:**
- **V1 Config:** `derive_v1_config_pda(program_id)` - version = 1
- **Default:** `derive_default_pda(program_id)` - version = 0

```rust
// Derive any version
let (pda, bump) = CompressibleConfig::derive_pda(
    &program_id,
    version // u16
);

// Derive V1 config (most common)
let v1_pda = CompressibleConfig::ctoken_v1_config_pda();
```


**State Validation Methods:**
- `validate_active()` - Requires state == Active (for new account creation)
- `validate_not_inactive()` - Requires state != Inactive (for claims/closing)

### Associated Instructions

**Light Registry Program:**
- `update_compressible_config` - Updates config state and parameters
- `withdraw_funding_pool` (discriminator: 105) - Withdraws from rent_sponsor pool

**Compressed Token Program (uses config):**
- `CreateTokenAccount` (discriminator: 18) - Creates ctoken with compressible extension
- `CreateAssociatedTokenAccount` (discriminator: 100) - Creates ATA with compressible
- `Claim` (discriminator: 104) - Claims rent using config parameters
- `CompressAndClose` (via Transfer2) - Uses compression_authority from config

**Registry Program (via wrapper):**
- `compress_and_close` - Registry-authorized compression using compression_authority

### Serialization

**Zero-copy (for programs):**
```rust
use bytemuck::pod_from_bytes;

// Direct deserialization (no discriminator check)
let config = pod_from_bytes::<CompressibleConfig>(&account_data[8..])?;

// Access fields directly
let version = config.version;
let is_active = config.state == 1;
```

**Borsh (for clients):**
```rust
use borsh::BorshDeserialize;

// Skip discriminator and deserialize
let config = CompressibleConfig::deserialize(&mut &account_data[8..])?;

// Or with discriminator check
if &account_data[..8] != CompressibleConfig::DISCRIMINATOR {
    return Err(Error::InvalidDiscriminator);
}
let config = CompressibleConfig::deserialize(&mut &account_data[8..])?;
```

**Anchor (when feature enabled):**
```rust
use anchor_lang::AccountDeserialize;

// Includes discriminator validation
let config = CompressibleConfig::try_deserialize(&mut &account_data[..])?;
```

### Security Notes
- Update authority can modify config state but cannot withdraw funds
- Withdrawal authority can only withdraw from rent_sponsor PDA pool
- Rent authority (Registry PDA) enables permissionless compression by a forester node when conditions met
- Config state determines instruction availability:
  - Active: All operations allowed
  - Deprecated: No new account creation, existing operations continue
  - Inactive: Config cannot be used

### Default Values
```rust
// CToken V1 defaults
RentConfig {
    base_rent: 1220,                    // BASE_RENT constant
    compression_cost: 11000, // COMPRESSION_COST + COMPRESSION_INCENTIVE
    lamports_per_byte_per_epoch: 10,                  // RENT_PER_BYTE constant
}

// Default address space (V1)
address_space[0] = pubkey!("amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx")
```

### Methods

**Constants:**
- `LEN = 256` - Account size in bytes

**State Validation:**
- `validate_active()` - Ensures config is Active (for account creation)
- `validate_not_inactive()` - Ensures config is not Inactive (for operations)

**Constructors:**
- `ctoken_v1(update, withdrawal)` - V1 config with default rent params
- `new_ctoken(version, active, update, withdrawal, rent)` - Custom ctoken config
- `new(...)` - Full constructor with all fields

**PDA Derivation:**
- `derive_pda(program_id, version)` - Derive config account address
- `ctoken_v1_config_pda()` - Get V1 config for Light Registry
- `derive_v1_config_pda(program_id)` - Get V1 config for any program
- `derive_default_pda(program_id)` - Get V0 config for any program

**Seed Helpers:**
- `get_compression_authority_seeds(version)` - Seeds for rent authority PDA
- `get_rent_sponsor_seeds(version)` - Seeds for rent recipient PDA
