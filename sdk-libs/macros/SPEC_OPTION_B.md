# SPEC: Option B - Separate Standard Types from Instruction Data

## Overview

Introduce separate instruction data fields for standard ATAs and Mints, completely decoupled from the program-specific `CompressedAccountVariant` enum. Clean architectural separation.

## Goals

1. Complete decoupling of standard types from program enum
2. Client can pass any number of ATAs/Mints without knowing program internals
3. Clean, auditable separation of concerns
4. Fully standardized handling with no program-specific code paths

---

## Instruction Data Format (Breaking Change)

### Current Format

```rust
pub struct DecompressMultipleAccountsIdempotentData<T> {
    pub proof: ValidityProof,
    pub compressed_accounts: Vec<CompressedAccountData<T>>,
    pub system_accounts_offset: u8,
}
```

### New Format

```rust
/// New instruction data format with separate fields for standard types.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressAccountsIdempotentData<T> {
    /// Validity proof covering ALL accounts (PDAs + ATAs + Mints).
    pub proof: ValidityProof,

    /// Program-specific compressed accounts (PDAs and program-owned tokens).
    pub compressed_accounts: Vec<CompressedAccountData<T>>,

    /// Standard ATAs - fixed derivation, wallet signs.
    pub standard_atas: Vec<PackedStandardAtaData>,

    /// Standard Mints - fixed derivation, no signature required.
    pub standard_mints: Vec<PackedStandardMintData>,

    /// Offset to system accounts in remaining_accounts.
    pub system_accounts_offset: u8,
}
```

---

## Data Structures

### StandardAtaData (Client-Side)

```rust
/// Standard ATA data for client-side instruction building.
/// Location: sdk-libs/compressible-client/src/types.rs (NEW FILE)
#[derive(Clone, Debug)]
pub struct StandardAtaData {
    /// Wallet owner - MUST be transaction signer.
    pub wallet: Pubkey,
    /// Mint pubkey.
    pub mint: Pubkey,
    /// Token data from indexer. CRITICAL: token_data.owner = ATA address.
    pub token_data: TokenData,
    /// Tree info from indexer.
    pub tree_info: TreeInfo,
}
```

### PackedStandardAtaData (Serialized)

```rust
/// Packed StandardAta for on-chain deserialization.
/// Location: sdk-libs/sdk/src/compressible/standard_types.rs (NEW FILE)
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct PackedStandardAtaData {
    /// Index of wallet in remaining_accounts (must be signer).
    pub wallet_index: u8,
    /// Index of mint in remaining_accounts.
    pub mint_index: u8,
    /// Index of ATA destination in remaining_accounts.
    pub ata_destination_index: u8,
    /// Packed token data (indices into remaining_accounts).
    pub token_data: PackedTokenData,
    /// Compressed account metadata.
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
}

/// Minimal packed token data for standard ATAs.
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct PackedTokenData {
    /// Index of owner (ATA address) in remaining_accounts.
    pub owner_index: u8,
    /// Index of mint in remaining_accounts.
    pub mint_index: u8,
    /// Token amount.
    pub amount: u64,
    /// Has delegate flag.
    pub has_delegate: bool,
    /// Delegate index (0 if no delegate).
    pub delegate_index: u8,
    /// Token data version (3 = ShaFlat).
    pub version: u8,
}
```

### StandardMintData (Client-Side)

```rust
/// Standard Mint data for client-side instruction building.
#[derive(Clone, Debug)]
pub struct StandardMintData {
    /// Mint seed pubkey (derives CMint via find_mint_address).
    pub mint_seed: Pubkey,
    /// Compressed mint with context from indexer.
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Rent payment in epochs (>= 2).
    pub rent_payment: u8,
    /// Lamports for future writes.
    pub write_top_up: u32,
    /// Tree info from indexer.
    pub tree_info: TreeInfo,
}
```

### PackedStandardMintData (Serialized)

```rust
/// Packed StandardMint for on-chain deserialization.
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct PackedStandardMintData {
    /// Index of mint_seed in remaining_accounts.
    pub mint_seed_index: u8,
    /// Index of CMint destination in remaining_accounts.
    pub cmint_destination_index: u8,
    /// Compressed mint with context.
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Rent payment in epochs.
    pub rent_payment: u8,
    /// Write top-up lamports.
    pub write_top_up: u32,
    /// Compressed account metadata.
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
}
```

