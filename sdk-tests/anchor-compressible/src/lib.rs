use anchor_lang::{
    prelude::*,
    solana_program::{instruction::AccountMeta, program::invoke, pubkey::Pubkey},
};
use anchor_spl::token_interface::TokenAccount;
use light_compressed_token_sdk::compressible::{
    process_compressed_token_accounts, PackedCompressedTokenDataWithContext,
};
use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;
use light_sdk::{
    account::Size,
    compressible::{
        compress_account, compress_account_on_init, compress_empty_account_on_init,
        prepare_accounts_for_compression_on_init, prepare_accounts_for_decompress_idempotent,
        process_initialize_compression_config_checked, process_update_compression_config,
        CompressAs, CompressibleConfig, CompressionInfo, HasCompressionInfo,
    },
    cpi::CpiInputs,
    derive_light_cpi_signer,
    error::LightSdkError,
    instruction::{
        account_meta::{CompressedAccountMeta, CompressedAccountMetaNoLamportsNoAddress},
        PackedAddressTreeInfo, ValidityProof,
    },
    light_hasher::{DataHasher, Hasher},
    sha::LightAccount,
    LightDiscriminator, LightHasher,
};
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall, CpiSigner};

use light_compressed_token_sdk::compressible::{
    DecompressAccountsInput, DecompressTokenAccount, TokenVariant,
};

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

// You can implement this for each of your token account derivation paths.
pub fn get_ctoken_signer_seeds<'a>(user: &'a Pubkey, mint: &'a Pubkey) -> (Pubkey, Vec<Vec<u8>>) {
    let mut seeds = vec![
        b"ctoken_signer".to_vec(),
        user.to_bytes().to_vec(),
        mint.to_bytes().to_vec(),
    ];
    let seeds_slice = seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>();
    let (pda, bump) = Pubkey::find_program_address(seeds_slice.as_slice(), &crate::ID);
    seeds.push(vec![bump]);
    (pda, seeds)
}

