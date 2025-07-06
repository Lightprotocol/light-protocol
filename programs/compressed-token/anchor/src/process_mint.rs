use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use light_compressed_account::{
    compressed_account::PackedCompressedAccountWithMerkleContext,
    instruction_data::{
        compressed_proof::CompressedProof, data::OutputCompressedAccountWithPackedContext,
    },
    pubkey::AsPubkey,
};
use light_system_program::program::LightSystemProgram;
use light_zero_copy::num_trait::ZeroCopyNumTrait;
#[cfg(target_os = "solana")]
use {
    crate::{
        check_spl_token_pool_derivation_with_index,
        process_transfer::{create_output_compressed_accounts, get_cpi_signer_seeds},
        spl_compression::spl_token_transfer,
    },
    light_compressed_account::hash_to_bn254_field_size_be,
    light_heap::{bench_sbf_end, bench_sbf_start, GLOBAL_ALLOCATOR},
};

use crate::{check_spl_token_pool_derivation, program::LightCompressedToken};

pub const COMPRESS: bool = false;
pub const MINT_TO: bool = true;

/// Mints tokens from an spl token mint to a list of compressed accounts and
/// stores minted tokens in spl token pool account.
///
/// Steps:
/// 1. Allocate memory for cpi instruction data. We allocate memory in the
///    beginning so that we can free all memory of the allocation prior to the
///    cpi in cpi_execute_compressed_transaction_mint_to.
/// 2. Mint SPL tokens to pool account.
/// 3. Create output compressed accounts, one for every pubkey and amount pair.
/// 4. Serialize cpi instruction data and free memory up to
///    pre_compressed_acounts_pos.
/// 5. Invoke system program to execute the compressed transaction.
#[allow(unused_variables)]
pub fn process_mint_to_or_compress<'info, const IS_MINT_TO: bool>(
    ctx: Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    recipient_pubkeys: &[impl AsPubkey],
    amounts: &[impl ZeroCopyNumTrait],
    lamports: Option<u64>,
    index: Option<u8>,
    bump: Option<u8>,
) -> Result<()> {
    if recipient_pubkeys.len() != amounts.len() {
        msg!(
            "recipient_pubkeys.len() {} !=  {} amounts.len()",
            recipient_pubkeys.len(),
            amounts.len()
        );
        return err!(crate::ErrorCode::PublicKeyAmountMissmatch);
    } else if recipient_pubkeys.is_empty() {
        msg!("recipient_pubkeys is empty");
        return err!(crate::ErrorCode::NoInputsProvided);
    }

    #[cfg(target_os = "solana")]
    {
        let option_compression_lamports = if lamports.unwrap_or(0) == 0 { 0 } else { 8 };

        let inputs_len =
            1 + 4 + 4 + 4 + amounts.len() * 162 + 1 + 1 + 1 + 1 + option_compression_lamports;
        // inputs_len =
        //   1                          Option<Proof>
        // + 4                          Vec::new()
        // + 4                          Vec::new()
        // + 4 + amounts.len() * 162    Vec<OutputCompressedAccountWithPackedContext>
        // + 1                          Option<relay_fee>
        // + 1 + 8                         Option<compression_lamports>
        // + 1                          is_compress
        // + 1                          Option<CpiContextAccount>
        let mut inputs = Vec::<u8>::with_capacity(inputs_len);
        // # SAFETY: the inputs vector needs to be allocated before this point.
        // All heap memory from this point on is freed prior to the cpi call.
        let pre_compressed_acounts_pos = GLOBAL_ALLOCATOR.get_heap_pos();
        bench_sbf_start!("tm_mint_spl_to_pool_pda");

        let (mint, compressed_mint_update_data) = if IS_MINT_TO {
            // EXISTING SPL MINT PATH
            mint_spl_to_pool_pda(&ctx, &amounts)?;
            (
                ctx.accounts.mint.as_ref().unwrap().key(),
                None::<CompressedProof>,
            )
        } else {
            // EXISTING BATCH COMPRESS PATH
            let mut amount = 0u64;
            for a in amounts {
                amount += (*a).into();
            }
            // # SAFETY: The index is always provided by batch compress.
            let index = index.unwrap();
            let from_account_info = &ctx.remaining_accounts[0];

            let mint =
                Pubkey::new_from_array(from_account_info.data.borrow()[..32].try_into().unwrap());
            check_spl_token_pool_derivation_with_index(
                &ctx.accounts.token_pool_pda.key(),
                &mint,
                index,
                bump,
            )?;
            spl_token_transfer(
                from_account_info.to_account_info(),
                ctx.accounts.token_pool_pda.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                amount,
            )?;
            (mint, None)
        };
        let hashed_mint = hash_to_bn254_field_size_be(mint.as_ref());

        let mut output_compressed_accounts =
            vec![OutputCompressedAccountWithPackedContext::default(); recipient_pubkeys.len()];
        let lamports_vec = lamports.map(|_| vec![lamports; amounts.len()]);
        create_output_compressed_accounts(
            &mut output_compressed_accounts,
            mint,
            recipient_pubkeys,
            None,
            None,
            &amounts,
            lamports_vec,
            &hashed_mint,
            // We ensure that the Merkle tree account is the first
            // remaining account in the cpi to the system program.
            &vec![0; amounts.len()],
            &[ctx.accounts.merkle_tree.to_account_info()],
        )?;
        bench_sbf_end!("tm_output_compressed_accounts");

        // Create compressed mint update data if needed
        let (input_compressed_accounts, proof) = (vec![], None);
        // Execute single CPI call with updated serialization
        cpi_execute_compressed_transaction_mint_to::<IS_MINT_TO>(
            &ctx,
            input_compressed_accounts.as_slice(),
            output_compressed_accounts,
            &mut inputs,
            proof,
            pre_compressed_acounts_pos,
        )?;

        // # SAFETY: the inputs vector needs to be allocated before this point.
        // This error should never be triggered.
        if inputs.len() != inputs_len {
            msg!(
                "Used memory {} is unequal allocated {} memory",
                inputs.len(),
                inputs_len
            );
            return err!(crate::ErrorCode::HeapMemoryCheckFailed);
        }
    }
    Ok(())
}

