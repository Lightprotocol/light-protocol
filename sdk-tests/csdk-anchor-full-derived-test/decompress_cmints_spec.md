# Decompress Multiple CMints Instruction Spec

## Objective

Add an instruction that decompresses multiple compressed mints (cmints) in one instruction by invoking the ctoken program via CPI.

## Critical Constraints (from ctoken program analysis)

### Constraint 1: DecompressMint Cannot Write to CPI Context

From `programs/compressed-token/program/src/compressed_token/mint_action/accounts.rs:465-468`:

```rust
if has_decompress_mint_action {
    msg!("Decompress mint not allowed when writing to cpi context");
    return Err(ErrorCode::CpiContextSetNotUsable.into());
}
```

**Impact**: DecompressMint cannot use `first_set_context=true` or `set_context=true`. This means:

- Multiple mints CANNOT batch their writes to CPI context
- Only the final operation in a CPI context chain can be a DecompressMint (with both flags=false)

### Constraint 2: MintAction Processes One Mint Per Instruction

From `program-libs/ctoken-interface/src/instructions/mint_action/instruction_data.rs`:

```rust
pub struct MintActionCompressedInstructionData {
    pub leaf_index: u32,       // SINGULAR
    pub root_index: u16,       // SINGULAR
    pub mint: Option<CompressedMintInstructionData>,  // SINGULAR
}
```

**Impact**: Each CPI call to ctoken program handles exactly one mint.

### Constraint 3: ZKP Proof Verification Requires Exact Input Match

The light-system-program's proof verification computes a hash chain from inputs and verifies against that.
If proof P was generated for [mint1, mint2] and we invoke with just [mint1], verification fails because:

- hash_chain([mint1]) != hash_chain([mint1, mint2])

**Impact**: For zkp proofs, cannot loop and reuse same proof for multiple mints!

## Available Patterns

### Pattern A: Mints-Only Loop with prove_by_index (Recommended for this use case)

When decompressing ONLY mints with `prove_by_index=true`:

- All mints must be in nullifier queue (prove_by_index=true)
- Loop through each mint
- Call `DecompressCMintCpi` for each (no CPI context)
- No zkp proof verification needed - just queue position check
- **This is the only way to decompress multiple mints in one instruction**

### Pattern B: PDAs First, Then One Mint (with CPI context)

When mixing PDAs + one mint:

- PDAs write to CPI context first
- Final mint executes (consumes context, both flags=false)
- Single proof verification
- **Limitation**: Only ONE mint at the end

### Pattern C: Multiple Separate Instructions (for zkp proofs)

- Each mint in a separate instruction with its own proof
- Required when mints need zkp verification

## Design Decision

**We'll implement Pattern A** - the mints-only loop with prove_by_index:

- Input: Vec of compressed mint data with merkle contexts
- Process: Loop, invoke `DecompressCMintCpi` for each
- No CPI context used (not possible for DecompressMint writes)
- **Requires**: All mints use `prove_by_index=true` (in nullifier queue)

## Instruction Design

### Instruction Name

`decompress_cmints`

### Parameters

```rust
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressCMintsParams {
    /// Validity proof covering all input mints
    pub proof: ValidityProof,
    /// Vec of mint data with merkle context (from indexer)
    pub cmints: Vec<CompressedMintDecompressData>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CompressedMintDecompressData {
    /// The pubkey used to seed the CMint PDA derivation
    pub mint_seed_pubkey: Pubkey,
    /// Complete compressed mint with merkle context
    pub compressed_mint_with_context: CompressedMintWithContext,
    /// Rent payment in epochs (0 or >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
}
```

### Accounts Structure

