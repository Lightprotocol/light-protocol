use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use light_hasher::DataHasher;
use psp_compressed_pda::{
    tlv::{Tlv, TlvDataElement},
    utxo::{OutUtxo, OutUtxoTuple},
    InstructionDataTransfer,
};

use crate::{AccountState, TokenTlvData};
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
              token::authority = mint_authority_pda
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
}

pub fn process_mint_to<'info>(
    ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    compression_public_keys: Vec<Pubkey>,
    amounts: Vec<u64>,
) -> Result<()> {
    if compression_public_keys.len() != amounts.len() {
        return err!(crate::ErrorCode::PublicKeyAmountMissmatch);
    }
    mint_spl_to_pool_pda(&ctx, &amounts)?;
    let out_utxos = create_out_utxos(
        ctx.accounts.mint.to_account_info().key(),
        compression_public_keys.as_slice(),
        &amounts,
    );
    cpi_execute_compressed_transaction_mint_to(&ctx, &out_utxos)?;
    Ok(())
}

pub fn create_out_utxos(mint_pubkey: Pubkey, pubkeys: &[Pubkey], amounts: &[u64]) -> Vec<OutUtxo> {
    pubkeys
        .iter()
        .zip(amounts.iter())
        .map(|(pubkey, amount)| {
            let token_tlv_data = TokenTlvData {
                mint: mint_pubkey,
                owner: *pubkey,
                amount: *amount,
                delegate: None,
                state: AccountState::Initialized,
                is_native: None,
                delegated_amount: 0,
            };
            let mut token_data = Vec::new();
            token_tlv_data.serialize(&mut token_data).unwrap();
            let data: TlvDataElement = TlvDataElement {
                discriminator: 2u64.to_le_bytes(),
                owner: crate::ID,
                data: token_data,
                data_hash: token_tlv_data.hash().unwrap(),
            };
            OutUtxo {
                owner: crate::ID,
                lamports: 0,
                data: Some(Tlv {
                    tlv_elements: vec![data],
                }),
            }
        })
        .collect()
}

#[inline(never)]
pub fn cpi_execute_compressed_transaction_mint_to<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    out_utxos: &[OutUtxo],
) -> Result<()> {
    let mut _out_utxos = Vec::<OutUtxoTuple>::new();
    for utxo in out_utxos.iter() {
        _out_utxos.push(OutUtxoTuple {
            out_utxo: utxo.clone(),
            index_mt_account: 0,
        });
    }

    let inputs_struct = InstructionDataTransfer {
        low_element_indices: Vec::new(),
        relay_fee: None,
        in_utxos: Vec::new(),
        out_utxos: _out_utxos,
        root_indices: Vec::new(),
        proof: None,
    };

    let mut inputs = Vec::new();
    InstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();
    let authority_bytes = ctx.accounts.authority.key().to_bytes();
    let mint_bytes = ctx.accounts.mint.key().to_bytes();
    let seeds = [
        MINT_AUTHORITY_SEED,
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
    ];
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(seeds.as_slice(), ctx.program_id);
    let bump = &[bump];
    let seeds = [
        MINT_AUTHORITY_SEED,
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
        bump,
    ];

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = psp_compressed_pda::cpi::accounts::TransferInstruction {
        signer: ctx.accounts.mint_authority_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .psp_account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        cpi_signature_account: None,
        invoking_program: None,
    };
    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_pda_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = vec![ctx.accounts.merkle_tree.to_account_info()];
    psp_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs)?;
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
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(seeds.as_slice(), ctx.program_id);
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
    pub compressed_pda_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
    /// CHECK: this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(mut, seeds = [b"cpi_authority", account_compression::ID.to_bytes().as_slice()], bump, seeds::program = psp_compressed_pda::ID,)]
    pub psp_account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK: this account will be checked by psp compressed pda program
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,
}

pub fn get_token_authority_pda(signer: &Pubkey, mint: &Pubkey) -> Pubkey {
    let signer_seed = signer.to_bytes();
    let mint_seed = mint.to_bytes();
    let seeds = &[
        MINT_AUTHORITY_SEED,
        signer_seed.as_slice(),
        mint_seed.as_slice(),
    ];
    let (address, _) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
    address
}

pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    let seeds = &[POOL_SEED, mint.as_ref()];
    let (address, _) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
    address
}

#[cfg(not(target_os = "solana"))]
pub mod mint_sdk {
    use account_compression::NOOP_PROGRAM_ID;
    use anchor_lang::{system_program, InstructionData, ToAccountMetas};
    use anchor_spl;
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{get_token_authority_pda, get_token_pool_pda};

    pub fn create_initiatialize_mint_instruction(
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
            compressed_pda_program: psp_compressed_pda::ID,
            registered_program_pda: psp_compressed_pda::utils::get_registered_program_pda(
                &psp_compressed_pda::ID,
            ),
            noop_program: NOOP_PROGRAM_ID,
            psp_account_compression_authority: psp_compressed_pda::utils::get_cpi_authority_pda(
                &psp_compressed_pda::ID,
            ),
            account_compression_program: account_compression::ID,
            merkle_tree: *merkle_tree,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }
}