// #[cfg(target_os = "solana")]
// fn mint_with_compressed_mint<'info>(
//     ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
//     amounts: &[impl ZeroCopyNumTrait],
//     compressed_inputs: &CompressedMintInputs,
// ) -> Result<(
//     Pubkey,
//     Option<(
//         PackedCompressedAccountWithMerkleContext,
//         OutputCompressedAccountWithPackedContext,
//     )>,
// )> {
//     let mint_pubkey = ctx
//         .accounts
//         .mint
//         .as_ref()
//         .ok_or(crate::ErrorCode::MintIsNone)?
//         .key();
//     let compressed_mint: CompressedMint = CompressedMint {
//         mint_authority: Some(ctx.accounts.authority.key()),
//         freeze_authority: if compressed_inputs
//             .compressed_mint_input
//             .freeze_authority_is_set
//         {
//             Some(compressed_inputs.compressed_mint_input.freeze_authority)
//         } else {
//             None
//         },
//         spl_mint: mint_pubkey,
//         supply: compressed_inputs.compressed_mint_input.supply,
//         decimals: compressed_inputs.compressed_mint_input.decimals,
//         is_decompressed: compressed_inputs.compressed_mint_input.is_decompressed,
//         num_extensions: compressed_inputs.compressed_mint_input.num_extensions,
//     };
//     // Create input compressed account for existing mint
//     let input_compressed_account = PackedCompressedAccountWithMerkleContext {
//         compressed_account: CompressedAccount {
//             owner: crate::ID.into(),
//             lamports: 0,
//             address: Some(compressed_inputs.address),
//             data: Some(CompressedAccountData {
//                 discriminator: COMPRESSED_MINT_DISCRIMINATOR,
//                 data: Vec::new(),
//                 // TODO: hash with hashed inputs
//                 data_hash: compressed_mint.hash().map_err(ProgramError::from)?,
//             }),
//         },
//         merkle_context: compressed_inputs.merkle_context,
//         root_index: compressed_inputs.root_index,
//         read_only: false,
//     };
//     let total_mint_amount: u64 = amounts.iter().map(|a| (*a).into()).sum();
//     let updated_compressed_mint = if compressed_mint.is_decompressed {
//         // SYNC WITH SPL MINT (SPL is source of truth)

//         // Mint to SPL token pool as normal
//         mint_spl_to_pool_pda(ctx, amounts)?;

