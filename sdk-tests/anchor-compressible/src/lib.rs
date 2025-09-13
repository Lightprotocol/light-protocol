use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::AccountMeta,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
    },
};
use anchor_spl::token_interface::TokenAccount;
use light_ctoken_types::{
    instructions::mint_action::CompressedMintWithContext, COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::{
    account::Size,
    compressible::{
        compress_account_on_init, compress_empty_account_on_init,
        compression_info::CompressedInitSpace, prepare_account_for_decompression_idempotent,
        prepare_accounts_for_compression_on_init, process_initialize_compression_config_checked,
        process_update_compression_config, CompressAs, CompressibleConfig, CompressionInfo,
        HasCompressionInfo, Pack, Unpack,
    },
    cpi::CpiInputs,
    derive_light_cpi_signer,
    instruction::{
        account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAccounts,
        PackedAddressTreeInfo, ValidityProof,
    },
    light_hasher::{DataHasher, Hasher},
    token::{CompressibleTokenDataWithVariant, PackedCompressibleTokenDataWithVariant},
    LightDiscriminator, LightHasher,
};
pub const POOL_VAULT_SEED: &str = "pool_vault";
pub const USER_RECORD_SEED: &str = "user_record";
pub const CTOKEN_SIGNER_SEED: &str = "ctoken_signer";
#[repr(u32)]
pub enum ErrorCode {
    InvalidAccountCount,
    InvalidRentRecipient,
    MintCreationFailed,
    MissingCompressedTokenProgram,
    MissingCompressedTokenProgramAuthorityPDA,
}
#[automatically_derived]
impl ::core::fmt::Debug for ErrorCode {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                ErrorCode::InvalidAccountCount => "InvalidAccountCount",
                ErrorCode::InvalidRentRecipient => "InvalidRentRecipient",
                ErrorCode::MintCreationFailed => "MintCreationFailed",
                ErrorCode::MissingCompressedTokenProgram => "MissingCompressedTokenProgram",
                ErrorCode::MissingCompressedTokenProgramAuthorityPDA => {
                    "MissingCompressedTokenProgramAuthorityPDA"
                }
            },
        )
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            ErrorCode::InvalidAccountCount => fmt.write_fmt(format_args!(
                "Invalid account count: PDAs and compressed accounts must match",
            )),
            ErrorCode::InvalidRentRecipient => {
                fmt.write_fmt(format_args!("Rent recipient does not match config"))
            }
            ErrorCode::MintCreationFailed => {
                fmt.write_fmt(format_args!("Failed to create compressed mint"))
            }
            ErrorCode::MissingCompressedTokenProgram => fmt.write_fmt(format_args!(
                "Compressed token program account not found in remaining accounts",
            )),
            ErrorCode::MissingCompressedTokenProgramAuthorityPDA => fmt.write_fmt(format_args!(
                "Compressed token program authority PDA account not found in remaining accounts",
            )),
        }
    }
}
extern crate alloc;
#[repr(u32)]
/// Auto-generated error codes for compressible instructions
/// These are separate from the user's ErrorCode enum to avoid conflicts
pub enum CompressibleInstructionError {
    InvalidRentRecipient,
    CTokenDecompressionNotImplemented,
    PdaDecompressionNotImplemented,
    TokenCompressionNotImplemented,
    PdaCompressionNotImplemented,
}
// Auto-generated client-side seed function
pub fn get_userrecord_seeds(owner: &Pubkey) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
    let mut seed_values = Vec::with_capacity(2usize + 1);
    seed_values.push((USER_RECORD_SEED.as_bytes()).to_vec());
    seed_values.push((owner.as_ref()).to_vec());
    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
    seed_values.push(<[_]>::into_vec(Box::new([bump])));
    (seed_values, pda)
}
/// Auto-generated client-side seed function
pub fn get_gamesession_seeds(session_id: u64) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
    let mut seed_values = Vec::with_capacity(2usize + 1);
    seed_values.push(("game_session".as_bytes()).to_vec());
    seed_values.push((session_id.to_le_bytes().as_ref()).to_vec());
    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
    seed_values.push(<[_]>::into_vec(Box::new([bump])));
    (seed_values, pda)
}
/// Auto-generated client-side seed function
pub fn get_placeholderrecord_seeds(
    placeholder_id: u64,
) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
    let mut seed_values = Vec::with_capacity(2usize + 1);
    seed_values.push(("placeholder_record".as_bytes()).to_vec());
    seed_values.push((placeholder_id.to_le_bytes().as_ref()).to_vec());
    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
    seed_values.push(<[_]>::into_vec(Box::new([bump])));
    (seed_values, pda)
}
/// Auto-generated client-side CToken seed function
pub fn get_ctokensigner_seeds(
    fee_payer: &anchor_lang::prelude::Pubkey,
    some_mint: &anchor_lang::prelude::Pubkey,
) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
    let mut seed_values = Vec::with_capacity(3usize + 1);
    seed_values.push((CTOKEN_SIGNER_SEED.as_bytes()).to_vec());
    seed_values.push((fee_payer.as_ref()).to_vec());
    seed_values.push((some_mint.as_ref()).to_vec());
    let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
    let (pda, bump) = anchor_lang::prelude::Pubkey::find_program_address(&seed_slices, &crate::ID);
    seed_values.push(<[_]>::into_vec(Box::new([bump])));
    (seed_values, pda)
}
/// Trait-based system for generic CToken variant seed handling
/// Users implement this trait for their CTokenAccountVariant enum
pub mod ctoken_seed_system {
    use super::*;
    /// Context struct providing access to ALL instruction accounts
    /// This gives users access to any account in the instruction context
    pub struct CTokenSeedContext<'a, 'info> {
        pub accounts: &'a DecompressAccountsIdempotent<'info>,
        pub remaining_accounts: &'a [anchor_lang::prelude::AccountInfo<'info>],
    }
    /// Trait that CToken variants implement to provide seed derivation
    /// Completely extensible - users can implement ANY seed logic with access to ALL accounts
    pub trait CTokenSeedProvider {
        fn get_seeds<'a, 'info>(
            &self,
            ctx: &CTokenSeedContext<'a, 'info>,
        ) -> (Vec<Vec<u8>>, Pubkey);
    }
}
/// Auto-generated CTokenSeedProvider implementation
impl ctoken_seed_system::CTokenSeedProvider for CTokenAccountVariant {
    fn get_seeds<'a, 'info>(
        &self,
        ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
    ) -> (Vec<Vec<u8>>, anchor_lang::prelude::Pubkey) {
        match self {
            CTokenAccountVariant::CTokenSigner => {
                let seed_1 = ctx.accounts.fee_payer.key().to_bytes();
                let seed_2 = ctx.accounts.some_mint.key().to_bytes();
                let seeds: &[&[u8]] = &[CTOKEN_SIGNER_SEED.as_bytes(), &seed_1, &seed_2];
                let (pda, bump) =
                    anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                seeds_vec.push(<[_]>::into_vec(Box::new([bump])));
                (seeds_vec, pda)
            }
            _ => {
                panic!("CToken variant not configured with seeds");
            }
        }
    }
}
use light_sdk_types::{CpiAccountsConfig, CpiAccountsSmall, CpiSigner};

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");
pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

