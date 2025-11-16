use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke, sysvar::clock::Clock},
};
use light_compressed_token_sdk::instructions::{
    create_mint_action_cpi, find_spl_mint_address, MintActionInputs,
};
use light_sdk::{
    compressible::{
        compress_account_on_init::prepare_compressed_account_on_init, CompressibleConfig,
    },
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
};
use light_sdk_types::{
    cpi_accounts::CpiAccountsConfig, cpi_context_write::CpiContextWriteAccounts,
};

use crate::{errors::ErrorCode, instruction_accounts::*, seeds::*, state::*, LIGHT_CPI_SIGNER};
pub fn create_user_record_and_game_session<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateUserRecordAndGameSession<'info>>,
    account_data: AccountCreationData,
    compression_params: CompressionParams,
) -> Result<()> {
    let user_record = &mut ctx.accounts.user_record;
    let game_session = &mut ctx.accounts.game_session;

    // Load your config checked.
    let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

    // Check that rent recipient matches your config.
    if ctx.accounts.rent_sponsor.key() != config.rent_sponsor {
        return Err(ProgramError::from(ErrorCode::RentRecipientMismatch).into());
    }

    // Set your account data.
    user_record.owner = ctx.accounts.user.key();
    user_record.name = account_data.user_name.clone();
    user_record.score = 11;

    game_session.session_id = account_data.session_id;
    game_session.player = ctx.accounts.user.key();
    game_session.game_type = account_data.game_type.clone();
    game_session.start_time = Clock::get()?.unix_timestamp as u64;
    game_session.end_time = None;
    game_session.score = 0;

    // Create CPI accounts from remaining accounts
    let cpi_accounts = CpiAccounts::new_with_config(
        ctx.accounts.user.as_ref(),
        ctx.remaining_accounts,
        CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
    );
    let cpi_context_pubkey = cpi_accounts.cpi_context().unwrap().key();
    let cpi_context_account = cpi_accounts.cpi_context().unwrap();

    // Prepare new address params. One per pda account.
    let user_new_address_params = compression_params
        .user_address_tree_info
        .into_new_address_params_assigned_packed(user_record.key().to_bytes().into(), Some(0));
    let game_new_address_params = compression_params
        .game_address_tree_info
        .into_new_address_params_assigned_packed(game_session.key().to_bytes().into(), Some(1));

    let mut all_compressed_infos = Vec::new();

    // Prepare user record for compression
    let user_record_info = user_record.to_account_info();
    let user_record_data_mut = &mut **user_record;
    let user_compressed_info = prepare_compressed_account_on_init::<UserRecord>(
        &user_record_info,
        user_record_data_mut,
        &config,
        compression_params.user_compressed_address,
        user_new_address_params,
        compression_params.user_output_state_tree_index,
        &cpi_accounts,
        &config.address_space,
        true, // with_data
    )?;

    all_compressed_infos.push(user_compressed_info);

    // Prepare game session for compression
    let game_session_info = game_session.to_account_info();
    let game_session_data_mut = &mut **game_session;
    let game_compressed_info = prepare_compressed_account_on_init::<GameSession>(
        &game_session_info,
        game_session_data_mut,
        &config,
        compression_params.game_compressed_address,
        game_new_address_params,
        compression_params.game_output_state_tree_index,
        &cpi_accounts,
        &config.address_space,
        true, // with_data
    )?;
    all_compressed_infos.push(game_compressed_info);

    let cpi_context_accounts = CpiContextWriteAccounts {
        fee_payer: cpi_accounts.fee_payer(),
        authority: cpi_accounts.authority().unwrap(),
        cpi_context: cpi_context_account,
        cpi_signer: LIGHT_CPI_SIGNER,
    };
    LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, compression_params.proof)
        .with_new_addresses(&[user_new_address_params, game_new_address_params])
        .with_account_infos(&all_compressed_infos)
        .write_to_cpi_context_first()
        .invoke_write_to_cpi_context_first(cpi_context_accounts)?;

    // these are custom seeds of the caller program that are used to derive the program owned onchain tokenb account PDA.
    // dual use: as owner of the compressed token account.
    let mint = find_spl_mint_address(&ctx.accounts.mint_signer.key()).0;
    let (_, token_account_address) = get_ctoken_signer_seeds(&ctx.accounts.user.key(), &mint);

    let actions = vec![
        light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
            recipients: vec![
                light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                    recipient: token_account_address, // TRY: THE DECOMPRESS TOKEN ACCOUNT ADDRES IS THE OWNER OF ITS COMPRESSIBLED VERSION.
                    amount: 1000,                     // Mint the full supply to the user
                },
                light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                    recipient: get_ctoken_signer2_seeds(&ctx.accounts.user.key()).1,
                    amount: 1000,
                },
                light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                    recipient: get_ctoken_signer3_seeds(&ctx.accounts.user.key()).1,
                    amount: 1000,
                },
                light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                    recipient: get_ctoken_signer4_seeds(
                        &ctx.accounts.user.key(),
                        &ctx.accounts.user.key(),
                    )
                    .1, // user as fee_payer
                    amount: 1000,
                },
                light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                    recipient: get_ctoken_signer5_seeds(&ctx.accounts.user.key(), &mint, 42).1, // Fixed index 42
                    amount: 1000,
                },
            ],
            token_account_version: 3,
        },
    ];

    let output_queue = *cpi_accounts.tree_accounts().unwrap()[0].key; // Same tree as PDA
    let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[1].key; // Same tree as PDA

    let mint_action_inputs = MintActionInputs {
        compressed_mint_inputs: compression_params.mint_with_context.clone(),
        mint_seed: ctx.accounts.mint_signer.key(),
        mint_bump: Some(compression_params.mint_bump),
        create_mint: true,
        authority: ctx.accounts.mint_authority.key(),
        payer: ctx.accounts.user.key(),
        proof: compression_params.proof.into(),
        actions,
        input_queue: None, // Not needed for create_mint: true
        output_queue,
        tokens_out_queue: Some(output_queue), // For MintTo actions
        address_tree_pubkey,
        token_pool: None, // Not needed for simple compressed mint creation
    };

    let mint_action_instruction = create_mint_action_cpi(
        mint_action_inputs,
        Some(light_ctoken_types::instructions::mint_action::CpiContext {
            address_tree_pubkey: address_tree_pubkey.to_bytes(),
            set_context: false,
            first_set_context: false,
            in_tree_index: 1, // address tree
            in_queue_index: 0,
            out_queue_index: 0,
            token_out_queue_index: 0,
            assigned_account_index: 2,
            read_only_address_trees: [0; 4],
        }),
        Some(cpi_context_pubkey),
    )
    .unwrap();

    // Get all account infos needed for the mint action
    let mut account_infos = cpi_accounts.to_account_infos();
    account_infos.push(
        ctx.accounts
            .compress_token_program_cpi_authority
            .to_account_info(),
    );
    account_infos.push(ctx.accounts.ctoken_program.to_account_info());
    account_infos.push(ctx.accounts.mint_authority.to_account_info());
    account_infos.push(ctx.accounts.mint_signer.to_account_info());
    account_infos.push(ctx.accounts.user.to_account_info());

    // Invoke the mint action instruction directly
    invoke(&mint_action_instruction, &account_infos)?;

    // at the end of the instruction we always clean up all onchain pdas that we compressed
    user_record.close(ctx.accounts.rent_sponsor.to_account_info())?;
    game_session.close(ctx.accounts.rent_sponsor.to_account_info())?;

    Ok(())
}
