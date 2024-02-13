// use account_compression::ConcurrentMerkleTreeAccount;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
// use psp_compressed_pda::program::PspCompressedPda;
// use light_merkle_tree_program::{program::LightMerkleTreeProgram, state_merkle_tree_from_bytes};
// use light_verifier_sdk::{public_transaction::PublicTransactionEvent, utxo::Utxo};

pub fn process_create_mint<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateMintInstruction<'info>>,
) -> Result<()> {
    cpi_register_merkle_tree_token_pool(&ctx)?;
    Ok(())
}

#[derive(Accounts)]
pub struct CreateMintInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// Mint authority, ensures that this program needs to be used as a proxy to mint tokens
    #[account(mut, seeds = [b"authority",authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump,)]
    pub authority_pda: UncheckedAccount<'info>,
    /// not sure whether this is going to work with the pda, but even if it doesn't we can just as well take a normal account, this is safe because every account can only exist once and you need the private key
    #[account(mut, constraint = mint.mint_authority.unwrap() == authority_pda.key())]
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    /// CHECK this account in merkle tree program
    #[account(mut)]
    pub registered_asset_pool_pda: UncheckedAccount<'info>,
    /// CHECK this account in merkle tree program
    pub registered_pool_type_pda: UncheckedAccount<'info>,
    /// CHECK this account in merkle tree program
    #[account(mut)]
    pub merkle_tree_pda_token: UncheckedAccount<'info>,
    /// CHECK this account in merkle tree program
    #[account(mut)]
    pub merkle_tree_authority_pda: UncheckedAccount<'info>,
    /// CHECK this account in merkle tree program
    #[account(mut)]
    pub token_authority: UncheckedAccount<'info>,
    /// CHECK this account in merkle tree program
    pub compressed_pda_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
}

pub fn cpi_register_merkle_tree_token_pool<'a, 'info>(
    ctx: &'a Context<'a, '_, '_, 'info, CreateMintInstruction<'info>>,
) -> Result<()> {
    let authority_bytes = ctx.accounts.authority.key().to_bytes();
    let mint_bytes = ctx.accounts.mint.key().to_bytes();
    let seeds = [
        b"authority".as_slice(),
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
    ];
    // let seeds = seeds.concat();
    let (address, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(seeds.as_slice(), ctx.program_id);
    // let (seed, bump) = get_seeds(ctx.program_id, &ctx.accounts.authority)?;
    let bump = &[bump];
    let seeds = [
        b"authority".as_slice(),
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
        bump,
    ];
    msg!("address: {:?}", address);
    msg!("seeds: {:?}", seeds);
    // let seeds = &[seeds, bump.as_slice()][..];
    // let signer_seeds = &[&seeds[..]];
    // let cpi_accounts = psp_authority_program::cpi::accounts::RegisterSplPool {
    //     registered_asset_pool_pda: ctx.accounts.registered_asset_pool_pda.to_account_info(),
    //     registered_pool_type_pda: ctx.accounts.registered_pool_type_pda.to_account_info(),
    //     authority: ctx.accounts.authority_pda.to_account_info(),
    //     mint: ctx.accounts.mint.to_account_info(),
    //     system_program: ctx.accounts.system_program.to_account_info(),
    //     token_program: ctx.accounts.token_program.to_account_info(),
    //     token_authority: ctx.accounts.token_authority.to_account_info(),
    //     merkle_tree_pda_token: ctx.accounts.merkle_tree_pda_token.to_account_info(),
    //     merkle_tree_authority_pda: ctx.accounts.merkle_tree_authority_pda.to_account_info(),
    // };
    // let cpi_ctx = CpiContext::new_with_signer(
    //     ctx.accounts.authority_program.to_account_info(),
    //     cpi_accounts,
    //     signer_seeds,
    // );
    // psp_authority_program::cpi::register_spl_pool(cpi_ctx)?;
    Ok(())
}

pub fn process_mint_to<'info>(
    _ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    _compression_public_keys: Vec<[u8; 32]>,
    _amounts: Vec<u64>,
) -> Result<()> {
    // // TODO: adapt for flexible number of amounts blocker is batched Merkle tree update
    // if amounts.len() != 2 {
    //     panic!("Only 2 amounts supported");
    // }
    // if compression_public_keys.len() != amounts.len() {
    //     return err!(crate::ErrorCode::PublicKeyAmountMissmatch);
    // }

    // let merkle_tree_account = ctx.accounts.merkle_tree_set.load_mut()?;
    // // let merkle_tree = state_merkle_tree_from_bytes(&merkle_tree_account.state_merkle_tree);

    // let mut utxos = Vec::with_capacity(compression_public_keys.len());
    // let mut utxo_indexes: Vec<u64> = Vec::with_capacity(compression_public_keys.len());
    // for (i, (public_key, amount)) in compression_public_keys.iter().zip(&amounts).enumerate() {
    //     let mut utxo = Utxo {
    //         version: 0,
    //         pool_type: 0,
    //         amounts: [0, *amount],
    //         spl_asset_mint: Some(ctx.accounts.mint.to_account_info().key()),
    //         owner: *public_key,
    //         blinding: [0u8; 32],
    //         data_hash: [0u8; 32],
    //         meta_hash: [0u8; 32],
    //         address: [0u8; 32],
    //         message: None,
    //     };
    //     utxo.update_blinding(
    //         ctx.accounts.merkle_tree_set.key(),
    //         (merkle_tree.next_index + i as u64) as usize,
    //     )
    //     .unwrap();
    //     utxo_indexes.push(merkle_tree.next_index + i as u64);
    //     utxos.push(utxo);
    // }
    // drop(merkle_tree_account);
    // let utxo_hashes = utxos
    //     .iter()
    //     .map(|utxo| utxo.hash().unwrap())
    //     .collect::<Vec<[u8; 32]>>();

    // mint_spl_to_merkle_tree(&ctx, amounts)?;

    // msg!("self.out_utxo_hashes.to_vec(), {:?}", utxo_hashes.to_vec());
    // msg!("out utxo version {:?}", utxos[0].version);
    // msg!("out utxo amounts {:?}", utxos[0].amounts);
    // msg!("out utxo spl_asset_mint {:?}", utxos[0].spl_asset_mint);
    // msg!("out utxo owner {:?}", utxos[0].owner);
    // msg!("out utxo blinding {:?}", utxos[0].blinding);
    // msg!("out utxo data_hash {:?}", utxos[0].data_hash);
    // msg!("out utxo meta_hash {:?}", utxos[0].meta_hash);
    // msg!("out utxo address {:?}", utxos[0].address);
    // msg!("out utxo message {:?}", utxos[0].message);
    // msg!("out utxo  utxo_hashes {:?}", utxo_hashes[0]);

    // msg!("out utxo 1 version {:?}", utxos[1].version);
    // msg!("out utxo 1 amounts {:?}", utxos[1].amounts);
    // msg!("out utxo 1 spl_asset_mint {:?}", utxos[1].spl_asset_mint);
    // msg!("out utxo 1 owner {:?}", utxos[1].owner);
    // msg!("out utxo 1 blinding {:?}", utxos[1].blinding);
    // msg!("out utxo 1 data_hash {:?}", utxos[1].data_hash);
    // msg!("out utxo 1 meta_hash {:?}", utxos[1].meta_hash);
    // msg!("out utxo 1 address {:?}", utxos[1].address);
    // msg!("out utxo 1 message {:?}", utxos[1].message);
    // msg!("out utxo 1 utxo_hashes {:?}", utxo_hashes[1]);

    // // TODO: switch to batched update
    // cpi_merkle_tree(&ctx, utxo_hashes)?;

    // let event = PublicTransactionEvent {
    //     in_utxo_hashes: Vec::<[u8; 32]>::new(),
    //     out_utxos: utxos.to_vec(),
    //     // .iter()
    //     // .map(|utxo| utxo.try_to_vec().unwrap())
    //     // .collect(),
    //     public_amount_spl: None,
    //     public_amount_sol: None,
    //     out_utxo_indexes: utxo_indexes,
    //     rpc_fee: None,
    //     message: None,
    //     transaction_hash: None,
    //     program_id: None,
    // };
    // light_verifier_sdk::cpi_instructions::invoke_indexer_transaction_event::<PublicTransactionEvent>(
    //     &event,
    //     &ctx.accounts.noop_program.to_account_info(),
    // )?;
    Ok(())
}