impl<'c, 'info> TokenVariant<'c, 'info> for CTokenAccountVariant {
    fn get_seeds(
        &self,
        input: &DecompressAccountsInput<'c, 'info>,
        token: &PackedCompressedTokenDataWithContext,
    ) -> std::result::Result<Vec<Vec<u8>>, LightSdkError> {
        match self {
            CTokenAccountVariant::CTokenSigner => {
                let fee_payer = input.fee_payer;
                let mint_info = input
                    .cpi_accounts
                    .get_tree_account_info(token.mint as usize)?
                    .key;
                let (_, seeds) = get_ctoken_signer_seeds(&fee_payer.key(), mint_info);
                Ok(seeds)
            }
            CTokenAccountVariant::Default(data) => Ok(data.clone()),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
#[repr(u8)]
pub enum CTokenAccountVariant {
    CTokenSigner = 0,
    /// General variant that will work for any pda.
    Default(Vec<Vec<u8>>),
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct PackedCompressedTokenDataWithAccountVariant {
    pub variant: CTokenAccountVariant, // 1 byte
    pub token_data: PackedCompressedTokenDataWithContext,
}

impl<'c, 'info> Into<DecompressTokenAccount<'c, 'info, CTokenAccountVariant>>
    for PackedCompressedTokenDataWithAccountVariant
{
    fn into(self) -> DecompressTokenAccount<'c, 'info, CTokenAccountVariant> {
        DecompressTokenAccount {
            variant: self.variant,
            token_data: self.token_data,
            phantom_data: std::marker::PhantomData,
        }
    }
}

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible {

    use light_compressed_account::address::derive_compressed_address;

    use light_compressed_token_sdk::instructions::{
        create_mint_action_cpi, find_spl_mint_address, MintActionInputs,
    };
    use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;

    use super::*;

    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        // 1. Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        user_record.owner = ctx.accounts.user.key();
        user_record.name = name;
        user_record.score = 11;

        // 2. Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // 3. Create CPI accounts
        let user_account_info = ctx.accounts.user.to_account_info();
        let cpi_accounts =
            CpiAccountsSmall::new(&user_account_info, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        let new_address_params = address_tree_info.into_new_address_params_assigned_packed(
            user_record.key().to_bytes(),
            true,
            Some(0),
        );

        compress_account_on_init::<UserRecord>(
            user_record,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
            proof,
        )?;

        // at the end of the instruction we always clean up all onchain pdas that we compressed
        user_record.close(ctx.accounts.rent_recipient.to_account_info())?;

        Ok(())
    }

    pub fn update_record(ctx: Context<UpdateRecord>, name: String, score: u64) -> Result<()> {
        let user_record = &mut ctx.accounts.user_record;

        user_record.name = name;
        user_record.score = score;

        // 1. Must manually set compression info
        user_record
            .compression_info_mut()
            .bump_last_written_slot()?;

        Ok(())
    }

    pub fn update_game_session(
        ctx: Context<UpdateGameSession>,
        _session_id: u64,
        new_score: u64,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;

        game_session.score = new_score;
        game_session.end_time = Some(Clock::get()?.unix_timestamp as u64);

        // Must manually set compression info
        game_session
            .compression_info_mut()
            .bump_last_written_slot()?;

        Ok(())
    }

    // auto-derived via macro.
    pub fn initialize_compression_config(
        ctx: Context<InitializeCompressionConfig>,
        compression_delay: u32,
        rent_recipient: Pubkey,
        address_space: Vec<Pubkey>,
    ) -> Result<()> {
        process_initialize_compression_config_checked(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.program_data.to_account_info(),
            &rent_recipient,
            address_space,
            compression_delay,
            0, // one global config for now, so bump is 0.
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )?;

        Ok(())
    }

    // auto-derived via macro.
    pub fn update_compression_config(
        ctx: Context<UpdateCompressionConfig>,
        new_compression_delay: Option<u32>,
        new_rent_recipient: Option<Pubkey>,
        new_address_space: Option<Vec<Pubkey>>,
        new_update_authority: Option<Pubkey>,
    ) -> Result<()> {
        process_update_compression_config(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            new_update_authority.as_ref(),
            new_rent_recipient.as_ref(),
            new_address_space,
            new_compression_delay,
            &crate::ID,
        )?;

        Ok(())
    }

    // auto-derived via macro. takes the tagged account structs via
    // add_compressible_accounts macro and derives the relevant variant type and
    // dispatcher. The instruction can be used with any number of any of the
    // tagged account structs. It's idempotent; it will not fail if the accounts
    // are already decompressed.
    pub fn decompress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
        // TODO: put into a single struct
        proof: ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        compressed_token_accounts: Vec<PackedCompressedTokenDataWithAccountVariant>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        let has_pdas = !compressed_accounts.is_empty();
        let has_tokens = !compressed_token_accounts.is_empty();
        // Load config
        let compression_config =
            CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;
        let address_space = compression_config.address_space[0];
        // create cpi_accounts
        let fee_payer_account_info = ctx.accounts.fee_payer.to_account_info();
        let mut pda_accounts_start = ctx.remaining_accounts.len()
            - compressed_accounts.len()
            - compressed_token_accounts.len();
        let (cpi_accounts, solana_accounts) = if has_pdas && has_tokens {
            pda_accounts_start += 1; // + cpi context
            (
                CpiAccountsSmall::new_with_config(
                    &fee_payer_account_info,
                    &ctx.remaining_accounts[system_accounts_offset as usize..],
                    CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
                ),
                &ctx.remaining_accounts[pda_accounts_start..],
            )
        } else {
            (
                CpiAccountsSmall::new(
                    &fee_payer_account_info,
                    &ctx.remaining_accounts[system_accounts_offset as usize..],
                    LIGHT_CPI_SIGNER,
                ),
                &ctx.remaining_accounts[pda_accounts_start..],
            )
        };

        let mut compressed_pda_infos = Vec::new();

        for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
            match compressed_data.data {
                CompressedAccountVariant::UserRecord(data) => {
                    // seeds for UserRecord. these are based on your custom
                    // program logic.
                    // 1. derive pda from seeds
                    // 2. verify pda matches the passed in pda
                    // 3. derive c_pda from solana_account_key and the config's
                    // address_space attach c_pda to the meta to save ixdata.
                    let seeds = [b"user_record".as_ref(), ctx.accounts.fee_payer.key.as_ref()];
                    let (pda, bump) = Pubkey::find_program_address(&seeds, &crate::ID);
                    let bump_slice = [bump];
                    let seeds_refs = vec![seeds[0], seeds[1], &bump_slice];
                    let seeds_refs_vec: Vec<_> = vec![seeds_refs.as_slice()];

                    let solana_account_key = solana_accounts[i].key;
                    if pda != *solana_account_key {
                        msg!(
                            "pda mismatch: expected {:?}, got {:?}",
                            pda,
                            solana_account_key
                        );
                        panic!()
                    }

                    let derived_c_pda = derive_compressed_address(
                        &solana_account_key.into(),
                        &address_space.into(),
                        &crate::ID.into(),
                    );

                    let meta_with_address = CompressedAccountMeta {
                        tree_info: compressed_data.meta.tree_info,
                        address: derived_c_pda,
                        output_state_tree_index: compressed_data.meta.output_state_tree_index,
                    };

                    // Create sha::LightAccount with correct UserRecord discriminator
                    let light_account = LightAccount::<'_, UserRecord>::new_mut(
                        &crate::ID,
                        &meta_with_address,
                        data,
                    )?;

                    // Process this single UserRecord account
                    let solana_account_slice = vec![&solana_accounts[i]];
                    let light_accounts = vec![light_account];

                    let cpi_accounts_box = Box::new(cpi_accounts.clone());

                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<UserRecord>(
                        solana_account_slice,
                        light_accounts,
                        seeds_refs_vec,
                        &cpi_accounts_box,
                        &ctx.accounts.rent_payer,
                        address_space,
                    )?;

                    compressed_pda_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::GameSession(data) => {
                    // seeds for GameSession. these are based on your custom
                    // program logic.
                    // 1. derive pda from seeds
                    // 2. verify pda matches the passed in pda
                    // 3. derive c_pda from solana_account_key and the config's
                    // address_space attach c_pda to the meta to save ixdata.
                    let session_id = data.session_id;
                    let session_id_le = session_id.to_le_bytes();
                    let seeds = [b"game_session".as_ref(), session_id_le.as_ref()];
                    let (pda, bump) = Pubkey::find_program_address(&seeds, &crate::ID);
                    let bump_slice = [bump];
                    let seeds_refs = [seeds[0], seeds[1], &bump_slice];
                    let seeds_refs_vec = vec![seeds_refs.as_slice()];

                    let solana_account_key = solana_accounts[i].key;
                    if pda != *solana_account_key {
                        panic!(
                            "pda mismatch: expected {:?}, got {:?}",
                            pda, solana_account_key
                        );
                    }

                    let derived_c_pda = derive_compressed_address(
                        &solana_account_key.into(),
                        &address_space.into(),
                        &crate::ID.into(),
                    );

                    let meta_with_address = CompressedAccountMeta {
                        tree_info: compressed_data.meta.tree_info,
                        address: derived_c_pda,
                        output_state_tree_index: compressed_data.meta.output_state_tree_index,
                    };

                    // Create sha::LightAccount with correct GameSession discriminator
                    let light_account = LightAccount::<'_, GameSession>::new_mut(
                        &crate::ID,
                        &meta_with_address,
                        data,
                    )?;

                    // Process this single GameSession account
                    let solana_account_slice = vec![&solana_accounts[i]];
                    let light_accounts = vec![light_account];

                    let cpi_accounts_box = Box::new(cpi_accounts.clone());

                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<GameSession>(
                        solana_account_slice,
                        light_accounts,
                        seeds_refs_vec,
                        &cpi_accounts_box,
                        &ctx.accounts.rent_payer,
                        address_space,
                    )?;
                    compressed_pda_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::PlaceholderRecord(data) => {
                    // seeds for PlaceholderRecord. these are based on your
                    // custom program logic.
                    // 1. derive pda from seeds
                    // 2. verify pda matches the passed in pda
                    // 3. derive c_pda from solana_account_key and the config's
                    // address_space attach c_pda to the meta to save ixdata.
                    let placeholder_id = data.placeholder_id;
                    let placeholder_id_le = placeholder_id.to_le_bytes();
                    let seeds = [b"placeholder_record".as_ref(), placeholder_id_le.as_ref()];
                    let (pda, bump) = Pubkey::find_program_address(&seeds, &crate::ID);
                    let bump_slice = [bump];
                    let seeds_refs = vec![seeds[0], seeds[1], &bump_slice];
                    let seeds_refs_vec: Vec<_> = vec![seeds_refs.as_slice()];

                    let solana_account_key = solana_accounts[i].key;
                    if pda != *solana_account_key {
                        panic!(
                            "pda mismatch: expected {:?}, got {:?}",
                            pda, solana_account_key
                        );
                    }

                    let derived_c_pda = derive_compressed_address(
                        &solana_account_key.into(),
                        &address_space.into(),
                        &crate::ID.into(),
                    );

                    let meta_with_address = CompressedAccountMeta {
                        tree_info: compressed_data.meta.tree_info,
                        address: derived_c_pda,
                        output_state_tree_index: compressed_data.meta.output_state_tree_index,
                    };

                    // Create sha::LightAccount with correct PlaceholderRecord discriminator
                    let light_account = LightAccount::<'_, PlaceholderRecord>::new_mut(
                        &crate::ID,
                        &meta_with_address,
                        data,
                    )?;

                    // Process this single PlaceholderRecord account
                    let solana_account_slice = vec![&solana_accounts[i]];
                    let light_accounts = vec![light_account];
                    let cpi_accounts_box = Box::new(cpi_accounts.clone());

                    let compressed_infos =
                        prepare_accounts_for_decompress_idempotent::<PlaceholderRecord>(
                            solana_account_slice,
                            light_accounts,
                            seeds_refs_vec,
                            &cpi_accounts_box,
                            &ctx.accounts.rent_payer,
                            address_space,
                        )?;

                    compressed_pda_infos.extend(compressed_infos);
                }
            }
        }

        if has_pdas && !has_tokens {
            // NO CPI CONTEXT.
            let cpi_inputs = CpiInputs::new(proof, compressed_pda_infos);
            cpi_inputs.invoke_light_system_program_small(cpi_accounts.clone())?;
        } else if has_pdas && has_tokens {
            // we only need a subset for the first (pda) cpi because we write into
            // the cpi_context.
            let system_cpi_accounts = CpiContextWriteAccounts {
                fee_payer: &fee_payer_account_info,
                authority: cpi_accounts.authority().unwrap(),
                cpi_context: cpi_accounts.cpi_context().unwrap(),
                cpi_signer: LIGHT_CPI_SIGNER,
            };
            let cpi_inputs = CpiInputs::new_first_cpi(compressed_pda_infos, vec![]);
            cpi_inputs.invoke_light_system_program_cpi_context(system_cpi_accounts)?;
        }
        // In macro we can put this behind ctoken feature so that ctoken crates are only required when ctokens are used
        if has_tokens {
            let compressed_token_program = ctx
                .accounts
                .compressed_token_program
                .as_ref()
                .ok_or_else(|| {
                    msg!("compressed_token_program is none but decompression instruction has ctoken accounts.");
                    ProgramError::NotEnoughAccountKeys
                })?;
            let compressed_token_cpi_authority = ctx
                .accounts
                .compressed_token_cpi_authority
                .as_ref()
                .ok_or_else(|| {
                    msg!("compressed_token_cpi_authority is none but decompression instruction has ctoken accounts.");
                    ProgramError::NotEnoughAccountKeys
                })?;
            process_compressed_token_accounts(
                compressed_token_accounts
                    .into_iter()
                    .map(|account| account.into())
                    .collect::<Vec<_>>(),
                DecompressAccountsInput {
                    fee_payer: &ctx.accounts.fee_payer,
                    rent_payer: &ctx.accounts.rent_payer,
                    config: &ctx.accounts.config,
                    compressed_token_program,
                    compressed_token_cpi_authority,
                    remaining_accounts: ctx.remaining_accounts,
                    cpi_accounts: &cpi_accounts,
                    proof,
                    has_tokens,
                    has_pdas,
                },
            )
            .map_err(ProgramError::from)?;
        }
        Ok(())
    }

    // Must be manually implemented.
    pub fn create_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateGameSession<'info>>,
        session_id: u64,
        game_type: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.game_session;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Set your account data.
        game_session.session_id = session_id;
        game_session.player = ctx.accounts.player.key();
        game_session.game_type = game_type;
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // Check that rent recipient matches your config.
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // Create CPI accounts.
        let player_account_info = ctx.accounts.player.to_account_info();
        let cpi_accounts = CpiAccountsSmall::new(
            &player_account_info,
            ctx.remaining_accounts,
            LIGHT_CPI_SIGNER,
        );

        // Prepare new address params. The cpda takes the address of the
        // compressible pda account as seed.
        let new_address_params = address_tree_info.into_new_address_params_assigned_packed(
            game_session.key().to_bytes(),
            true,
            Some(0),
        );

        // Call at the end of your init instruction to compress the pda account
        // safely. This also closes the pda account. The account can then be
        // decompressed by anyone at any time via the
        // decompress_accounts_idempotent instruction. Creates a unique cPDA to
        // ensure that the account cannot be re-inited only decompressed.
        compress_account_on_init::<GameSession>(
            game_session,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
            proof,
        )?;

        game_session.close(ctx.accounts.rent_recipient.to_account_info())?;

        Ok(())
    }

