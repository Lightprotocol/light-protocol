# SPEC: Option A - Standard Variants in Macro-Generated Enum

## Overview

Add `StandardAta` and `StandardMint` as always-present variants in the macro-generated `TokenAccountVariant` enum. Programs automatically get these standard variants without declaration.

## Goals

1. Enable decompression of arbitrary ATAs and Mints without per-program customization
2. Use fixed, known data structures and derivation logic
3. Maintain single unified enum for all account types
4. Zero breaking changes to existing programs that don't use standard types

---

## Data Structures

### StandardAtaData (New)

```rust
/// Standard ATA data for decompression.
/// Compressed TokenData.owner = ATA address (NOT wallet).
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct StandardAtaData {
    /// Wallet owner pubkey - MUST be a signer on the transaction.
    /// The ATA is derived from (wallet, light_token_program_id, mint).
    pub wallet: Pubkey,
    /// Mint pubkey for this token account.
    pub mint: Pubkey,
    /// Token data from compressed account.
    /// CRITICAL: token_data.owner = ATA address (not wallet).
    pub token_data: TokenData,
}
```

### PackedStandardAtaData (New)

```rust
/// Packed StandardAtaData with indices into remaining_accounts.
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct PackedStandardAtaData {
    /// Index of wallet in remaining_accounts (must be signer).
    pub wallet_index: u8,
    /// Index of mint in remaining_accounts.
    pub mint_index: u8,
    /// Index of ATA address in remaining_accounts (same as token_data.owner).
    pub ata_index: u8,
    /// Packed token data (owner/delegate/mint are indices).
    pub token_data: InputTokenDataCompressible,
}
```

### StandardMintData (Existing CompressedMintData - Reuse)

```rust
/// Already exists in ctoken-sdk/src/pack.rs as CompressedMintData.
/// Rename/alias to StandardMintData for clarity.
pub type StandardMintData = CompressedMintData;

#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedMintData {
    /// Mint seed pubkey (used to derive CMint PDA via find_mint_address).
    pub mint_seed_pubkey: Pubkey,
    /// Compressed mint with context (from indexer).
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Rent payment in epochs (must be >= 2).
    pub rent_payment: u8,
    /// Lamports for future write operations.
    pub write_top_up: u32,
}
```

---

## Enum Changes

### TokenAccountVariant (Modified)

The macro will always generate these standard variants:

```rust
// sdk-libs/macros/src/compressible/seed_providers.rs
pub fn generate_ctoken_account_variant_enum(specs: &[TokenSeedSpec]) -> Result<TokenStream> {
    // ... existing program-specific variants ...

    quote! {
        #[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
        pub enum TokenAccountVariant {
            // Program-specific variants (from macro args)
            #(#program_variants,)*

            // Standard variants (always present)
            /// Standard ATA - uses fixed derivation (wallet, light_token_program, mint).
            StandardAta,
            /// Standard Mint - uses fixed derivation find_mint_address(mint_seed).
            StandardMint,
        }
    }
}
```

### CompressedAccountVariant (Modified)

```rust
// sdk-libs/macros/src/compressible/variant_enum.rs
pub enum CompressedAccountVariant {
    // Program PDA variants
    #(#account_variants)*

    // Token variants
    PackedCTokenData(PackedCTokenData<TokenAccountVariant>),
    CTokenData(CTokenData<TokenAccountVariant>),

    // Mint variant (existing)
    CompressedMint(CompressedMintData),

    // NEW: Standard ATA variant (separate from CTokenData for cleaner handling)
    StandardAta(StandardAtaData),
    PackedStandardAta(PackedStandardAtaData),
}
```

---

## Trait Implementations

### TokenSeedProvider for StandardAta

```rust
impl TokenSeedProvider for TokenAccountVariant {
    // ... existing match arms ...

    fn get_seeds<'a, 'info>(
        &self,
        accounts: &'a Self::Accounts<'info>,
        remaining_accounts: &'a [AccountInfo<'info>],
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError> {
        match self {
            // ... existing program-specific arms ...

            TokenAccountVariant::StandardAta => {
                // StandardAta doesn't use program seeds - derivation is fixed.
                // Return empty seeds; the runtime handles ATA creation separately.
                Err(ProgramError::InvalidArgument) // Should not be called
            }
            TokenAccountVariant::StandardMint => {
                // StandardMint doesn't use program seeds - derivation is fixed.
                Err(ProgramError::InvalidArgument) // Should not be called
            }
        }
    }

    fn get_authority_seeds<'a, 'info>(...) -> Result<...> {
        match self {
            TokenAccountVariant::StandardAta => {
                Err(ProgramError::InvalidArgument) // ATAs don't need authority seeds
            }
            TokenAccountVariant::StandardMint => {
                Err(ProgramError::InvalidArgument) // Mints don't need authority seeds for decompress
            }
            // ... existing arms ...
        }
    }

    fn is_ata(&self) -> bool {
        matches!(self, TokenAccountVariant::StandardAta)
    }
}
```

### HasTokenVariant Updates

