# Accounts
- Compressed tokens can be decompressed to spl tokens. Spl tokens are not explicitly listed here.
- **description**
- **discriminator**
- **state layout**
- **serialization example**
- **hashing** (only for compressed accounts)
- **derivation:** (only for pdas)
- **associated instructions** (create, close, update)


## Solana Accounts
- The compressed token program uses

### CToken
- **description**
  struct `CToken`
  ctoken solana account with spl token compatible state layout
  path: `program-libs/ctoken-types/src/state/ctoken/ctoken_struct.rs`
  crate: `light-ctoken-types`
- **associated instructions**
  1. `CreateTokenAccount` `18`
  2. `CloseTokenAccount` `9`
  3. `CTokenTransfer` `3`
  4. `Transfer2` `104` - `Decompress`, `DecompressAndClose`
  5. `MintAction` `106` - `MintToCToken`
  6. `Claim` `107`
- **serialization example**
  borsh and zero copy deserialization deserialize the compressible extension, spl serialization only deserialize the base token data.
  zero copy: (always use in programs)
  ```rust
  use light_ctoken_types::state::ctoken::CToken;
  use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

  let (token, _) = CToken::zero_copy_at(&account_data)?;
  let (mut token, _) = CToken::zero_copy_at_mut(&mut account_data)?;
  ```

  borsh: (always use in client non solana program code)
  ```rust
  use borsh::BorshDeserialize;
  use light_ctoken_types::state::ctoken::CToken;

  let token = CToken::deserialize(&mut &account_data[..])?;
  ```

  spl serialization: (preferably use other serialization)
  ```rust
  use spl_pod::bytemuck::pod_from_bytes;
  use spl_token_2022::pod::PodAccount;

  let pod_account = pod_from_bytes::<PodAccount>(&account_data[..165])?;
  ```


### Associated CToken
- **description**
  struct `CToken`
  ctoken solana account with spl token compatible state layout
- **derivation:**
  seeds: [owner, ctoken_program_id, mint]
- the same as `CToken`


### Compressible Config
- owned by the LightRegistry program
- defined in path `program-libs/compressible/src/config.rs`
- crate: `light-compressible`


## Compressed Accounts

### Compressed Token
- compressed token account.
- version describes the hashing and the discriminator. (program-libs/ctoken-types/src/state/token_data_version.rs)
    pub enum TokenDataVersion {
        V1 = 1u8, // discriminator [2, 0, 0, 0, 0, 0, 0, 0], // 2 le (Poseidon hashed)
        V2 = 2u8, // discriminator [0, 0, 0, 0, 0, 0, 0, 3], // 3 be (Poseidon hashed)
        ShaFlat = 3u8, // discriminator [0, 0, 0, 0, 0, 0, 0, 4], // 4 be (Sha256 hash of borsh serialized data truncated to 31 bytes so that hash is less than be bn254 field size)
    }

### Compressed Mint

## Extensions
The compressed token program supports 2 extensions.

### TokenMetadata
- Mint extension, compatible with TokenMetada extension of Token2022.
- Only available in compressed mints.

### Compressible
- Token account extension, Token2022 does not have an equivalent extension.
- Only available in ctoken solana accounts (decompressed ctokens), not in compressed token accounts.
-