    // Must be manually implemented.
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
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
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
        let cpi_accounts = CpiAccountsSmall::new_with_config(
            ctx.accounts.user.as_ref(),
            ctx.remaining_accounts,
            CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
        );
        let cpi_context_pubkey = cpi_accounts.cpi_context().unwrap().key();
        let cpi_context_account = cpi_accounts.cpi_context().unwrap();

        // Prepare new address params. One per pda account.
        let user_new_address_params = compression_params
            .user_address_tree_info
            .into_new_address_params_assigned_packed(user_record.key().to_bytes(), true, Some(0));
        let game_new_address_params = compression_params
            .game_address_tree_info
            .into_new_address_params_assigned_packed(game_session.key().to_bytes(), true, Some(1));

        let mut all_compressed_infos = Vec::new();

        // Prepares the firstpda account for compression. compress the pda
        // account safely. This also closes the pda account. safely. This also
        // closes the pda account. The account can then be decompressed by
        // anyone at any time via the decompress_accounts_idempotent
        // instruction. Creates a unique cPDA to ensure that the account cannot
        // be re-inited only decompressed.
        let user_compressed_infos = prepare_accounts_for_compression_on_init::<UserRecord>(
            &mut [user_record],
            &[compression_params.user_compressed_address],
            &[user_new_address_params],
            &[compression_params.user_output_state_tree_index],
            &cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
        )?;

