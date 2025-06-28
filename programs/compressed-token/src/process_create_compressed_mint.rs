use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR, create_mint::CompressedMint,
    instructions::create_compressed_mint::CreateCompressedMintInstruction,
    process_transfer::get_cpi_signer_seeds,
};
use anchor_lang::prelude::*;
use light_compressed_account::{
    address::derive_address,
    compressed_account::{CompressedAccount, CompressedAccountData},
    instruction_data::{
        compressed_proof::CompressedProof,
        data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
        invoke_cpi::InstructionDataInvokeCpi,
    },
};

fn execute_cpi_invoke<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedMintInstruction<'info>>,
    inputs_struct: InstructionDataInvokeCpi,
) -> Result<()> {
    let invoking_program = ctx.accounts.self_program.to_account_info();

    let seeds = get_cpi_signer_seeds();
    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.fee_payer.to_account_info(),
        authority: ctx.accounts.cpi_authority_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program,
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: None,
    };

    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    let remaining_accounts = [
        ctx.accounts.address_merkle_tree.to_account_info(),
        ctx.accounts.output_queue.to_account_info(),
    ];

    cpi_ctx.remaining_accounts = remaining_accounts.to_vec();

    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    Ok(())
}

fn create_compressed_mint_account(
    mint_pda: Pubkey,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    address_merkle_tree_key: &Pubkey,
    address_merkle_tree_root_index: u16,
    proof: CompressedProof,
) -> Result<InstructionDataInvokeCpi> {
    // 1. Create CompressedMint struct
    let compressed_mint = CompressedMint {
        spl_mint: mint_pda,
        supply: 0,
        decimals,
        is_decompressed: false,
        mint_authority: Some(mint_authority),
        freeze_authority,
        num_extensions: 0,
    };

    // 2. Serialize the compressed mint data
    let mut compressed_mint_bytes = Vec::new();
    compressed_mint.serialize(&mut compressed_mint_bytes)?;

    // 3. Calculate data hash
    let data_hash = compressed_mint
        .hash()
        .map_err(|_| crate::ErrorCode::HashToFieldError)?;

    // 4. Create NewAddressParams onchain
    let new_address_params = NewAddressParamsPacked {
        seed: mint_pda.to_bytes(),
        address_merkle_tree_account_index: 0,
        address_queue_account_index: 0,
        address_merkle_tree_root_index,
    };

    // 5. Derive compressed account address
    let compressed_account_address = derive_address(
        &new_address_params.seed,
        &address_merkle_tree_key.to_bytes(),
        &crate::ID.to_bytes(),
    );

    // 6. Create compressed account data
    let compressed_account_data = CompressedAccountData {
        discriminator: COMPRESSED_MINT_DISCRIMINATOR,
        data: compressed_mint_bytes,
        data_hash,
    };

    // 7. Create output compressed account
    let output_compressed_account = OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID.into(),
            lamports: 0,
            data: Some(compressed_account_data),
            address: Some(compressed_account_address),
        },
        merkle_tree_index: 1,
    };

    Ok(InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![output_compressed_account],
        proof: Some(proof),
        new_address_params: vec![new_address_params],
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: None,
    })
}

pub fn process_create_compressed_mint<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateCompressedMintInstruction<'info>>,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    proof: CompressedProof,
    mint_bump: u8,
    address_merkle_tree_root_index: u16,
) -> Result<()> {
    // 1. Create mint PDA using provided bump
    let mint_pda = Pubkey::create_program_address(
        &[
            b"compressed_mint",
            ctx.accounts.mint_signer.key().as_ref(),
            &[mint_bump],
        ],
        &crate::ID,
    )
    .map_err(|_| crate::ErrorCode::InvalidTokenPoolPda)?;

    // 2. Create compressed mint account
    let inputs_struct = create_compressed_mint_account(
        mint_pda,
        decimals,
        mint_authority,
        freeze_authority,
        &ctx.accounts.address_merkle_tree.key(),
        address_merkle_tree_root_index,
        proof,
    )?;

    // 3. CPI to light-system-program
    execute_cpi_invoke(&ctx, inputs_struct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_rnd_create_compressed_mint_account() {
        let mut rng = rand::rngs::ThreadRng::default();
        let iter = 1_000;

        for _ in 0..iter {
            // 1. Generate random mint parameters
            let mint_pda = Pubkey::new_unique();
            let decimals = rng.gen_range(0..=18);
            let mint_authority = Pubkey::new_unique();
            let freeze_authority = if rng.gen_bool(0.5) {
                Some(Pubkey::new_unique())
            } else {
                None
            };
            let address_merkle_tree_key = Pubkey::new_unique();
            let address_merkle_tree_root_index = rng.gen_range(0..=u16::MAX);
            let proof = CompressedProof {
                a: [rng.gen(); 32],
                b: [rng.gen(); 64],
                c: [rng.gen(); 32],
            };

            // 2. Create expected compressed mint
            let expected_mint = CompressedMint {
                spl_mint: mint_pda,
                supply: 0,
                decimals,
                is_decompressed: false,
                mint_authority: Some(mint_authority),
                freeze_authority,
                num_extensions: 0,
            };

            let mut expected_mint_bytes = Vec::new();
            expected_mint.serialize(&mut expected_mint_bytes).unwrap();
            let expected_data_hash = expected_mint.hash().unwrap();

            let expected_compressed_account_data = CompressedAccountData {
                discriminator: COMPRESSED_MINT_DISCRIMINATOR,
                data: expected_mint_bytes,
                data_hash: expected_data_hash,
            };

            let expected_new_address_params = NewAddressParamsPacked {
                seed: mint_pda.to_bytes(),
                address_merkle_tree_account_index: 0,
                address_queue_account_index: 0,
                address_merkle_tree_root_index,
            };

            let expected_address = derive_address(
                &expected_new_address_params.seed,
                &address_merkle_tree_key.to_bytes(),
                &crate::ID.to_bytes(),
            );

            let expected_output_account = OutputCompressedAccountWithPackedContext {
                compressed_account: CompressedAccount {
                    owner: crate::ID.into(),
                    lamports: 0,
                    data: Some(expected_compressed_account_data),
                    address: Some(expected_address),
                },
                merkle_tree_index: 1,
            };
            let expected_instruction_data = InstructionDataInvokeCpi {
                relay_fee: None,
                input_compressed_accounts_with_merkle_context: Vec::new(),
                output_compressed_accounts: vec![expected_output_account],
                proof: Some(proof),
                new_address_params: vec![expected_new_address_params],
                compress_or_decompress_lamports: None,
                is_compress: false,
                cpi_context: None,
            };

            // 3. Call function under test
            let result = create_compressed_mint_account(
                mint_pda,
                decimals,
                mint_authority,
                freeze_authority,
                &address_merkle_tree_key,
                address_merkle_tree_root_index,
                proof,
            );

            // 4. Assert complete InstructionDataInvokeCpi struct
            assert!(result.is_ok());
            let actual_instruction_data = result.unwrap();
            assert_eq!(actual_instruction_data, expected_instruction_data);
        }
    }
}