// You can implement this for each of your token account derivation paths.
pub fn get_ctoken_signer_seeds<'a>(user: &'a Pubkey, mint: &'a Pubkey) -> (Vec<Vec<u8>>, Pubkey) {
    let mut seeds = vec![
        b"ctoken_signer".to_vec(),
        user.to_bytes().to_vec(),
        mint.to_bytes().to_vec(),
    ];
    let seeds_slice = seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>();
    let (pda, bump) = Pubkey::find_program_address(seeds_slice.as_slice(), &crate::ID);
    seeds.push(vec![bump]);
    (seeds, pda)
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
#[repr(u8)]
pub enum CTokenAccountVariant {
    CTokenSigner = 0,
    AssociatedTokenAccount = 255, // TODO: add support.
}

// Simple anchor program retrofitted with compressible accounts.
#[program]
pub mod anchor_compressible {

    use light_compressed_token_sdk::{
        instructions::{
            compress_and_close::compress_and_close_ctoken_accounts_signed,
            create_mint_action_cpi,
            create_token_account::{
                create_compressible_token_account_signed, CreateCompressibleTokenAccount,
                CreateCompressibleTokenAccountSigned,
            },
            decompress_full_ctoken_accounts_with_indices, derive_pool_pda, find_spl_mint_address,
            DecompressFullIndices, MintActionInputs,
        },
        CPI_AUTHORITY_PDA_SEED,
    };
    use light_sdk::compressible::{
        compress_account::prepare_account_for_compression, into_compressed_meta_with_address,
    };
    use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;

    use super::*;

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

    /// Compress multiple accounts (PDAs and token accounts) in a single instruction.
    pub fn compress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, 'info, 'info, CompressAccountsIdempotent<'info>>,
        proof: ValidityProof,
        compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
        signer_seeds: Vec<Vec<Vec<u8>>>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        let compression_config =
            CompressibleConfig::load_checked(&ctx.accounts.config, &crate::ID)?;
        if ctx.accounts.rent_recipient.key() != compression_config.rent_recipient {
            msg!(
                "rent recipient passed: {:?}",
                ctx.accounts.rent_recipient.key()
            );
            msg!(
                "rent recipient config: {:?}",
                compression_config.rent_recipient
            );
            panic!("Rent recipient does not match config");
            // return err!(CompressibleInstructionError::InvalidRentRecipient);
        }

        let cpi_accounts = CpiAccountsSmall::new(
            ctx.accounts.fee_payer.as_ref(),
            &ctx.remaining_accounts[system_accounts_offset as usize..],
            LIGHT_CPI_SIGNER,
        );

        // we use signer_seeds because compressed_accounts can be != accounts to
        // decompress.
        let pda_accounts_start = ctx.remaining_accounts.len() - signer_seeds.len();
        let solana_accounts = &ctx.remaining_accounts[pda_accounts_start..];

        // Implement for tokens and for each of your program's compressible
        // account types.
        let mut token_accounts_to_compress = Vec::new();
        let mut compressed_pda_infos = Vec::new();
        let mut user_records = Vec::new();
        let mut game_sessions = Vec::new();
        let mut placeholder_records = Vec::new();