```rust
#[derive(Accounts)]
pub struct DecompressCMints<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority for the mints (must sign)
    pub authority: Signer<'info>,

    /// Ctoken compressible config
    /// CHECK: Validated by ctoken program
    pub ctoken_compressible_config: AccountInfo<'info>,

    /// Ctoken rent sponsor
    /// CHECK: Validated by ctoken program
    #[account(mut)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    /// Light system program
    /// CHECK: Program ID validated
    pub light_system_program: AccountInfo<'info>,

    /// Ctoken program
    /// CHECK: Program ID validated
    pub ctoken_program: AccountInfo<'info>,

    /// Ctoken CPI authority
    /// CHECK: Validated by ctoken program
    pub ctoken_cpi_authority: AccountInfo<'info>,

    /// Registered program PDA
    /// CHECK: Validated by light system program
    pub registered_program_pda: AccountInfo<'info>,

    /// Account compression authority
    /// CHECK: Validated by account compression program
    pub account_compression_authority: AccountInfo<'info>,

    /// Account compression program
    /// CHECK: Program ID validated
    pub account_compression_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    // Remaining accounts:
    // - State tree
    // - Input queue
    // - Output queue
    // - Mint signer PDAs (one per mint)
    // - CMint PDAs (one per mint)
}
```

### Processing Logic

```rust
pub fn decompress_cmints(
    ctx: Context<DecompressCMints>,
    params: DecompressCMintsParams,
) -> Result<()> {
    let remaining = ctx.remaining_accounts;

    // Parse tree accounts (first 3 remaining accounts)
    let state_tree = &remaining[0];
    let input_queue = &remaining[1];
    let output_queue = &remaining[2];

    // Remaining accounts after trees: [mint_signer1, cmint1, mint_signer2, cmint2, ...]
    let mint_accounts = &remaining[3..];

    let system_accounts = SystemAccountInfos {
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        cpi_authority_pda: ctx.accounts.ctoken_cpi_authority.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    for (i, cmint_data) in params.cmints.iter().enumerate() {
        let mint_signer = &mint_accounts[i * 2];
        let cmint = &mint_accounts[i * 2 + 1];

        DecompressCMintCpi {
            mint_seed: mint_signer.clone(),
            authority: ctx.accounts.authority.to_account_info(),
            payer: ctx.accounts.fee_payer.to_account_info(),
            cmint: cmint.clone(),
            compressible_config: ctx.accounts.ctoken_compressible_config.to_account_info(),
            rent_sponsor: ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            state_tree: state_tree.clone(),
            input_queue: input_queue.clone(),
            output_queue: output_queue.clone(),
            system_accounts: system_accounts.clone(),
            compressed_mint_with_context: cmint_data.compressed_mint_with_context.clone(),
            proof: ValidityProof(params.proof.0),
            rent_payment: cmint_data.rent_payment,
            write_top_up: cmint_data.write_top_up,
        }
        .invoke()?;
    }

    Ok(())
}
```

## Client Code Structure

```rust
// 1. Create multiple compressed mints first (separate tx)
// 2. Fetch compressed mint data from indexer
let cmint1_compressed = indexer.get_compressed_account(cmint1_address).await;
let cmint2_compressed = indexer.get_compressed_account(cmint2_address).await;

// 3. Get validity proof for all mints
let proof_result = rpc.get_validity_proof(
    vec![cmint1_compressed.hash, cmint2_compressed.hash],
    vec![],
    None,
).await;

// 4. Build instruction
let params = DecompressCMintsParams {
    proof: proof_result.proof,
    cmints: vec![
        CompressedMintDecompressData {
            mint_seed_pubkey: mint_signer1,
            compressed_mint_with_context: /* from cmint1_compressed */,
            rent_payment: 2,
            write_top_up: 5000,
        },
        CompressedMintDecompressData {
            mint_seed_pubkey: mint_signer2,
            compressed_mint_with_context: /* from cmint2_compressed */,
            rent_payment: 2,
            write_top_up: 5000,
        },
    ],
};

// 5. Build remaining accounts
let remaining_accounts = vec![
    state_tree,
    input_queue,
    output_queue,
    mint_signer1_account,
    cmint1_account,
    mint_signer2_account,
    cmint2_account,
];

// 6. Execute
rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority]).await;
```

## Test Plan

1. Create 2 compressed mints in setup
2. Fetch their compressed account data from indexer
3. Get single validity proof for both
4. Call `decompress_cmints` with both mints
5. Assert both CMint PDAs exist on-chain
6. Assert compressed accounts show empty data (decompressed state)

## Notes

- Each DecompressMint CPI verifies the proof independently
- For zkp proofs, this means the same proof is verified N times (one per mint)
- This uses more CUs than CPI context batching, but CPI context batching is not available for DecompressMint
- For `prove_by_index=true` mints, no proof verification needed (just queue position check)