---

## Runtime Processing

### process_decompress_accounts_idempotent (Modified)

```rust
// sdk-libs/sdk/src/compressible/decompress_runtime.rs

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn process_decompress_accounts_idempotent<'info, Ctx>(
    ctx: &Ctx,
    remaining_accounts: &[AccountInfo<'info>],
    compressed_accounts: Vec<Ctx::CompressedData>,
    standard_atas: Vec<PackedStandardAtaData>,      // NEW
    standard_mints: Vec<PackedStandardMintData>,    // NEW
    proof: ValidityProof,
    system_accounts_offset: u8,
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
    seed_params: Option<&Ctx::SeedParams>,
) -> Result<(), ProgramError>
where
    Ctx: DecompressContext<'info>,
{
    // Determine what types we have
    let has_program_accounts = !compressed_accounts.is_empty();
    let has_standard_atas = !standard_atas.is_empty();
    let has_standard_mints = !standard_mints.is_empty();

    // Check ctoken accounts required
    if (has_standard_atas || has_standard_mints) {
        // Validate ctoken accounts are present
        ctx.ctoken_config().ok_or_else(|| {
            msg!("ctoken_config required for standard ATAs/Mints");
            ProgramError::NotEnoughAccountKeys
        })?;
        ctx.ctoken_rent_sponsor().ok_or_else(|| {
            msg!("ctoken_rent_sponsor required for standard ATAs/Mints");
            ProgramError::NotEnoughAccountKeys
        })?;
    }

    // Count types for CPI context batching
    let (has_tokens, has_pdas, has_mints) = check_account_types(&compressed_accounts);
    let has_any_tokens = has_tokens || has_standard_atas;
    let has_any_mints = has_mints || has_standard_mints;

    let type_count = has_any_tokens as u8 + has_pdas as u8 + has_any_mints as u8;
    let needs_cpi_context = type_count >= 2;

    // ... setup CPI accounts ...

    // 1. Process PDAs (if any) - from compressed_accounts
    let (compressed_pda_infos, compressed_token_accounts, program_mint_accounts) =
        ctx.collect_all_accounts(...)?;

    if !compressed_pda_infos.is_empty() {
        // ... existing PDA processing with CPI context ...
    }

    // 2. Process Mints (standard + program-specific)
    let all_mints: Vec<_> = standard_mints
        .into_iter()
        .map(|m| (m.into_compressed_mint_data(), m.meta))
        .chain(program_mint_accounts)
        .collect();

    if !all_mints.is_empty() {
        process_all_mints(
            ctx,
            &cpi_accounts,
            all_mints,
            proof,
            has_pdas,           // has_prior_context
            has_any_tokens,     // has_subsequent
        )?;
    }

    // 3. Process Tokens (standard ATAs + program-specific)
    if has_any_tokens {
        process_all_tokens(
            ctx,
            remaining_accounts,
            compressed_token_accounts,  // program-specific
            standard_atas,              // standard ATAs
            proof,
            &cpi_accounts,
            has_pdas || has_any_mints,  // has_prior_context
            program_id,
        )?;
    }

    Ok(())
}
```

### process_standard_atas (New Function)