        for (i, account_info) in solana_accounts.iter().enumerate() {
            if account_info.data_is_empty() {
                msg!("No data. Account already compressed or uninitialized. Skipping.");
                continue;
            }
            if account_info.owner == &COMPRESSED_TOKEN_PROGRAM_ID.into() {
                if let Ok(token_account) = InterfaceAccount::<TokenAccount>::try_from(account_info)
                {
                    let account_signer_seeds = signer_seeds[i].clone();

                    token_accounts_to_compress.push(
                        light_compressed_token_sdk::TokenAccountToCompress {
                            token_account,
                            signer_seeds: account_signer_seeds,
                        },
                    );
                }
            } else if account_info.owner == &crate::ID {
                let data = account_info.try_borrow_data()?;
                // if data.len() < 8 {
                //     msg!("No. Account already compressed or uninitialized. Skipping.");
                //     continue;
                // }

                let discriminator = &data[0..8];
                let meta = compressed_accounts[i];

                // TOOD: consider CHECKING seeds.
                match discriminator {
                    d if d == UserRecord::discriminator() => {
                        let mut anchor_account = Account::<UserRecord>::try_from(account_info)?;

                        let compressed_info = prepare_account_for_compression::<UserRecord>(
                            &crate::ID,
                            &mut anchor_account,
                            &meta,
                            &cpi_accounts,
                            &compression_config.compression_delay,
                            &compression_config.address_space,
                        )?;

                        user_records.push(anchor_account);
                        compressed_pda_infos.push(compressed_info);
                    }
                    d if d == GameSession::discriminator() => {
                        let mut anchor_account = Account::<GameSession>::try_from(account_info)?;
                        let compressed_info = prepare_account_for_compression::<GameSession>(
                            &crate::ID,
                            &mut anchor_account,
                            &meta,
                            &cpi_accounts,
                            &compression_config.compression_delay,
                            &compression_config.address_space,
                        )?;

                        game_sessions.push(anchor_account);
                        compressed_pda_infos.push(compressed_info);
                    }
                    d if d == PlaceholderRecord::discriminator() => {
                        let mut anchor_account =
                            Account::<PlaceholderRecord>::try_from(account_info)?;
                        let compressed_info = prepare_account_for_compression::<PlaceholderRecord>(
                            &crate::ID,
                            &mut anchor_account,
                            &meta,
                            &cpi_accounts,
                            &compression_config.compression_delay,
                            &compression_config.address_space,
                        )?;

                        placeholder_records.push(anchor_account);
                        compressed_pda_infos.push(compressed_info);
                    }
                    _ => {
                        panic!("Trying to compress with invalid account discriminator");
                    }
                }
            }
        }
        let has_pdas = !compressed_pda_infos.is_empty();
        let has_tokens = !token_accounts_to_compress.is_empty();

        // 1. compress and close token accounts in one CPI (no proof).
        if has_tokens {
            msg!("compressing and closing token accounts");
            {
                // Output queue = first tree account
                let output_queue_info = cpi_accounts.tree_accounts().unwrap()[0].clone();

                // Split CPI accounts into system base and packed accounts
                let all_cpi_infos = cpi_accounts.post_system_accounts().unwrap();

                // Add LIGHT_CPI_SIGNER seeds as the first entry
                let mut signer_seeds: Vec<Vec<&[u8]>> = token_accounts_to_compress
                    .iter()
                    .map(|t| t.signer_seeds.iter().map(|v| v.as_slice()).collect())
                    .collect();
                // LIGHT_CPI_SIGNER seeds constant, e.g. &[b"light-cpi-signer"]
                let mut all_signer_seeds: Vec<&[&[u8]]> =
                    Vec::with_capacity(signer_seeds.len() + 1);
                let authority_seeds = &[CPI_AUTHORITY_PDA_SEED, &[LIGHT_CPI_SIGNER.bump]];
                all_signer_seeds.push(authority_seeds);
                all_signer_seeds.extend(signer_seeds.iter().map(|v| v.as_slice()));
                let signer_seeds = all_signer_seeds;

                let ctoken_infos: Vec<anchor_lang::prelude::AccountInfo<'info>> =
                    token_accounts_to_compress
                        .iter()
                        .map(|t| t.token_account.to_account_info())
                        .collect();

                msg!("authority, {:?}", cpi_accounts.authority().unwrap());

                // Validate and forward compressed token rent recipient derived from authority
                let compressed_token_rent_recipient_info = ctx
                    .accounts
                    .compressed_token_rent_recipient
                    .to_account_info();

                let rent_auth_info = &ctx.accounts.compressed_token_rent_authority;

                let (derived_recipient, _bump) = derive_pool_pda(&rent_auth_info.key());
                if derived_recipient != *compressed_token_rent_recipient_info.key {
                    panic!("Derived compressed token rent recipient must match passed recipient");
                }

                // Invoke the signed variant (authority = token owner, no rent authority)
                compress_and_close_ctoken_accounts_signed(
                    ctx.accounts.fee_payer.to_account_info(),
                    cpi_accounts.authority().unwrap().to_account_info(),
                    false,
                    output_queue_info,
                    &ctoken_infos,
                    all_cpi_infos,
                    &signer_seeds,
                    compressed_token_rent_recipient_info,
                    ctx.accounts
                        .compressed_token_cpi_authority
                        .to_account_info(),
                    ctx.accounts.compressed_token_program.to_account_info(),
                    cpi_accounts
                        .registered_program_pda()
                        .unwrap()
                        .to_account_info(),
                    cpi_accounts.to_account_infos().as_slice(),
                )?;
            }
            msg!("token accounts compressed and closed");
        }
        // 2. compress and close PDAs in another CPI (with proof).
        if has_pdas {
            let cpi_inputs = CpiInputs::new(proof, compressed_pda_infos);
            cpi_inputs.invoke_light_system_program_small(cpi_accounts)?;
        }

        // Close all PDA accounts
        for anchor_account in user_records.iter() {
            anchor_account.close(ctx.accounts.rent_recipient.clone())?;
        }
        for anchor_account in game_sessions.iter() {
            anchor_account.close(ctx.accounts.rent_recipient.clone())?;
        }
        for anchor_account in placeholder_records.iter() {
            anchor_account.close(ctx.accounts.rent_recipient.clone())?;
        }