//         // Read updated SPL mint state for sync
//         let spl_mint_info = ctx
//             .accounts
//             .mint
//             .as_ref()
//             .ok_or(crate::ErrorCode::MintIsNone)?;
//         let spl_mint_data = spl_mint_info.data.borrow();
//         let spl_mint = anchor_spl::token::Mint::try_deserialize(&mut &spl_mint_data[..])?;

//         // Create updated compressed mint with synced state
//         let mut updated_compressed_mint = compressed_mint;
//         updated_compressed_mint.supply = spl_mint.supply;
//         updated_compressed_mint
//     } else {
//         // PURE COMPRESSED MINT - no SPL backing
//         let mut updated_compressed_mint = compressed_mint;
//         updated_compressed_mint.supply = updated_compressed_mint
//             .supply
//             .checked_add(total_mint_amount)
//             .ok_or(crate::ErrorCode::MintTooLarge)?;
//         updated_compressed_mint
//     };
//     let updated_data_hash = updated_compressed_mint
//         .hash()
//         .map_err(|_| crate::ErrorCode::HashToFieldError)?;

//     let mut updated_mint_bytes = Vec::new();
//     updated_compressed_mint.serialize(&mut updated_mint_bytes)?;

//     let updated_compressed_account_data = CompressedAccountData {
//         discriminator: COMPRESSED_MINT_DISCRIMINATOR,
//         data: updated_mint_bytes,
//         data_hash: updated_data_hash,
//     };

//     let output_compressed_mint_account = OutputCompressedAccountWithPackedContext {
//         compressed_account: CompressedAccount {
//             owner: crate::ID.into(),
//             lamports: 0,
//             address: Some(compressed_inputs.address),
//             data: Some(updated_compressed_account_data),
//         },
//         merkle_tree_index: compressed_inputs.output_merkle_tree_index,
//     };

//     Ok((
//         mint_pubkey,
//         Some((input_compressed_account, output_compressed_mint_account)),
//     ))
// }

#[cfg(target_os = "solana")]
#[inline(never)]
pub fn cpi_execute_compressed_transaction_mint_to<'info, const IS_MINT_TO: bool>(
    ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
    mint_to_compressed_account: &[PackedCompressedAccountWithMerkleContext],
    output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
    inputs: &mut Vec<u8>,
    proof: Option<CompressedProof>,
    pre_compressed_acounts_pos: usize,
) -> Result<()> {
    bench_sbf_start!("tm_cpi");

    let signer_seeds = get_cpi_signer_seeds();

    // 4300 CU for 10 accounts
    // 6700 CU for 20 accounts
    // 7,978 CU for 25 accounts
    serialize_mint_to_cpi_instruction_data_with_inputs(
        inputs,
        mint_to_compressed_account,
        &output_compressed_accounts,
        proof,
    );

    GLOBAL_ALLOCATOR.free_heap(pre_compressed_acounts_pos)?;

    use anchor_lang::InstructionData;

    // 826 CU
    let instructiondata = light_system_program::instruction::InvokeCpi {
        inputs: inputs.to_owned(),
    };
    let (sol_pool_pda, is_writable) = if let Some(pool_pda) = ctx.accounts.sol_pool_pda.as_ref() {
        // Account is some
        (pool_pda.to_account_info(), true)
    } else {
        // Account is None
        (ctx.accounts.light_system_program.to_account_info(), false)
    };

    // 1300 CU
    let mut account_infos = vec![
        ctx.accounts.fee_payer.to_account_info(),
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts.registered_program_pda.to_account_info(),
        ctx.accounts.noop_program.to_account_info(),
        ctx.accounts.account_compression_authority.to_account_info(),
        ctx.accounts.account_compression_program.to_account_info(),
        ctx.accounts.self_program.to_account_info(),
        sol_pool_pda,
        ctx.accounts.light_system_program.to_account_info(), // none compression_recipient
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.light_system_program.to_account_info(), // none cpi_context_account
        ctx.accounts.merkle_tree.to_account_info(),          // first remaining account
    ];
    // Don't add for batch compress
    if IS_MINT_TO {
        // Add remaining account metas (compressed mint merkle tree should be writable)
        for remaining in ctx.remaining_accounts {
            account_infos.push(remaining.to_account_info());
        }
    }

    // account_metas take 1k cu
    let mut accounts = vec![
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
            is_writable,
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
    // Don't add for batch compress
    if IS_MINT_TO {
        // Add remaining account metas (compressed mint merkle tree should be writable)
        for remaining in &account_infos[12..] {
            msg!(" remaining.key() {:?}", remaining.key());
            accounts.push(AccountMeta {
                pubkey: remaining.key(),
                is_signer: false,
                is_writable: remaining.is_writable,
            });
        }
    }
    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: light_system_program::ID,
        accounts,
        data: instructiondata.data(),
    };

    bench_sbf_end!("tm_cpi");
    bench_sbf_start!("tm_invoke");
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        account_infos.as_slice(),
        &[&signer_seeds[..]],
    )?;
    bench_sbf_end!("tm_invoke");
    Ok(())
}

