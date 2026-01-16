# Option A: Standard ATA/Mint Variants - State Flow Diagram

## Overview

Option A adds `StandardAta` and `PackedStandardAta` variants to the macro-generated `CompressedAccountVariant` enum, enabling unified decompression of arbitrary ATAs and mints alongside program-specific PDAs.

---

## High-Level Decompression Flow

```
                                    decompress_accounts_idempotent
                                              |
                                              v
                            +------------------------------------+
                            |    parse CompressedAccountData[]   |
                            +------------------------------------+
                                              |
                              +---------------+---------------+
                              |               |               |
                              v               v               v
                         +--------+     +---------+     +--------+
                         |  PDAs  |     | Tokens  |     | Mints  |
                         +--------+     +---------+     +--------+
                              |               |               |
                              v               v               v
                         +---------+   +---------------+  +--------+
                         | CPI to  |   | process_tokens|  | CPI to |
                         | Light   |   |   _runtime    |  | ctoken |
                         | System  |   +---------------+  | mint   |
                         +---------+         |            +--------+
                                             |
                       +---------------------+---------------------+
                       |                     |                     |
                       v                     v                     v
               +-------------+       +-------------+       +-------------+
               | Program PDA |       | StandardAta |       | CompressedMint |
               | Token (Vault)|      | (UserAta)   |       | (CMint)     |
               +-------------+       +-------------+       +-------------+
                       |                     |                     |
                       v                     v                     v
               derive seeds         derive_ctoken_ata    find_mint_address
               from variant         (wallet, mint)       (mint_seed)
                       |                     |                     |
                       v                     v                     v
               CreateCToken         CreateAssociated     DecompressMint
               AccountCpi           CTokenAccountCpi     Cpi
               (invoke_signed)      (invoke - wallet     (invoke)
                                     signs tx)
```

---

## Token Account Type Decision Tree

```
                        PackedCTokenData<V>
                                |
                                v
                    +---------------------------+
                    |  V.is_ata() returns what? |
                    +---------------------------+
                              |
           +------------------+------------------+
           |                                     |
       is_ata = true                        is_ata = false
           |                                     |
           v                                     v
    +----------------+                   +------------------+
    | Standard ATA   |                   | Program-owned    |
    | Derivation     |                   | Token Account    |
    +----------------+                   +------------------+
           |                                     |
           v                                     v
    derive_ctoken_ata(wallet, mint)      get_seeds() from variant
    wallet must be TX signer             program signs via CPI
           |                                     |
           v                                     v
    CreateAssociatedCTokenAccountCpi     CreateTokenAccountCpi
    .invoke() - no program signer        .invoke_signed(&[seeds])
```

---

## StandardAta Detailed Flow

```
                        Client Side
                        -----------
   StandardAtaInput {
       wallet: Pubkey,           // must sign TX
       mint: Pubkey,
       token_data: TokenData,    // owner = ATA address
       tree_info: TreeInfo,
   }
            |
            v
   pack_standard_ata()
            |
            +---> remaining_accounts.insert_or_get_config(wallet, signer=true)
            +---> remaining_accounts.insert_or_get(mint)
            +---> derive_ctoken_ata(wallet, mint) -> ata_address
            +---> remaining_accounts.insert_or_get(ata_address)
            +---> pack token_data indices
            |
            v
   PackedStandardAtaData {
       wallet_index: u8,
       mint_index: u8,
       ata_index: u8,
       token_data: InputTokenDataCompressible,
   }
            |
            v
   CompressedAccountData {
       meta: CompressedAccountMetaNoLamportsNoAddress,
       data: CompressedAccountVariant::PackedStandardAta(packed),
   }


                        Runtime Side
                        ------------
   collect_all_accounts()
            |
            v
   match CompressedAccountVariant::PackedStandardAta(packed)
            |
            v
   Extract to standard_ata_accounts: Vec<(PackedStandardAtaData, Meta)>
            |
            v
   process_decompress_tokens_runtime()
            |
            v
   for (packed_ata, meta) in standard_atas {
       |
       v
       // 1. Validate wallet is signer
       packed_accounts[wallet_index].is_signer? -> MissingRequiredSignature
       |
       v
       // 2. Verify ATA derivation
       derive_ctoken_ata(wallet, mint) == packed_accounts[ata_index]? -> InvalidAccountData
       |
       v
       // 3. Create ATA (idempotent)
       CreateAssociatedCTokenAccountCpi {
           owner: wallet,
           mint: mint,
           bump: derived_bump,
           compressible: {
               compression_only: true,  // ATAs must be compression_only
               ...
           },
           idempotent: true,
       }.invoke()  // No signer - wallet signs TX
       |
       v
       // 4. Build decompress indices with TLV
       DecompressFullIndices {
           source: MultiInputTokenDataWithContext {
               owner: ata_index,  // ATA address in merkle tree
               ...
           },
           destination_index: ata_index,
           tlv: Some([CompressedOnly { is_ata: true, owner_index: wallet_index, bump }]),
           is_ata: true,
       }
   }
            |
            v
   // Single Transfer2 CPI for all tokens (program PDAs + standard ATAs)
   decompress_full_ctoken_accounts_with_indices(...)
```