        Ok(())
    }

    // auto-derived via macro. takes the tagged account structs via
    // add_compressible_accounts macro and derives the relevant variant type and
    // dispatcher. The instruction can be used with any number of any of the
    // tagged account structs. It's idempotent; it will not fail if the accounts
    // are already decompressed.
    #[inline(never)]
    pub fn decompress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
        proof: light_sdk::instruction::ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        let compression_config = light_sdk::compressible::CompressibleConfig::load_checked(
            &ctx.accounts.config,
            &crate::ID,
        )?;
        let address_space = compression_config.address_space[0];
        #[inline(never)]
        fn check_account_types(compressed_accounts: &[CompressedAccountData]) -> (bool, bool) {
            let (mut has_tokens, mut has_pdas) = (false, false);
            for c in compressed_accounts {
                match c.data {
                    CompressedAccountVariant::CompressibleTokenAccountPacked(_) => {
                        has_tokens = true;
                    }
                    _ => has_pdas = true,
                }
                if has_tokens && has_pdas {
                    break;
                }
            }
            (has_tokens, has_pdas)
        }
        /// Helper function to process token decompression - separated to avoid stack overflow
        #[inline(never)]
        fn process_tokens<'a, 'b, 'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            remaining_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
            fee_payer: &anchor_lang::prelude::AccountInfo<'info>,
            compressed_token_program: &anchor_lang::prelude::UncheckedAccount<'info>,
            compressed_token_rent_payer: &anchor_lang::prelude::AccountInfo<'info>,
            compressed_token_rent_recipient: &anchor_lang::prelude::AccountInfo<'info>,
            compressed_token_rent_authority: &anchor_lang::prelude::AccountInfo<'info>,
            compressed_token_cpi_authority: &anchor_lang::prelude::UncheckedAccount<'info>,
            config: &anchor_lang::prelude::AccountInfo<'info>,
            compressed_token_accounts: Vec<(
                light_sdk::token::PackedCompressibleTokenDataWithVariant<CTokenAccountVariant>,
                light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            )>,
            proof: light_sdk::instruction::ValidityProof,
            cpi_accounts: &light_sdk::cpi::CpiAccountsSmall<'b, 'info>,
            post_system_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
            has_pdas: bool,
        ) -> Result<()> {
            let mut token_decompress_indices =
                Box::new(Vec::with_capacity(compressed_token_accounts.len()));
            let mut token_signers_seeds =
                Box::new(Vec::with_capacity(compressed_token_accounts.len() * 4));
            let packed_accounts = post_system_accounts;
            use crate::ctoken_seed_system::{CTokenSeedContext, CTokenSeedProvider};
            let seed_context = CTokenSeedContext {
                accounts,
                remaining_accounts,
            };
            let authority = cpi_accounts.authority().unwrap();
            let cpi_context = cpi_accounts.cpi_context().unwrap();

            let (derived_compressed_token_rent_recipient, payer_pda_bump) =
                derive_pool_pda(&compressed_token_rent_authority.key());

            msg!(
                "derived_compressed_token_rent_recipient: {:?}",
                derived_compressed_token_rent_recipient
            );
            msg!(
                "compressed_token_rent_recipient.key: {:?}",
                compressed_token_rent_recipient.key
            );
            msg!(
                "compressed_token_rent_payer.key: {:?}",
                compressed_token_rent_payer.key
            );
            msg!("payer_pda_bump: {:?}", payer_pda_bump);
            if derived_compressed_token_rent_recipient != *compressed_token_rent_recipient.key {
                panic!("Derived compressed token rent recipient must match compressed token rent recipient");
            }
            for (token_data, meta) in compressed_token_accounts.into_iter() {
                let owner_index: u8 = token_data.token_data.owner;
                let mint_index: u8 = token_data.token_data.mint;
                let mint_info = packed_accounts[mint_index as usize].to_account_info();
                let owner_info = packed_accounts[owner_index as usize].to_account_info();
                let (ctoken_signer_seeds, derived_token_account_address) =
                    token_data.variant.get_seeds(&seed_context);
                {
                    // let seed_slices: Vec<&[u8]> =
                    //     ctoken_signer_seeds.iter().map(|s| s.as_slice()).collect();
                    // must match
                    if derived_token_account_address != *owner_info.key {
                        msg!(
                            "derived_token_account_address: {:?}",
                            derived_token_account_address
                        );
                        msg!("owner_info.key: {:?}", owner_info.key);
                        panic!("Derived token account address must match owner_info.key");
                    }

                    // Clone to owned seed vectors so lifetimes are independent of source
                    let owned_signer_seeds: Vec<Vec<u8>> =
                        ctoken_signer_seeds.iter().map(|s| s.clone()).collect();

                    let inputs = CreateCompressibleTokenAccountSigned {
                        payer: fee_payer.clone().to_account_info(),
                        token_account: owner_info.clone(),
                        mint: mint_info.clone(),
                        owner: authority.clone().to_account_info(),
                        rent_authority: compressed_token_rent_authority.clone().to_account_info(),
                        rent_recipient: compressed_token_rent_recipient.clone().to_account_info(),
                        pre_pay_num_epochs: 1,
                        write_top_up_lamports: None,
                        payer_pda_bump,
                        signer_seeds: vec![owned_signer_seeds], // TODO: add seeds for the payer pda.
                    };
                    create_compressible_token_account_signed(inputs)?;
                }
                let decompress_index =
                    light_compressed_token_sdk::instructions::DecompressFullIndices::from((
                        token_data.token_data,
                        meta,
                        owner_index,
                    ));
                token_decompress_indices.push(decompress_index);
                token_signers_seeds.extend(ctoken_signer_seeds);
            }
            let ctoken_ix = light_compressed_token_sdk::instructions::decompress_full_ctoken_accounts_with_indices(
                    fee_payer.key(),
                    proof,
                    if has_pdas { Some(cpi_context.key()) } else { None },
                    &token_decompress_indices,
                    packed_accounts,
                )
                .map_err(anchor_lang::prelude::ProgramError::from)?;
            {
                let mut all_account_infos =
                    <[_]>::into_vec(Box::new([fee_payer.to_account_info()]));
                all_account_infos.extend(compressed_token_cpi_authority.to_account_infos());
                all_account_infos.extend(compressed_token_program.to_account_infos());
                all_account_infos.extend(compressed_token_rent_payer.to_account_infos());
                all_account_infos.extend(compressed_token_rent_recipient.to_account_infos());
                all_account_infos.extend(config.to_account_infos());
                all_account_infos.extend(cpi_accounts.to_account_infos());
                let seed_refs: Vec<&[u8]> =
                    token_signers_seeds.iter().map(|s| s.as_slice()).collect();
                anchor_lang::solana_program::program::invoke_signed(
                    &ctoken_ix,
                    all_account_infos.as_slice(),
                    &[seed_refs.as_slice()],
                )?;
            }
            Ok(())
        }
        let (has_tokens, has_pdas) = check_account_types(&compressed_accounts);
        if !has_tokens && !has_pdas {
            return Ok(());
        }
        let estimated_capacity = compressed_accounts.len();
        let mut compressed_token_accounts: Vec<(
            light_sdk::token::PackedCompressibleTokenDataWithVariant<CTokenAccountVariant>,
            light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
        )> = Vec::with_capacity(estimated_capacity);
        let mut compressed_pda_infos = Vec::with_capacity(estimated_capacity);
        let cpi_accounts = if has_tokens && has_pdas {
            light_sdk_types::CpiAccountsSmall::new_with_config(
                ctx.accounts.fee_payer.as_ref(),
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                light_sdk_types::CpiAccountsConfig::new_with_cpi_context(LIGHT_CPI_SIGNER),
            )
        } else {
            light_sdk_types::CpiAccountsSmall::new(
                ctx.accounts.fee_payer.as_ref(),
                &ctx.remaining_accounts[system_accounts_offset as usize..],
                LIGHT_CPI_SIGNER,
            )
        };
        let pda_accounts_start = ctx.remaining_accounts.len() - compressed_accounts.len();
        let solana_accounts = &ctx.remaining_accounts[pda_accounts_start..];
        let post_system_accounts = cpi_accounts.post_system_accounts().unwrap();
        for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
            let unpacked_data = compressed_data.data.unpack(post_system_accounts)?;
            match unpacked_data {
                CompressedAccountVariant::UserRecord(data) => {
                    let (seeds_vec, _) = {
                        let seeds: &[&[u8]] = &[USER_RECORD_SEED.as_bytes(), (data.owner).as_ref()];
                        let (pda, bump) =
                            anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                        seeds_vec.push(seeds[0usize].to_vec());
                        seeds_vec.push(seeds[1usize].to_vec());
                        seeds_vec.push(<[_]>::into_vec(Box::new([bump])));
                        (seeds_vec, pda)
                    };
                    let compressed_infos = {
                        let seed_refs = seeds_vec
                            .iter()
                            .map(|v| v.as_slice())
                            .collect::<Vec<&[u8]>>();
                        light_sdk::compressible::prepare_account_for_decompression_idempotent::<
                            UserRecord,
                        >(
                            &crate::ID,
                            data,
                            light_sdk::compressible::into_compressed_meta_with_address(
                                &compressed_data.meta,
                                &solana_accounts[i],
                                address_space,
                                &crate::ID,
                            ),
                            &solana_accounts[i],
                            &ctx.accounts.rent_payer,
                            &cpi_accounts,
                            seed_refs.as_slice(),
                        )?
                    };
                    compressed_pda_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::GameSession(data) => {
                    let (seeds_vec, _) = {
                        let seed_binding_1 = data.session_id.to_le_bytes();
                        let seeds: &[&[u8]] = &["game_session".as_bytes(), seed_binding_1.as_ref()];
                        let (pda, bump) =
                            anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                        seeds_vec.push(seeds[0usize].to_vec());
                        seeds_vec.push(seeds[1usize].to_vec());
                        seeds_vec.push(<[_]>::into_vec(Box::new([bump])));
                        (seeds_vec, pda)
                    };
                    let compressed_infos = {
                        let seed_refs = seeds_vec
                            .iter()
                            .map(|v| v.as_slice())
                            .collect::<Vec<&[u8]>>();
                        light_sdk::compressible::prepare_account_for_decompression_idempotent::<
                            GameSession,
                        >(
                            &crate::ID,
                            data,
                            light_sdk::compressible::into_compressed_meta_with_address(
                                &compressed_data.meta,
                                &solana_accounts[i],
                                address_space,
                                &crate::ID,
                            ),
                            &solana_accounts[i],
                            &ctx.accounts.rent_payer,
                            &cpi_accounts,
                            seed_refs.as_slice(),
                        )?
                    };
                    compressed_pda_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::PlaceholderRecord(data) => {
                    let (seeds_vec, _) = {
                        let seed_binding_1 = data.placeholder_id.to_le_bytes();
                        let seeds: &[&[u8]] =
                            &["placeholder_record".as_bytes(), seed_binding_1.as_ref()];
                        let (pda, bump) =
                            anchor_lang::prelude::Pubkey::find_program_address(seeds, &crate::ID);
                        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                        seeds_vec.push(seeds[0usize].to_vec());
                        seeds_vec.push(seeds[1usize].to_vec());
                        seeds_vec.push(<[_]>::into_vec(Box::new([bump])));
                        (seeds_vec, pda)
                    };
                    let compressed_infos = {
                        let seed_refs = seeds_vec
                            .iter()
                            .map(|v| v.as_slice())
                            .collect::<Vec<&[u8]>>();
                        light_sdk::compressible::prepare_account_for_decompression_idempotent::<
                            PlaceholderRecord,
                        >(
                            &crate::ID,
                            data,
                            light_sdk::compressible::into_compressed_meta_with_address(
                                &compressed_data.meta,
                                &solana_accounts[i],
                                address_space,
                                &crate::ID,
                            ),
                            &solana_accounts[i],
                            &ctx.accounts.rent_payer,
                            &cpi_accounts,
                            seed_refs.as_slice(),
                        )?
                    };
                    compressed_pda_infos.extend(compressed_infos);
                }
                CompressedAccountVariant::PackedUserRecord(_) => {
                    panic!("internal error: entered unreachable code");
                }
                CompressedAccountVariant::PackedGameSession(_) => {
                    panic!("internal error: entered unreachable code");
                }
                CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                    panic!("internal error: entered unreachable code");
                }
                CompressedAccountVariant::CompressibleTokenAccountPacked(data) => {
                    compressed_token_accounts.push((data, compressed_data.meta));
                }
                CompressedAccountVariant::CompressibleTokenData(_) => {
                    panic!("internal error: entered unreachable code");
                }
            }
        }
        let has_pdas = !compressed_pda_infos.is_empty();
        let has_tokens = !compressed_token_accounts.is_empty();
        if !has_pdas && !has_tokens {
            return Ok(());
        }
        let fee_payer = ctx.accounts.fee_payer.as_ref();
        let authority = cpi_accounts.authority().unwrap();
        let cpi_context = cpi_accounts.cpi_context().unwrap();
        if has_pdas && has_tokens {
            let system_cpi_accounts = light_sdk_types::cpi_context_write::CpiContextWriteAccounts {
                fee_payer,
                authority,
                cpi_context,
                cpi_signer: LIGHT_CPI_SIGNER,
            };
            let cpi_inputs =
                light_sdk::cpi::CpiInputs::new_first_cpi(compressed_pda_infos, Vec::new());
            cpi_inputs.invoke_light_system_program_cpi_context(system_cpi_accounts)?;
        } else if has_pdas {
            let cpi_inputs = light_sdk::cpi::CpiInputs::new(proof, compressed_pda_infos);
            cpi_inputs.invoke_light_system_program_small(cpi_accounts.clone())?;
        }
        if has_tokens {
            // for now, rent recipient PDA is always the same as the sponsor
            // PDA.
            let compressed_token_rent_recipient =
                ctx.accounts.compressed_token_rent_payer.to_account_info();
            process_tokens(
                &ctx.accounts,
                &ctx.remaining_accounts,
                &fee_payer,
                &ctx.accounts.compressed_token_program,
                &ctx.accounts.compressed_token_rent_payer.to_account_info(),
                &compressed_token_rent_recipient,
                &ctx.accounts.compressed_token_rent_authority, // recipient and payer are same and derived from rent_authority!
                &ctx.accounts.compressed_token_cpi_authority,
                &ctx.accounts.config,
                compressed_token_accounts,
                proof,
                &cpi_accounts,
                post_system_accounts,
                has_pdas,
            )?;
        }
        Ok(())
    }

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
            panic!("Rent recipient does not match config");
            // return err!(ErrorCode::InvalidRentRecipient);
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
            panic!("Rent recipient does not match config");
            // return err!(ErrorCode::InvalidRentRecipient);
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
            // return err!(ErrorCode::InvalidRentRecipient);
            panic!("Rent recipient does not match config");
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
            &[user_record],
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
            &[game_session],
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
        let (_, token_account_address) = get_ctoken_signer_seeds(&ctx.accounts.user.key(), &mint);

        let actions = vec![
            light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
                recipients: vec![
                    light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                        recipient: token_account_address, // TRY: THE DECOMPRESS TOKEN ACCOUNT ADDRES IS THE OWNER OF ITS COMPRESSIBLED VERSION.
                        amount: 1000,                     // Mint the full supply to the user
                    },
                ],
                token_account_version: 2,
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
            panic!("Rent recipient does not match config");
            // return err!(ErrorCode::InvalidRentRecipient);
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

        // Note we do not actually close this account yet because in this
        // example we only create _empty_ compressed account without fully
        // compressing it yet.
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
    pub rent_authority: Signer<'info>,
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

