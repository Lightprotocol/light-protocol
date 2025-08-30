use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::AccountMeta,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
    },
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
    instruction::{account_meta::CompressedAccountMeta, PackedAddressTreeInfo, ValidityProof},
    light_hasher::{DataHasher, Hasher},
    sha::LightAccount,
    LightDiscriminator, LightHasher,
};
use light_sdk_types::{
    CpiAccountsConfig, CpiAccountsSmall, CpiSigner, COMPRESSED_TOKEN_PROGRAM_ID,
};

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

// pub const ADDRESS_SPACE: [Pubkey; 1] = [pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK")];

// Simple structure for CToken decompression - no complex packing
#[derive(Debug, Clone)]
struct CTokenDecompressionInfo {
    /// Target decompressed account index
    target_account_index: usize,
    /// Token data
    token_data: CTokenAccountData,
    /// Compressed account metadata
    compressed_meta: CompressedAccountMeta,
    /// Bump for PDA
    #[allow(dead_code)]
    bump: u8,
}

// target
// TODO: use actual seeds or some.
pub fn mock_compressible_token_account_address(
    user: &Pubkey,
    mint_signer: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[b"ctoken_signer", user.as_ref(), mint_signer.as_ref()],
        program_id,
    )
    .0
}

// TODO: move to sdk. consider different derivation path.
// Helper function to derive PDA for program-owned CToken accounts
fn derive_ctoken_pda(target_account: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    // PDA is derived from the target decompressed account address This creates
    // association between compressed and decompressed accounts, without us
    // having to create compressed addresses for each token account.
    Pubkey::find_program_address(&[b"ctoken_signer", target_account.as_ref()], program_id)
}

/// Process CToken accounts and prepare for decompression
fn prepare_ctoken_accounts(
    ctoken_accounts: &[(usize, CompressedAccountData, CTokenAccountData, u8)],
    solana_accounts: &[AccountInfo],
    program_id: &Pubkey,
) -> Result<(Vec<CTokenDecompressionInfo>, Vec<(usize, Vec<Vec<u8>>)>)> {
    let mut infos = Vec::new();
    let mut pda_seeds_collection = Vec::new();

    for (index, compressed_data, token_data, bump) in ctoken_accounts {
        let target_account = solana_accounts[*index].key();

        // Determine if this account is program-owned
        let (expected_pda, pda_bump) = derive_ctoken_pda(&target_account, program_id);
        let is_program_owned = token_data.owner == expected_pda;

        // Store PDA seeds for program-owned accounts
        if is_program_owned {
            pda_seeds_collection.push((
                *index,
                vec![
                    b"ctoken_signer".to_vec(),
                    target_account.to_bytes().to_vec(),
                    vec![pda_bump],
                ],
            ));
        }

        infos.push(CTokenDecompressionInfo {
            target_account_index: *index,
            token_data: token_data.clone(),
            compressed_meta: compressed_data.meta.clone(),
            bump: *bump,
        });
    }

    Ok((infos, pda_seeds_collection))
}

/// Creates CTokenAccount2 instances for decompression - simplified
fn create_token_accounts_for_decompression_simple(
    infos: &[CTokenDecompressionInfo],
    mint_account_index: u8,
    owner_account_index: u8,
    target_accounts_offset: u8,
) -> Result<Vec<light_compressed_token_sdk::account2::CTokenAccount2>> {
    use light_compressed_token_sdk::account2::CTokenAccount2;
    use light_ctoken_types::instructions::transfer2::MultiInputTokenDataWithContext;

    let mut token_accounts = Vec::new();

    for (i, info) in infos.iter().enumerate() {
        // Extract tree info from CompressedAccountMeta
        let tree_info = &info.compressed_meta.tree_info;

        // Create input data for this specific account
        let input_data = MultiInputTokenDataWithContext {
            amount: info.token_data.amount,
            merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
                merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: tree_info.queue_pubkey_index,
                leaf_index: tree_info.leaf_index,
                prove_by_index: tree_info.prove_by_index,
            },
            root_index: tree_info.root_index,
            mint: mint_account_index,
            owner: owner_account_index,
            has_delegate: info.token_data.delegate.is_some(),
            delegate: 0, // Not supported yet
            version: 2,  // V2 for batched Merkle trees
        };

        // Create CTokenAccount2 for this specific account
        let mut ctoken_account = CTokenAccount2::new(vec![input_data], 0)
            .map_err(|_| ErrorCode::CTokenDecompressionNotImplemented)?;

        // Decompress to the specific target account
        let target_account_index = target_accounts_offset + i as u8;
        ctoken_account
            .decompress(info.token_data.amount, target_account_index)
            .map_err(|_| ErrorCode::CTokenDecompressionNotImplemented)?;

        token_accounts.push(ctoken_account);
    }

    Ok(token_accounts)
}