```rust
impl HasTokenVariant for CompressedAccountData {
    fn is_packed_ctoken(&self) -> bool {
        matches!(
            self.data,
            CompressedAccountVariant::PackedCTokenData(_)
            | CompressedAccountVariant::PackedStandardAta(_)
        )
    }

    fn is_compressed_mint(&self) -> bool {
        matches!(self.data, CompressedAccountVariant::CompressedMint(_))
    }

    fn is_standard_ata(&self) -> bool {
        matches!(
            self.data,
            CompressedAccountVariant::StandardAta(_)
            | CompressedAccountVariant::PackedStandardAta(_)
        )
    }
}
```

### Pack/Unpack for StandardAtaData

```rust
impl Pack for StandardAtaData {
    type Packed = PackedStandardAtaData;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        // Derive ATA address from wallet + mint
        let (ata_address, _bump) = derive_ctoken_ata(&self.wallet, &self.mint);

        // Insert all required accounts
        let wallet_index = remaining_accounts.insert_or_get_config(self.wallet, true, false); // signer
        let mint_index = remaining_accounts.insert_or_get(self.mint);
        let ata_index = remaining_accounts.insert_or_get(ata_address);

        // Pack token data
        let token_data = self.token_data.pack(remaining_accounts);

        PackedStandardAtaData {
            wallet_index,
            mint_index,
            ata_index,
            token_data,
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

## Runtime Processing

### process_decompress_tokens_runtime (Modified)

```rust
// sdk-libs/ctoken-sdk/src/compressible/decompress_runtime.rs

pub fn process_decompress_tokens_runtime<'info, 'a, 'b, V, A>(
    // ... existing params ...
    // ADD: standard ATAs
    standard_atas: Vec<(PackedStandardAtaData, CompressedAccountMetaNoLamportsNoAddress)>,
) -> Result<(), ProgramError> {
    // ... existing token processing ...

    // Process standard ATAs
    for (packed_ata, meta) in standard_atas.into_iter() {
        let wallet_info = &packed_accounts[packed_ata.wallet_index as usize];
        let mint_info = &packed_accounts[packed_ata.mint_index as usize];
        let ata_info = &packed_accounts[packed_ata.ata_index as usize];

        // Verify wallet is signer
        if !wallet_info.is_signer {
            msg!("StandardAta wallet must be signer: {:?}", wallet_info.key);
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Verify ATA derivation
        let (derived_ata, bump) = derive_ctoken_ata(wallet_info.key, mint_info.key);
        if derived_ata != *ata_info.key {
            msg!("ATA derivation mismatch: derived={:?}, provided={:?}", derived_ata, ata_info.key);
            return Err(ProgramError::InvalidAccountData);
        }

        // Create ATA if needed (idempotent)
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
                compression_only: true, // ATAs require compression_only
            },
            idempotent: true,
        }.invoke()?;

        // Build decompress indices
        let owner_index = packed_ata.token_data.owner; // ATA address index
        let wallet_account_index = packed_ata.wallet_index;

        let source = MultiInputTokenDataWithContext {
            owner: owner_index,
            amount: packed_ata.token_data.amount,
            has_delegate: packed_ata.token_data.has_delegate,
            delegate: packed_ata.token_data.delegate,
            mint: packed_ata.token_data.mint,
            version: packed_ata.token_data.version,
            merkle_context: meta.tree_info.into(),
            root_index: meta.tree_info.root_index,
        };

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

        let decompress_index = DecompressFullIndices {
            source,
            destination_index: packed_ata.ata_index,
            tlv: Some(tlv),
            is_ata: true,
        };
        token_decompress_indices.push(decompress_index);
    }

    // ... rest of existing logic (single Transfer2 CPI) ...
}
```

---

## collect_all_accounts (Modified)

```rust
// Macro-generated in __macro_helpers module

fn collect_all_accounts<'a, 'b, 'info>(
    // ... existing params ...
) -> Result<(
    Vec<CompressedAccountInfo>,           // PDAs
    Vec<(PackedCTokenData<V>, Meta)>,     // Program tokens
    Vec<(CompressedMintData, Meta)>,      // Mints
    Vec<(PackedStandardAtaData, Meta)>,   // NEW: Standard ATAs
), ProgramError> {
    // ... existing setup ...

    let mut standard_ata_accounts = Vec::new();

    for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
        let meta = compressed_data.meta;
        match compressed_data.data {
            // ... existing PDA arms ...

            CompressedAccountVariant::PackedCTokenData(data) => {
                compressed_token_accounts.push((data, meta));
            }
            CompressedAccountVariant::CompressedMint(data) => {
                compressed_mint_accounts.push((data, meta));
            }

            // NEW: Standard ATA handling
            CompressedAccountVariant::PackedStandardAta(data) => {
                standard_ata_accounts.push((data, meta));
            }
            CompressedAccountVariant::StandardAta(_) => {
                unreachable!("Unpacked StandardAta should not appear");
            }

            // ... other arms ...
        }
    }

    Ok((compressed_pda_infos, compressed_token_accounts, compressed_mint_accounts, standard_ata_accounts))
}
```

---

## Client-Side Changes

### compressible_instruction::decompress_accounts_idempotent (Modified)

```rust
// sdk-libs/compressible-client/src/lib.rs