        all_compressed_infos.extend(user_compressed_infos);

        // Process GameSession for compression. compress the pda account safely.
        // This also closes the pda account. The account can then be
        // decompressed by anyone at any time via the
        // decompress_accounts_idempotent instruction. Creates a unique cPDA to
        // ensure that the account cannot be re-inited only decompressed.
        let game_compressed_infos = prepare_accounts_for_compression_on_init::<GameSession>(
            &mut [game_session],
            &[compression_params.game_compressed_address],
            &[game_new_address_params],
            &[compression_params.game_output_state_tree_index],
            &cpi_accounts,
            &config.address_space,
            &ctx.accounts.rent_recipient,
        )?;
        all_compressed_infos.extend(game_compressed_infos);

        let cpi_inputs = CpiInputs::new_first_cpi(
            all_compressed_infos,
            vec![user_new_address_params, game_new_address_params],
        );

        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority().unwrap(),
            cpi_context: cpi_context_account,
            cpi_signer: LIGHT_CPI_SIGNER,
        };
        cpi_inputs.invoke_light_system_program_cpi_context(cpi_context_accounts)?;

        // these are custom seeds of the caller program that are used to derive the program owned onchain tokenb account PDA.
        // dual use: as owner of the compressed token account.
        let mint = find_spl_mint_address(&ctx.accounts.mint_signer.key()).0;
        let (token_account_address, _token_account_seeds) =
            get_ctoken_signer_seeds(&ctx.accounts.user.key(), &mint);

        let actions = vec![
            light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
                recipients: vec![
                    light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                        recipient: token_account_address, // TRY: THE DECOMPRESS TOKEN ACCOUNT ADDRES IS THE OWNER OF ITS COMPRESSIBLED VERSION.
                        amount: 1000,                     // Mint the full supply to the user
                    },
                ],
                lamports: None,
                token_account_version: 2,
            },
        ];

        let output_queue = *cpi_accounts.tree_accounts().unwrap()[0].key; // Same tree as PDA
        let address_tree_pubkey = *cpi_accounts.tree_accounts().unwrap()[1].key; // Same tree as PDA

        let mint_action_inputs = MintActionInputs {
            compressed_mint_inputs: compression_params.mint_with_context.clone().into(),
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
                set_context: false,
                first_set_context: false,
                in_tree_index: 1, // address tree
                in_queue_index: 0,
                out_queue_index: 0,
                token_out_queue_index: 0,
                assigned_account_index: 2,
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
        account_infos.push(ctx.accounts.compressed_token_program.to_account_info());
        account_infos.push(ctx.accounts.mint_authority.to_account_info());
        account_infos.push(ctx.accounts.mint_signer.to_account_info());
        account_infos.push(ctx.accounts.user.to_account_info());

        // Invoke the mint action instruction directly
        invoke(&mint_action_instruction, &account_infos)?;

        // at the end of the instruction we always clean up all onchain pdas that we compressed
        user_record.close(ctx.accounts.rent_recipient.to_account_info())?;
        game_session.close(ctx.accounts.rent_recipient.to_account_info())?;

        Ok(())
    }

    // Auto-derived via macro. Based on target account type, it will compress
    // the pda account safely. This also closes the pda account. The account can
    // then be decompressed by anyone at any time via the
    // decompress_accounts_idempotent instruction. Does not create a new cPDA.
    // but requires the existing (empty) compressed account to be passed in.
    pub fn compress_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressRecord<'info>>,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let user_record = &mut ctx.accounts.pda_to_compress;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let user_account_info = ctx.accounts.user.to_account_info();
        let cpi_accounts =
            CpiAccountsSmall::new(&user_account_info, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        compress_account::<UserRecord>(
            user_record,
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )?;

        Ok(())
    }

    /// Compresses a GameSession PDA with custom data using config values.
    /// This demonstrates the custom compression feature which allows resetting
    /// some fields (start_time, end_time, score) while keeping others (session_id, player, game_type).
    pub fn compress_game_session_with_custom_data<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressGameSession<'info>>,
        _session_id: u64,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let game_session = &mut ctx.accounts.pda_to_compress;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let player_account_info = ctx.accounts.player.to_account_info();
        let cpi_accounts = CpiAccountsSmall::new(
            &player_account_info,
            ctx.remaining_accounts,
            LIGHT_CPI_SIGNER,
        );

        compress_account::<GameSession>(
            game_session,
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )?;

        game_session.close(ctx.accounts.rent_recipient.to_account_info())?;

        Ok(())
    }

    /// Creates an empty compressed account while keeping the PDA intact.
    /// This demonstrates the compress_empty_account_on_init functionality.
    pub fn create_placeholder_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePlaceholderRecord<'info>>,
        placeholder_id: u64,
        name: String,
        proof: ValidityProof,
        compressed_address: [u8; 32],
        address_tree_info: PackedAddressTreeInfo,
        output_state_tree_index: u8,
    ) -> Result<()> {
        let placeholder_record = &mut ctx.accounts.placeholder_record;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        placeholder_record.owner = ctx.accounts.user.key();
        placeholder_record.name = name;
        placeholder_record.placeholder_id = placeholder_id;

        // Initialize compression_info for the PDA
        *placeholder_record.compression_info_mut_opt() =
            Some(super::CompressionInfo::new_decompressed()?);
        placeholder_record
            .compression_info_mut()
            .bump_last_written_slot()?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        // Create CPI accounts
        let user_account_info = ctx.accounts.user.to_account_info();
        let cpi_accounts =
            CpiAccountsSmall::new(&user_account_info, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        let new_address_params = address_tree_info.into_new_address_params_assigned_packed(
            placeholder_record.key().to_bytes(),
            true,
            Some(0),
        );

        // Use the new compress_empty_account_on_init function
        // This creates an empty compressed account but does NOT close the PDA
        compress_empty_account_on_init::<PlaceholderRecord>(
            placeholder_record,
            &compressed_address,
            &new_address_params,
            output_state_tree_index,
            cpi_accounts,
            &config.address_space,
            proof,
        )?;

        placeholder_record.close(ctx.accounts.rent_recipient.to_account_info())?;
        Ok(())
    }

    /// Compresses a PlaceholderRecord PDA using config values.
    pub fn compress_placeholder_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressPlaceholderRecord<'info>>,
        proof: ValidityProof,
        compressed_account_meta: CompressedAccountMeta,
    ) -> Result<()> {
        let placeholder_record = &mut ctx.accounts.pda_to_compress;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let user_account_info = ctx.accounts.user.to_account_info();
        let cpi_accounts =
            CpiAccountsSmall::new(&user_account_info, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        compress_account::<PlaceholderRecord>(
            placeholder_record,
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )?;

        placeholder_record.close(ctx.accounts.rent_recipient.to_account_info())?;

        Ok(())
    }
    /*
    pub fn compress_token_account_ctoken_signer<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressTokenAccountCtokenSigner<'info>>,
    ) -> Result<()> {
        let token_account = &mut ctx.accounts.token_account_to_compress;

        // Load config from the config account
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;

        // Verify rent recipient matches config
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return err!(ErrorCode::InvalidRentRecipient);
        }

        let user_account_info = ctx.accounts.user.to_account_info();
        let cpi_accounts =
            CpiAccountsSmall::new(&user_account_info, ctx.remaining_accounts, LIGHT_CPI_SIGNER);

        let mut compressed_token_infos = Vec::new();

        let (_, ctoken_signer_seeds) =
            get_ctoken_signer_seeds(&ctx.accounts.user.key(), &token_account.mint);

        // creates account_metas for CPI.
        let tree_accounts = cpi_accounts.tree_accounts().unwrap();
        let mut packed_accounts = Vec::with_capacity(tree_accounts.len());
        for account_info in tree_accounts {
            packed_accounts.push(account_meta_from_account_info(account_info));
        }

        // fields
        let owner = token_account.owner;
        let mint = token_account.mint;
        let amount = token_account.amount;
        let source_or_recipient = token_account.key();
        let authority = token_account.close_authority; // The owner of the token account is the authority for compression

        let system_program = cpi_accounts.system_program().unwrap();
        let compression =
            Compression::compress_ctoken(amount, mint, source_or_recipient, authority);
        let ctoken_account = CTokenAccount2 {
            inputs: vec![],
            output: MultiTokenTransferOutputData::default(),
            compression: Some(compression),
            delegate_is_set: false,
            method_used: true,
        };
        let inputs = Transfer2Inputs {
            validity_proof: ValidityProof::default(),
            transfer_config: Transfer2Config::new().filter_zero_amount_outputs(),
            meta_config: Transfer2AccountsMetaConfig::new(
                ctx.accounts.fee_payer.key(),
                packed_accounts,
            ),
            in_lamports: None,
            out_lamports: None,
            token_accounts: vec![ctoken_account],
        };

        let ctoken_ix = create_transfer2_instruction(inputs).map_err(ProgramError::from)?;

        // account_infos
        let mut all_account_infos = vec![ctx.accounts.fee_payer.to_account_info()];
        all_account_infos.extend(
            ctx.accounts
                .compressed_token_cpi_authority
                .to_account_infos(),
        );
        all_account_infos.extend(ctx.accounts.compressed_token_program.to_account_infos());
        all_account_infos.extend(ctx.accounts.config.to_account_infos());
        all_account_infos.extend(cpi_accounts.to_account_infos());

        // ctoken cpi
        let seed_refs = ctoken_signer_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>();
        invoke_signed(
            &ctoken_ix,
            all_account_infos.as_slice(),
            &[seed_refs.as_slice()],
        )?;
        token_account.close(ctx.accounts.rent_recipient.to_account_info())?;

        Ok(())
    }*/
}

