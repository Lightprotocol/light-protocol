# Compressed Mint Lifecycle

## Usage

```rust
#[derive(Accounts, LightAccounts)]
```

### Field Attribute

```
#[light_account(init, mint::signer = ..., mint::authority = ..., mint::decimals = ..., mint::seeds = ...)]
```

### Required Parameters

| Parameter | Description |
|-----------|-------------|
| `mint::signer` | AccountInfo that seeds the mint PDA |
| `mint::authority` | Mint authority (signer or PDA) field reference |
| `mint::decimals` | Token decimals (expression) |
| `mint::seeds` | PDA signer seeds for mint_signer (WITHOUT bump - bump is added automatically) |

### Optional Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `mint::bump` | Auto-derived | Explicit bump for mint_seeds (auto-derived using `find_program_address` if omitted) |
| `mint::freeze_authority` | None | Freeze authority field reference |
| `mint::authority_seeds` | None | PDA seeds if authority is a PDA (WITHOUT bump) |
| `mint::authority_bump` | Auto-derived | Explicit bump for authority_seeds (auto-derived if omitted) |
| `mint::rent_payment` | `16u8` | Decompression rent epochs (~24h) |
| `mint::write_top_up` | `766u32` | Write top-up lamports |

**Seed handling:**
- User provides base seeds WITHOUT bump in both `mint::seeds` and `mint::authority_seeds`
- Macro auto-derives bumps using `Pubkey::find_program_address()` if not explicitly provided
- Bumps are always appended as the final seed in signer seeds arrays

### TokenMetadata Extension (all-or-nothing)

| Parameter | Description |
|-----------|-------------|
| `mint::name` | Token name (`Vec<u8>`) |
| `mint::symbol` | Token symbol (`Vec<u8>`) |
| `mint::uri` | Token URI (`Vec<u8>`) |
| `mint::update_authority` | Metadata update authority field |
| `mint::additional_metadata` | Additional key-value pairs |

### Infrastructure (auto-detected by name)

```
fee_payer                            # Pays tx fee
light_token_config                   # Token program config
light_token_rent_sponsor             # Funds rent-free creation
light_token_cpi_authority            # CPI authority for signing
light_token_program                  # CToken program
system_program                       # System program
```

### Example

```rust
const MINT_SIGNER_SEED: &[u8] = b"mint_signer";

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateMintParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub decimals: u8,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(seeds = [MINT_SIGNER_SEED], bump)]
    pub mint_signer: AccountInfo<'info>,

    // Mint creation - uses CreateMintsCpi in pre_init
    #[light_account(init,
        mint::signer = mint_signer,                     // Seeds the mint PDA
        mint::authority = authority,                    // Mint authority
        mint::decimals = params.decimals,               // Decimals from params
        mint::seeds = &[MINT_SIGNER_SEED]              // Seeds WITHOUT bump
    )]
    pub cmint: Account<'info, Mint>,

    // Infrastructure for mint creation
    pub light_token_config: Account<'info, CompressibleConfig>,
    #[account(mut)]
    pub light_token_rent_sponsor: Account<'info, RentSponsor>,
    pub light_token_cpi_authority: AccountInfo<'info>,
}
```

