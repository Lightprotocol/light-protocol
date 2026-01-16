# Option A Implementation Plan

## Executive Summary

Add `StandardAta` and `PackedStandardAta` as always-present variants in `CompressedAccountVariant` to enable decompression of arbitrary ATAs without program-specific enum variants.

**Key insight**: The existing `CompressedMint` variant already handles standard mints. We only need to add support for standard ATAs.

---

## Phase 1: Data Structures (ctoken-sdk/src/pack.rs)

### 1.1 Add StandardAtaData struct

```rust
/// Standard ATA data for decompression.
/// The wallet owner signs the transaction (not the program).
/// TokenData.owner = ATA address (derived from wallet + mint).
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct StandardAtaData {
    /// Wallet owner pubkey - MUST be a signer on the transaction.
    pub wallet: Pubkey,
    /// Mint pubkey for this token account.
    pub mint: Pubkey,
    /// Token data from compressed account.
    /// CRITICAL: token_data.owner = ATA address (not wallet).
    pub token_data: TokenData,
}
```

### 1.2 Add PackedStandardAtaData struct

```rust
/// Packed StandardAtaData with indices into remaining_accounts.
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct PackedStandardAtaData {
    /// Index of wallet in remaining_accounts (must be signer).
    pub wallet_index: u8,
    /// Index of mint in remaining_accounts.
    pub mint_index: u8,
    /// Index of ATA address in remaining_accounts.
    pub ata_index: u8,
    /// Packed token data (owner/delegate/mint are indices).
    pub token_data: InputTokenDataCompressible,
}
```

### 1.3 Implement Pack/Unpack traits

```rust
impl Pack for StandardAtaData {
    type Packed = PackedStandardAtaData;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        let (ata_address, _bump) =
            crate::token::get_associated_ctoken_address_and_bump(&self.wallet, &self.mint);

        // Insert wallet as signer
        let wallet_index = remaining_accounts.insert_or_get_config(self.wallet, true, false);
        let mint_index = remaining_accounts.insert_or_get(self.mint);
        let ata_index = remaining_accounts.insert_or_get(ata_address);

        PackedStandardAtaData {
            wallet_index,
            mint_index,
            ata_index,
            token_data: self.token_data.pack(remaining_accounts),
        }
    }
}

impl Unpack for PackedStandardAtaData {
    type Unpacked = StandardAtaData;

    fn unpack(&self, remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError> {
        let wallet = *remaining_accounts
            .get(self.wallet_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?
            .key;
        let mint = *remaining_accounts
            .get(self.mint_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?
            .key;
        let token_data = self.token_data.unpack(remaining_accounts)?;

        Ok(StandardAtaData { wallet, mint, token_data })
    }
}
```

---

## Phase 2: Macro Changes (sdk-libs/macros)

### 2.1 variant_enum.rs - Add StandardAta variants

Update `compressed_account_variant` to include StandardAta variants:

```rust
let enum_def = quote! {
    #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
    pub enum CompressedAccountVariant {
        #(#account_variants)*
        PackedCTokenData(light_token_sdk::compat::PackedCTokenData<TokenAccountVariant>),
        CTokenData(light_token_sdk::compat::CTokenData<TokenAccountVariant>),
        CompressedMint(light_token_sdk::compat::CompressedMintData),
        // NEW: Standard ATA variants
        StandardAta(light_token_sdk::compat::StandardAtaData),
        PackedStandardAta(light_token_sdk::compat::PackedStandardAtaData),
    }
};
```

### 2.2 variant_enum.rs - Update trait implementations

Add match arms for StandardAta in:

- `DataHasher` impl (unreachable for packed)
- `HasCompressionInfo` impl (unreachable - token accounts don't have compression_info)
- `Size` impl (unreachable)
- `Pack` impl (StandardAta -> PackedStandardAta)
- `Unpack` impl (PackedStandardAta -> StandardAta)

### 2.3 decompress_context.rs - Update collect_all_accounts

Add handling for StandardAta in `collect_all_accounts`:

```rust
CompressedAccountVariant::PackedStandardAta(data) => {
    // Standard ATAs are processed alongside program tokens
    // They share the same token processing path with is_ata=true behavior
    standard_ata_accounts.push((data, meta));
}
CompressedAccountVariant::StandardAta(_) => {
    unreachable!("Unpacked StandardAta should not appear in packed instruction data");
}
```

### 2.4 instructions.rs - Update collect_all_accounts helper

Modify `collect_all_accounts` to return standard ATAs as a fourth tuple element:

```rust
fn collect_all_accounts<'a, 'b, 'info>(
    // ... params ...
) -> Result<(
    Vec<CompressedAccountInfo>,
    Vec<(PackedCTokenData<V>, Meta)>,
    Vec<(CompressedMintData, Meta)>,
    Vec<(PackedStandardAtaData, Meta)>,  // NEW
), ProgramError>
```

---

## Phase 3: Runtime Changes (ctoken-sdk/src/compressible)

### 3.1 decompress_runtime.rs - Process standard ATAs

Add processing for standard ATAs in `process_decompress_tokens_runtime`:

```rust
/// Process standard ATAs alongside program tokens.
/// Standard ATAs use the same Transfer2 CPI but:
/// 1. Don't require program-derived seeds
/// 2. Wallet must be a TX signer (validated here)
/// 3. ATA is derived from (wallet, light_token_program, mint)
pub fn process_standard_atas_in_token_flow<'info>(
    standard_atas: Vec<(PackedStandardAtaData, CompressedAccountMetaNoLamportsNoAddress)>,
    packed_accounts: &[AccountInfo<'info>],
    fee_payer: &AccountInfo<'info>,
    ctoken_config: &AccountInfo<'info>,
    ctoken_rent_sponsor: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    token_decompress_indices: &mut Vec<DecompressFullIndices>,
) -> Result<(), ProgramError> {
    for (packed_ata, meta) in standard_atas {
        let wallet_info = &packed_accounts[packed_ata.wallet_index as usize];
        let mint_info = &packed_accounts[packed_ata.mint_index as usize];
        let ata_info = &packed_accounts[packed_ata.ata_index as usize];

        // 1. Verify wallet is signer
        if !wallet_info.is_signer {
            msg!("StandardAta wallet must be signer: {:?}", wallet_info.key);
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 2. Verify ATA derivation
        let (derived_ata, bump) = derive_ctoken_ata(wallet_info.key, mint_info.key);
        if derived_ata != *ata_info.key {
            msg!("ATA derivation mismatch");
            return Err(ProgramError::InvalidAccountData);
        }

        // 3. Create ATA (idempotent)
        CreateAssociatedCTokenAccountCpi {
            payer: fee_payer.clone(),
            associated_token_account: ata_info.clone(),
            owner: wallet_info.clone(),
            mint: mint_info.clone(),
            system_program: cpi_accounts.system_program()?.clone(),
            bump,
            compressible: CompressibleParamsCpi {
                compressible_config: ctoken_config.clone(),
                rent_sponsor: ctoken_rent_sponsor.clone(),
                system_program: cpi_accounts.system_program()?.clone(),
                pre_pay_num_epochs: 2,
                lamports_per_write: None,
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            },
            idempotent: true,
        }.invoke()?;

        // 4. Build decompress indices with TLV for ATA
        let wallet_account_index = packed_accounts
            .iter()
            .position(|a| *a.key == *wallet_info.key)
            .ok_or(ProgramError::NotEnoughAccountKeys)? as u8;

        let tlv = vec![ExtensionInstructionData::CompressedOnly(
            CompressedOnlyExtensionInstructionData {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
                is_frozen: false,
                compression_index: 0,
                is_ata: true,
                bump,
                owner_index: wallet_account_index,
            },
        )];

        let source = MultiInputTokenDataWithContext {
            owner: packed_ata.token_data.owner, // ATA address index
            amount: packed_ata.token_data.amount,
            has_delegate: packed_ata.token_data.has_delegate,
            delegate: packed_ata.token_data.delegate,
            mint: packed_ata.token_data.mint,
            version: packed_ata.token_data.version,
            merkle_context: meta.tree_info.into(),
            root_index: meta.tree_info.root_index,
        };

        token_decompress_indices.push(DecompressFullIndices {
            source,
            destination_index: packed_ata.ata_index,
            tlv: Some(tlv),
            is_ata: true,
        });
    }

    Ok(())
}
```

### 3.2 Update function signature

Modify `process_decompress_tokens_runtime` to accept standard ATAs:

```rust
pub fn process_decompress_tokens_runtime<'info, 'a, 'b, V, A>(
    // ... existing params ...
    ctoken_accounts: Vec<(PackedCTokenData<V>, Meta)>,
    standard_ata_accounts: Vec<(PackedStandardAtaData, Meta)>,  // NEW
    // ...
) -> Result<(), ProgramError>
```

---

## Phase 4: SDK Runtime Changes (sdk/src/compressible)

### 4.1 decompress_runtime.rs - Update DecompressContext trait

Add method to handle standard ATAs in the trait:

```rust
/// Returns standard ATA accounts for separate processing.
fn standard_ata_accounts(&self) -> Vec<(PackedStandardAtaData, CompressedMeta)> {
    Vec::new() // Default: no standard ATAs
}
```

### 4.2 Update process_decompress_accounts_idempotent

Pass standard ATAs to process_tokens:

```rust
// After collect_all_accounts returns (pdas, tokens, mints, standard_atas)
ctx.process_tokens(
    // ... existing params ...
    compressed_token_accounts,
    standard_ata_accounts,  // NEW
    // ...
)?;
```

---

## Phase 5: Client Changes (compressible-client/src/lib.rs)

### 5.1 Add StandardAtaInput struct

```rust
/// Input for standard ATA decompression (no program-specific variant needed)
pub struct StandardAtaInput {
    /// Wallet owner - MUST sign the transaction
    pub wallet: Pubkey,
    /// Mint for the token
    pub mint: Pubkey,
    /// Token data from indexer (owner = ATA address)
    pub token_data: TokenData,
    /// Tree info for validity proof
    pub tree_info: TreeInfo,
}
```

### 5.2 Update decompress_accounts_idempotent signature

```rust
pub fn decompress_accounts_idempotent<T>(
    program_id: &Pubkey,
    discriminator: &[u8],
    decompressed_account_addresses: &[Pubkey],
    compressed_accounts: &[(CompressedAccount, T)],
    standard_atas: &[StandardAtaInput],  // NEW
    program_account_metas: &[AccountMeta],
    validity_proof_with_context: ValidityProofWithContext,
) -> Result<Instruction, Box<dyn std::error::Error>>
```

### 5.3 Pack standard ATAs in instruction builder

```rust
// Pack standard ATAs
for ata_input in standard_atas {
    let (ata_address, _) = derive_ctoken_ata(&ata_input.wallet, &ata_input.mint);

    // Insert wallet as signer
    remaining_accounts.insert_or_get_config(ata_input.wallet, true, false);
    remaining_accounts.insert_or_get(ata_input.mint);
    remaining_accounts.insert_or_get(ata_address);

    let standard_ata = StandardAtaData {
        wallet: ata_input.wallet,
        mint: ata_input.mint,
        token_data: ata_input.token_data.clone(),
    };
    let packed = standard_ata.pack(&mut remaining_accounts);

    typed_compressed_accounts.push(CompressedAccountData {
        meta: /* from validity_proof_with_context */,
        data: CompressedAccountVariant::PackedStandardAta(packed),
    });
}
```

---

## Phase 6: Testing

### 6.1 Update existing test

Modify `test_create_pdas_and_mint_auto` to:

1. Use `StandardAtaInput` for user ATA decompression
2. Verify wallet signer requirement
3. Test mixed batch (PDAs + program tokens + standard ATAs)

### 6.2 New test cases

1. **Standard ATA only**: Decompress single standard ATA
2. **Mixed batch**: PDAs + CompressedMint + StandardAta + program token
3. **Signer validation**: Ensure non-signer wallet fails
4. **ATA derivation validation**: Ensure wrong wallet/mint combo fails

---

## Implementation Order

1. **ctoken-sdk/src/pack.rs** - Add data structures (30 min)
2. **macros/src/compressible/variant_enum.rs** - Add variants + trait impls (45 min)
3. **macros/src/compressible/decompress_context.rs** - Handle new variants (30 min)
4. **ctoken-sdk/src/compressible/decompress_runtime.rs** - Process standard ATAs (60 min)
5. **sdk/src/compressible/decompress_runtime.rs** - Update trait + processor (30 min)
6. **compressible-client/src/lib.rs** - Client helpers (45 min)
7. **Test updates** - Verify functionality (60 min)

**Total estimated time: 5-6 hours**

---

## Open Questions (Resolved)

1. **Q: Should standard ATAs use a separate variant or share `PackedCTokenData`?**
   **A: Separate variant (`PackedStandardAta`) for cleaner handling and explicit wallet index.**

2. **Q: How does the client know which accounts are standard ATAs vs program tokens?**
   **A: Client explicitly creates `StandardAtaInput` vs wrapping in program's `TokenAccountVariant`.**

3. **Q: Do standard ATAs require `cmint_authority` account?**
   **A: No. Standard ATAs only need wallet signer. `cmint_authority` is only for mint decompression.**

4. **Q: Can standard ATAs be mixed with program tokens in single instruction?**
   **A: Yes. All tokens (standard + program) are batched into single Transfer2 CPI.**