#[derive(Accounts)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        // discriminator + owner + string len + name + score +
        // option<compression_info>. Note that in the onchain space
        // CompressionInfo is always Some.
        space = 8 + 32 + 4 + 32 + 8 + 10,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    /// Needs to be here for the init anchor macro to work.
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(placeholder_id: u64)]
pub struct CreatePlaceholderRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        // discriminator + compression_info + owner + string len + name + placeholder_id
        space = 8 + 10 + 32 + 4 + 32 + 8,
        seeds = [b"placeholder_record", placeholder_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub placeholder_record: Account<'info, PlaceholderRecord>,
    /// Needs to be here for the init anchor macro to work.
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(account_data: AccountCreationData)]
pub struct CreateUserRecordAndGameSession<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        // discriminator + owner + string len + name + score +
        // option<compression_info>. Note that in the onchain space
        // CompressionInfo is always Some.
        space = 8 + 32 + 4 + 32 + 8 + 10,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    #[account(
        init,
        payer = user,
        // discriminator + option<compression_info> + session_id + player +
        // string len + game_type + start_time + end_time(Option) + score
        space = 8 + 10 + 8 + 32 + 4 + 32 + 8 + 9 + 8,
        seeds = [b"game_session", account_data.session_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub game_session: Account<'info, GameSession>,

    // Compressed mint creation accounts - only token-specific ones needed
    /// The mint signer used for PDA derivation
    pub mint_signer: Signer<'info>,

    /// The mint authority used for PDA derivation
    pub mint_authority: Signer<'info>,

    /// Compressed token program
    /// CHECK: Program ID validated using COMPRESSED_TOKEN_PROGRAM_ID constant
    pub compressed_token_program: UncheckedAccount<'info>,

    /// CHECK: CPI authority of the compressed token program
    pub compress_token_program_cpi_authority: UncheckedAccount<'info>,

    /// Needs to be here for the init anchor macro to work.
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(session_id: u64)]
pub struct CreateGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        init,
        payer = player,
        space = 8 + 9 + 8 + 32 + 4 + 32 + 8 + 9 + 8, // discriminator + compression_info + session_id + player + string len + game_type + start_time + end_time(Option) + score
        seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub game_session: Account<'info, GameSession>,
    pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
        constraint = user_record.owner == user.key()
    )]
    pub user_record: Account<'info, UserRecord>,
}

