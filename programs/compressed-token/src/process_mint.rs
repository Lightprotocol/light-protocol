use crate::{AccountState, TokenData};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use light_compressed_pda::compressed_account::{CompressedAccount, CompressedAccountData};
use light_hasher::DataHasher;
use std::mem;
pub const POOL_SEED: &[u8] = b"pool";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority_pda";

/// creates a token pool account which is owned by the token authority pda
#[derive(Accounts)]
pub struct CreateMintInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init,
              seeds = [
                POOL_SEED, &mint.key().to_bytes(),
              ],
              bump,
              payer = fee_payer,
              token::mint = mint,
              token::authority = cpi_authority_pda,
    )]
    pub token_pool_pda: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK:
    #[account(mut, seeds=[MINT_AUTHORITY_SEED, authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump)]
    pub mint_authority_pda: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK: TODO
    #[account(seeds = [b"cpi_authority"], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

#[allow(unused_variables)]
pub fn process_mint_to<'info>(
    ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    compression_public_keys: Vec<Pubkey>,
    amounts: Vec<u64>,
) -> Result<()> {
    if compression_public_keys.len() != amounts.len() {
        msg!(
            "compression_public_keys.len() {} !=  {} amounts.len()",
            compression_public_keys.len(),
            amounts.len()
        );
        return err!(crate::ErrorCode::PublicKeyAmountMissmatch);
    }

    #[cfg(target_os = "solana")]
    {
        let inputs_len =
    // struct
    mem::size_of::<InstructionDataTransfer>()
    // `output_compressed_accounts`
    + mem::size_of::<CompressedAccount>() * amounts.len()
    // `output_state_merkle_tree_account_indices`
    + amounts.len()+ mem::size_of::<Option::<CompressedCpiContext>>();
        let mut inputs = Vec::<u8>::with_capacity(inputs_len);
        // # SAFETY: the inputs vector needs to be allocated before this point.
        // All heap memory from this point on is freed prior to the cpi call.
        let pre_compressed_acounts_pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();

        mint_spl_to_pool_pda(&ctx, &amounts)?;
        let mut output_compressed_accounts =
            vec![CompressedAccount::default(); compression_public_keys.len()];
        create_output_compressed_accounts(
            &mut output_compressed_accounts,
            ctx.accounts.mint.to_account_info().key(),
            compression_public_keys.as_slice(),
            &amounts,
            None,
        );

        cpi_execute_compressed_transaction_mint_to(
            &ctx,
            output_compressed_accounts,
            &mut inputs,
            pre_compressed_acounts_pos,
        )?;
    }
    Ok(())
}

pub fn create_output_compressed_accounts(
    output_compressed_accounts: &mut [CompressedAccount],
    mint_pubkey: Pubkey,
    pubkeys: &[Pubkey],
    amounts: &[u64],
    lamports: Option<&[Option<u64>]>,
) {
    for (i, (pubkey, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
        let mut token_data_bytes = Vec::with_capacity(mem::size_of::<TokenData>());
        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();

        let token_data = TokenData {
            mint: mint_pubkey,
            owner: *pubkey,
            amount: *amount,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
        };

        token_data.serialize(&mut token_data_bytes).unwrap();

        let data: CompressedAccountData = CompressedAccountData {
            discriminator: 2u64.to_le_bytes(),
            data: token_data_bytes,
            data_hash: token_data.hash().unwrap(),
        };
        let lamports = lamports.and_then(|lamports| lamports[i]).unwrap_or(0);

        output_compressed_accounts[i] = CompressedAccount {
            owner: crate::ID,
            lamports,
            data: Some(data),
            address: None,
        };
        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos);
    }
}

#[cfg(target_os = "solana")]
#[inline(never)]
pub fn cpi_execute_compressed_transaction_mint_to<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    output_compressed_accounts: Vec<CompressedAccount>,
    inputs: &mut Vec<u8>,
    pre_compressed_acounts_pos: usize,
) -> Result<()> {
    let authority_bytes = ctx.accounts.authority.key().to_bytes();
    let mint_bytes = ctx.accounts.mint.key().to_bytes();
    let seeds = [
        MINT_AUTHORITY_SEED,
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
    ];
    let (_, bump) = Pubkey::find_program_address(seeds.as_slice(), ctx.program_id);
    let bump = &[bump];
    let seeds = [
        MINT_AUTHORITY_SEED,
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
        bump,
    ];

    let len = output_compressed_accounts.len();
    let inputs_struct = light_compressed_pda::InstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::with_capacity(0),
        output_compressed_accounts,
        output_state_merkle_tree_account_indices: vec![0u8; len],
        input_root_indices: Vec::with_capacity(0),
        proof: None,
        new_address_params: Vec::with_capacity(0),
        compression_lamports: None,
        is_compress: false,
        signer_seeds: Some(seeds.iter().map(|seed| seed.to_vec()).collect()),
        cpi_context: Some(light_compressed_pda::CompressedCpiContext {
            execute: true,
            cpi_signature_account_index: 0,
        }),
    };

    InstructionDataTransfer::serialize(&inputs_struct, inputs)?;

    light_heap::GLOBAL_ALLOCATOR.free_heap(pre_compressed_acounts_pos);

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = light_compressed_pda::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.fee_payer.to_account_info(),
        authority: ctx.accounts.mint_authority_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: Some(ctx.accounts.self_program.to_account_info()),
        compressed_sol_pda: None,
        compression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_signature_account: None,
    };
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = vec![ctx.accounts.merkle_tree.to_account_info()];
    light_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs.to_owned())?;
    Ok(())
}