#[inline(never)]
pub fn cpi_merkle_tree<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    utxo_hashes: Vec<[u8; 32]>,
) -> Result<()> {
    let mut merkle_tree_accounts = Vec::new();
    for _ in 0..utxo_hashes.len() {
        merkle_tree_accounts.push(ctx.accounts.merkle_tree_set.to_account_info().clone());
    }
    // light_verifier_sdk::cpi_instructions::insert_two_leaves_parallel_cpi(
    //     ctx.program_id,
    //     &ctx.accounts
    //         .account_compression_program
    //         .to_account_info()
    //         .clone(),
    //     &ctx.accounts
    //         .psp_account_compression_authority
    //         .to_account_info()
    //         .clone(),
    //     &ctx.accounts
    //         .registered_verifier_pda
    //         .to_account_info()
    //         .clone(),
    //     utxo_hashes,
    //     merkle_tree_accounts,
    //     &ctx.accounts.noop_program.to_account_info(),
    // )?;
    Ok(())
}
#[inline(never)]
pub fn mint_spl_to_merkle_tree<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    amounts: Vec<u64>,
) -> Result<()> {
    let mut mint_amount: u64 = 0;
    for amount in amounts.iter() {
        mint_amount = mint_amount.saturating_add(*amount);
    }
    let authority_bytes = ctx.accounts.authority.key().to_bytes();
    let mint_bytes = ctx.accounts.mint.key().to_bytes();
    let seeds = [
        b"authority".as_slice(),
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
    ];
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(seeds.as_slice(), ctx.program_id);
    let bump = &[bump];
    let seeds = [
        b"authority".as_slice(),
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
        bump,
    ];
    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = anchor_spl::token::MintTo {
        authority: ctx.accounts.authority_pda.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.merkle_tree_pda_token.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    anchor_spl::token::mint_to(cpi_ctx, mint_amount)?;
    Ok(())
}

#[derive(Accounts)]
pub struct MintToInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// Check is checked in Merkle tree program
    #[account(mut)]
    pub merkle_tree_authority: UncheckedAccount<'info>,
    /// Check that mint authority is derived from signer
    #[account(mut, seeds = [b"authority", authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump,)]
    pub authority_pda: UncheckedAccount<'info>,
    /// Check that authority is mint authority
    #[account(mut, constraint = mint.mint_authority.unwrap() == authority_pda.key())]
    pub mint: Account<'info, Mint>,
    // pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    /// CHECK this account
    #[account(mut)]
    pub registered_verifier_pda: UncheckedAccount<'info>,
    /// CHECK this account
    #[account(mut)]
    pub merkle_tree_pda_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub merkle_tree_set: UncheckedAccount<'info>, // AccountLoader<'info, ConcurrentMerkleTreeAccount>,
    /// CHECK this account
    pub noop_program: UncheckedAccount<'info>,
    pub merkle_tree_program: UncheckedAccount<'info>, //Program<'info, LightMerkleTreeProgram>,
    pub account_compression_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
    /// CHECK this account in psp account compression program
    #[account(mut)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
}
