#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda};
use light_sdk_macros::{light_instruction, rentfree_program};
use light_sdk_types::CpiSigner;

pub mod errors;
pub mod instruction_accounts;
pub mod state;

pub use instruction_accounts::*;
pub use state::{GameSession, PackedGameSession, PackedUserRecord, PlaceholderRecord, UserRecord};

#[inline]
pub fn max_key(left: &Pubkey, right: &Pubkey) -> [u8; 32] {
    if left > right {
        left.to_bytes()
    } else {
        right.to_bytes()
    }
}

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

pub const PROGRAM_RENT_SPONSOR_DATA: ([u8; 32], u8) =
    derive_light_rent_sponsor_pda!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah", 1);

#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

<<<<<<< HEAD
#[add_compressible_instructions(
    // Complex PDA account types with seed specifications using BOTH ctx.accounts.* AND data.*
    // UserRecord: uses ctx accounts (authority, mint_authority) + data fields (owner, category_id)
    UserRecord = ("user_record", ctx.authority, ctx.mint_authority, data.owner, data.category_id.to_le_bytes()),
    // GameSession: uses max_key expression with ctx.accounts + data.session_id
    GameSession = ("game_session", max_key(&ctx.user.key(), &ctx.authority.key()), data.session_id.to_le_bytes()),
    // PlaceholderRecord: mixes ctx accounts and data for seeds
    PlaceholderRecord = ("placeholder_record", ctx.authority, ctx.some_account, data.placeholder_id.to_le_bytes(), data.counter.to_le_bytes()),
    // Token variant (Light Token account) with authority for compression signing
    CTokenSigner = (is_token, "ctoken_signer", ctx.fee_payer, ctx.mint, authority = LIGHT_CPI_SIGNER),
    // Instruction data fields used in seed expressions above
    owner = Pubkey,
    category_id = u64,
    session_id = u64,
    placeholder_id = u64,
    counter = u32,
)]
#[program]
pub mod csdk_anchor_full_derived_test {
    #![allow(clippy::too_many_arguments)]
    use anchor_lang::solana_program::{program::invoke, sysvar::clock::Clock};
    use light_compressed_account::instruction_data::traits::LightInstructionData;
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
    use light_token_interface::instructions::mint_action::{
        MintActionCompressedInstructionData, MintToCompressedAction, Recipient,
    };
    use light_token_sdk::compressed_token::{
        create_compressed_mint::find_mint_address, mint_action::MintActionMetaConfig,
    };
=======
pub const GAME_SESSION_SEED: &str = "game_session";

#[rentfree_program]
#[program]
pub mod csdk_anchor_full_derived_test {
    #![allow(clippy::too_many_arguments)]
>>>>>>> a606eb113 (wip)

    use super::*;
    use crate::{
        instruction_accounts::CreatePdasAndMintAuto,
        state::{GameSession, UserRecord},
        FullAutoWithMintParams, LIGHT_CPI_SIGNER,
    };

    #[light_instruction]
    pub fn create_pdas_and_mint_auto<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePdasAndMintAuto<'info>>,
        params: FullAutoWithMintParams,
    ) -> Result<()> {
        use anchor_lang::solana_program::sysvar::clock::Clock;
        use light_ctoken_sdk::ctoken::{
            CTokenMintToCpi, CreateCTokenAccountCpi, CreateCTokenAtaCpi,
        };

        let user_record = &mut ctx.accounts.user_record;
        user_record.owner = params.owner;
        user_record.name = "Auto Created User With Mint".to_string();
        user_record.score = 0;
        user_record.category_id = params.category_id;

        let game_session = &mut ctx.accounts.game_session;
        game_session.session_id = params.session_id;
        game_session.player = ctx.accounts.fee_payer.key();
        game_session.game_type = "Auto Game With Mint".to_string();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

<<<<<<< HEAD
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
            &config,
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
            &config,
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

        let mint = find_mint_address(&ctx.accounts.mint_signer.key()).0;

        // Use the generated client seed function for Light Token signer (generated by add_compressible_instructions macro)
        let (_, token_account_address) = get_ctokensigner_seeds(&ctx.accounts.user.key(), &mint);

        let output_queue = *cpi_accounts.tree_accounts().unwrap()[0].key;
        let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[1].key;

        // Build instruction data using the correct API
        let proof = compression_params.proof.0.unwrap_or_default();
        let instruction_data = MintActionCompressedInstructionData::new_mint(
            0, // root_index for new addresses
            proof,
            compression_params.mint_with_context.mint.clone().unwrap(),
        )
        .with_mint_to_compressed(MintToCompressedAction {
            token_account_version: 3,
            recipients: vec![Recipient::new(token_account_address, 1000)],
        })
        .with_cpi_context(
            light_token_interface::instructions::mint_action::CpiContext {
                address_tree_pubkey: address_tree_pubkey.to_bytes(),
                set_context: false,
                first_set_context: false,
                in_tree_index: 1,
                in_queue_index: 0,
                out_queue_index: 0,
                token_out_queue_index: 0,
                assigned_account_index: 2,
                read_only_address_trees: [0; 4],
            },
        );
=======
        let cmint_key = ctx.accounts.cmint.key();
        CreateCTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.vault.to_account_info(),
            mint: ctx.accounts.cmint.to_account_info(),
            owner: ctx.accounts.vault_authority.key(),
        }
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            crate::instruction_accounts::VAULT_SEED,
            cmint_key.as_ref(),
            &[params.vault_bump],
        ])?;
>>>>>>> a606eb113 (wip)

        CreateCTokenAtaCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            owner: ctx.accounts.fee_payer.to_account_info(),
            mint: ctx.accounts.cmint.to_account_info(),
            ata: ctx.accounts.user_ata.to_account_info(),
            bump: params.user_ata_bump,
        }
        .idempotent()
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        )
        .invoke()?;

        if params.vault_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.vault.to_account_info(),
                amount: params.vault_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

<<<<<<< HEAD
        let account_metas = config.to_account_metas();

        // Serialize instruction data
        let data = instruction_data.data().map_err(ProgramError::from)?;

        // Build mint action instruction
        let mint_action_instruction = solana_program::instruction::Instruction {
            program_id: light_token_interface::LIGHT_TOKEN_PROGRAM_ID.into(),
            accounts: account_metas,
            data,
        };

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
=======
        if params.user_ata_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.user_ata.to_account_info(),
                amount: params.user_ata_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }
>>>>>>> a606eb113 (wip)

        Ok(())
    }
}