#[derive(Accounts)]
pub struct CompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,

    // Required token-specific accounts (always needed for mixed compression)
    /// Compressed token program
    /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
    pub compressed_token_program: UncheckedAccount<'info>,

    /// CPI authority PDA of the compressed token program
    /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
    pub compressed_token_cpi_authority: UncheckedAccount<'info>,

    /// Compressed token rent recipient when token accounts are present
    /// CHECK: Must equal PDA derived from compressed_token_rent_authority
    #[account(mut)]
    pub compressed_token_rent_recipient: UncheckedAccount<'info>,

    /// Compressed token rent authority when token accounts are present
    /// CHECK: Authority used to derive rent recipient PDA
    pub compressed_token_rent_authority: UncheckedAccount<'info>,
    // Remaining accounts:
    // - After system_accounts_offset: Light Protocol system accounts for CPI and tree accounts,... subject to packing.
    // - Last N accounts: Accounts to compress (PDAs and/or token accounts)
}

#[derive(Accounts)]
pub struct CompressMultipleTokenAccounts<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// The authority that owns all token accounts being compressed
    /// CHECK: Validated by the SDK
    pub authority: AccountInfo<'info>,
    /// CHECK: CPI authority of the compressed token program
    pub compressed_token_cpi_authority: UncheckedAccount<'info>,
    /// CHECK: Compressed token program
    pub compressed_token_program: UncheckedAccount<'info>,
    /// The global config account
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,
    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
    // Remaining accounts:
    // - First N accounts: Token accounts to compress
    // - After that: Light Protocol system accounts
}