```rust
// sdk-libs/ctoken-sdk/src/compressible/standard_ata.rs (NEW FILE)

/// Process standard ATAs in unified flow.
/// Handles ATA creation (idempotent) and builds decompress indices.
#[inline(never)]
pub fn process_standard_atas<'info>(
    standard_atas: Vec<PackedStandardAtaData>,
    packed_accounts: &[AccountInfo<'info>],
    fee_payer: &AccountInfo<'info>,
    ctoken_config: &AccountInfo<'info>,
    ctoken_rent_sponsor: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    decompress_indices: &mut Vec<DecompressFullIndices>,
) -> Result<(), ProgramError> {
    for packed_ata in standard_atas {
        // Get accounts from indices
        let wallet_info = packed_accounts
            .get(packed_ata.wallet_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let mint_info = packed_accounts
            .get(packed_ata.mint_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let ata_info = packed_accounts
            .get(packed_ata.ata_destination_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        // CRITICAL: Verify wallet is signer
        if !wallet_info.is_signer {
            msg!("StandardAta: wallet must be signer: {:?}", wallet_info.key);
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Derive and verify ATA address
        let (derived_ata, bump) = derive_ctoken_ata(wallet_info.key, mint_info.key);
        if derived_ata != *ata_info.key {
            msg!(
                "StandardAta: derivation mismatch. wallet={:?}, mint={:?}, expected={:?}, got={:?}",
                wallet_info.key, mint_info.key, derived_ata, ata_info.key
            );
            return Err(ProgramError::InvalidAccountData);
        }

        // Verify token_data.owner matches ATA address
        let owner_info = packed_accounts
            .get(packed_ata.token_data.owner_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        if *owner_info.key != derived_ata {
            msg!(
                "StandardAta: token_data.owner must equal ATA address. owner={:?}, ata={:?}",
                owner_info.key, derived_ata
            );
            return Err(ProgramError::InvalidAccountData);
        }

        // Create ATA (idempotent)
        CreateAssociatedCTokenAccountCpi {
            payer: fee_payer.clone(),
            associated_token_account: ata_info.clone(),
            owner: wallet_info.clone(),
            mint: mint_info.clone(),
            system_program: system_program.clone(),
            bump,
            compressible: CompressibleParamsCpi {
                compressible_config: ctoken_config.clone(),
                rent_sponsor: ctoken_rent_sponsor.clone(),
                system_program: system_program.clone(),
                pre_pay_num_epochs: 2,
                lamports_per_write: None,
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            },
            idempotent: true,
        }.invoke()?;

        // Build decompress indices
        let source = MultiInputTokenDataWithContext {
            owner: packed_ata.token_data.owner_index,
            amount: packed_ata.token_data.amount,
            has_delegate: packed_ata.token_data.has_delegate,
            delegate: packed_ata.token_data.delegate_index,
            mint: packed_ata.token_data.mint_index,
            version: packed_ata.token_data.version,
            merkle_context: packed_ata.meta.tree_info.into(),
            root_index: packed_ata.meta.tree_info.root_index,
        };

        // Build TLV for ATA
        let tlv = vec![ExtensionInstructionData::CompressedOnly(
            CompressedOnlyExtensionInstructionData {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
                is_frozen: false,
                compression_index: 0,
                is_ata: true,
                bump,
                owner_index: packed_ata.wallet_index,
            },
        )];

        decompress_indices.push(DecompressFullIndices {
            source,
            destination_index: packed_ata.ata_destination_index,
            tlv: Some(tlv),
            is_ata: true,
        });
    }

    Ok(())
}
```

### process_standard_mints (New Function)