#[inline(never)]
pub fn serialize_mint_to_cpi_instruction_data_with_inputs(
    inputs: &mut Vec<u8>,
    input_compressed_accounts: &[PackedCompressedAccountWithMerkleContext],
    output_compressed_accounts: &[OutputCompressedAccountWithPackedContext],
    proof: Option<CompressedProof>,
) {
    // proof (option)
    if let Some(proof) = proof {
        inputs.extend_from_slice(&[1u8]); // Some
        proof.serialize(inputs).unwrap();
    } else {
        inputs.extend_from_slice(&[0u8]); // None
    }

    // new_address_params (empty for mint operations)
    inputs.extend_from_slice(&[0u8; 4]);

    // input_compressed_accounts_with_merkle_context
    let input_len = input_compressed_accounts.len();
    inputs.extend_from_slice(&[(input_len as u8), 0, 0, 0]);
    for input_account in input_compressed_accounts.iter() {
        input_account.serialize(inputs).unwrap();
    }

    // output_compressed_accounts
    let output_len = output_compressed_accounts.len();
    inputs.extend_from_slice(&[(output_len as u8), 0, 0, 0]);
    let mut sum_lamports = 0u64;
    for compressed_account in output_compressed_accounts.iter() {
        compressed_account.serialize(inputs).unwrap();
        sum_lamports = sum_lamports
            .checked_add(compressed_account.compressed_account.lamports)
            .unwrap();
    }

    // None relay_fee
    inputs.extend_from_slice(&[0u8; 1]);

    if sum_lamports != 0 {
        inputs.extend_from_slice(&[1u8; 1]);
        inputs.extend_from_slice(&sum_lamports.to_le_bytes());
        inputs.extend_from_slice(&[1u8; 1]); // is compress bool = true
    } else {
        inputs.extend_from_slice(&[0u8; 2]); // None compression lamports, is compress bool = false
    }

    // None compressed_cpi_context
    inputs.extend_from_slice(&[0u8]);
}

// #[cfg(target_os = "solana")]
// fn create_compressed_mint_update_accounts(
//     updated_compressed_mint: CompressedMint,
//     compressed_inputs: CompressedMintInputs,
// ) -> Result<(
//     PackedCompressedAccountWithMerkleContext,
//     OutputCompressedAccountWithPackedContext,
// )> {
//     // Create input compressed account for existing mint
//     let input_compressed_account = PackedCompressedAccountWithMerkleContext {
//         compressed_account: CompressedAccount {
//             owner: crate::ID.into(),
//             lamports: 0,
//             address: Some(compressed_inputs.address),
//             data: Some(CompressedAccountData {
//                 discriminator: COMPRESSED_MINT_DISCRIMINATOR,
//                 data: Vec::new(),
//                 data_hash: updated_compressed_mint.hash().map_err(ProgramError::from)?,
//             }),
//         },
//         merkle_context: compressed_inputs.merkle_context,
//         root_index: compressed_inputs.root_index,
//         read_only: false,
//     };
//     msg!(
//         "compressed_inputs.merkle_context: {:?}",
//         compressed_inputs.merkle_context
//     );

//     // Create output compressed account for updated mint
//     let mut updated_mint_bytes = Vec::new();
//     updated_compressed_mint.serialize(&mut updated_mint_bytes)?;
//     let updated_data_hash = updated_compressed_mint
//         .hash()
//         .map_err(|_| crate::ErrorCode::HashToFieldError)?;

//     let updated_compressed_account_data = CompressedAccountData {
//         discriminator: COMPRESSED_MINT_DISCRIMINATOR,
//         data: updated_mint_bytes,
//         data_hash: updated_data_hash,
//     };