// TODO: split into one ix with ctoken and one without.
#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// The global config account
    /// CHECK: load_checked.
    pub config: AccountInfo<'info>,
    /// UNCHECKED: Anyone can pay to init PDAs.
    #[account(mut)]
    pub rent_payer: Signer<'info>,
    /// UNCHECKED: Anyone can pay to init compressed tokens.
    #[account(mut)]
    pub compressed_token_rent_payer: UncheckedAccount<'info>,
    /// CHECK: Required for seed derivation - validated by program logic
    pub compressed_token_rent_authority: AccountInfo<'info>,
    /// Compressed token program (always required in mixed variant)
    /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
    pub compressed_token_program: UncheckedAccount<'info>,
    /// CPI authority PDA of the compressed token program (always required in mixed variant)
    /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
    pub compressed_token_cpi_authority: UncheckedAccount<'info>,
    /// CHECK: Required for seed derivation - validated by program logic
    pub some_mint: UncheckedAccount<'info>,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedGameSession {
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    pub player: u8,
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedPlaceholderRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: u8,
    pub name: String,
    pub placeholder_id: u64,
}

/// Auto-derived via macro. Unified enum that can hold any account type. Crucial
/// for dispatching multiple compressed accounts of different types in
/// decompress_accounts_idempotent.
/// Implements: Default, DataHasher, LightDiscriminator, HasCompressionInfo.
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedAccountVariant {
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),
    GameSession(GameSession),
    PackedGameSession(PackedGameSession),
    PlaceholderRecord(PlaceholderRecord),
    PackedPlaceholderRecord(PackedPlaceholderRecord),
    CompressibleTokenAccountPacked(
        light_sdk::token::PackedCompressibleTokenDataWithVariant<CTokenAccountVariant>,
    ),
    CompressibleTokenData(light_sdk::token::CompressibleTokenDataWithVariant<CTokenAccountVariant>),
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
            Self::PackedUserRecord(_) => unreachable!(),
            Self::GameSession(data) => data.hash::<H>(),
            Self::PlaceholderRecord(data) => data.hash::<H>(),
            Self::CompressibleTokenAccountPacked(_) => unreachable!(),
            Self::CompressibleTokenData(_) => unreachable!(),
            Self::PackedGameSession(_) => unreachable!(),
            Self::PackedPlaceholderRecord(_) => unreachable!(),
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
            Self::PackedUserRecord(_) => unreachable!(),
            Self::GameSession(data) => data.compression_info(),
            Self::PlaceholderRecord(data) => data.compression_info(),
            Self::CompressibleTokenAccountPacked(_) => unreachable!(),
            Self::CompressibleTokenData(_) => unreachable!(),
            Self::PackedGameSession(_) => unreachable!(),
            Self::PackedPlaceholderRecord(_) => unreachable!(),
        }
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info_mut(),
            Self::PackedUserRecord(_) => unreachable!(),
            Self::GameSession(data) => data.compression_info_mut(),
            Self::PlaceholderRecord(data) => data.compression_info_mut(),
            Self::CompressibleTokenAccountPacked(_) => unreachable!(),
            Self::CompressibleTokenData(_) => unreachable!(),
            Self::PackedGameSession(_) => unreachable!(),
            Self::PackedPlaceholderRecord(_) => unreachable!(),
        }
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        match self {
            Self::UserRecord(data) => data.compression_info_mut_opt(),
            Self::PackedUserRecord(_) => unreachable!(),
            Self::GameSession(data) => data.compression_info_mut_opt(),
            Self::PlaceholderRecord(data) => data.compression_info_mut_opt(),
            Self::CompressibleTokenAccountPacked(_) => unreachable!(),
            Self::CompressibleTokenData(_) => unreachable!(),
            Self::PackedGameSession(_) => unreachable!(),
            Self::PackedPlaceholderRecord(_) => unreachable!(),
        }
    }

    fn set_compression_info_none(&mut self) {
        match self {
            Self::UserRecord(data) => data.set_compression_info_none(),
            Self::PackedUserRecord(_) => unreachable!(),
            Self::GameSession(data) => data.set_compression_info_none(),
            Self::PlaceholderRecord(data) => data.set_compression_info_none(),
            Self::CompressibleTokenAccountPacked(_) => unreachable!(),
            Self::CompressibleTokenData(_) => unreachable!(),
            Self::PackedGameSession(_) => unreachable!(),
            Self::PackedPlaceholderRecord(_) => unreachable!(),
        }
    }
}

