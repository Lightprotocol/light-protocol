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
  path: `program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs`
  crate: `light-ctoken-interface`
- **associated instructions**
  1. `CTokenTransfer` `3`
  2. `CTokenApprove` `4`
  3. `CTokenRevoke` `5`
  4. `CTokenMintTo` `7`
  5. `CTokenBurn` `8`
  6. `CloseTokenAccount` `9`
  7. `CTokenFreezeAccount` `10`
  8. `CTokenThawAccount` `11`
  9. `CTokenTransferChecked` `12`
  10. `CTokenMintToChecked` `14`
  11. `CTokenBurnChecked` `15`
  12. `CreateTokenAccount` `18`
  13. `CreateAssociatedCTokenAccount` `100`
  14. `Transfer2` `101` - `Decompress`, `DecompressAndClose`
  15. `CreateAssociatedTokenAccountIdempotent` `102`
  16. `MintAction` `103` - `MintToCToken`
  17. `Claim` `104`
  18. `WithdrawFundingPool` `105`
- **serialization example**
  borsh and zero copy deserialization deserialize the compressible extension, spl serialization only deserialize the base token data.
  zero copy: (always use in programs)
  ```rust
  use light_ctoken_interface::state::ctoken::CToken;
  use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

  let (token, _) = CToken::zero_copy_at(&account_data)?;
  let (mut token, _) = CToken::zero_copy_at_mut(&mut account_data)?;
  ```

  borsh: (always use in client non solana program code)
  ```rust
  use borsh::BorshDeserialize;
  use light_ctoken_interface::state::ctoken::CToken;

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
- version describes the hashing and the discriminator. (program-libs/ctoken-interface/src/state/compressed_token/token_data_version.rs)
    pub enum TokenDataVersion {
        V1 = 1u8, // discriminator [2, 0, 0, 0, 0, 0, 0, 0], // 2 le (Poseidon hashed)
        V2 = 2u8, // discriminator [0, 0, 0, 0, 0, 0, 0, 3], // 3 be (Poseidon hashed)
        ShaFlat = 3u8, // discriminator [0, 0, 0, 0, 0, 0, 0, 4], // 4 be (Sha256 hash of borsh serialized data truncated to 31 bytes so that hash is less than be bn254 field size)
    }

### Compressed Mint

## Extensions
The compressed token program supports multiple extensions defined in `program-libs/ctoken-interface/src/state/extensions/`.

### Mint Extensions

#### TokenMetadata
- Mint extension, compatible with TokenMetadata extension of Token2022.
- Only available in compressed mints.
- Path: `program-libs/ctoken-interface/src/state/extensions/token_metadata.rs`

### Token Account Extensions

#### Compressible
- Token account extension, Token2022 does not have an equivalent extension.
- Only available in ctoken solana accounts (decompressed ctokens), not in compressed token accounts.
- Stores compression info (rent sponsor, config, creation slot, etc.) for rent management.
- Path: `program-libs/ctoken-interface/src/state/extensions/compressible.rs`

#### CompressedOnly
- Marker extension indicating the account can only exist in compressed form.
- Path: `program-libs/ctoken-interface/src/state/extensions/compressed_only.rs`

#### Pausable
- Token account extension compatible with Token2022 PausableAccount extension.
- Path: `program-libs/ctoken-interface/src/state/extensions/pausable.rs`

#### PermanentDelegate
- Token account extension compatible with Token2022 PermanentDelegate extension.
- Path: `program-libs/ctoken-interface/src/state/extensions/permanent_delegate.rs`

#### TransferFee
- Token account extension compatible with Token2022 TransferFee extension.
- Path: `program-libs/ctoken-interface/src/state/extensions/transfer_fee.rs`

#### TransferHook
- Token account extension compatible with Token2022 TransferHook extension.
- Path: `program-libs/ctoken-interface/src/state/extensions/transfer_hook.rs`