//     let output_compressed_mint_account = OutputCompressedAccountWithPackedContext {
//         compressed_account: CompressedAccount {
//             owner: crate::ID.into(),
//             lamports: 0,
//             address: Some(compressed_inputs.address),
//             data: Some(updated_compressed_account_data),
//         },
//         merkle_tree_index: compressed_inputs.output_merkle_tree_index,
//     };
//     msg!(
//         "compressed_inputs.output_merkle_tree_index {}",
//         compressed_inputs.output_merkle_tree_index
//     );

//     Ok((input_compressed_account, output_compressed_mint_account))
// }

// #[cfg(target_os = "solana")]
// #[inline(never)]
// pub fn cpi_execute_compressed_transaction_mint_to_with_inputs<'info>(
//     ctx: &Context<'_, '_, '_, 'info, MintToInstruction<'info>>,
//     input_compressed_accounts: Vec<PackedCompressedAccountWithMerkleContext>,
//     output_compressed_accounts: Vec<OutputCompressedAccountWithPackedContext>,
//     proof: Option<CompressedProof>,
//     inputs: &mut Vec<u8>,
//     pre_compressed_accounts_pos: usize,
// ) -> Result<()> {
//     bench_sbf_start!("tm_cpi_mint_update");

//     let signer_seeds = get_cpi_signer_seeds();

//     // Serialize CPI instruction data with inputs
//     serialize_mint_to_cpi_instruction_data_with_inputs(
//         inputs,
//         &input_compressed_accounts,
//         &output_compressed_accounts,
//         proof,
//     );

//     GLOBAL_ALLOCATOR.free_heap(pre_compressed_accounts_pos)?;

//     use anchor_lang::InstructionData;

//     let instructiondata = light_system_program::instruction::InvokeCpi {
//         inputs: inputs.to_owned(),
//     };

//     let (sol_pool_pda, is_writable) = if let Some(pool_pda) = ctx.accounts.sol_pool_pda.as_ref() {
//         (pool_pda.to_account_info(), true)
//     } else {
//         (ctx.accounts.light_system_program.to_account_info(), false)
//     };

//     // Build account infos including both output merkle tree and remaining accounts (compressed mint merkle tree)
//     let mut account_infos = vec![
//         ctx.accounts.fee_payer.to_account_info(),
//         ctx.accounts.cpi_authority_pda.to_account_info(),
//         ctx.accounts.registered_program_pda.to_account_info(),
//         ctx.accounts.noop_program.to_account_info(),
//         ctx.accounts.account_compression_authority.to_account_info(),
//         ctx.accounts.account_compression_program.to_account_info(),
//         ctx.accounts.self_program.to_account_info(),
//         sol_pool_pda,
//         ctx.accounts.light_system_program.to_account_info(),
//         ctx.accounts.system_program.to_account_info(),
//         ctx.accounts.light_system_program.to_account_info(), // cpi_context_account placeholder
//         ctx.accounts.merkle_tree.to_account_info(),          // output merkle tree
//     ];

//     // Add remaining accounts (compressed mint merkle tree, etc.)
//     account_infos.extend_from_slice(ctx.remaining_accounts);

//     // Build account metas
//     let mut accounts = vec![
//         AccountMeta::new(account_infos[0].key(), true), // fee_payer
//         AccountMeta::new_readonly(account_infos[1].key(), true), // cpi_authority_pda (signer)
//         AccountMeta::new_readonly(account_infos[2].key(), false), // registered_program_pda
//         AccountMeta::new_readonly(account_infos[3].key(), false), // noop_program
//         AccountMeta::new_readonly(account_infos[4].key(), false), // account_compression_authority
//         AccountMeta::new_readonly(account_infos[5].key(), false), // account_compression_program
//         AccountMeta::new_readonly(account_infos[6].key(), false), // self_program
//         AccountMeta::new(account_infos[7].key(), is_writable), // sol_pool_pda
//         AccountMeta::new_readonly(account_infos[8].key(), false), // decompression_recipient placeholder
//         AccountMeta::new_readonly(account_infos[9].key(), false), // system_program
//         AccountMeta::new_readonly(account_infos[10].key(), false), // cpi_context_account placeholder
//         AccountMeta::new(account_infos[11].key(), false),          // output merkle tree (writable)
//     ];

//     // Add remaining account metas (compressed mint merkle tree should be writable)
//     for remaining in &account_infos[12..] {
//         accounts.push(AccountMeta::new(remaining.key(), false));
//     }

