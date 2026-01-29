# Compressed PDA Lifecycle

## Usage

```
#[derive(Accounts, LightAccounts)]
```

Generates `LightPreInit::light_pre_init()` impl, called by `#[light_program]` wrapper.

### Execution Flow

`#[light_program]` wraps instruction handlers:
1. Anchor deserialization
2. `light_pre_init()`  <-- injected here
3. Your handler code
4. Anchor serialization

### Field Attribute

```
#[light_account(init)]               # Registers compressed address, reimburses rent
#[light_account(init, zero_copy)]    # Same, for AccountLoader<T> with Pod types
```

### Field Types

```
Account<'info, T>                    # Standard
Box<Account<'info, T>>               # Large accounts (stack overflow prevention)
AccountLoader<'info, T>              # Zero-copy, requires zero_copy keyword
```

### Infrastructure (auto-detected by name)

```
fee_payer                            # Pays tx fee, receives rent reimbursement
compression_config                   # LightConfig PDA - address space, rent params
pda_rent_sponsor                     # Funds rent reimbursement to fee_payer
```

### Instruction Params

`#[instruction(params: MyParams)]` required on accounts struct.
Macro looks for `create_accounts_proof` field in params:
- `params.create_accounts_proof` if params is a struct
- `create_accounts_proof` if passed directly as instruction arg

`CreateAccountsProof` contains:
- `address_tree_info` - Merkle tree for address registration
- `output_state_tree_index` - Which state tree to write to