/// Builds the Transfer2 configuration based on CPI context requirements
fn build_transfer_config(
    with_cpi_context: bool,
    cpi_accounts: &CpiAccountsSmall<'_, AccountInfo>,
) -> Result<light_compressed_token_sdk::instructions::transfer2::Transfer2Config> {
    use light_compressed_token_sdk::instructions::transfer2::Transfer2Config;

    if with_cpi_context {
        Ok(Transfer2Config {
            cpi_context_pubkey: Some(
                cpi_accounts
                    .cpi_context()
                    .map_err(|_| ErrorCode::MissingCompressedTokenProgram)?
                    .key(),
            ),
            cpi_context: Some(
                light_compressed_account::instruction_data::cpi_context::CompressedCpiContext {
                    set_context: false,           // false for first write
                    first_set_context: true,      // true to clear any previous context
                    cpi_context_account_index: 0, // Index of CPI context account
                },
            ),
            ..Transfer2Config::default()
        })
    } else {
        Ok(Transfer2Config::default())
    }
}

// Main entry point for CToken decompression - SIMPLIFIED
// Uses CPI context to write state transition without settling (Light Protocol optimization)
fn invoke_ctoken_decompression<'a, 'info>(
    fee_payer: &AccountInfo<'info>,
    compressed_token_program: &UncheckedAccount<'info>,
    compressed_token_cpi_authority: &UncheckedAccount<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    program_id: Pubkey,
    ctoken_accounts: Vec<(usize, CompressedAccountData, CTokenAccountData, u8)>,
    solana_accounts: &[AccountInfo<'info>], // Target decompressed accounts
    cpi_accounts: &CpiAccountsSmall<'a, AccountInfo<'info>>,
    proof: ValidityProof,
    with_cpi_context: bool,
) -> Result<()> {
    use light_compressed_token_sdk::instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Inputs,
    };

    // Validate compressed token program ID
    let expected_program_id = Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID);
    if compressed_token_program.key() != expected_program_id {
        msg!(
            "Invalid compressed token program: expected {}, got {}",
            expected_program_id,
            compressed_token_program.key()
        );
        return err!(ErrorCode::MissingCompressedTokenProgram);
    }

    // Process CToken accounts
    let (infos, pda_seeds_collection) =
        prepare_ctoken_accounts(&ctoken_accounts, solana_accounts, &program_id)?;

    // Validate all accounts have the same mint (for now, single mint support)
    if !infos.is_empty() {
        let first_mint = infos[0].token_data.mint;

        for info in &infos {
            if info.token_data.mint != first_mint {
                msg!("Error: Multiple mints not supported in single decompression");
                return Err(ProgramError::InvalidArgument.into());
            }
            if info.token_data.delegate.is_some() {
                msg!("Error: Delegate functionality is not yet implemented");
                return Err(ProgramError::InvalidArgument.into());
            }
        }
    }

    // Build simple account metas - directly use the accounts we need
    let mut packed_account_metas = Vec::new();

    // Get the first token info to extract mint and owner
    let first_info = &infos[0];

    // Add mint (index 0)
    packed_account_metas.push(AccountMeta::new_readonly(first_info.token_data.mint, false));



    // Add all target accounts (writable for decompression)
    // PDAs that are being created need to be signers!
    for info in &infos {
        let target_account = *solana_accounts[info.target_account_index].key;
        // Check if this target account is a PDA (program-owned)
        let is_pda = pda_seeds_collection
            .iter()
            .any(|(idx, _)| *idx == info.target_account_index);
        
        // PDAs being created/initialized must be signers
        packed_account_metas.push(AccountMeta::new(target_account, is_pda));
    }

    msg!("packed_account_metas: {:?}", packed_account_metas);

    // Create token accounts for decompression
    let token_accounts = create_token_accounts_for_decompression_simple(
        &infos, 0, // mint is at index 0
        1, // owner is at index 1
        2, // target accounts start at index 2
    )?;

    msg!("created cTokenAccounts2: {:?}", token_accounts);

    // Build transfer configuration
    let transfer_config = build_transfer_config(with_cpi_context, cpi_accounts)?;

    msg!("transfer_config: {:?}", transfer_config);

    // Create the transfer2 instruction
    let ix = {
        let inputs = Transfer2Inputs {
            validity_proof: proof.into(),
            transfer_config,
            meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
                fee_payer.key(),
                packed_account_metas,
            ),
            in_lamports: None,
            out_lamports: None,
            token_accounts,
        };

        create_transfer2_instruction(inputs)
            .map_err(|_| ErrorCode::CTokenDecompressionNotImplemented)?
    };

    msg!("transfer2 ix: {:?}", ix);

    // Build account infos for CPI - SIMPLE AND DIRECT
    let mut account_infos = Vec::new();

    // Add fixed accounts
    account_infos.push(compressed_token_cpi_authority.to_account_info());
    account_infos.push(compressed_token_program.to_account_info());

    // Add mint account - find it in remaining_accounts
    let mint_pubkey = first_info.token_data.mint;
    let mint_account = remaining_accounts
        .iter()
        .find(|acc| acc.key() == mint_pubkey)
        .ok_or_else(|| {
            msg!(
                "Error: Mint account {} not found in remaining_accounts",
                mint_pubkey
            );
            ProgramError::NotEnoughAccountKeys
        })?;
    account_infos.push(mint_account.clone());

    // Add owner account - find it in solana_accounts or remaining_accounts
    let owner_pubkey = first_info.token_data.owner;
    let owner_account = solana_accounts
        .iter()
        .find(|acc| acc.key() == owner_pubkey)
        .or_else(|| remaining_accounts.iter().find(|acc| acc.key() == owner_pubkey))
        .ok_or_else(|| {
            msg!(
                "Error: Owner account {} not found in accounts",
                owner_pubkey
            );
            ProgramError::NotEnoughAccountKeys
        })?;
    account_infos.push(owner_account.clone());

    // Add target accounts
    for info in &infos {
        account_infos.push(solana_accounts[info.target_account_index].to_account_info());
    }

    // Add CPI context if needed
    if with_cpi_context {
        let cpi_context = cpi_accounts.cpi_context().map_err(|_| {
            msg!("Error: Missing CPI context account");
            ErrorCode::MissingCompressedTokenProgram
        })?;
        account_infos.push(cpi_context.to_account_info());
    }

    msg!("account_infos len: {}", account_infos.len());
    for (i, acc) in account_infos.iter().enumerate() {
        msg!("account_infos[{}]: {}", i, acc.key());
    }

    // Execute CPI with appropriate signing
    if !pda_seeds_collection.is_empty() {
        // For program-owned accounts, use invoke_signed with PDA seeds
        let seeds_refs: Vec<Vec<&[u8]>> = pda_seeds_collection
            .iter()
            .map(|(_, seeds)| seeds.iter().map(|s| s.as_slice()).collect())
            .collect();
        let seeds_slice_refs: Vec<&[&[u8]]> = seeds_refs.iter().map(|s| s.as_slice()).collect();
        invoke_signed(&ix, &account_infos, &seeds_slice_refs)?;
    } else {
        // For external accounts, use regular invoke
        invoke(&ix, &account_infos)?;
    }

    Ok(())
}

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible {

    use light_compressed_token_sdk::instructions::{create_mint_action_cpi, MintActionInputs};
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
        proof: ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        bumps: Vec<u8>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        // Get PDA accounts from remaining accounts (current client implementation)
        msg!(
            "program: passed system_accounts_offset: {:?}",
            system_accounts_offset
        );
        let pda_accounts_end = system_accounts_offset as usize;
        let solana_accounts = &ctx.remaining_accounts[..pda_accounts_end];

        msg!("program: solana_accounts len: {:?}", solana_accounts.len());
        msg!(
            "program: remaining_accounts len: {:?}",
            ctx.remaining_accounts.len()
        );
        // Validate we have matching number of PDAs, compressed accounts, and bumps
        if solana_accounts.len() != compressed_accounts.len()
            || solana_accounts.len() != bumps.len()
        {
            return err!(ErrorCode::InvalidAccountCount);
        }

        let fee_payer_account_info = ctx.accounts.fee_payer.to_account_info();
        let config = CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER);
        let cpi_accounts = CpiAccountsSmall::new_with_config(
            &fee_payer_account_info,
            &ctx.remaining_accounts[system_accounts_offset as usize..],
            config,
        );

        msg!(
            "program: cpi_accounts len: {:?}",
            cpi_accounts.account_infos().len()
        );
        msg!("program: tree_accounts: {:?}", cpi_accounts.tree_accounts());

        // Get address space from config checked.
        let config = CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;
        let address_space = config.address_space[0];

        // Separate CToken accounts from other compressible accounts
        let mut ctoken_accounts = Vec::new();
        let mut other_compressed_infos = Vec::new();

        for (i, (compressed_data, &bump)) in compressed_accounts
            .into_iter()
            .zip(bumps.iter())
            .enumerate()
        {
            let bump_slice = [bump];

            match compressed_data.data {
                CompressedAccountVariant::CTokenAccount(ref data) => {
                    // Collect CToken accounts for batch processing via compressed token program
                    ctoken_accounts.push((i, compressed_data.clone(), data.clone(), bump));
                }
                CompressedAccountVariant::UserRecord(data) => {
                    let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                    for seed in &compressed_data.seeds {
                        seeds_refs.push(seed.as_slice());
                    }
                    seeds_refs.push(&bump_slice);

                    // Create sha::LightAccount with correct UserRecord discriminator
                    let light_account = LightAccount::<'_, UserRecord>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data,
                    )?;

                    // Process this single UserRecord account
                    let solana_account_slice = vec![&solana_accounts[i]];
                    let light_accounts = vec![light_account];
                    let seeds_slice = seeds_refs.as_slice();
                    let seeds_array = vec![seeds_slice];
                    let cpi_accounts_box = Box::new(cpi_accounts.clone());

                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<UserRecord>(
                        solana_account_slice,
                        light_accounts,
                        seeds_array,
                        &cpi_accounts_box,
                        &ctx.accounts.rent_payer,
                        address_space,
                    )?;

                    other_compressed_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::GameSession(data) => {
                    // Build seeds refs without cloning - pre-allocate capacity
                    let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                    for seed in &compressed_data.seeds {
                        seeds_refs.push(seed.as_slice());
                    }
                    seeds_refs.push(&bump_slice);

                    // Create sha::LightAccount with correct GameSession discriminator
                    let light_account = LightAccount::<'_, GameSession>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data,
                    )?;

                    // Process this single GameSession account
                    let solana_account_slice = vec![&solana_accounts[i]];
                    let light_accounts = vec![light_account];
                    let seeds_slice = seeds_refs.as_slice();
                    let seeds_array = vec![seeds_slice];
                    let cpi_accounts_box = Box::new(cpi_accounts.clone());

                    let compressed_infos = prepare_accounts_for_decompress_idempotent::<GameSession>(
                        solana_account_slice,
                        light_accounts,
                        seeds_array,
                        &cpi_accounts_box,
                        &ctx.accounts.rent_payer,
                        address_space,
                    )?;
                    other_compressed_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::PlaceholderRecord(data) => {
                    let mut seeds_refs = Vec::with_capacity(compressed_data.seeds.len() + 1);
                    for seed in &compressed_data.seeds {
                        seeds_refs.push(seed.as_slice());
                    }
                    seeds_refs.push(&bump_slice);

                    // Create sha::LightAccount with correct PlaceholderRecord discriminator
                    let light_account = LightAccount::<'_, PlaceholderRecord>::new_mut(
                        &crate::ID,
                        &compressed_data.meta,
                        data,
                    )?;

                    // Process this single PlaceholderRecord account
                    let solana_account_slice = vec![&solana_accounts[i]];
                    let light_accounts = vec![light_account];
                    let seeds_slice = seeds_refs.as_slice();
                    let seeds_array = vec![seeds_slice];
                    let cpi_accounts_box = Box::new(cpi_accounts.clone());

                    let compressed_infos =
                        prepare_accounts_for_decompress_idempotent::<PlaceholderRecord>(
                            solana_account_slice,
                            light_accounts,
                            seeds_array,
                            &cpi_accounts_box,
                            &ctx.accounts.rent_payer,
                            address_space,
                        )?;

                    other_compressed_infos.extend(compressed_infos);
                }
            }
        }

        // Process accounts using Light Protocol's CPI context optimization:
        // 1. CToken decompression writes STF to context (doesn't settle)
        // 2. PDA decompression settles everything in one invocation

        let has_ctoken_accounts = !ctoken_accounts.is_empty();
        let has_pda_accounts = !other_compressed_infos.is_empty();

        if has_ctoken_accounts && has_pda_accounts {
            // Both CTokens and PDAs: use CPI context for efficiency
            for (i, account) in ctx.remaining_accounts.iter().enumerate() {
                msg!("remaining_accounts[{}]: {}", i, account.key());
            }

            for (i, account) in ctoken_accounts.iter().enumerate() {
                msg!("ctoken_accounts[{}]: {:?}", i, account);
            }

            // Step 1: CToken decompression writes to CPI context (no settlement)
            super::invoke_ctoken_decompression(
                &ctx.accounts.fee_payer,
                ctx.accounts.compressed_token_program.as_ref().unwrap(),
                ctx.accounts
                    .compressed_token_cpi_authority
                    .as_ref()
                    .unwrap(),
                &ctx.remaining_accounts[system_accounts_offset as usize..], // Pass only the system accounts
                *ctx.program_id,
                ctoken_accounts,
                solana_accounts,
                &cpi_accounts,
                proof.clone(),
                true, // with_cpi_context = true since we have PDAs to process next
            )?;

            // Step 2: PDA decompression settles both CToken STF (from context) and PDA STF
            let cpi_inputs = CpiInputs::new(proof, other_compressed_infos).with_last_cpi_context(0); // Read and settle the CPI context
            cpi_inputs.invoke_light_system_program_small(cpi_accounts)?;
        } else if has_ctoken_accounts {
            // Only CTokens: invoke without CPI context (settles immediately)
            super::invoke_ctoken_decompression(
                &ctx.accounts.fee_payer,
                ctx.accounts.compressed_token_program.as_ref().unwrap(),
                ctx.accounts
                    .compressed_token_cpi_authority
                    .as_ref()
                    .unwrap(),
                &ctx.remaining_accounts[system_accounts_offset as usize..], // Pass only the system accounts
                *ctx.program_id,
                ctoken_accounts,
                solana_accounts,
                &cpi_accounts,
                proof,
                false, // with_cpi_context = false, no PDAs to process
            )?;
        } else if has_pda_accounts {
            // Only PDAs: normal invocation without CPI context
            let cpi_inputs = CpiInputs::new(proof, other_compressed_infos);
            cpi_inputs.invoke_light_system_program_small(cpi_accounts)?;
        } else {
            msg!("No compressed accounts to decompress");
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
        msg!("program: 0011 - create_user_record_and_game_session");
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

        msg!(
            "program: cpi_accounts.cpi_context(): {:?}",
            cpi_accounts.cpi_context()
        );

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

        msg!("invoke .pda");

        let cpi_context_accounts = CpiContextWriteAccounts {
            fee_payer: cpi_accounts.fee_payer(),
            authority: cpi_accounts.authority().unwrap(),
            cpi_context: cpi_context_account,
            cpi_signer: LIGHT_CPI_SIGNER,
        };
        cpi_inputs.invoke_light_system_program_cpi_context(cpi_context_accounts)?;

        // these are custom seeds of the caller program that are used to derive the program owned onchain tokenb account PDA.
        // dual use: as owner of the compressed token account.
        let token_account_address = mock_compressible_token_account_address(
            &ctx.accounts.user.key(),
            &ctx.accounts.mint_signer.key(),
            &crate::ID,
        );
        msg!("user: {:?}", ctx.accounts.user.key());
        msg!("mint_signer: {:?}", ctx.accounts.mint_signer.key());
        msg!(
            "derived_recipient_owner: {:?}",
            token_account_address.to_string()
        );
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

        msg!("invoke token start!");
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
        // account_infos.push(ctx.accounts.token_account.to_account_info());
        msg!(
            "mint_action_instruction {:?}",
            mint_action_instruction.accounts
        );
        // msg!("account_infos {:?}", account_infos);
        msg!(
            "account infos pubkeys {:?}",
            account_infos
                .iter()
                .map(|info| info.key)
                .collect::<Vec<_>>()
        );
        // Invoke the mint action instruction directly
        invoke(&mint_action_instruction, &account_infos)?;

        msg!("invoke token done!");
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

        Ok(())
    }
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