//     let instruction = anchor_lang::solana_program::instruction::Instruction {
//         program_id: light_system_program::ID,
//         accounts,
//         data: instructiondata.data(),
//     };

//     bench_sbf_end!("tm_cpi_mint_update");
//     bench_sbf_start!("tm_invoke_mint_update");
//     anchor_lang::solana_program::program::invoke_signed(
//         &instruction,
//         account_infos.as_slice(),
//         &[&signer_seeds[..]],
//     )?;
//     bench_sbf_end!("tm_invoke_mint_update");
//     Ok(())
// }

#[inline(never)]
pub fn mint_spl_to_pool_pda(
    ctx: &Context<MintToInstruction>,
    amounts: &[impl ZeroCopyNumTrait],
) -> Result<()> {
    check_spl_token_pool_derivation(
        &ctx.accounts.token_pool_pda.key(),
        &ctx.accounts.mint.as_ref().unwrap().key(),
    )?;
    let mut mint_amount: u64 = 0;
    for amount in amounts.iter() {
        mint_amount = mint_amount
            .checked_add((*amount).into())
            .ok_or(crate::ErrorCode::MintTooLarge)?;
    }

    let pre_token_balance = TokenAccount::try_deserialize(
        &mut &ctx.accounts.token_pool_pda.to_account_info().data.borrow()[..],
    )?
    .amount;
    let cpi_accounts = anchor_spl::token_interface::MintTo {
        mint: ctx.accounts.mint.as_ref().unwrap().to_account_info(),
        to: ctx.accounts.token_pool_pda.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    anchor_spl::token_interface::mint_to(cpi_ctx, mint_amount)?;

    let post_token_balance = TokenAccount::try_deserialize(
        &mut &ctx.accounts.token_pool_pda.to_account_info().data.borrow()[..],
    )?
    .amount;
    // Guard against unexpected behavior of the SPL token program.
    if post_token_balance != pre_token_balance + mint_amount {
        msg!(
            "post_token_balance {} != pre_token_balance {} + mint_amount {}",
            post_token_balance,
            pre_token_balance,
            mint_amount
        );
        return err!(crate::ErrorCode::SplTokenSupplyMismatch);
    }
    Ok(())
}

#[derive(Accounts)]
pub struct MintToInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: is checked by mint account macro.
    pub authority: Signer<'info>,
    /// CHECK: checked implicitly by signing the cpi
    pub cpi_authority_pda: UncheckedAccount<'info>,
    /// CHECK: implicitly by invoking spl token program
    #[account(mut)]
    pub mint: Option<UncheckedAccount<'info>>,
    /// CHECK: with check_spl_token_pool_derivation().
    #[account(mut)]
    pub token_pool_pda: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    /// CHECK: (different program) checked in account compression program
    pub registered_program_pda: UncheckedAccount<'info>,
    /// CHECK: (different program) checked in system and account compression
    /// programs
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: checked implicitly by signing the cpi in system program
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: this account in account compression program
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (different program) will be checked by the system program
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,
    /// CHECK: (different program) will be checked by the system program
    pub self_program: Program<'info, LightCompressedToken>,
    pub system_program: Program<'info, System>,
    /// CHECK: (different program) will be checked by the system program
    #[account(mut)]
    pub sol_pool_pda: Option<AccountInfo<'info>>,
}

#[cfg(not(target_os = "solana"))]
pub mod mint_sdk {
    use anchor_lang::{system_program, InstructionData, ToAccountMetas};
    use light_system_program::utils::get_sol_pool_pda;
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::{
        get_token_pool_pda, get_token_pool_pda_with_index, process_transfer::get_cpi_authority_pda,
    };