### Example

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct Create<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub compression_config: AccountInfo<'info>,
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(init, payer = fee_payer, space = 8 + T::INIT_SPACE, seeds = [...], bump)]
    #[light_account(init)]
    pub record: Account<'info, T>,

    pub system_program: Program<'info, System>,
}
```

### LightPreInit (per PDA field)

1. Extract account info + key
2. Resolve address tree from CPI accounts
3. Init CompressionInfo from config
4. Call `prepare_compressed_account_on_init` (hash, register address)
5. Reimburse rent from sponsor to fee_payer

---

## Runtime

State machine: **No Account -> Decompressed <-> Compressed**

### Known Limitations

- **compression_authority validation NOT implemented** - See TODO at `compress.rs:120-127` and `compress.rs:144-147`

### Inconsistencies

1. **Config validation error codes inconsistent**:
   - Init phase: returns `ConstraintViolation`
   - Compress/Decompress phases: returns `InvalidAccountData`

## 1. Init Phase

### Checks

| Check | Location | Error |
|-------|----------|-------|
| Empty accounts (skip if empty) | `init.rs:111-113` | - |
| Rent sponsor PDA derivation | `init.rs:127-133` | `InvalidSeeds` |
| Config version == 1 | `config.rs:94-101` | `ConstraintViolation` |
| Address space == 1 | `config.rs:102-108` | `ConstraintViolation` |
| Config bump == 0 | `config.rs:109-116` | `ConstraintViolation` |
| Config owner == program_id | `config.rs:126-133` | `ConstraintViolation` |
| Config PDA derivation | `config.rs:145-153` | `ConstraintViolation` |
| Hash computation | `init.rs:73` | `HasherError` |

### State Changes

- **On-chain**: PDA created, `CompressionInfo` initialized with `Decompressed` state
- **Off-chain**: Address registered with `DECOMPRESSED_PDA_DISCRIMINATOR` `[255,255,255,255,255,255,255,0]`
- **Data**: PDA pubkey bytes (32 bytes)

## 2. Compress Phase

### Accounts Layout

```
[0] fee_payer          (Signer, mut)
[1] config             (LightConfig PDA)
[2] rent_sponsor       (mut)
[3] compression_authority
[system_offset..]      Light system accounts for CPI
[end-n..]              PDA accounts to compress
```

### Checks

| Check | Location | Error |
|-------|----------|-------|
| Instruction data deser | `compress.rs:98-101` | `InvalidInstructionData` |
| Config owner == program_id | `config.rs:126-133` | `InvalidAccountData` |
| Config version/address_space/bump | `config.rs:94-117` | `InvalidAccountData` |
| Config PDA derivation | `config.rs:145-153` | `InvalidAccountData` |
| rent_sponsor == config.rent_sponsor | `compress.rs:136-143` | `InvalidAccountData` |
| system_accounts_offset valid | `compress.rs:149-157` | `InvalidInstructionData` |
| pda_accounts_start valid | `compress.rs:177-183` | `InvalidInstructionData` |
| Account not owned by program | `compress.rs:194-196` | Skip (continue) |
| Account empty | `compress.rs:190-192` | Skip (continue) |
| Rent compressibility check | `compress.rs:279-284` | `Custom(1)` |

### Data Processing

1. **Input**: Placeholder with `DECOMPRESSED_PDA_DISCRIMINATOR` and PDA pubkey as data hash
2. **Hash**: `Sha256::hash(borsh_data)` with first byte zeroed
3. **CompressionInfo**: Canonicalized to `CompressionInfo::compressed()` before hashing
4. **Output**: Actual account data with real discriminator

## 3. Decompress Phase

### Accounts Layout

```
[0] fee_payer          (Signer, mut)
[1] config             (LightConfig PDA)
[2] rent_sponsor       (mut)
[system_offset..]      Light system accounts for CPI
[end-n..]              PDA accounts to decompress
```

### Checks

| Check | Location | Error |
|-------|----------|-------|
| Instruction data deser | `decompress.rs:141-142` | `InvalidInstructionData` |
| Config owner == program_id | `config.rs:126-133` | `InvalidAccountData` |
| Config version/address_space/bump | `config.rs:94-117` | `InvalidAccountData` |
| Config PDA derivation | `config.rs:145-153` | `InvalidAccountData` |
| Rent sponsor PDA derived | `decompress.rs:161-169` | `InvalidAccountData` |
| system_accounts_offset valid | `decompress.rs:174-177` | `InvalidInstructionData` |
| Idempotency (discriminator != 0) | `pda.rs:44-50` | Skip (Ok) |
| Unpack succeeds | `pda.rs:56-58` | `InvalidAccountData` |
| Hash matches proof | CPI | `ProofVerificationFailed` |

### Data Processing

1. Seeds from `packed.seed_vec()` + bump
2. Hash: `Sha256::hash(borsh_data)` with first byte zeroed
3. Space: `8 + max(data_len, INIT_SPACE)`
4. PDA created via `create_pda_account` with rent sponsor signing
5. Discriminator written, `CompressionInfo` set to `Decompressed`

## 4. CompressionInfo

24 bytes, Pod-compatible. Defined in `sdk/src/interface/compression_info.rs:166-197`.

| Field | Type | Offset | Size | Purpose |
|-------|------|--------|------|---------|
| `last_claimed_slot` | `u64` | 0 | 8 | Rent tracking epoch boundary |
| `lamports_per_write` | `u32` | 8 | 4 | Top-up amount per write |
| `config_version` | `u16` | 12 | 2 | Config version at init |
| `state` | `CompressionState` | 14 | 1 | 0=Uninit, 1=Decompressed, 2=Compressed |
| `_padding` | `u8` | 15 | 1 | Alignment |
| `rent_config` | `RentConfig` | 16 | 8 | Rent parameters |

## 5. Verification

### PDA Compressed

1. On-chain PDA closed (owner == System Program, data empty)
2. Compressed account exists (query via RPC with PDA pubkey as address seed)
3. Data hash matches: `Sha256::hash(borsh_data)[0] = 0`
4. Discriminator is real account type (not `DECOMPRESSED_PDA_DISCRIMINATOR`)

### PDA Decompressed

1. PDA exists at derived address
2. First 8 bytes match `LIGHT_DISCRIMINATOR`
3. `compression_info.state == CompressionState::Decompressed`
4. Seeds + bump derive to expected PDA address
5. Compressed account nullified (zero discriminator, empty data)

### Hash Verification

```rust
use light_hasher::{Hasher, Sha256};

let data_bytes = account.try_to_vec()?;
let mut data_hash = Sha256::hash(&data_bytes)?;
data_hash[0] = 0;  // Protocol convention
```

## Source Files

| Phase | Macro | Runtime |
|-------|-------|---------|
| Init | `macros/src/light_pdas/accounts/pda.rs` | `sdk/src/interface/init.rs` |
| Compress | `macros/src/light_pdas/program/compress.rs` | `sdk/src/interface/compress.rs` |
| Decompress | `macros/src/light_pdas/program/decompress.rs` | `sdk/src/interface/decompress.rs`, `pda.rs` |
| Config | - | `sdk/src/interface/config.rs` |
| CompressionInfo | - | `sdk/src/interface/compression_info.rs` |