impl Size for CompressedAccountVariant {
    fn size(&self) -> usize {
        match self {
            Self::UserRecord(data) => data.size(),
            Self::PackedUserRecord(_) => unreachable!(),
            Self::GameSession(data) => data.size(),
            Self::PlaceholderRecord(data) => data.size(),
            Self::CompressibleTokenAccountPacked(_) => unreachable!(),
            Self::CompressibleTokenData(_) => unreachable!(),
            Self::PackedGameSession(_) => unreachable!(),
            Self::PackedPlaceholderRecord(_) => unreachable!(),
        }
    }
}

// Pack implementation for CompressedAccountVariant
// This delegates to the underlying type's Pack implementation
impl Pack for CompressedAccountVariant {
    type Packed = Self;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        match self {
            Self::PackedUserRecord(_) => unreachable!(),
            Self::UserRecord(data) => Self::PackedUserRecord(data.pack(remaining_accounts)),
            Self::GameSession(data) => Self::GameSession(data.pack(remaining_accounts)),
            Self::PlaceholderRecord(data) => Self::PlaceholderRecord(data.pack(remaining_accounts)),
            Self::CompressibleTokenAccountPacked(_) => {
                unreachable!()
            }
            Self::CompressibleTokenData(data) => {
                Self::CompressibleTokenAccountPacked(data.pack(remaining_accounts))
            }
            Self::PackedGameSession(_) => unreachable!(),
            Self::PackedPlaceholderRecord(_) => unreachable!(),
        }
    }
}