#[inline(never)]
pub fn mint_spl_to_pool_pda<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    amounts: &[u64],
) -> Result<()> {
    let mut mint_amount: u64 = 0;
    for amount in amounts.iter() {
        mint_amount = mint_amount.saturating_add(*amount);
    }
    let authority_bytes = ctx.accounts.authority.key().to_bytes();
    let mint_bytes = ctx.accounts.mint.key().to_bytes();
    let seeds = [
        MINT_AUTHORITY_SEED,
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
    ];
    let (_, bump) = Pubkey::find_program_address(seeds.as_slice(), ctx.program_id);
    let bump = &[bump];
    let seeds = [
        MINT_AUTHORITY_SEED,
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
        bump,
    ];

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = anchor_spl::token::MintTo {
        authority: ctx.accounts.mint_authority_pda.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_pool_pda.to_account_info(),
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
    // This is the cpi signer
    /// CHECK: that mint authority is derived from signer
    #[account(mut, seeds = [MINT_AUTHORITY_SEED, authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump,)]
    pub mint_authority_pda: UncheckedAccount<'info>,
    /// CHECK: that authority is mint authority
    #[account(mut, constraint = mint.mint_authority.unwrap() == mint_authority_pda.key())]
    pub mint: Account<'info, Mint>,
    /// CHECK: this account
    #[account(mut)]
    pub token_pool_pda: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub compressed_pda_program: Program<'info, light_compressed_pda::program::LightCompressedPda>,
    /// CHECK: this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(mut, seeds = [b"cpi_authority"], bump, seeds::program = light_compressed_pda::ID,)]
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK: this account will be checked by psp compressed pda program
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,
    pub self_program: Program<'info, crate::program::LightCompressedToken>,
    pub system_program: Program<'info, System>,
}

pub fn get_token_authority_pda(signer: &Pubkey, mint: &Pubkey) -> Pubkey {
    let signer_seed = signer.to_bytes();
    let mint_seed = mint.to_bytes();
    let seeds = &[
        MINT_AUTHORITY_SEED,
        signer_seed.as_slice(),
        mint_seed.as_slice(),
    ];
    let (address, _) = Pubkey::find_program_address(seeds, &crate::ID);
    address
}

pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    let seeds = &[POOL_SEED, mint.as_ref()];
    let (address, _) = Pubkey::find_program_address(seeds, &crate::ID);
    address
}

#[cfg(not(target_os = "solana"))]
pub mod mint_sdk {
    use account_compression::NOOP_PROGRAM_ID;
    use anchor_lang::{system_program, InstructionData, ToAccountMetas};
    use anchor_spl;
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{get_cpi_authority_pda, get_token_authority_pda, get_token_pool_pda};

    pub fn create_initialize_mint_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction {
        let token_pool_pda = get_token_pool_pda(mint);
        let mint_authority_pda = get_token_authority_pda(authority, mint);
        let instruction_data = crate::instruction::CreateMint {};

        let accounts = crate::accounts::CreateMintInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            token_pool_pda,
            system_program: system_program::ID,
            mint: *mint,
            mint_authority_pda,
            token_program: anchor_spl::token::ID,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }

    pub fn create_mint_to_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        mint: &Pubkey,
        merkle_tree: &Pubkey,
        amounts: Vec<u64>,
        public_keys: Vec<Pubkey>,
    ) -> Instruction {
        let token_pool_pda = get_token_pool_pda(mint);
        let mint_authority_pda = get_token_authority_pda(authority, mint);
        let instruction_data = crate::instruction::MintTo {
            amounts,
            public_keys,
        };

        let accounts = crate::accounts::MintToInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            mint_authority_pda,
            mint: *mint,
            token_pool_pda,
            token_program: anchor_spl::token::ID,
            compressed_pda_program: light_compressed_pda::ID,
            registered_program_pda: light_compressed_pda::utils::get_registered_program_pda(
                &light_compressed_pda::ID,
            ),
            noop_program: NOOP_PROGRAM_ID,
            account_compression_authority: light_compressed_pda::utils::get_cpi_authority_pda(
                &light_compressed_pda::ID,
            ),
            account_compression_program: account_compression::ID,
            merkle_tree: *merkle_tree,
            self_program: crate::ID,
            system_program: system_program::ID,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }
}