#[derive(Accounts)]
#[instruction(session_id: u64)]
pub struct UpdateGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
        bump,
        constraint = game_session.player == player.key()
    )]
    pub game_session: Account<'info, GameSession>,
}

#[derive(Accounts)]
pub struct CompressRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
        constraint = pda_to_compress.owner == user.key()
    )]
    pub pda_to_compress: Account<'info, UserRecord>,
    // pub system_program: Program<'info, System>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(session_id: u64)]
pub struct CompressGameSession<'info> {
    #[account(mut)]
    pub player: Signer<'info>,
    #[account(
        mut,
        seeds = [b"game_session", session_id.to_le_bytes().as_ref()],
        bump,
        constraint = pda_to_compress.player == player.key()
    )]
    pub pda_to_compress: Account<'info, GameSession>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CompressPlaceholderRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        constraint = pda_to_compress.owner == user.key()
    )]
    pub pda_to_compress: Account<'info, PlaceholderRecord>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CompressTokenAccountCtokenSigner<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: todo
    pub user: UncheckedAccount<'info>,
    /// CHECK: todo
    compressed_token_cpi_authority: UncheckedAccount<'info>,
    /// CHECK: todo
    compressed_token_program: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"ctoken_signer", user.key().as_ref(), token_account_to_compress.mint.as_ref()],
        bump,
    )]
    pub token_account_to_compress: InterfaceAccount<'info, TokenAccount>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
}

