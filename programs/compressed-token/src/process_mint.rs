use crate::{
    constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
    token_data::{AccountState, TokenData},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use light_hasher::Poseidon;
use light_heap::{bench_sbf_end, bench_sbf_start};
use light_system_program::{
    sdk::compressed_account::{CompressedAccount, CompressedAccountData},
    OutputCompressedAccountWithPackedContext,
};
use light_utils::hash_to_bn254_field_size_be;

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
    bump: u8,
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
    mem::size_of::<light_system_program::InstructionDataInvokeCpi>()
    // `output_compressed_accounts`
    + mem::size_of::<CompressedAccount>() * amounts.len()
    // `output_state_merkle_tree_account_indices`
    + amounts.len() + mem::size_of::<Option::<light_system_program::sdk::CompressedCpiContext>>()+ 8;
        let mut inputs = Vec::<u8>::with_capacity(inputs_len);
        // safety buffer prior to heap pos
        let buffer = vec![0u8; 8];
        // # SAFETY: the inputs vector needs to be allocated before this point.
        // All heap memory from this point on is freed prior to the cpi call.
        let pre_compressed_acounts_pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
        bench_sbf_start!("tm_mint_spl_to_pool_pda");
        let authority_bytes = ctx.accounts.authority.key().to_bytes();
        let mint_bytes = ctx.accounts.mint.key().to_bytes();

        let bump = &[bump];
        let seeds = [
            MINT_AUTHORITY_SEED,
            authority_bytes.as_slice(),
            mint_bytes.as_slice(),
            bump,
        ];
        let signer_seeds = &[&seeds[..]];
        // 7,912 CU
        mint_spl_to_pool_pda(&ctx, &amounts, signer_seeds)?;
        bench_sbf_end!("tm_mint_spl_to_pool_pda");
        let hashed_mint =
            hash_to_bn254_field_size_be(ctx.accounts.mint.to_account_info().key().as_ref())
                .unwrap()
                .0;
        bench_sbf_start!("tm_output_compressed_accounts");
        let mut output_compressed_accounts = vec![
            OutputCompressedAccountWithPackedContext::default(
            );
            compression_public_keys.len()
        ];
        create_output_compressed_accounts(
            &mut output_compressed_accounts,
            ctx.accounts.mint.to_account_info().key(),
            compression_public_keys.as_slice(),
            &amounts,
            None,
            &hashed_mint,
            &vec![0u8; amounts.len()],
        )?;
        bench_sbf_end!("tm_output_compressed_accounts");

        cpi_execute_compressed_transaction_mint_to(
            &ctx,
            output_compressed_accounts,
            &mut inputs,
            pre_compressed_acounts_pos,
            signer_seeds,
        )?;
    }
    Ok(())
}

pub fn create_output_compressed_accounts(
    output_compressed_accounts: &mut [OutputCompressedAccountWithPackedContext],
    mint_pubkey: Pubkey,
    pubkeys: &[Pubkey],
    amounts: &[u64],
    lamports: Option<&[Option<u64>]>,
    hashed_mint: &[u8; 32],
    merkle_tree_indices: &[u8],
) -> Result<()> {
    for (i, (pubkey, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
        // 1,147 CU
        let mut token_data_bytes = Vec::with_capacity(mem::size_of::<TokenData>());
        #[cfg(target_os = "solana")]
        let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();

        // 1,000 CU token data and serialize
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
        bench_sbf_start!("token_data_hash");
        let pubkey_hashed = hash_to_bn254_field_size_be(pubkey.as_ref()).unwrap().0;
        let amount_bytes = amount.to_le_bytes();
        // 7,200 CU
        let data: CompressedAccountData = CompressedAccountData {
            discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            data: token_data_bytes,
            data_hash: TokenData::hash_with_hashed_values::<Poseidon>(
                hashed_mint,
                &pubkey_hashed,
                &amount_bytes,
                &None,
            )
            .map_err(ProgramError::from)?,
        };
        bench_sbf_end!("token_data_hash");
        let lamports = lamports.and_then(|lamports| lamports[i]).unwrap_or(0);
        // 1k CU
        output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID,
                lamports,
                data: Some(data),
                address: None,
            },
            merkle_tree_index: merkle_tree_indices[i],
        };

        #[cfg(target_os = "solana")]
        light_heap::GLOBAL_ALLOCATOR.free_heap(pos);
    }
    Ok(())
}