### Example with TokenMetadata Extension

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateTokenParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub decimals: u8,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateTokenParams)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub update_authority: AccountInfo<'info>,

    #[account(seeds = [b"mint"], bump)]
    pub mint_signer: AccountInfo<'info>,

    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = params.decimals,
        mint::seeds = &[b"mint"],
        mint::name = params.name.clone(),
        mint::symbol = params.symbol.clone(),
        mint::uri = params.uri.clone(),
        mint::update_authority = update_authority
    )]
    pub token_mint: Account<'info, Mint>,

    pub light_token_config: Account<'info, CompressibleConfig>,
    #[account(mut)]
    pub light_token_rent_sponsor: Account<'info, RentSponsor>,
    pub light_token_cpi_authority: AccountInfo<'info>,
}
```

---

## Mint Derivation

Mints are derived from a `mint_signer` pubkey:

```rust
pub fn find_mint_address(mint_seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.as_ref()],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}
```

**Key characteristics:**
- Mint address derived from `mint_signer` pubkey
- `COMPRESSED_MINT_SEED` is a constant prefix
- Derived by light-token-program

---

## Runtime

State machine: **No Account -> Compressed+Decompressed -> Compressed <-> Decompressed**

### Lifecycle Comparison

| Aspect | PDA | Mint |
|--------|-----|------|
| State tracking | `CompressionInfo` embedded | `CompressionInfo` + `MintMetadata` |
| Derivation | User-defined seeds | From `mint_signer` pubkey |
| Creation | Compressed only OR decompressed | Both compressed AND decompressed |
| Compress | Authority required | Permissionless (when rent expired) |
| Decompress | Authority required | Authority required |

---

## 1. Init Phase (Creation)

Creates **both** a compressed mint **and** a decompressed Mint Solana account in a single instruction.

### Accounts Layout

```
[0] light_system_program   (readonly)
[1] mint_seed              (signer)     - Seeds the mint PDA
[2] authority              (signer)     - Mint authority
[3] compressible_config    (readonly)   - Light token config
[4] mint                   (writable)   - Mint PDA to create
[5] rent_sponsor           (writable)   - Rent sponsor
[6] fee_payer              (signer)     - Pays for creation
[7..] system accounts                   - CPI accounts
```

### Checks

| Check | Error |
|-------|-------|
| Mint signer is signer | `ProgramError::MissingRequiredSignature` |
| Authority is signer (if no authority_seeds) | `ProgramError::MissingRequiredSignature` |
| Config version valid | `TokenError::InvalidAccountData` |
| Proof valid | `SystemProgramError::ProofVerificationFailed` |

### State Changes

- **On-chain**: Mint PDA created with `CompressionInfo`
- **Off-chain**: Compressed mint registered with address
- **Mint metadata**: `mint_decompressed = false` initially

### CreateMintParams

```rust
pub struct CreateMintParams {
    pub decimals: u8,
    pub address_merkle_tree_root_index: u16,
    pub mint_authority: Pubkey,
    pub proof: CompressedProof,
    pub compression_address: [u8; 32],
    pub mint: Pubkey,
    pub bump: u8,
    pub freeze_authority: Option<Pubkey>,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
    pub rent_payment: u8,      // Default: 16 (~24 hours)
    pub write_top_up: u32,     // Default: 766 (~3 hours per write)
}
```

---

## 2. Compress Phase

Compresses and closes the Mint Solana account. **Permissionless** when `is_compressible()` returns true (rent expired).

### Checks

| Check | Error |
|-------|-------|
| Mint exists (unless idempotent) | `InvalidAccountData` |
| `is_compressible()` returns true | `InvalidAccountData` |
| Not combined with DecompressMint | `InvalidInstructionData` |

### State Changes

- **On-chain**: Mint PDA closed, lamports returned to `rent_sponsor`
- **Off-chain**: Compressed mint state preserved
- **Mint metadata**: `mint_decompressed = false`

### CompressAndCloseMintAction

```rust
pub struct CompressAndCloseMintAction {
    /// If non-zero, succeed silently when Mint doesn't exist
    pub idempotent: u8,
}
```

**Idempotent mode**: Useful for foresters to handle already-compressed mints without failing.

---

## 3. Decompress Phase

Creates an on-chain Mint PDA from compressed state. Requires authority signature.

### Accounts Layout

```
[0] light_system_program              (readonly)
[1] authority                         (signer)   - Mint authority
[2] compressible_config               (readonly)
[3] mint                              (writable) - Mint PDA to create
[4] rent_sponsor                      (writable)
[5] fee_payer                         (signer)
[6] cpi_authority_pda                 (readonly) - CPI infrastructure
[7] registered_program_pda            (readonly)
[8] account_compression_authority     (readonly)
[9] account_compression_program       (readonly)
[10] system_program                   (readonly)
[11] output_queue                     (writable)
[12] state_tree                       (readonly)
[13] input_queue                      (readonly)
```

Note: `mint_seed` is not included for decompress (only needed for create/init).

### Checks

| Check | Error |
|-------|-------|
| Compressed mint proof valid | `SystemProgramError::ProofVerificationFailed` |
| Authority matches compressed mint authority | `TokenError::InvalidAccountData` |
| `rent_payment >= 2` | `ProgramError::InvalidInstructionData` |

### State Changes

- **On-chain**: Mint PDA created/updated
- **Off-chain**: Compressed mint updated with `mint_decompressed = true`

### DecompressMintAction

```rust
pub struct DecompressMintAction {
    pub rent_payment: u8,    // Epochs (must be >= 2)
    pub write_top_up: u32,   // Lamports for future writes
}
```

---

## 4. Mint Data Structure

```rust
pub struct Mint {
    pub base: BaseMint,
    pub metadata: MintMetadata,
    pub reserved: [u8; 16],              // T22 layout compatibility
    pub account_type: u8,                // ACCOUNT_TYPE_MINT = 1
    pub compression: CompressionInfo,
    pub extensions: Option<Vec<ExtensionStruct>>,
}