```rust
// sdk-libs/ctoken-sdk/src/compressible/standard_mint.rs (NEW FILE)

/// Process standard mints via CPI to ctoken program.
#[inline(never)]
pub fn process_standard_mints<'info>(
    standard_mints: Vec<PackedStandardMintData>,
    packed_accounts: &[AccountInfo<'info>],
    fee_payer: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'_, 'info>,
    ctoken_config: &AccountInfo<'info>,
    ctoken_rent_sponsor: &AccountInfo<'info>,
    ctoken_cpi_authority: &AccountInfo<'info>,
    proof: ValidityProof,
    has_prior_context: bool,
    has_subsequent: bool,
) -> Result<(), ProgramError> {
    if standard_mints.is_empty() {
        return Ok(());
    }

    let mint_count = standard_mints.len();
    let last_idx = mint_count - 1;

    let mints_only = !has_prior_context && !has_subsequent;
    let cpi_context_account = if mints_only {
        None
    } else {
        Some(cpi_accounts.cpi_context()?.clone())
    };

    // Build system accounts once
    let system_accounts = SystemAccountInfos {
        light_system_program: cpi_accounts.get_account_info(0)?.clone(),
        cpi_authority_pda: cpi_accounts.authority()?.clone(),
        registered_program_pda: cpi_accounts.registered_program_pda()?.clone(),
        account_compression_authority: cpi_accounts.account_compression_authority()?.clone(),
        account_compression_program: cpi_accounts.account_compression_program()?.clone(),
        system_program: cpi_accounts.system_program()?.clone(),
    };

    let state_tree = cpi_accounts.get_tree_account_info(0)?;
    let input_queue = cpi_accounts.get_tree_account_info(1)?;
    let output_queue = cpi_accounts.get_tree_account_info(2)?;

    for (idx, packed_mint) in standard_mints.into_iter().enumerate() {
        // Get accounts from indices
        let mint_seed_info = packed_accounts
            .get(packed_mint.mint_seed_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let cmint_info = packed_accounts
            .get(packed_mint.cmint_destination_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        // Verify CMint derivation
        let (derived_cmint, _) = find_mint_address(mint_seed_info.key);
        if derived_cmint != *cmint_info.key {
            msg!(
                "StandardMint: derivation mismatch. mint_seed={:?}, expected={:?}, got={:?}",
                mint_seed_info.key, derived_cmint, cmint_info.key
            );
            return Err(ProgramError::InvalidAccountData);
        }

        if mints_only {
            // Direct execution
            DecompressMintCpi {
                mint_seed: mint_seed_info.clone(),
                authority: fee_payer.clone(), // No authority check for decompress
                payer: fee_payer.clone(),
                cmint: cmint_info.clone(),
                compressible_config: ctoken_config.clone(),
                rent_sponsor: ctoken_rent_sponsor.clone(),
                state_tree: state_tree.clone(),
                input_queue: input_queue.clone(),
                output_queue: output_queue.clone(),
                system_accounts: system_accounts.clone(),
                compressed_mint_with_context: packed_mint.compressed_mint_with_context,
                proof: ValidityProof(proof.0),
                rent_payment: packed_mint.rent_payment,
                write_top_up: packed_mint.write_top_up,
            }.invoke()?;
        } else {
            // CPI context batching
            let is_first = !has_prior_context && idx == 0;
            let is_last = idx == last_idx;
            let should_execute = is_last && !has_subsequent;

            let cpi_ctx = if should_execute {
                CpiContext { first_set_context: false, set_context: false, ..Default::default() }
            } else if is_first {
                CpiContext { first_set_context: true, set_context: false, ..Default::default() }
            } else {
                CpiContext { first_set_context: false, set_context: true, ..Default::default() }
            };

            DecompressCMintCpiWithContext {
                mint_seed: mint_seed_info.clone(),
                authority: fee_payer.clone(),
                payer: fee_payer.clone(),
                cmint: cmint_info.clone(),
                compressible_config: ctoken_config.clone(),
                rent_sponsor: ctoken_rent_sponsor.clone(),
                state_tree: state_tree.clone(),
                input_queue: input_queue.clone(),
                output_queue: output_queue.clone(),
                cpi_context_account: cpi_context_account.as_ref().unwrap().clone(),
                system_accounts: system_accounts.clone(),
                ctoken_cpi_authority: ctoken_cpi_authority.clone(),
                compressed_mint_with_context: packed_mint.compressed_mint_with_context,
                proof: ValidityProof(proof.0),
                rent_payment: packed_mint.rent_payment,
                write_top_up: packed_mint.write_top_up,
                cpi_context: cpi_ctx,
            }.invoke()?;
        }
    }

    Ok(())
}
```

---

## Client-Side Changes

### decompress_accounts_idempotent (Rewritten)