// TODO: split into one ix with ctoken and one without.
#[derive(Accounts, Debug)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// UNCHECKED: Anyone can pay to init.
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    /// The global config account
    /// CHECK: load_checked.
    pub config: AccountInfo<'info>,

    // CToken-specific accounts (optional, only needed when decompressing CToken accounts)
    /// Compressed token program
    /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
    pub compressed_token_program: Option<UncheckedAccount<'info>>,

    /// CPI authority PDA of the compressed token program
    /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
    pub compressed_token_cpi_authority: Option<UncheckedAccount<'info>>,
    // Remaining accounts:
    // - First N accounts: PDA accounts to decompress into (native CToken accounts)
    // - After system_accounts_offset: Light Protocol system accounts for CPI
    //
    // For CToken decompression, the PDA accounts must be native CToken accounts
    // owned by the compressed token program (cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m)
}

#[derive(Accounts)]
pub struct InitializeCompressionConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Config PDA is created and validated by the SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    /// The program's data account
    /// CHECK: Program data account is validated by the SDK
    pub program_data: AccountInfo<'info>,
    /// The program's upgrade authority (must sign)
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateCompressionConfig<'info> {
    /// CHECK: Config PDA is created and validated by the SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    /// Must match the update authority stored in config
    pub authority: Signer<'info>,
}

/// Auto-derived via macro. Unified enum that can hold any account type. Crucial
/// for dispatching multiple compressed accounts of different types in
/// decompress_accounts_idempotent.
/// Implements: Default, DataHasher, LightDiscriminator, HasCompressionInfo.
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedAccountVariant {
    UserRecord(UserRecord),
    GameSession(GameSession),
    PlaceholderRecord(PlaceholderRecord),
}

impl Default for CompressedAccountVariant {
    fn default() -> Self {
        Self::UserRecord(UserRecord::default())
    }
}

impl DataHasher for CompressedAccountVariant {
    fn hash<H: Hasher>(&self) -> std::result::Result<[u8; 32], light_hasher::HasherError> {
        match self {
            Self::UserRecord(data) => data.hash::<H>(),
            Self::GameSession(data) => data.hash::<H>(),
            Self::PlaceholderRecord(data) => data.hash::<H>(),
        }
    }
}

impl LightDiscriminator for CompressedAccountVariant {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8]; // This won't be used directly
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl HasCompressionInfo for CompressedAccountVariant {
    fn compression_info(&self) -> &CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info(),
            Self::GameSession(data) => data.compression_info(),
            Self::PlaceholderRecord(data) => data.compression_info(),
        }
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info_mut(),
            Self::GameSession(data) => data.compression_info_mut(),
            Self::PlaceholderRecord(data) => data.compression_info_mut(),
        }
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        match self {
            Self::UserRecord(data) => data.compression_info_mut_opt(),
            Self::GameSession(data) => data.compression_info_mut_opt(),
            Self::PlaceholderRecord(data) => data.compression_info_mut_opt(),
        }
    }

    fn set_compression_info_none(&mut self) {
        match self {
            Self::UserRecord(data) => data.set_compression_info_none(),
            Self::GameSession(data) => data.set_compression_info_none(),
            Self::PlaceholderRecord(data) => data.set_compression_info_none(),
        }
    }
}

impl Size for CompressedAccountVariant {
    fn size(&self) -> usize {
        match self {
            Self::UserRecord(data) => data.size(),
            Self::GameSession(data) => data.size(),
            Self::PlaceholderRecord(data) => data.size(),
        }
    }
}

// Auto-derived via macro. Ix data implemented for Variant.
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData {
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    pub data: CompressedAccountVariant,
}

#[derive(Default, Debug, LightHasher, LightDiscriminator, InitSpace)]
#[account]
pub struct UserRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}

// impl HasCompressionInfo for CTokenAccountData {
//     fn compression_info(&self) -> &CompressionInfo {
//         self.compression_info
//             .as_ref()
//             .expect("CompressionInfo must be Some on-chain")
//     }
//     fn compression_info_mut(&mut self) -> &mut CompressionInfo {
//         self.compression_info
//             .as_mut()
//             .expect("CompressionInfo must be Some on-chain")
//     }

//     fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
//         &mut self.compression_info
//     }