// Unpack implementation for CompressedAccountVariant
// This delegates to the underlying type's Unpack implementation
impl Unpack for CompressedAccountVariant {
    type Unpacked = Self;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError> {
        match self {
            Self::PackedUserRecord(data) => Ok(Self::UserRecord(data.unpack(remaining_accounts)?)),
            Self::UserRecord(_) => unreachable!(),
            Self::GameSession(data) => Ok(Self::GameSession(data.unpack(remaining_accounts)?)),
            Self::PlaceholderRecord(data) => {
                Ok(Self::PlaceholderRecord(data.unpack(remaining_accounts)?))
            }
            Self::CompressibleTokenAccountPacked(_data) => Ok(self.clone()), // as-is
            Self::CompressibleTokenData(_data) => unreachable!(),            // as-is
            Self::PackedGameSession(_data) => unreachable!(),
            Self::PackedPlaceholderRecord(_data) => unreachable!(),
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

impl CompressedInitSpace for UserRecord {
    const COMPRESSED_INIT_SPACE: usize = Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE;
}

impl CompressedInitSpace for GameSession {
    const COMPRESSED_INIT_SPACE: usize = Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE;
}

impl CompressedInitSpace for PlaceholderRecord {
    const COMPRESSED_INIT_SPACE: usize = Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE;
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

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedUserRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: u8,
    pub name: String,
    pub score: u64,
}

// Identity Pack implementation - no custom packing needed for PDA types
impl Pack for UserRecord {
    type Packed = PackedUserRecord;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        PackedUserRecord {
            compression_info: None,
            owner: remaining_accounts.insert_or_get(self.owner),
            name: self.name.clone(),
            score: self.score,
        }
    }
}

// Identity Unpack implementation - PDA types are sent unpacked
impl Unpack for UserRecord {
    type Unpacked = Self;

    fn unpack(
        &self,
        _remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError> {
        Ok(self.clone())
    }
}

// Identity Pack implementation - no custom packing needed for PDA types
impl Pack for PackedUserRecord {
    type Packed = Self;

    fn pack(&self, _remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        self.clone()
    }
}

// Identity Unpack implementation - PDA types are sent unpacked
impl Unpack for PackedUserRecord {
    type Unpacked = UserRecord;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError> {
        Ok(UserRecord {
            compression_info: None,
            owner: *remaining_accounts[self.owner as usize].key,
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

// Identity Pack implementation - no custom packing needed for PDA types
impl Pack for GameSession {
    type Packed = Self;

    fn pack(&self, _remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        self.clone()
    }
}

// Identity Unpack implementation - PDA types are sent unpacked
impl Unpack for GameSession {
    type Unpacked = Self;

    fn unpack(
        &self,
        _remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError> {
        Ok(self.clone())
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

// Identity Pack implementation - no custom packing needed for PDA types
impl Pack for PlaceholderRecord {
    type Packed = Self;

    fn pack(&self, _remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        self.clone()
    }
}

// Identity Unpack implementation - PDA types are sent unpacked
impl Unpack for PlaceholderRecord {
    type Unpacked = Self;

    fn unpack(
        &self,
        _remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError> {
        Ok(self.clone())
    }
}

// #[error_code]
// pub enum CompressibleInstructionError {
//     #[msg("Invalid account count: PDAs and compressed accounts must match")]
//     InvalidAccountCount,
//     #[msg("Rent recipient does not match config")]
//     InvalidRentRecipient,
//     #[msg("Failed to create compressed mint")]
//     MintCreationFailed,
//     #[msg("Compressed token program account not found in remaining accounts")]
//     MissingCompressedTokenProgram,
//     #[msg("Compressed token program authority PDA account not found in remaining accounts")]
//     MissingCompressedTokenProgramAuthorityPDA,

//     #[msg("CToken decompression not yet implemented")]
//     CTokenDecompressionNotImplemented,
// }

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

/// Information about a token account to compress
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TokenAccountInfo {
    pub user: Pubkey,
    pub mint: Pubkey,
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