```rust
// sdk-libs/compressible-client/src/lib.rs

/// Build decompress_accounts_idempotent instruction with separate standard types.
#[allow(clippy::too_many_arguments)]
pub fn decompress_accounts_idempotent<T>(
    program_id: &Pubkey,
    discriminator: &[u8],
    // Program-specific accounts
    decompressed_pda_addresses: &[Pubkey],
    compressed_accounts: &[(CompressedAccount, T)],
    // Standard types (NEW)
    standard_atas: &[StandardAtaData],
    standard_mints: &[StandardMintData],
    // Accounts
    program_account_metas: &[AccountMeta],
    validity_proof_with_context: ValidityProofWithContext,
) -> Result<Instruction, Box<dyn std::error::Error>>
where
    T: Pack + Clone + std::fmt::Debug,
{
    let mut remaining_accounts = PackedAccounts::default();

    // Determine if we need CPI context
    let has_pdas = !compressed_accounts.is_empty();
    let has_tokens_or_atas = compressed_accounts.iter().any(|(ca, _)| ca.owner == LIGHT_TOKEN_PROGRAM_ID.into())
        || !standard_atas.is_empty();
    let has_mints = !standard_mints.is_empty();

    let needs_cpi_context = (has_pdas as u8 + has_tokens_or_atas as u8 + has_mints as u8) >= 2;

    // Setup system accounts
    if needs_cpi_context {
        let cpi_context = compressed_accounts.first()
            .or_else(|| standard_atas.first().map(|_| /* get from proof */))
            .or_else(|| standard_mints.first().map(|_| /* get from proof */))
            .ok_or("No accounts to process")?
            .0.tree_info.cpi_context.unwrap();

        remaining_accounts.add_system_accounts_v2(
            SystemAccountMetaConfig::new_with_cpi_context(*program_id, cpi_context)
        )?;
    } else {
        remaining_accounts.add_system_accounts_v2(
            SystemAccountMetaConfig::new(*program_id)
        )?;
    }

    // Pack output queue
    let output_queue = get_output_queue(&validity_proof_with_context.accounts[0].tree_info);
    let output_state_tree_index = remaining_accounts.insert_or_get(output_queue);

    // Pack tree infos
    let packed_tree_infos = validity_proof_with_context.pack_tree_infos(&mut remaining_accounts);

    // 1. Pack program-specific compressed accounts
    let mut typed_compressed_accounts = Vec::new();
    for (i, (compressed_account, data)) in compressed_accounts.iter().enumerate() {
        remaining_accounts.insert_or_get(compressed_account.tree_info.queue);
        let tree_info = packed_tree_infos.state_trees.as_ref().unwrap().packed_tree_infos[i];
        let packed_data = data.pack(&mut remaining_accounts);

        typed_compressed_accounts.push(CompressedAccountData {
            meta: CompressedAccountMetaNoLamportsNoAddress { tree_info, output_state_tree_index },
            data: packed_data,
        });
    }

    // 2. Pack standard ATAs
    let mut packed_standard_atas = Vec::new();
    for ata in standard_atas {
        // Derive ATA address
        let (ata_address, _) = derive_ctoken_ata(&ata.wallet, &ata.mint);

        // Insert accounts (wallet as signer)
        let wallet_index = remaining_accounts.insert_or_get_config(ata.wallet, true, false);
        let mint_index = remaining_accounts.insert_or_get(ata.mint);
        let ata_destination_index = remaining_accounts.insert_or_get(ata_address);

        // Pack token data
        // CRITICAL: token_data.owner = ATA address (from compressed account)
        let owner_index = remaining_accounts.insert_or_get(ata.token_data.owner); // ATA address
        let delegate_index = ata.token_data.delegate
            .map(|d| remaining_accounts.insert_or_get(d))
            .unwrap_or(0);

        // Get tree info for this account from validity proof
        let tree_info_idx = compressed_accounts.len() + packed_standard_atas.len();
        let tree_info = packed_tree_infos.state_trees.as_ref().unwrap().packed_tree_infos[tree_info_idx];

        packed_standard_atas.push(PackedStandardAtaData {
            wallet_index,
            mint_index,
            ata_destination_index,
            token_data: PackedTokenData {
                owner_index,
                mint_index,
                amount: ata.token_data.amount,
                has_delegate: ata.token_data.delegate.is_some(),
                delegate_index,
                version: 3, // ShaFlat
            },
            meta: CompressedAccountMetaNoLamportsNoAddress { tree_info, output_state_tree_index },
        });
    }

    // 3. Pack standard mints
    let mut packed_standard_mints = Vec::new();
    for mint in standard_mints {
        let (cmint_address, _) = find_mint_address(&mint.mint_seed);

        let mint_seed_index = remaining_accounts.insert_or_get(mint.mint_seed);
        let cmint_destination_index = remaining_accounts.insert_or_get(cmint_address);

        let tree_info_idx = compressed_accounts.len() + packed_standard_atas.len() + packed_standard_mints.len();
        let tree_info = packed_tree_infos.state_trees.as_ref().unwrap().packed_tree_infos[tree_info_idx];

        packed_standard_mints.push(PackedStandardMintData {
            mint_seed_index,
            cmint_destination_index,
            compressed_mint_with_context: mint.compressed_mint_with_context.clone(),
            rent_payment: mint.rent_payment,
            write_top_up: mint.write_top_up,
            meta: CompressedAccountMetaNoLamportsNoAddress { tree_info, output_state_tree_index },
        });
    }

    // Build accounts
    let mut accounts = program_account_metas.to_vec();
    let (system_accounts, system_accounts_offset, _) = remaining_accounts.to_account_metas();
    accounts.extend(system_accounts);

    // Add PDA destination accounts
    for pda in decompressed_pda_addresses {
        accounts.push(AccountMeta::new(*pda, false));
    }

    // Add ATA destination accounts
    for ata in standard_atas {
        let (ata_address, _) = derive_ctoken_ata(&ata.wallet, &ata.mint);
        accounts.push(AccountMeta::new(ata_address, false));
    }

    // Add CMint destination accounts
    for mint in standard_mints {
        let (cmint_address, _) = find_mint_address(&mint.mint_seed);
        accounts.push(AccountMeta::new(cmint_address, false));
    }

    // Serialize instruction data
    let instruction_data = DecompressAccountsIdempotentData {
        proof: validity_proof_with_context.proof,
        compressed_accounts: typed_compressed_accounts,
        standard_atas: packed_standard_atas,
        standard_mints: packed_standard_mints,
        system_accounts_offset: system_accounts_offset as u8,
    };

    let serialized = instruction_data.try_to_vec()?;
    let mut data = Vec::with_capacity(discriminator.len() + serialized.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(&serialized);

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
```

