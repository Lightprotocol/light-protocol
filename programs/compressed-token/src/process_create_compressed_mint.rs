use std::vec;

use crate::MINT_AUTHORITY_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token::Mint;
use light_hasher::poseidon::Poseidon;
use light_hasher::Hasher;
use light_utils::hash_to_bn254_field_size_le;
use psp_compressed_pda::compressed_account::derive_address;
use psp_compressed_pda::utils::CompressedProof;
use psp_compressed_pda::{
    compressed_account::{CompressedAccount, CompressedAccountData},
    InstructionDataTransfer,
};
/// creates a token pool account which is owned by the token authority pda
#[derive(Accounts)]
pub struct CreateCompressedMintAccountInstruction<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:
    pub mint: Account<'info, Mint>,
    /// CHECK:
    #[account(mut, seeds=[MINT_AUTHORITY_SEED, authority.key().to_bytes().as_slice(), mint.key().to_bytes().as_slice()], bump)]
    pub mint_authority_pda: AccountInfo<'info>,
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
    /// CHECK: this account will be checked by psp compressed pda program
    pub address_merkle_tree: UncheckedAccount<'info>,
    /// CHECK: this account will be checked by psp compressed pda program
    #[account(mut)]
    pub address_merkle_tree_queue: UncheckedAccount<'info>,
}

pub fn process_create_compressed_mint_account<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateCompressedMintAccountInstruction<'info>>,
    address_non_inclusion_proof: CompressedProof,
    address_merkle_tree_root_index: u16,
) -> Result<()> {
    let seed = ctx.accounts.mint.key().to_bytes();
    let address = derive_address(&ctx.accounts.address_merkle_tree.key(), &seed)?;
    let data: CompressedAccountData = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: ctx.accounts.mint.to_account_info().data.borrow().to_vec(),
        data_hash: data_hash(&ctx.accounts.mint).unwrap(),
    };
    let output_compressed_account = CompressedAccount {
        owner: crate::ID,
        lamports: 0u64,
        data: Some(data),
        address: Some(address),
    };

    cpi_execute_compressed_transaction_create_compressed_mint(
        &ctx,
        &[output_compressed_account],
        address_non_inclusion_proof,
        seed,
        address_merkle_tree_root_index,
    )
}

#[inline(never)]
pub fn cpi_execute_compressed_transaction_create_compressed_mint<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedMintAccountInstruction<'info>>,
    output_compressed_accounts: &[CompressedAccount],
    address_non_inclusion_proof: CompressedProof,
    seed: [u8; 32],
    address_merkle_tree_root_index: u16,
) -> Result<()> {
    let inputs_struct = InstructionDataTransfer {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: output_compressed_accounts.to_vec(),
        output_state_merkle_tree_account_indices: vec![0u8],
        input_root_indices: Vec::new(),
        proof: Some(address_non_inclusion_proof),
        new_address_seeds: vec![seed],
        address_merkle_tree_root_indices: vec![address_merkle_tree_root_index],
        address_merkle_tree_account_indices: vec![1u8],
        address_queue_account_indices: vec![2u8],
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

    cpi_ctx.remaining_accounts = vec![
        ctx.accounts.merkle_tree.to_account_info(),
        ctx.accounts.address_merkle_tree.to_account_info(),
        ctx.accounts.address_merkle_tree_queue.to_account_info(),
    ];
    psp_compressed_pda::cpi::execute_compressed_transaction(cpi_ctx, inputs)?;
    Ok(())
}

pub fn data_hash(
    mint: &Mint,
) -> std::prelude::v1::Result<[u8; 32], light_hasher::errors::HasherError> {
    let mint_authority = match mint.mint_authority {
        COption::Some(mint_authority) => {
            hash_to_bn254_field_size_le(mint_authority.to_bytes().as_slice())
                .unwrap()
                .0
        }
        COption::None => [0u8; 32],
    };
    let freeze_authority = match mint.freeze_authority {
        COption::Some(freeze_authority) => {
            hash_to_bn254_field_size_le(freeze_authority.to_bytes().as_slice())
                .unwrap()
                .0
        }
        COption::None => [0u8; 32],
    };
    Poseidon::hashv(&[
        mint_authority.as_slice(),
        mint.supply.to_le_bytes().as_slice(),
        mint.decimals.to_le_bytes().as_slice(),
        [mint.is_initialized as u8].as_slice(),
        freeze_authority.as_slice(),
    ])
}

#[cfg(not(target_os = "solana"))]
pub mod create_compressed_mint_sdk {
    use account_compression::NOOP_PROGRAM_ID;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use psp_compressed_pda::utils::CompressedProof;
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::get_token_authority_pda;

    pub fn create_compressed_mint_account_to_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        mint: &Pubkey,
        merkle_tree: &Pubkey,
        address_merkle_tree: &Pubkey,
        address_merkle_tree_queue: &Pubkey,
        address_non_inclusion_proof: CompressedProof,
        address_merkle_tree_root_index: u16,
    ) -> Instruction {
        let mint_authority_pda = get_token_authority_pda(authority, mint);
        let instruction_data = crate::instruction::CreateCompressedMintAccount {
            address_non_inclusion_proof,
            address_merkle_tree_root_index,
        };

        let accounts = crate::accounts::CreateCompressedMintAccountInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            mint_authority_pda,
            mint: *mint,
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
            address_merkle_tree: *address_merkle_tree,
            address_merkle_tree_queue: *address_merkle_tree_queue,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }
}