#[cfg(target_os = "solana")]
#[inline(never)]
pub fn cpi_execute_compressed_transaction_mint_to<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    inputs: &mut Vec<u8>,
    pre_compressed_acounts_pos: usize,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    light_heap::bench_sbf_start!("tm_cpi");

    // 4300 CU for 10 accounts
    // 6700 CU for 20 accounts
    // 7,978 CU for 25 accounts
    serialize_mint_to_cpi_instruction_data(inputs, output_compressed_accounts, signer_seeds);

    light_heap::GLOBAL_ALLOCATOR.free_heap(pre_compressed_acounts_pos);

    use anchor_lang::InstructionData;

    // 826 CU
    let instructiondata = light_system_program::instruction::InvokeCpi {
        inputs: inputs.to_owned(),
    };

    // 1300 CU
    let account_infos = vec![
        ctx.accounts.fee_payer.to_account_info(),
        ctx.accounts.mint_authority_pda.to_account_info(),
        ctx.accounts.registered_program_pda.to_account_info(),
        ctx.accounts.noop_program.to_account_info(),
        ctx.accounts.account_compression_authority.to_account_info(),
        ctx.accounts.account_compression_program.to_account_info(),
        ctx.accounts.self_program.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(), // none compressed_sol_pda
        ctx.accounts.light_system_program.to_account_info(), // none compression_recipient
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(), // none cpi_context_account
        ctx.accounts.merkle_tree.to_account_info(),
    ];

    // account_metas take 1k cu
    let accounts = vec![
        AccountMeta {
            pubkey: account_infos[0].key(),
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: account_infos[1].key(),
            is_signer: true,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[2].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[3].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[4].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[5].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[6].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[7].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[8].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta::new_readonly(account_infos[9].key(), false),
        AccountMeta {
            pubkey: account_infos[10].key(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: account_infos[11].key(),
            is_signer: false,
            is_writable: true,
        },
    ];

    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: light_system_program::ID,
        accounts,
        // data: inputs.to_owned(),
        data: instructiondata.data(),
    };

    light_heap::bench_sbf_end!("tm_cpi");
    light_heap::bench_sbf_start!("tm_invoke_signed");
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        account_infos.as_slice(),
        signer_seeds,
    )?;
    light_heap::bench_sbf_end!("tm_invoke_signed");
    Ok(())
}

#[inline(never)]
pub fn serialize_mint_to_cpi_instruction_data(
    inputs: &mut Vec<u8>,
    output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    seeds: &[&[&[u8]]],
) {
    let len = output_compressed_accounts.len();
    // proof (option None)
    inputs.extend_from_slice(&[0u8]);
    // empty vecs: address_params, (no) input_root_indices, input_compressed_accounts_with_merkle_context
    inputs.extend_from_slice(&[0u8; 8]);
    inputs.extend_from_slice(&[(len as u8), 0, 0, 0]);
    // output_compressed_accounts
    for compressed_account in output_compressed_accounts.iter() {
        compressed_account.serialize(inputs).unwrap();
    }
    // relay_fee and compression lamports, is compress bool
    inputs.extend_from_slice(&[0u8; 3]);
    // seeds
    let seeds: Vec<Vec<u8>> = seeds[0].iter().map(|seed| seed.to_vec()).collect();

    seeds.serialize(inputs).unwrap();
    inputs.extend_from_slice(&[0u8]);
}