---

## Macro Changes

### Instruction Handler (Modified)

```rust
// sdk-libs/macros/src/compressible/instructions.rs

fn generate_decompress_instruction_entrypoint(...) -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            standard_atas: Vec<light_sdk::compressible::PackedStandardAtaData>,   // NEW
            standard_mints: Vec<light_sdk::compressible::PackedStandardMintData>, // NEW
            system_accounts_offset: u8,
            #seed_params
        ) -> Result<()> {
            __processor_functions::process_decompress_accounts_idempotent(
                &ctx.accounts,
                &ctx.remaining_accounts,
                proof,
                compressed_accounts,
                standard_atas,   // NEW
                standard_mints,  // NEW
                system_accounts_offset,
                #seed_args
            )
        }
    })
}
```

### Processor Function (Modified)

```rust
fn generate_process_decompress_accounts_idempotent(...) -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn process_decompress_accounts_idempotent<'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            standard_atas: Vec<light_sdk::compressible::PackedStandardAtaData>,
            standard_mints: Vec<light_sdk::compressible::PackedStandardMintData>,
            system_accounts_offset: u8,
            #params
        ) -> Result<()> {
            light_sdk::compressible::process_decompress_accounts_idempotent(
                accounts,
                remaining_accounts,
                compressed_accounts,
                standard_atas,
                standard_mints,
                proof,
                system_accounts_offset,
                LIGHT_CPI_SIGNER,
                &crate::ID,
                #seed_params_arg,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}
```

---

## Accounts Struct (Same as Option A)

```rust
#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub config: AccountInfo<'info>,
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,

    // Required when standard ATAs or Mints present
    #[account(mut)]
    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>,
    pub ctoken_config: Option<AccountInfo<'info>>,
    pub light_token_program: Option<AccountInfo<'info>>,
    pub ctoken_cpi_authority: Option<AccountInfo<'info>>,

    // ... program-specific optional accounts ...
}
```

---

## Validation Rules

Same as Option A:

1. **Standard ATA validation:**
   - `wallet` must be signer
   - `derive_ctoken_ata(wallet, mint) == ata_destination`
   - `token_data.owner == ata_destination` (ATA address)

2. **Standard Mint validation:**
   - `find_mint_address(mint_seed) == cmint_destination`
   - No signature required

3. **Account requirements:**
   - Standard types present => ctoken accounts required

---

## Files to Modify

1. `sdk-libs/sdk/src/compressible/mod.rs` - Export new types
2. `sdk-libs/sdk/src/compressible/standard_types.rs` (NEW) - PackedStandardAtaData, PackedStandardMintData
3. `sdk-libs/sdk/src/compressible/decompress_runtime.rs` - New signature, delegate to standard handlers
4. `sdk-libs/ctoken-sdk/src/compressible/mod.rs` - Export new functions
5. `sdk-libs/ctoken-sdk/src/compressible/standard_ata.rs` (NEW) - process_standard_atas
6. `sdk-libs/ctoken-sdk/src/compressible/standard_mint.rs` (NEW) - process_standard_mints
7. `sdk-libs/compressible-client/src/lib.rs` - New instruction builder
8. `sdk-libs/compressible-client/src/types.rs` (NEW) - StandardAtaData, StandardMintData
9. `sdk-libs/macros/src/compressible/instructions.rs` - New params in generated code

---

## Migration

1. All existing callers must update to new instruction format
2. Tests need to pass empty vecs for standard_atas/standard_mints if not using
3. No backward compatibility - clean break