---

## CompressedAccountVariant Enum (After Option A)

```rust
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedAccountVariant {
    // === Program-specific PDA variants (macro-generated) ===
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),
    GameSession(GameSession),
    PackedGameSession(PackedGameSession),

    // === Token variants (macro-generated) ===
    PackedCTokenData(PackedCTokenData<TokenAccountVariant>),
    CTokenData(CTokenData<TokenAccountVariant>),

    // === Mint variant (existing) ===
    CompressedMint(CompressedMintData),

    // === NEW: Standard ATA variants (always present) ===
    StandardAta(StandardAtaData),
    PackedStandardAta(PackedStandardAtaData),
}
```

---

## CPI Context Batching (Multi-Type Decompression)

```
   Execution Order: PDAs -> Mints -> Tokens (tokens always last)

   Case 1: Single Type (no CPI context)
   ------------------------------------
   PDAs only:   LightSystemProgramCpi.invoke()
   Mints only:  DecompressMintCpi.invoke()
   Tokens only: Transfer2 CPI invoke()

   Case 2: Multi-Type (with CPI context batching)
   ----------------------------------------------

   PDAs      Mints     Tokens     CPI Context Action
   ----      -----     ------     ------------------
   Yes       No        No         execute directly (no context)
   No        Yes       No         execute directly (no context)
   No        No        Yes        execute directly (no context)

   Yes       Yes       No         PDAs: first_set_context
                                  Mints: execute (consume)

   Yes       No        Yes        PDAs: first_set_context
                                  Tokens: execute (consume)

   No        Yes       Yes        Mints: first_set_context
                                  Tokens: execute (consume)

   Yes       Yes       Yes        PDAs: first_set_context
                                  Mints: set_context
                                  Tokens: execute (consume)
```

---

## Validation Rules

### StandardAta Validation

1. **Wallet signer check**: `remaining_accounts[wallet_index].is_signer == true`
2. **ATA derivation check**: `derive_ctoken_ata(wallet, mint) == remaining_accounts[ata_index].key`
3. **Owner consistency**: `token_data.owner` (index) points to ATA address (not wallet)

### CompressedMint Validation

1. **CMint derivation**: `find_mint_address(mint_seed) == cmint_pda`
2. **Authority**: fee_payer must be mint authority OR explicit cmint_authority provided

### Program Token (Vault) Validation

1. **PDA derivation**: `get_seeds(variant, accounts)` returns matching PDA
2. **Authority derivation**: `get_authority_seeds(variant, accounts)` returns owner PDA
3. **Program signing**: invoke_signed with seed-derived bumps

---

## Account Lookup Indices

```
remaining_accounts layout (after system accounts):
+--------------------------------------------------------------------+
| idx | account                   | usage                             |
+--------------------------------------------------------------------+
|  0  | output_queue              | state tree output                 |
|  1  | state_tree                | merkle tree                        |
|  2  | input_queue               | nullifier queue                    |
| ... | tree accounts             | from validity proof                |
| n   | wallet (signer)           | StandardAta wallet owner           |
| n+1 | mint                      | token mint                          |
| n+2 | ATA address               | derived from wallet+mint           |
| n+3 | vault_authority           | program-owned token authority      |
| n+4 | cmint_pda                 | CMint address                       |
| ... | other accounts            |                                    |
| end | decompressed PDAs/tokens  | accounts being decompressed        |
+--------------------------------------------------------------------+

PackedStandardAtaData {
    wallet_index: n,     // points to signer wallet
    mint_index: n+1,     // points to mint
    ata_index: n+2,      // points to derived ATA
    token_data: {
        owner: n+2,      // ATA address (matches compressed token owner)
        mint: n+1,
        amount: ...,
        ...
    }
}
```

---

## Files Changed Summary

```
sdk-libs/
  macros/src/compressible/
    variant_enum.rs        # Add StandardAta, PackedStandardAta variants
    decompress_context.rs  # Handle StandardAta in collect_all_accounts
    instructions.rs        # Update collect_all_accounts helper

  ctoken-sdk/src/
    pack.rs                # Add StandardAtaData, PackedStandardAtaData + Pack/Unpack
    compressible/
      decompress_runtime.rs # Process standard ATAs in token flow
      mod.rs               # Re-export new types

  compressible-client/src/
    lib.rs                 # Add StandardAtaInput, update decompress helper

  sdk/src/compressible/
    decompress_runtime.rs  # Pass standard ATAs to process_tokens
```