pub struct BaseMint {
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
}

pub struct MintMetadata {
    pub version: u8,                     // Version 3 = ShaFlat
    pub mint_decompressed: bool,         // true = on-chain is source of truth
    pub mint: Pubkey,                    // PDA derived from mint_signer
    pub mint_signer: [u8; 32],           // Signer used to derive mint
    pub bump: u8,                        // Bump from PDA derivation
}
```

### Hash Computation

```rust
impl Mint {
    pub fn hash(&self) -> Result<[u8; 32], TokenError> {
        match self.metadata.version {
            3 => Ok(Sha256BE::hash(self.try_to_vec()?.as_slice())?),
            _ => Err(TokenError::InvalidTokenDataVersion),
        }
    }
}
```

---

## 5. Verification

### Mint Decompressed

1. Mint PDA exists at derived address: `find_mint_address(mint_signer)`
2. `base.is_initialized == true`
3. `account_type == ACCOUNT_TYPE_MINT` (1)
4. `metadata.mint_decompressed == true`
5. Owner is `LIGHT_TOKEN_PROGRAM_ID`

### Mint Compressed

1. On-chain Mint PDA closed (data empty)
2. Compressed mint exists (query via RPC)
3. `metadata.mint_decompressed == false`
4. `metadata.version == 3`

### Derivation Verification

```rust
use light_token::instruction::find_mint_address;

let (expected_mint, expected_bump) = find_mint_address(&mint_signer);
assert_eq!(mint_pubkey, expected_mint);
```

### Compressed Address Derivation

```rust
pub fn derive_mint_compressed_address(
    mint_seed: &Pubkey,
    address_tree_pubkey: &Pubkey,
) -> [u8; 32] {
    derive_address(
        &find_mint_address(mint_seed).0.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}
```

---

## 6. Validation Rules

1. **Required fields**: `mint::signer`, `mint::authority`, `mint::decimals`, `mint::seeds`

2. **TokenMetadata all-or-nothing**: `name`, `symbol`, `uri` must all be specified together

3. **Optional metadata requires core**: `update_authority` and `additional_metadata` require core metadata fields

4. **Authority signer check**: If `authority_seeds` not provided, authority must be a transaction signer

---

## Source Files

| Component | Location |
|-----------|----------|
| Mint creation | `token-sdk/src/instruction/create_mint.rs` |
| Mint decompression | `token-sdk/src/instruction/decompress_mint.rs` |
| Mint structure | `token-interface/src/state/mint/compressed_mint.rs` |
| Compress action | `token-interface/src/instructions/mint_action/compress_and_close_mint.rs` |
| Derivation | `token-sdk/src/instruction/create_mint.rs:391-396` |

## Related

- [pda.md](./pda.md) - Compressed PDAs
- [token.md](./token.md) - Token accounts (vaults)
- [associated_token.md](./associated_token.md) - Associated token accounts
- [architecture.md](./architecture.md) - LightAccounts overview