//     fn set_compression_info_none(&mut self) {
//         self.compression_info = None;
//     }
// }

// impl Size for CTokenAccountData {
//     fn size(&self) -> usize {
//         Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
//     }
// }
// impl CompressAs for CTokenAccountData {
//     type Output = Self;

//     fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
//         // Return owned data with compression_info = None for compressed storage
//         std::borrow::Cow::Owned(Self {
//             compression_info: None,
//             mint: self.mint,
//             owner: self.owner,
//             amount: self.amount,
//             delegate: self.delegate,
//             state: self.state,
//             is_native: self.is_native,
//             delegated_amount: self.delegated_amount,
//             close_authority: self.close_authority,
//         })
//     }
// }

// Auto-derived via macro.
impl HasCompressionInfo for UserRecord {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

impl Size for UserRecord {
    fn size(&self) -> usize {
        Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
    }
}

impl CompressAs for UserRecord {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        // Simple case: return owned data with compression_info = None
        // We can't return Cow::Borrowed because compression_info must always be None for compressed storage
        std::borrow::Cow::Owned(Self {
            compression_info: None, // ALWAYS None for compressed storage
            owner: self.owner,
            name: self.name.clone(),
            score: self.score,
        })
    }
}

// Your existing account structs must be manually extended:
// 1. Add compression_info field to the struct, with type
//    Option<CompressionInfo>.
// 2. add a #[skip] field for the compression_info field.
// 3. Add LightHasher, LightDiscriminator.
// 4. Add #[hash] attribute to ALL fields that can be >31 bytes. (eg Pubkeys,
//    Strings)
#[derive(Default, Debug, LightHasher, LightDiscriminator, InitSpace)]
#[account]
pub struct GameSession {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}

// Auto-derived via macro.
impl HasCompressionInfo for GameSession {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

impl Size for GameSession {
    fn size(&self) -> usize {
        Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
    }
}

impl CompressAs for GameSession {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        // Custom compression: return owned data with modified fields
        std::borrow::Cow::Owned(Self {
            compression_info: None,            // ALWAYS None for compressed storage
            session_id: self.session_id,       // KEEP - identifier
            player: self.player,               // KEEP - identifier
            game_type: self.game_type.clone(), // KEEP - core property
            start_time: 0,                     // RESET - clear timing
            end_time: None,                    // RESET - clear timing
            score: 0,                          // RESET - clear progress
        })
    }
}

// PlaceholderRecord - demonstrates empty compressed account creation
// The PDA remains intact while an empty compressed account is created
#[derive(Default, Debug, LightHasher, LightDiscriminator, InitSpace)]
#[account]
pub struct PlaceholderRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub placeholder_id: u64,
}

impl HasCompressionInfo for PlaceholderRecord {
    fn compression_info(&self) -> &CompressionInfo {
        self.compression_info
            .as_ref()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        self.compression_info
            .as_mut()
            .expect("CompressionInfo must be Some on-chain")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

impl Size for PlaceholderRecord {
    fn size(&self) -> usize {
        Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
    }
}

impl CompressAs for PlaceholderRecord {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        std::borrow::Cow::Owned(Self {
            compression_info: None,
            owner: self.owner,
            name: self.name.clone(),
            placeholder_id: self.placeholder_id,
        })
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid account count: PDAs and compressed accounts must match")]
    InvalidAccountCount,
    #[msg("Rent recipient does not match config")]
    InvalidRentRecipient,
    #[msg("Failed to create compressed mint")]
    MintCreationFailed,
    #[msg("Compressed token program account not found in remaining accounts")]
    MissingCompressedTokenProgram,
    #[msg("Compressed token program authority PDA account not found in remaining accounts")]
    MissingCompressedTokenProgramAuthorityPDA,

    #[msg("CToken decompression not yet implemented")]
    CTokenDecompressionNotImplemented,
}

// Add these struct definitions before the program module
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AccountCreationData {
    pub user_name: String,
    pub session_id: u64,
    pub game_type: String,
    // TODO: Add mint metadata fields when implementing mint functionality
    pub mint_name: String,
    pub mint_symbol: String,
    pub mint_uri: String,
    pub mint_decimals: u8,
    pub mint_supply: u64,
    pub mint_update_authority: Option<Pubkey>,
    pub mint_freeze_authority: Option<Pubkey>,
    pub additional_metadata: Option<Vec<(String, String)>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CompressionParams {
    pub proof: ValidityProof,
    pub user_compressed_address: [u8; 32],
    pub user_address_tree_info: PackedAddressTreeInfo,
    pub user_output_state_tree_index: u8,
    pub game_compressed_address: [u8; 32],
    pub game_address_tree_info: PackedAddressTreeInfo,
    pub game_output_state_tree_index: u8,
    // TODO: Add mint compression parameters when implementing mint functionality
    // pub mint_compressed_address: [u8; 32],
    // pub mint_address_tree_info: PackedAddressTreeInfo,
    // pub mint_output_state_tree_index: u8,
    pub mint_bump: u8,
    pub mint_with_context: CompressedMintWithContext,
}

#[inline]
pub fn account_meta_from_account_info(account_info: &AccountInfo) -> AccountMeta {
    AccountMeta {
        pubkey: *account_info.key,
        is_signer: account_info.is_signer,
        is_writable: account_info.is_writable,
    }
}
