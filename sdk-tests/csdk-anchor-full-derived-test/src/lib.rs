#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::derive_light_cpi_signer;
use light_sdk_macros::add_compressible_instructions;
use light_sdk_types::CpiSigner;

pub mod errors;
pub mod instruction_accounts;
pub mod state;

pub use instruction_accounts::*;
pub use state::{
    AccountCreationData, CompressionParams, GameSession, PackedGameSession,
    PackedPlaceholderRecord, PackedUserRecord, PlaceholderRecord, UserRecord,
};

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

#[add_compressible_instructions(
    // PDA account types with seed specifications
    UserRecord = ("user_record", data.owner),
    GameSession = ("game_session", data.session_id.to_le_bytes()),
    PlaceholderRecord = ("placeholder_record", data.placeholder_id.to_le_bytes()),
    // Token variant (CToken account) with authority for compression signing
    CTokenSigner = (is_token, "ctoken_signer", ctx.fee_payer, ctx.mint, authority = LIGHT_CPI_SIGNER),
    // Instruction data fields used in seed expressions aboved
    owner = Pubkey,
    session_id = u64,
    placeholder_id = u64,
)]
#[program]
pub mod csdk_anchor_full_derived_test {
    use anchor_lang::solana_program::{program::invoke, sysvar::clock::Clock};
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

    use super::*;
    use crate::{
        errors::ErrorCode,
        state::{GameSession, UserRecord},
        LIGHT_CPI_SIGNER,
    };

    pub fn create_user_record_and_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateUserRecordAndGameSession<'info>>,
        account_data: AccountCreationData,
        compression_params: CompressionParams,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;
        let game_session = &mut ctx.accounts.game_session;

        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        if ctx.accounts.rent_sponsor.key() != config.rent_sponsor {
            return Err(ErrorCode::RentRecipientMismatch.into());
        }

        user_record.owner = ctx.accounts.user.key();
        user_record.name = account_data.user_name.clone();
        user_record.score = 11;

        game_session.session_id = account_data.session_id;
        game_session.player = ctx.accounts.user.key();
        game_session.game_type = account_data.game_type.clone();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        let cpi_accounts = CpiAccounts::new_with_config(
            ctx.accounts.user.as_ref(),
            ctx.remaining_accounts,
            CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
        );
        let cpi_context_pubkey = cpi_accounts.cpi_context().unwrap().key();
        let cpi_context_account = cpi_accounts.cpi_context().unwrap();

        let user_new_address_params = compression_params
            .user_address_tree_info
            .into_new_address_params_assigned_packed(user_record.key().to_bytes().into(), Some(0));
        let game_new_address_params = compression_params
            .game_address_tree_info
            .into_new_address_params_assigned_packed(game_session.key().to_bytes().into(), Some(1));

        let mut all_compressed_infos = Vec::new();

        let user_record_info = user_record.to_account_info();
        let user_record_data_mut = &mut **user_record;
        let user_compressed_info = prepare_compressed_account_on_init::<UserRecord>(
            &user_record_info,
            user_record_data_mut,
            compression_params.user_compressed_address,
            user_new_address_params,
            compression_params.user_output_state_tree_index,
            &cpi_accounts,
            &config.address_space,
            true,
        )?;
        all_compressed_infos.push(user_compressed_info);

        let game_session_info = game_session.to_account_info();
        let game_session_data_mut = &mut **game_session;
        let game_compressed_info = prepare_compressed_account_on_init::<GameSession>(
            &game_session_info,
            game_session_data_mut,
            compression_params.game_compressed_address,
            game_new_address_params,
            compression_params.game_output_state_tree_index,
            &cpi_accounts,
            &config.address_space,
            true,
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

        let mint = find_spl_mint_address(&ctx.accounts.mint_signer.key()).0;

        // Use the generated client seed function for CToken signer (generated by add_compressible_instructions macro)
        let (_, token_account_address) = get_ctokensigner_seeds(&ctx.accounts.user.key(), &mint);

        let actions = vec![
            light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
                recipients: vec![
                    light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                        recipient: token_account_address,
                        amount: 1000,
                    },
                ],
                token_account_version: 3,
            },
        ];

        let output_queue = *cpi_accounts.tree_accounts().unwrap()[0].key;
        let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[1].key;

        let mint_action_inputs = MintActionInputs {
            compressed_mint_inputs: compression_params.mint_with_context.clone(),
            mint_seed: ctx.accounts.mint_signer.key(),
            mint_bump: Some(compression_params.mint_bump),
            create_mint: true,
            authority: ctx.accounts.mint_authority.key(),
            payer: ctx.accounts.user.key(),
            proof: compression_params.proof.into(),
            actions,
            input_queue: None,
            output_queue,
            tokens_out_queue: Some(output_queue),
            address_tree_pubkey,
            token_pool: None,
        };

        let mint_action_instruction = create_mint_action_cpi(
            mint_action_inputs,
            Some(light_ctoken_types::instructions::mint_action::CpiContext {
                address_tree_pubkey: address_tree_pubkey.to_bytes(),
                set_context: false,
                first_set_context: false,
                in_tree_index: 1,
                in_queue_index: 0,
                out_queue_index: 0,
                token_out_queue_index: 0,
                assigned_account_index: 2,
                read_only_address_trees: [0; 4],
            }),
            Some(cpi_context_pubkey),
        )
        .unwrap();

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

        invoke(&mint_action_instruction, &account_infos)?;

        user_record.close(ctx.accounts.rent_sponsor.to_account_info())?;
        game_session.close(ctx.accounts.rent_sponsor.to_account_info())?;

        Ok(())
    }
}
