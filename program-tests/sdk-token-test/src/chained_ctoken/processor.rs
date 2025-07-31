use super::CreateCompressedMint;
use crate::chained_ctoken::create_mint::{
    create_compressed_mint, CreateCompressedMintInstructionData,
};
use crate::chained_ctoken::create_pda::process_create_escrow_pda;
use crate::chained_ctoken::mint_to::{mint_to_compressed, MintToCompressedInstructionData};
use anchor_lang::prelude::*;
use light_compressed_token_sdk::ValidityProof;
use light_ctoken_types::instructions::extensions::ExtensionInstructionData;
use light_ctoken_types::instructions::mint_to_compressed::CompressedMintInputs;
use light_ctoken_types::state::CompressedMint;
use light_ctoken_types::{COMPRESSED_MINT_SEED, COMPRESSED_TOKEN_PROGRAM_ID};
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall};

pub fn process_chained_ctoken<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, CreateCompressedMint<'info>>,
    input: CreateCompressedMintInstructionData,
    mint_input: MintToCompressedInstructionData,
    pda_proof: ValidityProof,
    output_tree_index: u8,
    amount: u64,
    address: [u8; 32],
    new_address_params: light_sdk::address::NewAddressParamsAssignedPacked,
) -> Result<()> {
    let config = CpiAccountsConfig {
        cpi_signer: crate::LIGHT_CPI_SIGNER,
        cpi_context: true,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };

    let cpi_accounts = CpiAccountsSmall::new_with_config(
        ctx.accounts.payer.as_ref(),
        ctx.remaining_accounts,
        config,
    );
    let spl_mint: Pubkey = Pubkey::create_program_address(
        &[
            COMPRESSED_MINT_SEED,
            ctx.accounts.mint_seed.key().as_ref(),
            &[input.mint_bump],
        ],
        &COMPRESSED_TOKEN_PROGRAM_ID.into(),
    )
    .unwrap()
    .into();
    msg!(
        "input.compressed_mint_address {:?}",
        input.compressed_mint_address
    );
    let compressed_mint_inputs = CompressedMintInputs {
        leaf_index: 1, // TODO: get from output queue
        prove_by_index: true,
        root_index: 0,
        address: input.compressed_mint_address,
        compressed_mint_input: CompressedMint {
            version: input.version,
            mint_authority: Some(ctx.accounts.mint_authority.key().into()),
            spl_mint: spl_mint.into(),
            decimals: input.decimals,
            supply: 0,
            is_decompressed: false,
            freeze_authority: None,
            extensions: None,
        },
    };
    // First CPI call: create compressed mint
    create_compressed_mint(&ctx, input, &cpi_accounts)?;
    /*
        // Second CPI call: mint to compressed tokens
        mint_to_compressed(
            &ctx,
            mint_input.clone(),
            compressed_mint_inputs,
            &cpi_accounts,
        )?;
    */
    msg!("address {:?}", address);
    msg!("cpi_accounts {:?}", cpi_accounts.tree_pubkeys());
    // Third CPI call: create compressed escrow PDA
    process_create_escrow_pda(
        pda_proof,
        output_tree_index,
        amount,
        address,
        new_address_params,
        cpi_accounts,
    )?;

    Ok(())
}