// TODO: split into one ix with ctoken and one without.
#[derive(Accounts)]
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
    CTokenAccount(CTokenAccountData), // Must always be included if used for cTokens.
}
// CToken account structure that matches native token accounts
#[derive(Default, Debug, LightHasher, LightDiscriminator, InitSpace)]
#[account]
pub struct CTokenAccountData {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub mint: Pubkey,
    #[hash]
    pub owner: Pubkey,
    pub amount: u64,
    #[hash]
    pub delegate: Option<Pubkey>,
    pub state: u8, // AccountState: Initialized/Frozen
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    #[hash]
    pub close_authority: Option<Pubkey>,
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
            Self::CTokenAccount(data) => data.hash::<H>(),
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
            Self::CTokenAccount(data) => data.compression_info(),
        }
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info_mut(),
            Self::GameSession(data) => data.compression_info_mut(),
            Self::PlaceholderRecord(data) => data.compression_info_mut(),
            Self::CTokenAccount(data) => data.compression_info_mut(),
        }
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        match self {
            Self::UserRecord(data) => data.compression_info_mut_opt(),
            Self::GameSession(data) => data.compression_info_mut_opt(),
            Self::PlaceholderRecord(data) => data.compression_info_mut_opt(),
            Self::CTokenAccount(data) => data.compression_info_mut_opt(),
        }
    }

    fn set_compression_info_none(&mut self) {
        match self {
            Self::UserRecord(data) => data.set_compression_info_none(),
            Self::GameSession(data) => data.set_compression_info_none(),
            Self::PlaceholderRecord(data) => data.set_compression_info_none(),
            Self::CTokenAccount(data) => data.set_compression_info_none(),
        }
    }
}

impl Size for CompressedAccountVariant {
    fn size(&self) -> usize {
        match self {
            Self::UserRecord(data) => data.size(),
            Self::GameSession(data) => data.size(),
            Self::PlaceholderRecord(data) => data.size(),
            Self::CTokenAccount(data) => data.size(),
        }
    }
}

// Auto-derived via macro. Ix data implemented for Variant.
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData {
    pub meta: CompressedAccountMeta,
    pub data: CompressedAccountVariant,
    pub seeds: Vec<Vec<u8>>,
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

impl HasCompressionInfo for CTokenAccountData {
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

impl Size for CTokenAccountData {
    fn size(&self) -> usize {
        Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
    }
}
impl CompressAs for CTokenAccountData {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        // Return owned data with compression_info = None for compressed storage
        std::borrow::Cow::Owned(Self {
            compression_info: None,
            mint: self.mint,
            owner: self.owner,
            amount: self.amount,
            delegate: self.delegate,
            state: self.state,
            is_native: self.is_native,
            delegated_amount: self.delegated_amount,
            close_authority: self.close_authority,
        })
    }
}

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