    pub fn create_create_token_pool_instruction(
        fee_payer: &Pubkey,
        mint: &Pubkey,
        is_token_22: bool,
    ) -> Instruction {
        let token_pool_pda = get_token_pool_pda(mint);
        let instruction_data = crate::instruction::CreateTokenPool {};

        let token_program: Pubkey = if is_token_22 {
            anchor_spl::token_2022::ID
        } else {
            anchor_spl::token::ID
        };
        let accounts = crate::accounts::CreateTokenPoolInstruction {
            fee_payer: *fee_payer,
            token_pool_pda,
            system_program: system_program::ID,
            mint: *mint,
            token_program,
            cpi_authority_pda: get_cpi_authority_pda().0,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }

    pub fn create_add_token_pool_instruction(
        fee_payer: &Pubkey,
        mint: &Pubkey,
        token_pool_index: u8,
        is_token_22: bool,
    ) -> Instruction {
        let token_pool_pda = get_token_pool_pda_with_index(mint, token_pool_index);
        let existing_token_pool_pda =
            get_token_pool_pda_with_index(mint, token_pool_index.saturating_sub(1));
        let instruction_data = crate::instruction::AddTokenPool { token_pool_index };

        let token_program: Pubkey = if is_token_22 {
            anchor_spl::token_2022::ID
        } else {
            anchor_spl::token::ID
        };
        let accounts = crate::accounts::AddTokenPoolInstruction {
            fee_payer: *fee_payer,
            token_pool_pda,
            system_program: system_program::ID,
            mint: *mint,
            token_program,
            cpi_authority_pda: get_cpi_authority_pda().0,
            existing_token_pool_pda,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_mint_to_instruction(
        fee_payer: &Pubkey,
        authority: &Pubkey,
        mint: &Pubkey,
        merkle_tree: &Pubkey,
        amounts: Vec<u64>,
        public_keys: Vec<Pubkey>,
        lamports: Option<u64>,
        token_2022: bool,
        token_pool_index: u8,
    ) -> Instruction {
        let token_pool_pda = get_token_pool_pda_with_index(mint, token_pool_index);

        let instruction_data = crate::instruction::MintTo {
            amounts,
            public_keys,
            lamports,
        };
        let sol_pool_pda = if lamports.is_some() {
            Some(get_sol_pool_pda())
        } else {
            None
        };
        let token_program = if token_2022 {
            anchor_spl::token_2022::ID
        } else {
            anchor_spl::token::ID
        };

        let accounts = crate::accounts::MintToInstruction {
            fee_payer: *fee_payer,
            authority: *authority,
            cpi_authority_pda: get_cpi_authority_pda().0,
            mint: Some(*mint),
            token_pool_pda,
            token_program,
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
            sol_pool_pda,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }
}

#[cfg(test)]
mod test {
    use light_compressed_account::{
        compressed_account::{CompressedAccount, CompressedAccountData},
        instruction_data::{
            data::OutputCompressedAccountWithPackedContext, invoke_cpi::InstructionDataInvokeCpi,
        },
    };

    use super::*;
    use crate::{
        constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
        token_data::{AccountState, TokenData},
    };

    #[test]
    fn test_manual_ix_data_serialization_borsh_compat() {
        let pubkeys = [Pubkey::new_unique(), Pubkey::new_unique()];
        let amounts = [1, 2];
        let mint_pubkey = Pubkey::new_unique();
        let mut output_compressed_accounts =
            vec![OutputCompressedAccountWithPackedContext::default(); pubkeys.len()];
        for (i, (pubkey, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
            let mut token_data_bytes = Vec::with_capacity(std::mem::size_of::<TokenData>());
            let token_data = TokenData {
                mint: mint_pubkey,
                owner: *pubkey,
                amount: *amount,
                delegate: None,
                state: AccountState::Initialized,
                tlv: None,
            };

            token_data.serialize(&mut token_data_bytes).unwrap();

            let data = CompressedAccountData {
                discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                data: token_data_bytes,
                data_hash: token_data.hash_legacy().unwrap(),
            };
            let lamports = 0;

            output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: crate::ID.into(),
                    lamports,
                    data: Some(data),
                    address: None,
                },
                merkle_tree_index: 0,
            };
        }

        let mut inputs = Vec::<u8>::new();
        serialize_mint_to_cpi_instruction_data_with_inputs(
            &mut inputs,
            &[],
            &output_compressed_accounts,
            None,
        );
        let inputs_struct = InstructionDataInvokeCpi {
            relay_fee: None,
            input_compressed_accounts_with_merkle_context: Vec::with_capacity(0),
            output_compressed_accounts: output_compressed_accounts.clone(),
            proof: None,
            new_address_params: Vec::with_capacity(0),
            compress_or_decompress_lamports: None,
            is_compress: false,
            cpi_context: None,
        };
        let mut reference = Vec::<u8>::new();
        inputs_struct.serialize(&mut reference).unwrap();

        assert_eq!(inputs.len(), reference.len());
        for (j, i) in inputs.iter().zip(reference.iter()).enumerate() {
            println!("j: {} i: {} {}", j, i.0, i.1);
            assert_eq!(i.0, i.1);
        }
        assert_eq!(inputs, reference);
    }

    #[test]
    fn test_manual_ix_data_serialization_borsh_compat_random() {
        use rand::Rng;

        for _ in 0..10000 {
            let mut rng = rand::thread_rng();
            let pubkeys = [Pubkey::new_unique(), Pubkey::new_unique()];
            let amounts = [rng.gen_range(0..1_000_000_000_000), rng.gen_range(1..100)];
            let mint_pubkey = Pubkey::new_unique();
            let mut output_compressed_accounts =
                vec![OutputCompressedAccountWithPackedContext::default(); pubkeys.len()];
            for (i, (pubkey, amount)) in pubkeys.iter().zip(amounts.iter()).enumerate() {
                let mut token_data_bytes = Vec::with_capacity(std::mem::size_of::<TokenData>());
                let token_data = TokenData {
                    mint: mint_pubkey,
                    owner: *pubkey,
                    amount: *amount,
                    delegate: None,
                    state: AccountState::Initialized,
                    tlv: None,
                };

                token_data.serialize(&mut token_data_bytes).unwrap();

                let data = CompressedAccountData {
                    discriminator: TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                    data: token_data_bytes,
                    data_hash: token_data.hash_legacy().unwrap(),
                };
                let lamports = rng.gen_range(0..1_000_000_000_000);

                output_compressed_accounts[i] = OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner: crate::ID.into(),
                        lamports,
                        data: Some(data),
                        address: None,
                    },
                    merkle_tree_index: 0,
                };
            }

            // Randomly test with or without compressed mint inputs
            let (input_compressed_accounts, expected_inputs, proof) = if rng.gen_bool(0.5) {
                // Test with compressed mint inputs (50% chance)
                let input_mint_account = PackedCompressedAccountWithMerkleContext {
                    compressed_account: CompressedAccount {
                        owner: crate::ID.into(),
                        lamports: 0,
                        address: Some([rng.gen::<u8>(); 32]),
                        data: Some(CompressedAccountData {
                            discriminator: crate::constants::COMPRESSED_MINT_DISCRIMINATOR,
                            data: vec![rng.gen::<u8>(); 32],
                            data_hash: [rng.gen::<u8>(); 32],
                        }),
                    },
                    merkle_context: PackedMerkleContext {
                        merkle_tree_pubkey_index: rng.gen_range(0..10),
                        queue_pubkey_index: rng.gen_range(0..10),
                        leaf_index: rng.gen_range(0..1000),
                        prove_by_index: rng.gen_bool(0.5),
                    },
                    root_index: rng.gen_range(0..100),
                    read_only: false,
                };

                let proof = if rng.gen_bool(0.3) {
                    Some(CompressedProof {
                        a: [rng.gen::<u8>(); 32],
                        b: [rng.gen::<u8>(); 64],
                        c: [rng.gen::<u8>(); 32],
                    })
                } else {
                    None
                };

                (
                    vec![input_mint_account.clone()],
                    vec![input_mint_account],
                    proof,
                )
            } else {
                // Test without compressed mint inputs (50% chance)
                (Vec::new(), Vec::new(), None)
            };

            let mut inputs = Vec::<u8>::new();
            serialize_mint_to_cpi_instruction_data_with_inputs(
                &mut inputs,
                &input_compressed_accounts,
                &output_compressed_accounts,
                proof,
            );
            let sum = output_compressed_accounts
                .iter()
                .map(|x| x.compressed_account.lamports)
                .sum::<u64>();
            let inputs_struct = InstructionDataInvokeCpi {
                relay_fee: None,
                input_compressed_accounts_with_merkle_context: expected_inputs,
                output_compressed_accounts: output_compressed_accounts.clone(),
                proof,
                new_address_params: Vec::with_capacity(0),
                compress_or_decompress_lamports: Some(sum),
                is_compress: true,
                cpi_context: None,
            };
            let mut reference = Vec::<u8>::new();
            inputs_struct.serialize(&mut reference).unwrap();

            assert_eq!(inputs.len(), reference.len());
            for i in inputs.iter().zip(reference.iter()) {
                assert_eq!(i.0, i.1);
            }
            assert_eq!(inputs, reference);
        }
    }
}