#[inline(never)]
pub fn mint_spl_to_pool_pda<'info>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    amounts: &[u64],
    _signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut mint_amount: u64 = 0;
    for amount in amounts.iter() {
        mint_amount = mint_amount.saturating_add(*amount);
    }

    let cpi_accounts = anchor_spl::token::MintTo {
       // authority: ctx.accounts.mint_authority_pda.to_account_info(),
       authority: ctx.accounts.authority.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_pool_pda.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
       // signer_seeds,
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
    //  constraint = mint.mint_authority.unwrap() == mint_authority_pda.key
    /// CHECK: that authority is mint authority
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK: this account
    #[account(mut)]
    pub token_pool_pda: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
    /// CHECK: this account
    #[account(mut)]
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: this account
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in psp account compression program
    #[account(mut, seeds = [b"cpi_authority"], bump, seeds::program = light_system_program::ID,)]
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

pub fn get_token_authority_pda(signer: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    let signer_seed = signer.to_bytes();
    let mint_seed = mint.to_bytes();
    let seeds = &[
        MINT_AUTHORITY_SEED,
        signer_seed.as_slice(),
        mint_seed.as_slice(),
    ];
    Pubkey::find_program_address(seeds, &crate::ID)
}

pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    let seeds = &[POOL_SEED, mint.as_ref()];
    let (address, _) = Pubkey::find_program_address(seeds, &crate::ID);
    address
}

#[cfg(not(target_os = "solana"))]
pub mod mint_sdk {
    use crate::{get_cpi_authority_pda, get_token_authority_pda, get_token_pool_pda};
    use anchor_lang::{system_program, InstructionData, ToAccountMetas};
    use anchor_spl;
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    pub fn create_initialize_mint_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        mint: &Pubkey,
    ) -> Instruction {
        let token_pool_pda = get_token_pool_pda(mint);
        let mint_authority_pda = get_token_authority_pda(authority, mint).0;
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
        let (mint_authority_pda, bump) = get_token_authority_pda(authority, mint);
        let instruction_data = crate::instruction::MintTo {
            amounts,
            public_keys,
            bump,
        };

        let accounts = crate::accounts::MintToInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            mint_authority_pda,
            mint: *mint,
            token_pool_pda,
            token_program: anchor_spl::token::ID,
            light_system_program: light_system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
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

#[test]
fn test_manual_ix_data_serialization_borsh_compat() {
    let pubkeys = vec![Pubkey::new_unique(), Pubkey::new_unique()];
    let amounts = vec![1, 2];
    let mint_pubkey = Pubkey::new_unique();
    let mut output_compressed_accounts =
        vec![OutputCompressedAccountWithPackedContext::default(); pubkeys.len()];
    for (i, (pubkey, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
        let mut token_data_bytes = Vec::with_capacity(mem::size_of::<TokenData>());
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
        use light_hasher::DataHasher;

        let data: CompressedAccountData = CompressedAccountData {
            discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
            data: token_data_bytes,
            data_hash: token_data.hash::<Poseidon>().unwrap(),
        };
        let lamports = 0;

        output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: crate::ID,
                lamports,
                data: Some(data),
                address: None,
            },
            merkle_tree_index: 0,
        };
    }
    let authority_bytes = Pubkey::new_unique().to_bytes();
    let mint_bytes = Pubkey::new_unique().to_bytes();
    let bump = &[255];
    let seeds = [
        MINT_AUTHORITY_SEED,
        authority_bytes.as_slice(),
        mint_bytes.as_slice(),
        bump,
    ];
    let inputs_struct = light_system_program::InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::with_capacity(0),
        output_compressed_accounts: output_compressed_accounts.clone(),
        proof: None,
        new_address_params: Vec::with_capacity(0),
        compression_lamports: None,
        is_compress: false,
        signer_seeds: seeds.iter().map(|seed| seed.to_vec()).collect(),
        cpi_context: None,
    };
    let mut reference = Vec::<u8>::new();
    inputs_struct.serialize(&mut reference).unwrap();
    let mut inputs = Vec::<u8>::new();
    serialize_mint_to_cpi_instruction_data(&mut inputs, output_compressed_accounts, &[&seeds]);

    assert_eq!(inputs.len(), reference.len());
    for (j, i) in inputs.iter().zip(reference.iter()).enumerate() {
        println!("j: {} i: {} {}", j, i.0, i.1);
        assert_eq!(i.0, i.1);
    }
    assert_eq!(inputs, reference);
}