pub fn decompress_accounts_idempotent<T>(
    program_id: &Pubkey,
    discriminator: &[u8],
    decompressed_account_addresses: &[Pubkey],
    compressed_accounts: &[(CompressedAccount, T)],
    // NEW: Standard ATAs
    standard_atas: &[StandardAtaInput],
    // NEW: Standard Mints
    standard_mints: &[StandardMintInput],
    program_account_metas: &[AccountMeta],
    validity_proof_with_context: ValidityProofWithContext,
) -> Result<Instruction, Box<dyn std::error::Error>>
where
    T: Pack + Clone + std::fmt::Debug,
{
    // ... existing setup ...

    // Pack standard ATAs
    for ata_input in standard_atas {
        let (ata_address, _) = derive_ctoken_ata(&ata_input.wallet, &ata_input.mint);
        remaining_accounts.insert_or_get_config(ata_input.wallet, true, false); // signer
        remaining_accounts.insert_or_get(ata_input.mint);
        remaining_accounts.insert_or_get(ata_address);

        // Build StandardAtaData and pack
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

    // Pack standard mints
    for mint_input in standard_mints {
        let (cmint_address, _) = find_mint_address(&mint_input.mint_seed);
        remaining_accounts.insert_or_get(mint_input.mint_seed);
        remaining_accounts.insert_or_get(cmint_address);

        typed_compressed_accounts.push(CompressedAccountData {
            meta: /* from validity_proof_with_context */,
            data: CompressedAccountVariant::CompressedMint(CompressedMintData {
                mint_seed_pubkey: mint_input.mint_seed,
                compressed_mint_with_context: mint_input.compressed_mint_with_context.clone(),
                rent_payment: mint_input.rent_payment,
                write_top_up: mint_input.write_top_up,
            }),
        });
    }

    // ... rest of instruction building ...
}

/// Input for standard ATA decompression
pub struct StandardAtaInput {
    pub wallet: Pubkey,      // Must be tx signer
    pub mint: Pubkey,
    pub token_data: TokenData,  // owner = ATA address
    pub tree_info: TreeInfo,
}

/// Input for standard mint decompression
pub struct StandardMintInput {
    pub mint_seed: Pubkey,
    pub compressed_mint_with_context: CompressedMintWithContext,
    pub rent_payment: u8,
    pub write_top_up: u32,
    pub tree_info: TreeInfo,
}
```

---

## DecompressAccountsIdempotent Accounts (Modified)

```rust
// Macro-generated accounts struct

#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Program's compressible config
    pub config: AccountInfo<'info>,

    /// Program's rent sponsor
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,

    // CToken accounts - REQUIRED if any tokens/ATAs/mints present
    /// CToken rent sponsor (ctoken program's rent sponsor PDA)
    #[account(mut)]
    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>,

    /// CToken compressible config
    pub ctoken_config: Option<AccountInfo<'info>>,

    /// CToken program
    pub light_token_program: Option<AccountInfo<'info>>,

    /// CToken CPI authority
    pub ctoken_cpi_authority: Option<AccountInfo<'info>>,

    // ... other optional accounts for program-specific seeds ...
}
```

The runtime will validate that ctoken accounts are Some when standard ATAs/mints are present.

---

## Validation Rules

1. **Standard ATA validation:**
   - `wallet` must be a signer in the transaction
   - `derive_ctoken_ata(wallet, mint)` must equal the ATA destination account
   - `token_data.owner` (ATA address) must match derived ATA

2. **Standard Mint validation:**
   - `find_mint_address(mint_seed)` must equal the CMint destination account
   - No signature required (mint authority doesn't need to sign for decompress)

3. **Account requirements:**
   - If any standard ATAs or mints present: ctoken_config, ctoken_rent_sponsor, light_token_program, ctoken_cpi_authority must be Some
   - If only PDAs: ctoken accounts can be None

---

## Files to Modify

1. `sdk-libs/macros/src/compressible/seed_providers.rs` - Add StandardAta/StandardMint variants
2. `sdk-libs/macros/src/compressible/variant_enum.rs` - Add StandardAta variant handling
3. `sdk-libs/macros/src/compressible/instructions.rs` - Update collect_all_accounts
4. `sdk-libs/macros/src/compressible/decompress_context.rs` - Update trait impl
5. `sdk-libs/ctoken-sdk/src/pack.rs` - Add StandardAtaData, PackedStandardAtaData
6. `sdk-libs/ctoken-sdk/src/compressible/decompress_runtime.rs` - Handle standard ATAs
7. `sdk-libs/sdk/src/compressible/decompress_runtime.rs` - Update process_decompress_accounts_idempotent
8. `sdk-libs/compressible-client/src/lib.rs` - Add standard ATA/mint params

---

## Migration Path

1. Existing programs: No changes required, StandardAta/StandardMint variants available automatically
2. New programs: Can use standard types without declaring in macro
3. Tests: Update to use new client helper signature
