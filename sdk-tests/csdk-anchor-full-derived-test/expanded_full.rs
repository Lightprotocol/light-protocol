#![feature(prelude_import)]
#![allow(deprecated)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;
use anchor_lang::prelude::*;
use light_sdk::derive_light_cpi_signer;
use light_sdk_macros::add_compressible_instructions;
use light_sdk_types::CpiSigner;
pub mod errors {
    use anchor_lang::prelude::ProgramError;
    #[repr(u32)]
    pub enum ErrorCode {
        RentRecipientMismatch,
    }
    impl From<ErrorCode> for ProgramError {
        fn from(e: ErrorCode) -> Self {
            ProgramError::Custom(e as u32)
        }
    }
}
pub mod instruction_accounts {
    use anchor_lang::prelude::*;
    use crate::state::*;
    #[instruction(account_data:AccountCreationData)]
    pub struct CreateUserRecordAndGameSession<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(
            init,
            payer = user,
            space = 8+32+4+32+8+10,
            seeds = [b"user_record",
            user.key().as_ref()],
            bump,
        )]
        pub user_record: Account<'info, UserRecord>,
        #[account(
            init,
            payer = user,
            space = 8+10+8+32+4+32+8+9+8,
            seeds = [b"game_session",
            account_data.session_id.to_le_bytes().as_ref()],
            bump,
        )]
        pub game_session: Account<'info, GameSession>,
        /// The mint signer used for PDA derivation
        pub mint_signer: Signer<'info>,
        /// The mint authority used for PDA derivation
        pub mint_authority: Signer<'info>,
        /// Compressed token program
        /// CHECK: Program ID validated using C_TOKEN_PROGRAM_ID constant
        pub ctoken_program: UncheckedAccount<'info>,
        /// CHECK: CPI authority of the compressed token program
        pub compress_token_program_cpi_authority: UncheckedAccount<'info>,
        /// Needs to be here for the init anchor macro to work.
        pub system_program: Program<'info, System>,
        /// Global compressible config
        /// CHECK: Config is validated by the SDK's load_checked method
        pub config: AccountInfo<'info>,
        /// Rent recipient - must match config
        /// CHECK: Rent recipient is validated against the config
        #[account(mut)]
        pub rent_sponsor: AccountInfo<'info>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, CreateUserRecordAndGameSessionBumps>
    for CreateUserRecordAndGameSession<'info>
    where
        'info: 'info,
    {
        #[inline(never)]
        fn try_accounts(
            __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >],
            __ix_data: &[u8],
            __bumps: &mut CreateUserRecordAndGameSessionBumps,
            __reallocs: &mut std::collections::BTreeSet<
                anchor_lang::solana_program::pubkey::Pubkey,
            >,
        ) -> anchor_lang::Result<Self> {
            let mut __ix_data = __ix_data;
            struct __Args {
                account_data: AccountCreationData,
            }
            impl borsh::ser::BorshSerialize for __Args
            where
                AccountCreationData: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.account_data, writer)?;
                    Ok(())
                }
            }
            impl anchor_lang::idl::build::IdlBuild for __Args {
                fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                    Some(anchor_lang::idl::types::IdlTypeDef {
                        name: Self::get_full_path(),
                        docs: ::alloc::vec::Vec::new(),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Named(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlField {
                                                name: "account_data".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Defined {
                                                    name: <AccountCreationData>::get_full_path(),
                                                    generics: ::alloc::vec::Vec::new(),
                                                },
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {
                    if let Some(ty) = <AccountCreationData>::create_type() {
                        types.insert(<AccountCreationData>::get_full_path(), ty);
                        <AccountCreationData>::insert_types(types);
                    }
                }
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "csdk_anchor_full_derived_test::instruction_accounts",
                                "__Args",
                            ),
                        )
                    })
                }
            }
            impl borsh::de::BorshDeserialize for __Args
            where
                AccountCreationData: borsh::BorshDeserialize,
            {
                fn deserialize_reader<R: borsh::maybestd::io::Read>(
                    reader: &mut R,
                ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                    Ok(Self {
                        account_data: borsh::BorshDeserialize::deserialize_reader(
                            reader,
                        )?,
                    })
                }
            }
            let __Args { account_data } = __Args::deserialize(&mut __ix_data)
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let user: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("user"))?;
            if __accounts.is_empty() {
                return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
            }
            let user_record = &__accounts[0];
            *__accounts = &__accounts[1..];
            if __accounts.is_empty() {
                return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
            }
            let game_session = &__accounts[0];
            *__accounts = &__accounts[1..];
            let mint_signer: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("mint_signer"))?;
            let mint_authority: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("mint_authority"))?;
            let ctoken_program: UncheckedAccount = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_program"))?;
            let compress_token_program_cpi_authority: UncheckedAccount = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| {
                    e.with_account_name("compress_token_program_cpi_authority")
                })?;
            let system_program: anchor_lang::accounts::program::Program<System> = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("system_program"))?;
            let config: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("config"))?;
            let rent_sponsor: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("rent_sponsor"))?;
            let __anchor_rent = Rent::get()?;
            let (__pda_address, __bump) = Pubkey::find_program_address(
                &[b"user_record", user.key().as_ref()],
                __program_id,
            );
            __bumps.user_record = __bump;
            if user_record.key() != __pda_address {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintSeeds,
                        )
                        .with_account_name("user_record")
                        .with_pubkeys((user_record.key(), __pda_address)),
                );
            }
            let user_record = ({
                #[inline(never)]
                || {
                    let actual_field = AsRef::<AccountInfo>::as_ref(&user_record);
                    let actual_owner = actual_field.owner;
                    let space = 8 + 32 + 4 + 32 + 8 + 10;
                    let pa: anchor_lang::accounts::account::Account<UserRecord> = if !false
                        || actual_owner
                            == &anchor_lang::solana_program::system_program::ID
                    {
                        let __current_lamports = user_record.lamports();
                        if __current_lamports == 0 {
                            let space = space;
                            let lamports = __anchor_rent.minimum_balance(space);
                            let cpi_accounts = anchor_lang::system_program::CreateAccount {
                                from: user.to_account_info(),
                                to: user_record.to_account_info(),
                            };
                            let cpi_context = anchor_lang::context::CpiContext::new(
                                system_program.to_account_info(),
                                cpi_accounts,
                            );
                            anchor_lang::system_program::create_account(
                                cpi_context
                                    .with_signer(
                                        &[&[b"user_record", user.key().as_ref(), &[__bump][..]][..]],
                                    ),
                                lamports,
                                space as u64,
                                __program_id,
                            )?;
                        } else {
                            if user.key() == user_record.key() {
                                return Err(
                                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                                            error_name: anchor_lang::error::ErrorCode::TryingToInitPayerAsProgramAccount
                                                .name(),
                                            error_code_number: anchor_lang::error::ErrorCode::TryingToInitPayerAsProgramAccount
                                                .into(),
                                            error_msg: anchor_lang::error::ErrorCode::TryingToInitPayerAsProgramAccount
                                                .to_string(),
                                            error_origin: Some(
                                                anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                                    filename: "sdk-tests/csdk-anchor-full-derived-test/src/instruction_accounts.rs",
                                                    line: 5u32,
                                                }),
                                            ),
                                            compared_values: None,
                                        })
                                        .with_pubkeys((user.key(), user_record.key())),
                                );
                            }
                            let required_lamports = __anchor_rent
                                .minimum_balance(space)
                                .max(1)
                                .saturating_sub(__current_lamports);
                            if required_lamports > 0 {
                                let cpi_accounts = anchor_lang::system_program::Transfer {
                                    from: user.to_account_info(),
                                    to: user_record.to_account_info(),
                                };
                                let cpi_context = anchor_lang::context::CpiContext::new(
                                    system_program.to_account_info(),
                                    cpi_accounts,
                                );
                                anchor_lang::system_program::transfer(
                                    cpi_context,
                                    required_lamports,
                                )?;
                            }
                            let cpi_accounts = anchor_lang::system_program::Allocate {
                                account_to_allocate: user_record.to_account_info(),
                            };
                            let cpi_context = anchor_lang::context::CpiContext::new(
                                system_program.to_account_info(),
                                cpi_accounts,
                            );
                            anchor_lang::system_program::allocate(
                                cpi_context
                                    .with_signer(
                                        &[&[b"user_record", user.key().as_ref(), &[__bump][..]][..]],
                                    ),
                                space as u64,
                            )?;
                            let cpi_accounts = anchor_lang::system_program::Assign {
                                account_to_assign: user_record.to_account_info(),
                            };
                            let cpi_context = anchor_lang::context::CpiContext::new(
                                system_program.to_account_info(),
                                cpi_accounts,
                            );
                            anchor_lang::system_program::assign(
                                cpi_context
                                    .with_signer(
                                        &[&[b"user_record", user.key().as_ref(), &[__bump][..]][..]],
                                    ),
                                __program_id,
                            )?;
                        }
                        match anchor_lang::accounts::account::Account::try_from_unchecked(
                            &user_record,
                        ) {
                            Ok(val) => val,
                            Err(e) => return Err(e.with_account_name("user_record")),
                        }
                    } else {
                        match anchor_lang::accounts::account::Account::try_from(
                            &user_record,
                        ) {
                            Ok(val) => val,
                            Err(e) => return Err(e.with_account_name("user_record")),
                        }
                    };
                    if false {
                        if space != actual_field.data_len() {
                            return Err(
                                anchor_lang::error::Error::from(
                                        anchor_lang::error::ErrorCode::ConstraintSpace,
                                    )
                                    .with_account_name("user_record")
                                    .with_values((space, actual_field.data_len())),
                            );
                        }
                        if actual_owner != __program_id {
                            return Err(
                                anchor_lang::error::Error::from(
                                        anchor_lang::error::ErrorCode::ConstraintOwner,
                                    )
                                    .with_account_name("user_record")
                                    .with_pubkeys((*actual_owner, *__program_id)),
                            );
                        }
                        {
                            let required_lamports = __anchor_rent.minimum_balance(space);
                            if pa.to_account_info().lamports() < required_lamports {
                                return Err(
                                    anchor_lang::error::Error::from(
                                            anchor_lang::error::ErrorCode::ConstraintRentExempt,
                                        )
                                        .with_account_name("user_record"),
                                );
                            }
                        }
                    }
                    Ok(pa)
                }
            })()?;
            if !AsRef::<AccountInfo>::as_ref(&user_record).is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("user_record"),
                );
            }
            if !__anchor_rent
                .is_exempt(
                    user_record.to_account_info().lamports(),
                    user_record.to_account_info().try_data_len()?,
                )
            {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintRentExempt,
                        )
                        .with_account_name("user_record"),
                );
            }
            let __anchor_rent = Rent::get()?;
            let (__pda_address, __bump) = Pubkey::find_program_address(
                &[b"game_session", account_data.session_id.to_le_bytes().as_ref()],
                __program_id,
            );
            __bumps.game_session = __bump;
            if game_session.key() != __pda_address {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintSeeds,
                        )
                        .with_account_name("game_session")
                        .with_pubkeys((game_session.key(), __pda_address)),
                );
            }
            let game_session = ({
                #[inline(never)]
                || {
                    let actual_field = AsRef::<AccountInfo>::as_ref(&game_session);
                    let actual_owner = actual_field.owner;
                    let space = 8 + 10 + 8 + 32 + 4 + 32 + 8 + 9 + 8;
                    let pa: anchor_lang::accounts::account::Account<GameSession> = if !false
                        || actual_owner
                            == &anchor_lang::solana_program::system_program::ID
                    {
                        let __current_lamports = game_session.lamports();
                        if __current_lamports == 0 {
                            let space = space;
                            let lamports = __anchor_rent.minimum_balance(space);
                            let cpi_accounts = anchor_lang::system_program::CreateAccount {
                                from: user.to_account_info(),
                                to: game_session.to_account_info(),
                            };
                            let cpi_context = anchor_lang::context::CpiContext::new(
                                system_program.to_account_info(),
                                cpi_accounts,
                            );
                            anchor_lang::system_program::create_account(
                                cpi_context
                                    .with_signer(
                                        &[
                                            &[
                                                b"game_session",
                                                account_data.session_id.to_le_bytes().as_ref(),
                                                &[__bump][..],
                                            ][..],
                                        ],
                                    ),
                                lamports,
                                space as u64,
                                __program_id,
                            )?;
                        } else {
                            if user.key() == game_session.key() {
                                return Err(
                                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                                            error_name: anchor_lang::error::ErrorCode::TryingToInitPayerAsProgramAccount
                                                .name(),
                                            error_code_number: anchor_lang::error::ErrorCode::TryingToInitPayerAsProgramAccount
                                                .into(),
                                            error_msg: anchor_lang::error::ErrorCode::TryingToInitPayerAsProgramAccount
                                                .to_string(),
                                            error_origin: Some(
                                                anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                                    filename: "sdk-tests/csdk-anchor-full-derived-test/src/instruction_accounts.rs",
                                                    line: 5u32,
                                                }),
                                            ),
                                            compared_values: None,
                                        })
                                        .with_pubkeys((user.key(), game_session.key())),
                                );
                            }
                            let required_lamports = __anchor_rent
                                .minimum_balance(space)
                                .max(1)
                                .saturating_sub(__current_lamports);
                            if required_lamports > 0 {
                                let cpi_accounts = anchor_lang::system_program::Transfer {
                                    from: user.to_account_info(),
                                    to: game_session.to_account_info(),
                                };
                                let cpi_context = anchor_lang::context::CpiContext::new(
                                    system_program.to_account_info(),
                                    cpi_accounts,
                                );
                                anchor_lang::system_program::transfer(
                                    cpi_context,
                                    required_lamports,
                                )?;
                            }
                            let cpi_accounts = anchor_lang::system_program::Allocate {
                                account_to_allocate: game_session.to_account_info(),
                            };
                            let cpi_context = anchor_lang::context::CpiContext::new(
                                system_program.to_account_info(),
                                cpi_accounts,
                            );
                            anchor_lang::system_program::allocate(
                                cpi_context
                                    .with_signer(
                                        &[
                                            &[
                                                b"game_session",
                                                account_data.session_id.to_le_bytes().as_ref(),
                                                &[__bump][..],
                                            ][..],
                                        ],
                                    ),
                                space as u64,
                            )?;
                            let cpi_accounts = anchor_lang::system_program::Assign {
                                account_to_assign: game_session.to_account_info(),
                            };
                            let cpi_context = anchor_lang::context::CpiContext::new(
                                system_program.to_account_info(),
                                cpi_accounts,
                            );
                            anchor_lang::system_program::assign(
                                cpi_context
                                    .with_signer(
                                        &[
                                            &[
                                                b"game_session",
                                                account_data.session_id.to_le_bytes().as_ref(),
                                                &[__bump][..],
                                            ][..],
                                        ],
                                    ),
                                __program_id,
                            )?;
                        }
                        match anchor_lang::accounts::account::Account::try_from_unchecked(
                            &game_session,
                        ) {
                            Ok(val) => val,
                            Err(e) => return Err(e.with_account_name("game_session")),
                        }
                    } else {
                        match anchor_lang::accounts::account::Account::try_from(
                            &game_session,
                        ) {
                            Ok(val) => val,
                            Err(e) => return Err(e.with_account_name("game_session")),
                        }
                    };
                    if false {
                        if space != actual_field.data_len() {
                            return Err(
                                anchor_lang::error::Error::from(
                                        anchor_lang::error::ErrorCode::ConstraintSpace,
                                    )
                                    .with_account_name("game_session")
                                    .with_values((space, actual_field.data_len())),
                            );
                        }
                        if actual_owner != __program_id {
                            return Err(
                                anchor_lang::error::Error::from(
                                        anchor_lang::error::ErrorCode::ConstraintOwner,
                                    )
                                    .with_account_name("game_session")
                                    .with_pubkeys((*actual_owner, *__program_id)),
                            );
                        }
                        {
                            let required_lamports = __anchor_rent.minimum_balance(space);
                            if pa.to_account_info().lamports() < required_lamports {
                                return Err(
                                    anchor_lang::error::Error::from(
                                            anchor_lang::error::ErrorCode::ConstraintRentExempt,
                                        )
                                        .with_account_name("game_session"),
                                );
                            }
                        }
                    }
                    Ok(pa)
                }
            })()?;
            if !AsRef::<AccountInfo>::as_ref(&game_session).is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("game_session"),
                );
            }
            if !__anchor_rent
                .is_exempt(
                    game_session.to_account_info().lamports(),
                    game_session.to_account_info().try_data_len()?,
                )
            {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintRentExempt,
                        )
                        .with_account_name("game_session"),
                );
            }
            if !AsRef::<AccountInfo>::as_ref(&user).is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("user"),
                );
            }
            if !&rent_sponsor.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("rent_sponsor"),
                );
            }
            Ok(CreateUserRecordAndGameSession {
                user,
                user_record,
                game_session,
                mint_signer,
                mint_authority,
                ctoken_program,
                compress_token_program_cpi_authority,
                system_program,
                config,
                rent_sponsor,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info>
    for CreateUserRecordAndGameSession<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.user.to_account_infos());
            account_infos.extend(self.user_record.to_account_infos());
            account_infos.extend(self.game_session.to_account_infos());
            account_infos.extend(self.mint_signer.to_account_infos());
            account_infos.extend(self.mint_authority.to_account_infos());
            account_infos.extend(self.ctoken_program.to_account_infos());
            account_infos
                .extend(self.compress_token_program_cpi_authority.to_account_infos());
            account_infos.extend(self.system_program.to_account_infos());
            account_infos.extend(self.config.to_account_infos());
            account_infos.extend(self.rent_sponsor.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for CreateUserRecordAndGameSession<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.user.to_account_metas(None));
            account_metas.extend(self.user_record.to_account_metas(None));
            account_metas.extend(self.game_session.to_account_metas(None));
            account_metas.extend(self.mint_signer.to_account_metas(None));
            account_metas.extend(self.mint_authority.to_account_metas(None));
            account_metas.extend(self.ctoken_program.to_account_metas(None));
            account_metas
                .extend(
                    self.compress_token_program_cpi_authority.to_account_metas(None),
                );
            account_metas.extend(self.system_program.to_account_metas(None));
            account_metas.extend(self.config.to_account_metas(None));
            account_metas.extend(self.rent_sponsor.to_account_metas(None));
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info>
    for CreateUserRecordAndGameSession<'info>
    where
        'info: 'info,
    {
        fn exit(
            &self,
            program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        ) -> anchor_lang::Result<()> {
            anchor_lang::AccountsExit::exit(&self.user, program_id)
                .map_err(|e| e.with_account_name("user"))?;
            anchor_lang::AccountsExit::exit(&self.user_record, program_id)
                .map_err(|e| e.with_account_name("user_record"))?;
            anchor_lang::AccountsExit::exit(&self.game_session, program_id)
                .map_err(|e| e.with_account_name("game_session"))?;
            anchor_lang::AccountsExit::exit(&self.rent_sponsor, program_id)
                .map_err(|e| e.with_account_name("rent_sponsor"))?;
            Ok(())
        }
    }
    pub struct CreateUserRecordAndGameSessionBumps {
        pub user_record: u8,
        pub game_session: u8,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for CreateUserRecordAndGameSessionBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "CreateUserRecordAndGameSessionBumps",
                "user_record",
                &self.user_record,
                "game_session",
                &&self.game_session,
            )
        }
    }
    impl Default for CreateUserRecordAndGameSessionBumps {
        fn default() -> Self {
            CreateUserRecordAndGameSessionBumps {
                user_record: u8::MAX,
                game_session: u8::MAX,
            }
        }
    }
    impl<'info> anchor_lang::Bumps for CreateUserRecordAndGameSession<'info>
    where
        'info: 'info,
    {
        type Bumps = CreateUserRecordAndGameSessionBumps;
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
    /// instead of an `AccountInfo`. This is useful for clients that want
    /// to generate a list of accounts, without explicitly knowing the
    /// order all the fields should be in.
    ///
    /// To access the struct in this module, one should use the sibling
    /// `accounts` module (also generated), which re-exports this.
    pub(crate) mod __client_accounts_create_user_record_and_game_session {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`CreateUserRecordAndGameSession`].
        pub struct CreateUserRecordAndGameSession {
            pub user: Pubkey,
            pub user_record: Pubkey,
            pub game_session: Pubkey,
            ///The mint signer used for PDA derivation
            pub mint_signer: Pubkey,
            ///The mint authority used for PDA derivation
            pub mint_authority: Pubkey,
            ///Compressed token program
            pub ctoken_program: Pubkey,
            pub compress_token_program_cpi_authority: Pubkey,
            ///Needs to be here for the init anchor macro to work.
            pub system_program: Pubkey,
            ///Global compressible config
            pub config: Pubkey,
            ///Rent recipient - must match config
            pub rent_sponsor: Pubkey,
        }
        impl borsh::ser::BorshSerialize for CreateUserRecordAndGameSession
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.user, writer)?;
                borsh::BorshSerialize::serialize(&self.user_record, writer)?;
                borsh::BorshSerialize::serialize(&self.game_session, writer)?;
                borsh::BorshSerialize::serialize(&self.mint_signer, writer)?;
                borsh::BorshSerialize::serialize(&self.mint_authority, writer)?;
                borsh::BorshSerialize::serialize(&self.ctoken_program, writer)?;
                borsh::BorshSerialize::serialize(
                    &self.compress_token_program_cpi_authority,
                    writer,
                )?;
                borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                borsh::BorshSerialize::serialize(&self.config, writer)?;
                borsh::BorshSerialize::serialize(&self.rent_sponsor, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for CreateUserRecordAndGameSession {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            "Generated client accounts for [`CreateUserRecordAndGameSession`]."
                                .into(),
                        ]),
                    ),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(
                            anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(
                                    ::alloc::boxed::box_new([
                                        anchor_lang::idl::types::IdlField {
                                            name: "user".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "user_record".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "game_session".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "mint_signer".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "The mint signer used for PDA derivation".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "mint_authority".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "The mint authority used for PDA derivation".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_program".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new(["Compressed token program".into()]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "compress_token_program_cpi_authority".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "system_program".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Needs to be here for the init anchor macro to work.".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "config".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Global compressible config".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "rent_sponsor".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Rent recipient - must match config".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                    ]),
                                ),
                            ),
                        ),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) {}
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "csdk_anchor_full_derived_test::instruction_accounts::__client_accounts_create_user_record_and_game_session",
                            "CreateUserRecordAndGameSession",
                        ),
                    )
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CreateUserRecordAndGameSession {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.user,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.user_record,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.game_session,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.mint_signer,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.mint_authority,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.ctoken_program,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.compress_token_program_cpi_authority,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.system_program,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.config,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.rent_sponsor,
                            false,
                        ),
                    );
                account_metas
            }
        }
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a CPI struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is an
    /// AccountInfo.
    ///
    /// To access the struct in this module, one should use the sibling
    /// [`cpi::accounts`] module (also generated), which re-exports this.
    pub(crate) mod __cpi_client_accounts_create_user_record_and_game_session {
        use super::*;
        /// Generated CPI struct of the accounts for [`CreateUserRecordAndGameSession`].
        pub struct CreateUserRecordAndGameSession<'info> {
            pub user: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub user_record: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            pub game_session: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///The mint signer used for PDA derivation
            pub mint_signer: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///The mint authority used for PDA derivation
            pub mint_authority: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///Compressed token program
            pub ctoken_program: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            pub compress_token_program_cpi_authority: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///Needs to be here for the init anchor macro to work.
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///Global compressible config
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///Rent recipient - must match config
            pub rent_sponsor: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas
        for CreateUserRecordAndGameSession<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.user),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.user_record),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.game_session),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.mint_signer),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.mint_authority),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.ctoken_program),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(
                                &self.compress_token_program_cpi_authority,
                            ),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.system_program),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.config),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.rent_sponsor),
                            false,
                        ),
                    );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info>
        for CreateUserRecordAndGameSession<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.user));
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.user_record),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.game_session),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.mint_signer),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.mint_authority,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_program,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.compress_token_program_cpi_authority,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.system_program,
                        ),
                    );
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.rent_sponsor),
                    );
                account_infos
            }
        }
    }
    impl<'info> CreateUserRecordAndGameSession<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlAccount,
            >,
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            if let Some(ty) = <UserRecord>::create_type() {
                let account = anchor_lang::idl::types::IdlAccount {
                    name: ty.name.clone(),
                    discriminator: UserRecord::DISCRIMINATOR.into(),
                };
                accounts.insert(account.name.clone(), account);
                types.insert(ty.name.clone(), ty);
                <UserRecord>::insert_types(types);
            }
            if let Some(ty) = <GameSession>::create_type() {
                let account = anchor_lang::idl::types::IdlAccount {
                    name: ty.name.clone(),
                    discriminator: GameSession::DISCRIMINATOR.into(),
                };
                accounts.insert(account.name.clone(), account);
                types.insert(ty.name.clone(), ty);
                <GameSession>::insert_types(types);
            }
            <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "user".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "user_record".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "game_session".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "mint_signer".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "The mint signer used for PDA derivation".into(),
                            ]),
                        ),
                        writable: false,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "mint_authority".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "The mint authority used for PDA derivation".into(),
                            ]),
                        ),
                        writable: false,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_program".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new(["Compressed token program".into()]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "compress_token_program_cpi_authority".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "system_program".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Needs to be here for the init anchor macro to work.".into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Global compressible config".into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "rent_sponsor".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Rent recipient - must match config".into(),
                            ]),
                        ),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                ]),
            )
        }
    }
}
pub mod state {
    use anchor_lang::prelude::*;
    use light_ctoken_types::instructions::mint_action::CompressedMintWithContext;
    use light_sdk::{
        compressible::CompressionInfo,
        instruction::{PackedAddressTreeInfo, ValidityProof},
        LightDiscriminator, LightHasher,
    };
    use light_sdk_macros::{Compressible, CompressiblePack};
    pub struct UserRecord {
        #[skip]
        pub compression_info: Option<CompressionInfo>,
        #[hash]
        pub owner: Pubkey,
        #[max_len(32)]
        pub name: String,
        pub score: u64,
    }
    impl borsh::ser::BorshSerialize for UserRecord
    where
        Option<CompressionInfo>: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_info, writer)?;
            borsh::BorshSerialize::serialize(&self.owner, writer)?;
            borsh::BorshSerialize::serialize(&self.name, writer)?;
            borsh::BorshSerialize::serialize(&self.score, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for UserRecord {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <CompressionInfo>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "owner".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Pubkey,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "name".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::String,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "score".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <CompressionInfo>::create_type() {
                types.insert(<CompressionInfo>::get_full_path(), ty);
                <CompressionInfo>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "UserRecord",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for UserRecord
    where
        Option<CompressionInfo>: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_info: borsh::BorshDeserialize::deserialize_reader(reader)?,
                owner: borsh::BorshDeserialize::deserialize_reader(reader)?,
                name: borsh::BorshDeserialize::deserialize_reader(reader)?,
                score: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UserRecord {
        #[inline]
        fn clone(&self) -> UserRecord {
            UserRecord {
                compression_info: ::core::clone::Clone::clone(&self.compression_info),
                owner: ::core::clone::Clone::clone(&self.owner),
                name: ::core::clone::Clone::clone(&self.name),
                score: ::core::clone::Clone::clone(&self.score),
            }
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountSerialize for UserRecord {
        fn try_serialize<W: std::io::Write>(
            &self,
            writer: &mut W,
        ) -> anchor_lang::Result<()> {
            if writer.write_all(UserRecord::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountDeserialize for UserRecord {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < UserRecord::DISCRIMINATOR.len() {
                return Err(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into(),
                );
            }
            let given_disc = &buf[..UserRecord::DISCRIMINATOR.len()];
            if UserRecord::DISCRIMINATOR != given_disc {
                return Err(
                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                            error_name: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .name(),
                            error_code_number: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .into(),
                            error_msg: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .to_string(),
                            error_origin: Some(
                                anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                    filename: "sdk-tests/csdk-anchor-full-derived-test/src/state.rs",
                                    line: 19u32,
                                }),
                            ),
                            compared_values: None,
                        })
                        .with_account_name("UserRecord"),
                );
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[UserRecord::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into()
                })
        }
    }
    #[automatically_derived]
    impl anchor_lang::Discriminator for UserRecord {
        const DISCRIMINATOR: &'static [u8] = &[210, 252, 132, 218, 191, 85, 173, 167];
    }
    #[automatically_derived]
    impl anchor_lang::Owner for UserRecord {
        fn owner() -> Pubkey {
            crate::ID
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for UserRecord {
        #[inline]
        fn default() -> UserRecord {
            UserRecord {
                compression_info: ::core::default::Default::default(),
                owner: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                score: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UserRecord {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "UserRecord",
                "compression_info",
                &self.compression_info,
                "owner",
                &self.owner,
                "name",
                &self.name,
                "score",
                &&self.score,
            )
        }
    }
    impl ::light_hasher::to_byte_array::ToByteArray for UserRecord {
        const NUM_FIELDS: usize = 4usize;
        fn to_byte_array(
            &self,
        ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
            use ::light_hasher::to_byte_array::ToByteArray;
            use ::light_hasher::hash_to_field_size::HashToFieldSize;
            use ::light_hasher::Hasher;
            let mut result = ::light_hasher::Poseidon::hashv(
                &[
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                            self.owner.as_ref(),
                        )
                        .as_slice(),
                    self.name.to_byte_array()?.as_slice(),
                    self.score.to_byte_array()?.as_slice(),
                ],
            )?;
            if ::light_hasher::Poseidon::ID != ::light_hasher::Poseidon::ID {
                result[0] = 0;
            }
            Ok(result)
        }
    }
    impl ::light_hasher::DataHasher for UserRecord {
        fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
        where
            H: ::light_hasher::Hasher,
        {
            use ::light_hasher::DataHasher;
            use ::light_hasher::Hasher;
            use ::light_hasher::to_byte_array::ToByteArray;
            use ::light_hasher::hash_to_field_size::HashToFieldSize;
            {
                if std::env::var("RUST_BACKTRACE").is_ok() {
                    let debug_prints: Vec<[u8; 32]> = <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                                self.owner.as_ref(),
                            ),
                            self.name.to_byte_array()?,
                            self.score.to_byte_array()?,
                        ]),
                    );
                    {
                        ::std::io::_print(
                            format_args!("DataHasher::hash inputs {0:?}\n", debug_prints),
                        );
                    };
                }
            }
            let mut result = H::hashv(
                &[
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                            self.owner.as_ref(),
                        )
                        .as_slice(),
                    self.name.to_byte_array()?.as_slice(),
                    self.score.to_byte_array()?.as_slice(),
                ],
            )?;
            if H::ID != ::light_hasher::Poseidon::ID {
                result[0] = 0;
            }
            Ok(result)
        }
    }
    impl LightDiscriminator for UserRecord {
        const LIGHT_DISCRIMINATOR: [u8; 8] = [210, 252, 132, 218, 191, 85, 173, 167];
        const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        fn discriminator() -> [u8; 8] {
            Self::LIGHT_DISCRIMINATOR
        }
    }
    #[automatically_derived]
    impl anchor_lang::Space for UserRecord {
        const INIT_SPACE: usize = 0
            + (1 + <CompressionInfo as anchor_lang::Space>::INIT_SPACE) + 32 + (4 + 32)
            + 8;
    }
    impl light_sdk::compressible::HasCompressionInfo for UserRecord {
        fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
            self.compression_info
                .as_ref()
                .expect("CompressionInfo must be Some on-chain")
        }
        fn compression_info_mut(
            &mut self,
        ) -> &mut light_sdk::compressible::CompressionInfo {
            self.compression_info
                .as_mut()
                .expect("CompressionInfo must be Some on-chain")
        }
        fn compression_info_mut_opt(
            &mut self,
        ) -> &mut Option<light_sdk::compressible::CompressionInfo> {
            &mut self.compression_info
        }
        fn set_compression_info_none(&mut self) {
            self.compression_info = None;
        }
    }
    impl light_sdk::account::Size for UserRecord {
        fn size(&self) -> usize {
            Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
        }
    }
    impl light_sdk::compressible::CompressAs for UserRecord {
        type Output = Self;
        fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
            std::borrow::Cow::Owned(Self {
                compression_info: None,
                owner: self.owner,
                name: self.name.clone(),
                score: self.score,
            })
        }
    }
    impl light_sdk::compressible::compression_info::CompressedInitSpace for UserRecord {
        const COMPRESSED_INIT_SPACE: usize = Self::INIT_SPACE
            - (0 + <CompressionInfo>::INIT_SPACE);
    }
    pub struct PackedUserRecord {
        pub compression_info: Option<CompressionInfo>,
        pub owner: u8,
        pub name: String,
        pub score: u64,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PackedUserRecord {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "PackedUserRecord",
                "compression_info",
                &self.compression_info,
                "owner",
                &self.owner,
                "name",
                &self.name,
                "score",
                &&self.score,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PackedUserRecord {
        #[inline]
        fn clone(&self) -> PackedUserRecord {
            PackedUserRecord {
                compression_info: ::core::clone::Clone::clone(&self.compression_info),
                owner: ::core::clone::Clone::clone(&self.owner),
                name: ::core::clone::Clone::clone(&self.name),
                score: ::core::clone::Clone::clone(&self.score),
            }
        }
    }
    impl borsh::ser::BorshSerialize for PackedUserRecord
    where
        Option<CompressionInfo>: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_info, writer)?;
            borsh::BorshSerialize::serialize(&self.owner, writer)?;
            borsh::BorshSerialize::serialize(&self.name, writer)?;
            borsh::BorshSerialize::serialize(&self.score, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for PackedUserRecord {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <CompressionInfo>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "owner".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "name".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::String,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "score".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <CompressionInfo>::create_type() {
                types.insert(<CompressionInfo>::get_full_path(), ty);
                <CompressionInfo>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "PackedUserRecord",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for PackedUserRecord
    where
        Option<CompressionInfo>: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_info: borsh::BorshDeserialize::deserialize_reader(reader)?,
                owner: borsh::BorshDeserialize::deserialize_reader(reader)?,
                name: borsh::BorshDeserialize::deserialize_reader(reader)?,
                score: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl light_sdk::compressible::Pack for UserRecord {
        type Packed = PackedUserRecord;
        #[inline(never)]
        fn pack(
            &self,
            remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
        ) -> Self::Packed {
            PackedUserRecord {
                compression_info: None,
                owner: remaining_accounts.insert_or_get(self.owner),
                name: self.name.clone(),
                score: self.score,
            }
        }
    }
    impl light_sdk::compressible::Unpack for UserRecord {
        type Unpacked = Self;
        #[inline(never)]
        fn unpack(
            &self,
            _remaining_accounts: &[anchor_lang::prelude::AccountInfo],
        ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
            Ok(self.clone())
        }
    }
    impl light_sdk::compressible::Pack for PackedUserRecord {
        type Packed = Self;
        #[inline(never)]
        fn pack(
            &self,
            _remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
        ) -> Self::Packed {
            self.clone()
        }
    }
    impl light_sdk::compressible::Unpack for PackedUserRecord {
        type Unpacked = UserRecord;
        #[inline(never)]
        fn unpack(
            &self,
            remaining_accounts: &[anchor_lang::prelude::AccountInfo],
        ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
            Ok(UserRecord {
                compression_info: None,
                owner: *remaining_accounts[self.owner as usize].key,
                name: self.name.clone(),
                score: self.score,
            })
        }
    }
    #[compress_as(start_time = 0, end_time = None, score = 0)]
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
    impl borsh::ser::BorshSerialize for GameSession
    where
        Option<CompressionInfo>: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        Option<u64>: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_info, writer)?;
            borsh::BorshSerialize::serialize(&self.session_id, writer)?;
            borsh::BorshSerialize::serialize(&self.player, writer)?;
            borsh::BorshSerialize::serialize(&self.game_type, writer)?;
            borsh::BorshSerialize::serialize(&self.start_time, writer)?;
            borsh::BorshSerialize::serialize(&self.end_time, writer)?;
            borsh::BorshSerialize::serialize(&self.score, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for GameSession {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <CompressionInfo>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "session_id".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "player".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Pubkey,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "game_type".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::String,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "start_time".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "end_time".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::U64),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "score".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <CompressionInfo>::create_type() {
                types.insert(<CompressionInfo>::get_full_path(), ty);
                <CompressionInfo>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "GameSession",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for GameSession
    where
        Option<CompressionInfo>: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        Option<u64>: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_info: borsh::BorshDeserialize::deserialize_reader(reader)?,
                session_id: borsh::BorshDeserialize::deserialize_reader(reader)?,
                player: borsh::BorshDeserialize::deserialize_reader(reader)?,
                game_type: borsh::BorshDeserialize::deserialize_reader(reader)?,
                start_time: borsh::BorshDeserialize::deserialize_reader(reader)?,
                end_time: borsh::BorshDeserialize::deserialize_reader(reader)?,
                score: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for GameSession {
        #[inline]
        fn clone(&self) -> GameSession {
            GameSession {
                compression_info: ::core::clone::Clone::clone(&self.compression_info),
                session_id: ::core::clone::Clone::clone(&self.session_id),
                player: ::core::clone::Clone::clone(&self.player),
                game_type: ::core::clone::Clone::clone(&self.game_type),
                start_time: ::core::clone::Clone::clone(&self.start_time),
                end_time: ::core::clone::Clone::clone(&self.end_time),
                score: ::core::clone::Clone::clone(&self.score),
            }
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountSerialize for GameSession {
        fn try_serialize<W: std::io::Write>(
            &self,
            writer: &mut W,
        ) -> anchor_lang::Result<()> {
            if writer.write_all(GameSession::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountDeserialize for GameSession {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < GameSession::DISCRIMINATOR.len() {
                return Err(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into(),
                );
            }
            let given_disc = &buf[..GameSession::DISCRIMINATOR.len()];
            if GameSession::DISCRIMINATOR != given_disc {
                return Err(
                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                            error_name: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .name(),
                            error_code_number: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .into(),
                            error_msg: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .to_string(),
                            error_origin: Some(
                                anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                    filename: "sdk-tests/csdk-anchor-full-derived-test/src/state.rs",
                                    line: 40u32,
                                }),
                            ),
                            compared_values: None,
                        })
                        .with_account_name("GameSession"),
                );
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[GameSession::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into()
                })
        }
    }
    #[automatically_derived]
    impl anchor_lang::Discriminator for GameSession {
        const DISCRIMINATOR: &'static [u8] = &[150, 116, 20, 197, 205, 121, 220, 240];
    }
    #[automatically_derived]
    impl anchor_lang::Owner for GameSession {
        fn owner() -> Pubkey {
            crate::ID
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for GameSession {
        #[inline]
        fn default() -> GameSession {
            GameSession {
                compression_info: ::core::default::Default::default(),
                session_id: ::core::default::Default::default(),
                player: ::core::default::Default::default(),
                game_type: ::core::default::Default::default(),
                start_time: ::core::default::Default::default(),
                end_time: ::core::default::Default::default(),
                score: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for GameSession {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "compression_info",
                "session_id",
                "player",
                "game_type",
                "start_time",
                "end_time",
                "score",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.compression_info,
                &self.session_id,
                &self.player,
                &self.game_type,
                &self.start_time,
                &self.end_time,
                &&self.score,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "GameSession",
                names,
                values,
            )
        }
    }
    impl ::light_hasher::to_byte_array::ToByteArray for GameSession {
        const NUM_FIELDS: usize = 7usize;
        fn to_byte_array(
            &self,
        ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
            use ::light_hasher::to_byte_array::ToByteArray;
            use ::light_hasher::hash_to_field_size::HashToFieldSize;
            use ::light_hasher::Hasher;
            let mut result = ::light_hasher::Poseidon::hashv(
                &[
                    self.session_id.to_byte_array()?.as_slice(),
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                            self.player.as_ref(),
                        )
                        .as_slice(),
                    self.game_type.to_byte_array()?.as_slice(),
                    self.start_time.to_byte_array()?.as_slice(),
                    self.end_time.to_byte_array()?.as_slice(),
                    self.score.to_byte_array()?.as_slice(),
                ],
            )?;
            if ::light_hasher::Poseidon::ID != ::light_hasher::Poseidon::ID {
                result[0] = 0;
            }
            Ok(result)
        }
    }
    impl ::light_hasher::DataHasher for GameSession {
        fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
        where
            H: ::light_hasher::Hasher,
        {
            use ::light_hasher::DataHasher;
            use ::light_hasher::Hasher;
            use ::light_hasher::to_byte_array::ToByteArray;
            use ::light_hasher::hash_to_field_size::HashToFieldSize;
            {
                if std::env::var("RUST_BACKTRACE").is_ok() {
                    let debug_prints: Vec<[u8; 32]> = <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            self.session_id.to_byte_array()?,
                            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                                self.player.as_ref(),
                            ),
                            self.game_type.to_byte_array()?,
                            self.start_time.to_byte_array()?,
                            self.end_time.to_byte_array()?,
                            self.score.to_byte_array()?,
                        ]),
                    );
                    {
                        ::std::io::_print(
                            format_args!("DataHasher::hash inputs {0:?}\n", debug_prints),
                        );
                    };
                }
            }
            let mut result = H::hashv(
                &[
                    self.session_id.to_byte_array()?.as_slice(),
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                            self.player.as_ref(),
                        )
                        .as_slice(),
                    self.game_type.to_byte_array()?.as_slice(),
                    self.start_time.to_byte_array()?.as_slice(),
                    self.end_time.to_byte_array()?.as_slice(),
                    self.score.to_byte_array()?.as_slice(),
                ],
            )?;
            if H::ID != ::light_hasher::Poseidon::ID {
                result[0] = 0;
            }
            Ok(result)
        }
    }
    impl LightDiscriminator for GameSession {
        const LIGHT_DISCRIMINATOR: [u8; 8] = [150, 116, 20, 197, 205, 121, 220, 240];
        const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        fn discriminator() -> [u8; 8] {
            Self::LIGHT_DISCRIMINATOR
        }
    }
    #[automatically_derived]
    impl anchor_lang::Space for GameSession {
        const INIT_SPACE: usize = 0
            + (1 + <CompressionInfo as anchor_lang::Space>::INIT_SPACE) + 8 + 32
            + (4 + 32) + 8 + (1 + 8) + 8;
    }
    impl light_sdk::compressible::HasCompressionInfo for GameSession {
        fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
            self.compression_info
                .as_ref()
                .expect("CompressionInfo must be Some on-chain")
        }
        fn compression_info_mut(
            &mut self,
        ) -> &mut light_sdk::compressible::CompressionInfo {
            self.compression_info
                .as_mut()
                .expect("CompressionInfo must be Some on-chain")
        }
        fn compression_info_mut_opt(
            &mut self,
        ) -> &mut Option<light_sdk::compressible::CompressionInfo> {
            &mut self.compression_info
        }
        fn set_compression_info_none(&mut self) {
            self.compression_info = None;
        }
    }
    impl light_sdk::account::Size for GameSession {
        fn size(&self) -> usize {
            Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
        }
    }
    impl light_sdk::compressible::CompressAs for GameSession {
        type Output = Self;
        fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
            std::borrow::Cow::Owned(Self {
                compression_info: None,
                session_id: self.session_id,
                player: self.player,
                game_type: self.game_type.clone(),
                start_time: 0,
                end_time: None,
                score: 0,
            })
        }
    }
    impl light_sdk::compressible::compression_info::CompressedInitSpace for GameSession {
        const COMPRESSED_INIT_SPACE: usize = Self::INIT_SPACE
            - (0 + <CompressionInfo>::INIT_SPACE + 8);
    }
    pub struct PackedGameSession {
        pub compression_info: Option<CompressionInfo>,
        pub session_id: u64,
        pub player: u8,
        pub game_type: String,
        pub start_time: u64,
        pub end_time: Option<u64>,
        pub score: u64,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PackedGameSession {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "compression_info",
                "session_id",
                "player",
                "game_type",
                "start_time",
                "end_time",
                "score",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.compression_info,
                &self.session_id,
                &self.player,
                &self.game_type,
                &self.start_time,
                &self.end_time,
                &&self.score,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "PackedGameSession",
                names,
                values,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PackedGameSession {
        #[inline]
        fn clone(&self) -> PackedGameSession {
            PackedGameSession {
                compression_info: ::core::clone::Clone::clone(&self.compression_info),
                session_id: ::core::clone::Clone::clone(&self.session_id),
                player: ::core::clone::Clone::clone(&self.player),
                game_type: ::core::clone::Clone::clone(&self.game_type),
                start_time: ::core::clone::Clone::clone(&self.start_time),
                end_time: ::core::clone::Clone::clone(&self.end_time),
                score: ::core::clone::Clone::clone(&self.score),
            }
        }
    }
    impl borsh::ser::BorshSerialize for PackedGameSession
    where
        Option<CompressionInfo>: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        Option<u64>: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_info, writer)?;
            borsh::BorshSerialize::serialize(&self.session_id, writer)?;
            borsh::BorshSerialize::serialize(&self.player, writer)?;
            borsh::BorshSerialize::serialize(&self.game_type, writer)?;
            borsh::BorshSerialize::serialize(&self.start_time, writer)?;
            borsh::BorshSerialize::serialize(&self.end_time, writer)?;
            borsh::BorshSerialize::serialize(&self.score, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for PackedGameSession {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <CompressionInfo>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "session_id".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "player".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "game_type".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::String,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "start_time".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "end_time".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::U64),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "score".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <CompressionInfo>::create_type() {
                types.insert(<CompressionInfo>::get_full_path(), ty);
                <CompressionInfo>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "PackedGameSession",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for PackedGameSession
    where
        Option<CompressionInfo>: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        Option<u64>: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_info: borsh::BorshDeserialize::deserialize_reader(reader)?,
                session_id: borsh::BorshDeserialize::deserialize_reader(reader)?,
                player: borsh::BorshDeserialize::deserialize_reader(reader)?,
                game_type: borsh::BorshDeserialize::deserialize_reader(reader)?,
                start_time: borsh::BorshDeserialize::deserialize_reader(reader)?,
                end_time: borsh::BorshDeserialize::deserialize_reader(reader)?,
                score: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl light_sdk::compressible::Pack for GameSession {
        type Packed = PackedGameSession;
        #[inline(never)]
        fn pack(
            &self,
            remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
        ) -> Self::Packed {
            PackedGameSession {
                compression_info: None,
                session_id: self.session_id,
                player: remaining_accounts.insert_or_get(self.player),
                game_type: self.game_type.clone(),
                start_time: self.start_time,
                end_time: self.end_time,
                score: self.score,
            }
        }
    }
    impl light_sdk::compressible::Unpack for GameSession {
        type Unpacked = Self;
        #[inline(never)]
        fn unpack(
            &self,
            _remaining_accounts: &[anchor_lang::prelude::AccountInfo],
        ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
            Ok(self.clone())
        }
    }
    impl light_sdk::compressible::Pack for PackedGameSession {
        type Packed = Self;
        #[inline(never)]
        fn pack(
            &self,
            _remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
        ) -> Self::Packed {
            self.clone()
        }
    }
    impl light_sdk::compressible::Unpack for PackedGameSession {
        type Unpacked = GameSession;
        #[inline(never)]
        fn unpack(
            &self,
            remaining_accounts: &[anchor_lang::prelude::AccountInfo],
        ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
            Ok(GameSession {
                compression_info: None,
                session_id: self.session_id,
                player: *remaining_accounts[self.player as usize].key,
                game_type: self.game_type.clone(),
                start_time: self.start_time,
                end_time: self.end_time,
                score: self.score,
            })
        }
    }
    pub struct PlaceholderRecord {
        #[skip]
        pub compression_info: Option<CompressionInfo>,
        #[hash]
        pub owner: Pubkey,
        #[max_len(32)]
        pub name: String,
        pub placeholder_id: u64,
    }
    impl borsh::ser::BorshSerialize for PlaceholderRecord
    where
        Option<CompressionInfo>: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_info, writer)?;
            borsh::BorshSerialize::serialize(&self.owner, writer)?;
            borsh::BorshSerialize::serialize(&self.name, writer)?;
            borsh::BorshSerialize::serialize(&self.placeholder_id, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for PlaceholderRecord {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <CompressionInfo>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "owner".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Pubkey,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "name".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::String,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "placeholder_id".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <CompressionInfo>::create_type() {
                types.insert(<CompressionInfo>::get_full_path(), ty);
                <CompressionInfo>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "PlaceholderRecord",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for PlaceholderRecord
    where
        Option<CompressionInfo>: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_info: borsh::BorshDeserialize::deserialize_reader(reader)?,
                owner: borsh::BorshDeserialize::deserialize_reader(reader)?,
                name: borsh::BorshDeserialize::deserialize_reader(reader)?,
                placeholder_id: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PlaceholderRecord {
        #[inline]
        fn clone(&self) -> PlaceholderRecord {
            PlaceholderRecord {
                compression_info: ::core::clone::Clone::clone(&self.compression_info),
                owner: ::core::clone::Clone::clone(&self.owner),
                name: ::core::clone::Clone::clone(&self.name),
                placeholder_id: ::core::clone::Clone::clone(&self.placeholder_id),
            }
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountSerialize for PlaceholderRecord {
        fn try_serialize<W: std::io::Write>(
            &self,
            writer: &mut W,
        ) -> anchor_lang::Result<()> {
            if writer.write_all(PlaceholderRecord::DISCRIMINATOR).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            if AnchorSerialize::serialize(self, writer).is_err() {
                return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
            }
            Ok(())
        }
    }
    #[automatically_derived]
    impl anchor_lang::AccountDeserialize for PlaceholderRecord {
        fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            if buf.len() < PlaceholderRecord::DISCRIMINATOR.len() {
                return Err(
                    anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into(),
                );
            }
            let given_disc = &buf[..PlaceholderRecord::DISCRIMINATOR.len()];
            if PlaceholderRecord::DISCRIMINATOR != given_disc {
                return Err(
                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                            error_name: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .name(),
                            error_code_number: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .into(),
                            error_msg: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .to_string(),
                            error_origin: Some(
                                anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                    filename: "sdk-tests/csdk-anchor-full-derived-test/src/state.rs",
                                    line: 63u32,
                                }),
                            ),
                            compared_values: None,
                        })
                        .with_account_name("PlaceholderRecord"),
                );
            }
            Self::try_deserialize_unchecked(buf)
        }
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
            let mut data: &[u8] = &buf[PlaceholderRecord::DISCRIMINATOR.len()..];
            AnchorDeserialize::deserialize(&mut data)
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into()
                })
        }
    }
    #[automatically_derived]
    impl anchor_lang::Discriminator for PlaceholderRecord {
        const DISCRIMINATOR: &'static [u8] = &[70, 2, 95, 178, 67, 74, 56, 8];
    }
    #[automatically_derived]
    impl anchor_lang::Owner for PlaceholderRecord {
        fn owner() -> Pubkey {
            crate::ID
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for PlaceholderRecord {
        #[inline]
        fn default() -> PlaceholderRecord {
            PlaceholderRecord {
                compression_info: ::core::default::Default::default(),
                owner: ::core::default::Default::default(),
                name: ::core::default::Default::default(),
                placeholder_id: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PlaceholderRecord {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "PlaceholderRecord",
                "compression_info",
                &self.compression_info,
                "owner",
                &self.owner,
                "name",
                &self.name,
                "placeholder_id",
                &&self.placeholder_id,
            )
        }
    }
    impl ::light_hasher::to_byte_array::ToByteArray for PlaceholderRecord {
        const NUM_FIELDS: usize = 4usize;
        fn to_byte_array(
            &self,
        ) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
            use ::light_hasher::to_byte_array::ToByteArray;
            use ::light_hasher::hash_to_field_size::HashToFieldSize;
            use ::light_hasher::Hasher;
            let mut result = ::light_hasher::Poseidon::hashv(
                &[
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                            self.owner.as_ref(),
                        )
                        .as_slice(),
                    self.name.to_byte_array()?.as_slice(),
                    self.placeholder_id.to_byte_array()?.as_slice(),
                ],
            )?;
            if ::light_hasher::Poseidon::ID != ::light_hasher::Poseidon::ID {
                result[0] = 0;
            }
            Ok(result)
        }
    }
    impl ::light_hasher::DataHasher for PlaceholderRecord {
        fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
        where
            H: ::light_hasher::Hasher,
        {
            use ::light_hasher::DataHasher;
            use ::light_hasher::Hasher;
            use ::light_hasher::to_byte_array::ToByteArray;
            use ::light_hasher::hash_to_field_size::HashToFieldSize;
            {
                if std::env::var("RUST_BACKTRACE").is_ok() {
                    let debug_prints: Vec<[u8; 32]> = <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                                self.owner.as_ref(),
                            ),
                            self.name.to_byte_array()?,
                            self.placeholder_id.to_byte_array()?,
                        ]),
                    );
                    {
                        ::std::io::_print(
                            format_args!("DataHasher::hash inputs {0:?}\n", debug_prints),
                        );
                    };
                }
            }
            let mut result = H::hashv(
                &[
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                            self.owner.as_ref(),
                        )
                        .as_slice(),
                    self.name.to_byte_array()?.as_slice(),
                    self.placeholder_id.to_byte_array()?.as_slice(),
                ],
            )?;
            if H::ID != ::light_hasher::Poseidon::ID {
                result[0] = 0;
            }
            Ok(result)
        }
    }
    impl LightDiscriminator for PlaceholderRecord {
        const LIGHT_DISCRIMINATOR: [u8; 8] = [70, 2, 95, 178, 67, 74, 56, 8];
        const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
        fn discriminator() -> [u8; 8] {
            Self::LIGHT_DISCRIMINATOR
        }
    }
    #[automatically_derived]
    impl anchor_lang::Space for PlaceholderRecord {
        const INIT_SPACE: usize = 0
            + (1 + <CompressionInfo as anchor_lang::Space>::INIT_SPACE) + 32 + (4 + 32)
            + 8;
    }
    impl light_sdk::compressible::HasCompressionInfo for PlaceholderRecord {
        fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
            self.compression_info
                .as_ref()
                .expect("CompressionInfo must be Some on-chain")
        }
        fn compression_info_mut(
            &mut self,
        ) -> &mut light_sdk::compressible::CompressionInfo {
            self.compression_info
                .as_mut()
                .expect("CompressionInfo must be Some on-chain")
        }
        fn compression_info_mut_opt(
            &mut self,
        ) -> &mut Option<light_sdk::compressible::CompressionInfo> {
            &mut self.compression_info
        }
        fn set_compression_info_none(&mut self) {
            self.compression_info = None;
        }
    }
    impl light_sdk::account::Size for PlaceholderRecord {
        fn size(&self) -> usize {
            Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE
        }
    }
    impl light_sdk::compressible::CompressAs for PlaceholderRecord {
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
    impl light_sdk::compressible::compression_info::CompressedInitSpace
    for PlaceholderRecord {
        const COMPRESSED_INIT_SPACE: usize = Self::INIT_SPACE
            - (0 + <CompressionInfo>::INIT_SPACE);
    }
    pub struct PackedPlaceholderRecord {
        pub compression_info: Option<CompressionInfo>,
        pub owner: u8,
        pub name: String,
        pub placeholder_id: u64,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PackedPlaceholderRecord {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "PackedPlaceholderRecord",
                "compression_info",
                &self.compression_info,
                "owner",
                &self.owner,
                "name",
                &self.name,
                "placeholder_id",
                &&self.placeholder_id,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for PackedPlaceholderRecord {
        #[inline]
        fn clone(&self) -> PackedPlaceholderRecord {
            PackedPlaceholderRecord {
                compression_info: ::core::clone::Clone::clone(&self.compression_info),
                owner: ::core::clone::Clone::clone(&self.owner),
                name: ::core::clone::Clone::clone(&self.name),
                placeholder_id: ::core::clone::Clone::clone(&self.placeholder_id),
            }
        }
    }
    impl borsh::ser::BorshSerialize for PackedPlaceholderRecord
    where
        Option<CompressionInfo>: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_info, writer)?;
            borsh::BorshSerialize::serialize(&self.owner, writer)?;
            borsh::BorshSerialize::serialize(&self.name, writer)?;
            borsh::BorshSerialize::serialize(&self.placeholder_id, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for PackedPlaceholderRecord {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <CompressionInfo>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "owner".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "name".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::String,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "placeholder_id".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U64,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <CompressionInfo>::create_type() {
                types.insert(<CompressionInfo>::get_full_path(), ty);
                <CompressionInfo>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "PackedPlaceholderRecord",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for PackedPlaceholderRecord
    where
        Option<CompressionInfo>: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_info: borsh::BorshDeserialize::deserialize_reader(reader)?,
                owner: borsh::BorshDeserialize::deserialize_reader(reader)?,
                name: borsh::BorshDeserialize::deserialize_reader(reader)?,
                placeholder_id: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl light_sdk::compressible::Pack for PlaceholderRecord {
        type Packed = PackedPlaceholderRecord;
        #[inline(never)]
        fn pack(
            &self,
            remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
        ) -> Self::Packed {
            PackedPlaceholderRecord {
                compression_info: None,
                owner: remaining_accounts.insert_or_get(self.owner),
                name: self.name.clone(),
                placeholder_id: self.placeholder_id,
            }
        }
    }
    impl light_sdk::compressible::Unpack for PlaceholderRecord {
        type Unpacked = Self;
        #[inline(never)]
        fn unpack(
            &self,
            _remaining_accounts: &[anchor_lang::prelude::AccountInfo],
        ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
            Ok(self.clone())
        }
    }
    impl light_sdk::compressible::Pack for PackedPlaceholderRecord {
        type Packed = Self;
        #[inline(never)]
        fn pack(
            &self,
            _remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
        ) -> Self::Packed {
            self.clone()
        }
    }
    impl light_sdk::compressible::Unpack for PackedPlaceholderRecord {
        type Unpacked = PlaceholderRecord;
        #[inline(never)]
        fn unpack(
            &self,
            remaining_accounts: &[anchor_lang::prelude::AccountInfo],
        ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
            Ok(PlaceholderRecord {
                compression_info: None,
                owner: *remaining_accounts[self.owner as usize].key,
                name: self.name.clone(),
                placeholder_id: self.placeholder_id,
            })
        }
    }
    pub struct AccountCreationData {
        pub user_name: String,
        pub session_id: u64,
        pub game_type: String,
        pub mint_name: String,
        pub mint_symbol: String,
        pub mint_uri: String,
        pub mint_decimals: u8,
        pub mint_supply: u64,
        pub mint_update_authority: Option<Pubkey>,
        pub mint_freeze_authority: Option<Pubkey>,
        pub additional_metadata: Option<Vec<(String, String)>>,
    }
    impl borsh::ser::BorshSerialize for AccountCreationData
    where
        String: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u64: borsh::ser::BorshSerialize,
        Option<Pubkey>: borsh::ser::BorshSerialize,
        Option<Pubkey>: borsh::ser::BorshSerialize,
        Option<Vec<(String, String)>>: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.user_name, writer)?;
            borsh::BorshSerialize::serialize(&self.session_id, writer)?;
            borsh::BorshSerialize::serialize(&self.game_type, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_name, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_symbol, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_uri, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_decimals, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_supply, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_update_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_freeze_authority, writer)?;
            borsh::BorshSerialize::serialize(&self.additional_metadata, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for AccountCreationData {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            None
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {}
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "AccountCreationData",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for AccountCreationData
    where
        String: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u64: borsh::BorshDeserialize,
        Option<Pubkey>: borsh::BorshDeserialize,
        Option<Pubkey>: borsh::BorshDeserialize,
        Option<Vec<(String, String)>>: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                user_name: borsh::BorshDeserialize::deserialize_reader(reader)?,
                session_id: borsh::BorshDeserialize::deserialize_reader(reader)?,
                game_type: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint_name: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint_symbol: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint_uri: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint_decimals: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint_supply: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint_update_authority: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                mint_freeze_authority: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                additional_metadata: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for AccountCreationData {
        #[inline]
        fn clone(&self) -> AccountCreationData {
            AccountCreationData {
                user_name: ::core::clone::Clone::clone(&self.user_name),
                session_id: ::core::clone::Clone::clone(&self.session_id),
                game_type: ::core::clone::Clone::clone(&self.game_type),
                mint_name: ::core::clone::Clone::clone(&self.mint_name),
                mint_symbol: ::core::clone::Clone::clone(&self.mint_symbol),
                mint_uri: ::core::clone::Clone::clone(&self.mint_uri),
                mint_decimals: ::core::clone::Clone::clone(&self.mint_decimals),
                mint_supply: ::core::clone::Clone::clone(&self.mint_supply),
                mint_update_authority: ::core::clone::Clone::clone(
                    &self.mint_update_authority,
                ),
                mint_freeze_authority: ::core::clone::Clone::clone(
                    &self.mint_freeze_authority,
                ),
                additional_metadata: ::core::clone::Clone::clone(
                    &self.additional_metadata,
                ),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for AccountCreationData {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "user_name",
                "session_id",
                "game_type",
                "mint_name",
                "mint_symbol",
                "mint_uri",
                "mint_decimals",
                "mint_supply",
                "mint_update_authority",
                "mint_freeze_authority",
                "additional_metadata",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.user_name,
                &self.session_id,
                &self.game_type,
                &self.mint_name,
                &self.mint_symbol,
                &self.mint_uri,
                &self.mint_decimals,
                &self.mint_supply,
                &self.mint_update_authority,
                &self.mint_freeze_authority,
                &&self.additional_metadata,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(
                f,
                "AccountCreationData",
                names,
                values,
            )
        }
    }
    pub struct TokenAccountInfo {
        pub user: Pubkey,
        pub mint: Pubkey,
    }
    impl borsh::ser::BorshSerialize for TokenAccountInfo
    where
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.user, writer)?;
            borsh::BorshSerialize::serialize(&self.mint, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for TokenAccountInfo {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "user".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Pubkey,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "mint".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Pubkey,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {}
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "TokenAccountInfo",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for TokenAccountInfo
    where
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                user: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    pub struct CompressionParams {
        pub proof: ValidityProof,
        pub user_compressed_address: [u8; 32],
        pub user_address_tree_info: PackedAddressTreeInfo,
        pub user_output_state_tree_index: u8,
        pub game_compressed_address: [u8; 32],
        pub game_address_tree_info: PackedAddressTreeInfo,
        pub game_output_state_tree_index: u8,
        pub mint_bump: u8,
        pub mint_with_context: CompressedMintWithContext,
    }
    impl borsh::ser::BorshSerialize for CompressionParams
    where
        ValidityProof: borsh::ser::BorshSerialize,
        [u8; 32]: borsh::ser::BorshSerialize,
        PackedAddressTreeInfo: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        [u8; 32]: borsh::ser::BorshSerialize,
        PackedAddressTreeInfo: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
        CompressedMintWithContext: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.user_compressed_address, writer)?;
            borsh::BorshSerialize::serialize(&self.user_address_tree_info, writer)?;
            borsh::BorshSerialize::serialize(
                &self.user_output_state_tree_index,
                writer,
            )?;
            borsh::BorshSerialize::serialize(&self.game_compressed_address, writer)?;
            borsh::BorshSerialize::serialize(&self.game_address_tree_info, writer)?;
            borsh::BorshSerialize::serialize(
                &self.game_output_state_tree_index,
                writer,
            )?;
            borsh::BorshSerialize::serialize(&self.mint_bump, writer)?;
            borsh::BorshSerialize::serialize(&self.mint_with_context, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for CompressionParams {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: ::alloc::vec::Vec::new(),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "proof".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <ValidityProof>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "user_compressed_address".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Array(
                                            Box::new(anchor_lang::idl::types::IdlType::U8),
                                            anchor_lang::idl::types::IdlArrayLen::Value(32),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "user_address_tree_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <PackedAddressTreeInfo>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "user_output_state_tree_index".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "game_compressed_address".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Array(
                                            Box::new(anchor_lang::idl::types::IdlType::U8),
                                            anchor_lang::idl::types::IdlArrayLen::Value(32),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "game_address_tree_info".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <PackedAddressTreeInfo>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "game_output_state_tree_index".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "mint_bump".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "mint_with_context".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <CompressedMintWithContext>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <ValidityProof>::create_type() {
                types.insert(<ValidityProof>::get_full_path(), ty);
                <ValidityProof>::insert_types(types);
            }
            if let Some(ty) = <PackedAddressTreeInfo>::create_type() {
                types.insert(<PackedAddressTreeInfo>::get_full_path(), ty);
                <PackedAddressTreeInfo>::insert_types(types);
            }
            if let Some(ty) = <PackedAddressTreeInfo>::create_type() {
                types.insert(<PackedAddressTreeInfo>::get_full_path(), ty);
                <PackedAddressTreeInfo>::insert_types(types);
            }
            if let Some(ty) = <CompressedMintWithContext>::create_type() {
                types.insert(<CompressedMintWithContext>::get_full_path(), ty);
                <CompressedMintWithContext>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::state",
                        "CompressionParams",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for CompressionParams
    where
        ValidityProof: borsh::BorshDeserialize,
        [u8; 32]: borsh::BorshDeserialize,
        PackedAddressTreeInfo: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        [u8; 32]: borsh::BorshDeserialize,
        PackedAddressTreeInfo: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
        CompressedMintWithContext: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                user_compressed_address: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                user_address_tree_info: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                user_output_state_tree_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                game_compressed_address: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                game_address_tree_info: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                game_output_state_tree_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                mint_bump: borsh::BorshDeserialize::deserialize_reader(reader)?,
                mint_with_context: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
}
pub use instruction_accounts::*;
pub use state::{
    AccountCreationData, CompressionParams, GameSession, PackedGameSession,
    PackedPlaceholderRecord, PackedUserRecord, PlaceholderRecord, UserRecord,
};
/// The static program ID
pub static ID: anchor_lang::solana_program::pubkey::Pubkey = anchor_lang::solana_program::pubkey::Pubkey::new_from_array([
    210u8, 105u8, 70u8, 12u8, 221u8, 105u8, 241u8, 17u8, 213u8, 13u8, 33u8, 136u8, 234u8,
    19u8, 98u8, 172u8, 171u8, 195u8, 107u8, 245u8, 165u8, 128u8, 107u8, 144u8, 114u8,
    191u8, 208u8, 249u8, 245u8, 228u8, 112u8, 58u8,
]);
/// Const version of `ID`
pub const ID_CONST: anchor_lang::solana_program::pubkey::Pubkey = anchor_lang::solana_program::pubkey::Pubkey::new_from_array([
    210u8, 105u8, 70u8, 12u8, 221u8, 105u8, 241u8, 17u8, 213u8, 13u8, 33u8, 136u8, 234u8,
    19u8, 98u8, 172u8, 171u8, 195u8, 107u8, 245u8, 165u8, 128u8, 107u8, 144u8, 114u8,
    191u8, 208u8, 249u8, 245u8, 228u8, 112u8, 58u8,
]);
/// Confirms that a given pubkey is equivalent to the program ID
pub fn check_id(id: &anchor_lang::solana_program::pubkey::Pubkey) -> bool {
    id == &ID
}
/// Returns the program ID
pub fn id() -> anchor_lang::solana_program::pubkey::Pubkey {
    ID
}
/// Const version of `ID`
pub const fn id_const() -> anchor_lang::solana_program::pubkey::Pubkey {
    ID_CONST
}
pub const LIGHT_CPI_SIGNER: CpiSigner = CpiSigner {
    program_id: [
        210, 105, 70, 12, 221, 105, 241, 17, 213, 13, 33, 136, 234, 19, 98, 172, 171,
        195, 107, 245, 165, 128, 107, 144, 114, 191, 208, 249, 245, 228, 112, 58,
    ],
    cpi_signer: [
        7, 140, 176, 100, 222, 137, 149, 7, 120, 159, 248, 116, 14, 4, 91, 218, 226, 34,
        112, 177, 126, 72, 240, 6, 250, 166, 152, 59, 65, 132, 35, 95,
    ],
    bump: 251u8,
};
const _: () = {
    const COMPRESSED_SIZE: usize = 8
        + <UserRecord as light_sdk::compressible::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
    if COMPRESSED_SIZE > 800 {
        {
            ::core::panicking::panic_fmt(
                format_args!(
                    "Compressed account \'UserRecord\' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com",
                ),
            );
        };
    }
};
const _: () = {
    const COMPRESSED_SIZE: usize = 8
        + <GameSession as light_sdk::compressible::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
    if COMPRESSED_SIZE > 800 {
        {
            ::core::panicking::panic_fmt(
                format_args!(
                    "Compressed account \'GameSession\' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com",
                ),
            );
        };
    }
};
const _: () = {
    const COMPRESSED_SIZE: usize = 8
        + <PlaceholderRecord as light_sdk::compressible::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
    if COMPRESSED_SIZE > 800 {
        {
            ::core::panicking::panic_fmt(
                format_args!(
                    "Compressed account \'PlaceholderRecord\' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com",
                ),
            );
        };
    }
};
#[repr(u32)]
/// Auto-generated error codes for compressible instructions
/// These are separate from the user's ErrorCode enum to avoid conflicts
pub enum CompressibleInstructionError {
    InvalidRentSponsor,
    MissingSeedAccount,
    AtaDoesNotUseSeedDerivation,
    CTokenDecompressionNotImplemented,
    PdaDecompressionNotImplemented,
    TokenCompressionNotImplemented,
    PdaCompressionNotImplemented,
}
#[automatically_derived]
impl ::core::fmt::Debug for CompressibleInstructionError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                CompressibleInstructionError::InvalidRentSponsor => "InvalidRentSponsor",
                CompressibleInstructionError::MissingSeedAccount => "MissingSeedAccount",
                CompressibleInstructionError::AtaDoesNotUseSeedDerivation => {
                    "AtaDoesNotUseSeedDerivation"
                }
                CompressibleInstructionError::CTokenDecompressionNotImplemented => {
                    "CTokenDecompressionNotImplemented"
                }
                CompressibleInstructionError::PdaDecompressionNotImplemented => {
                    "PdaDecompressionNotImplemented"
                }
                CompressibleInstructionError::TokenCompressionNotImplemented => {
                    "TokenCompressionNotImplemented"
                }
                CompressibleInstructionError::PdaCompressionNotImplemented => {
                    "PdaCompressionNotImplemented"
                }
            },
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for CompressibleInstructionError {
    #[inline]
    fn clone(&self) -> CompressibleInstructionError {
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for CompressibleInstructionError {}
impl CompressibleInstructionError {
    /// Gets the name of this [#enum_name].
    pub fn name(&self) -> String {
        match self {
            CompressibleInstructionError::InvalidRentSponsor => {
                "InvalidRentSponsor".to_string()
            }
            CompressibleInstructionError::MissingSeedAccount => {
                "MissingSeedAccount".to_string()
            }
            CompressibleInstructionError::AtaDoesNotUseSeedDerivation => {
                "AtaDoesNotUseSeedDerivation".to_string()
            }
            CompressibleInstructionError::CTokenDecompressionNotImplemented => {
                "CTokenDecompressionNotImplemented".to_string()
            }
            CompressibleInstructionError::PdaDecompressionNotImplemented => {
                "PdaDecompressionNotImplemented".to_string()
            }
            CompressibleInstructionError::TokenCompressionNotImplemented => {
                "TokenCompressionNotImplemented".to_string()
            }
            CompressibleInstructionError::PdaCompressionNotImplemented => {
                "PdaCompressionNotImplemented".to_string()
            }
        }
    }
}
impl From<CompressibleInstructionError> for u32 {
    fn from(e: CompressibleInstructionError) -> u32 {
        e as u32 + anchor_lang::error::ERROR_CODE_OFFSET
    }
}
impl From<CompressibleInstructionError> for anchor_lang::error::Error {
    fn from(error_code: CompressibleInstructionError) -> anchor_lang::error::Error {
        anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
            error_name: error_code.name(),
            error_code_number: error_code.into(),
            error_msg: error_code.to_string(),
            error_origin: None,
            compared_values: None,
        })
    }
}
impl std::fmt::Display for CompressibleInstructionError {
    fn fmt(
        &self,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            CompressibleInstructionError::InvalidRentSponsor => {
                fmt.write_fmt(format_args!("Rent sponsor does not match config"))
            }
            CompressibleInstructionError::MissingSeedAccount => {
                fmt.write_fmt(
                    format_args!(
                        "Required seed account is missing for decompression - check that all seed accounts for compressed accounts are provided",
                    ),
                )
            }
            CompressibleInstructionError::AtaDoesNotUseSeedDerivation => {
                fmt.write_fmt(
                    format_args!(
                        "ATA variants use SPL ATA derivation, not seed-based PDA derivation",
                    ),
                )
            }
            CompressibleInstructionError::CTokenDecompressionNotImplemented => {
                fmt.write_fmt(format_args!("CToken decompression not yet implemented"))
            }
            CompressibleInstructionError::PdaDecompressionNotImplemented => {
                fmt.write_fmt(
                    format_args!(
                        "PDA decompression not implemented in token-only variant",
                    ),
                )
            }
            CompressibleInstructionError::TokenCompressionNotImplemented => {
                fmt.write_fmt(
                    format_args!("Token compression not implemented in PDA-only variant"),
                )
            }
            CompressibleInstructionError::PdaCompressionNotImplemented => {
                fmt.write_fmt(
                    format_args!("PDA compression not implemented in token-only variant"),
                )
            }
        }
    }
}
/// Auto-generated CTokenAccountVariant enum from token seed specifications
#[repr(u8)]
pub enum CTokenAccountVariant {
    CTokenSigner = 0u8,
}
impl borsh::ser::BorshSerialize for CTokenAccountVariant {
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        let variant_idx: u8 = match self {
            CTokenAccountVariant::CTokenSigner => 0u8,
        };
        writer.write_all(&variant_idx.to_le_bytes())?;
        match self {
            CTokenAccountVariant::CTokenSigner => {}
        }
        Ok(())
    }
}
impl anchor_lang::idl::build::IdlBuild for CTokenAccountVariant {
    fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
        Some(anchor_lang::idl::types::IdlTypeDef {
            name: Self::get_full_path(),
            docs: <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    "Auto-generated CTokenAccountVariant enum from token seed specifications"
                        .into(),
                ]),
            ),
            serialization: anchor_lang::idl::types::IdlSerialization::default(),
            repr: Some(
                anchor_lang::idl::types::IdlRepr::Rust(anchor_lang::idl::types::IdlReprModifier {
                    packed: false,
                    align: None,
                }),
            ),
            generics: ::alloc::vec::Vec::new(),
            ty: anchor_lang::idl::types::IdlTypeDefTy::Enum {
                variants: <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "CTokenSigner".into(),
                            fields: None,
                        },
                    ]),
                ),
            },
        })
    }
    fn insert_types(
        types: &mut std::collections::BTreeMap<
            String,
            anchor_lang::idl::types::IdlTypeDef,
        >,
    ) {}
    fn get_full_path() -> String {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(
                format_args!(
                    "{0}::{1}",
                    "csdk_anchor_full_derived_test",
                    "CTokenAccountVariant",
                ),
            )
        })
    }
}
impl borsh::de::BorshDeserialize for CTokenAccountVariant {
    fn deserialize_reader<R: borsh::maybestd::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        let tag = <u8 as borsh::de::BorshDeserialize>::deserialize_reader(reader)?;
        <Self as borsh::de::EnumExt>::deserialize_variant(reader, tag)
    }
}
impl borsh::de::EnumExt for CTokenAccountVariant {
    fn deserialize_variant<R: borsh::maybestd::io::Read>(
        reader: &mut R,
        variant_idx: u8,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        let mut return_value = match variant_idx {
            0u8 => CTokenAccountVariant::CTokenSigner,
            _ => {
                return Err(
                    borsh::maybestd::io::Error::new(
                        borsh::maybestd::io::ErrorKind::InvalidInput,
                        ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!("Unexpected variant index: {0:?}", variant_idx),
                            )
                        }),
                    ),
                );
            }
        };
        Ok(return_value)
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for CTokenAccountVariant {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "CTokenSigner")
    }
}
#[automatically_derived]
impl ::core::clone::Clone for CTokenAccountVariant {
    #[inline]
    fn clone(&self) -> CTokenAccountVariant {
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for CTokenAccountVariant {}
pub enum CompressedAccountVariant {
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),
    GameSession(GameSession),
    PackedGameSession(PackedGameSession),
    PlaceholderRecord(PlaceholderRecord),
    PackedPlaceholderRecord(PackedPlaceholderRecord),
    PackedCTokenData(
        light_compressed_token_sdk::compat::PackedCTokenData<CTokenAccountVariant>,
    ),
    CTokenData(light_compressed_token_sdk::compat::CTokenData<CTokenAccountVariant>),
}
#[automatically_derived]
impl ::core::clone::Clone for CompressedAccountVariant {
    #[inline]
    fn clone(&self) -> CompressedAccountVariant {
        match self {
            CompressedAccountVariant::UserRecord(__self_0) => {
                CompressedAccountVariant::UserRecord(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
            CompressedAccountVariant::PackedUserRecord(__self_0) => {
                CompressedAccountVariant::PackedUserRecord(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
            CompressedAccountVariant::GameSession(__self_0) => {
                CompressedAccountVariant::GameSession(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
            CompressedAccountVariant::PackedGameSession(__self_0) => {
                CompressedAccountVariant::PackedGameSession(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
            CompressedAccountVariant::PlaceholderRecord(__self_0) => {
                CompressedAccountVariant::PlaceholderRecord(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
            CompressedAccountVariant::PackedPlaceholderRecord(__self_0) => {
                CompressedAccountVariant::PackedPlaceholderRecord(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
            CompressedAccountVariant::PackedCTokenData(__self_0) => {
                CompressedAccountVariant::PackedCTokenData(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
            CompressedAccountVariant::CTokenData(__self_0) => {
                CompressedAccountVariant::CTokenData(
                    ::core::clone::Clone::clone(__self_0),
                )
            }
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for CompressedAccountVariant {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            CompressedAccountVariant::UserRecord(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "UserRecord",
                    &__self_0,
                )
            }
            CompressedAccountVariant::PackedUserRecord(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "PackedUserRecord",
                    &__self_0,
                )
            }
            CompressedAccountVariant::GameSession(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "GameSession",
                    &__self_0,
                )
            }
            CompressedAccountVariant::PackedGameSession(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "PackedGameSession",
                    &__self_0,
                )
            }
            CompressedAccountVariant::PlaceholderRecord(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "PlaceholderRecord",
                    &__self_0,
                )
            }
            CompressedAccountVariant::PackedPlaceholderRecord(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "PackedPlaceholderRecord",
                    &__self_0,
                )
            }
            CompressedAccountVariant::PackedCTokenData(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "PackedCTokenData",
                    &__self_0,
                )
            }
            CompressedAccountVariant::CTokenData(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "CTokenData",
                    &__self_0,
                )
            }
        }
    }
}
impl borsh::ser::BorshSerialize for CompressedAccountVariant
where
    UserRecord: borsh::ser::BorshSerialize,
    PackedUserRecord: borsh::ser::BorshSerialize,
    GameSession: borsh::ser::BorshSerialize,
    PackedGameSession: borsh::ser::BorshSerialize,
    PlaceholderRecord: borsh::ser::BorshSerialize,
    PackedPlaceholderRecord: borsh::ser::BorshSerialize,
    light_compressed_token_sdk::compat::PackedCTokenData<
        CTokenAccountVariant,
    >: borsh::ser::BorshSerialize,
    light_compressed_token_sdk::compat::CTokenData<
        CTokenAccountVariant,
    >: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        let variant_idx: u8 = match self {
            CompressedAccountVariant::UserRecord(..) => 0u8,
            CompressedAccountVariant::PackedUserRecord(..) => 1u8,
            CompressedAccountVariant::GameSession(..) => 2u8,
            CompressedAccountVariant::PackedGameSession(..) => 3u8,
            CompressedAccountVariant::PlaceholderRecord(..) => 4u8,
            CompressedAccountVariant::PackedPlaceholderRecord(..) => 5u8,
            CompressedAccountVariant::PackedCTokenData(..) => 6u8,
            CompressedAccountVariant::CTokenData(..) => 7u8,
        };
        writer.write_all(&variant_idx.to_le_bytes())?;
        match self {
            CompressedAccountVariant::UserRecord(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            CompressedAccountVariant::PackedUserRecord(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            CompressedAccountVariant::GameSession(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            CompressedAccountVariant::PackedGameSession(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            CompressedAccountVariant::PlaceholderRecord(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            CompressedAccountVariant::PackedPlaceholderRecord(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            CompressedAccountVariant::PackedCTokenData(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            CompressedAccountVariant::CTokenData(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
        }
        Ok(())
    }
}
impl anchor_lang::idl::build::IdlBuild for CompressedAccountVariant {
    fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
        Some(anchor_lang::idl::types::IdlTypeDef {
            name: Self::get_full_path(),
            docs: ::alloc::vec::Vec::new(),
            serialization: anchor_lang::idl::types::IdlSerialization::default(),
            repr: None,
            generics: ::alloc::vec::Vec::new(),
            ty: anchor_lang::idl::types::IdlTypeDefTy::Enum {
                variants: <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "UserRecord".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <UserRecord>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "PackedUserRecord".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <PackedUserRecord>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "GameSession".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <GameSession>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "PackedGameSession".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <PackedGameSession>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "PlaceholderRecord".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <PlaceholderRecord>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "PackedPlaceholderRecord".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <PackedPlaceholderRecord>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "PackedCTokenData".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <light_compressed_token_sdk::compat::PackedCTokenData<
                                                    CTokenAccountVariant,
                                                >>::get_full_path(),
                                                generics: <[_]>::into_vec(
                                                    ::alloc::boxed::box_new([
                                                        anchor_lang::idl::types::IdlGenericArg::Type {
                                                            ty: anchor_lang::idl::types::IdlType::Defined {
                                                                name: <CTokenAccountVariant>::get_full_path(),
                                                                generics: ::alloc::vec::Vec::new(),
                                                            },
                                                        },
                                                    ]),
                                                ),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "CTokenData".into(),
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlType::Defined {
                                                name: <light_compressed_token_sdk::compat::CTokenData<
                                                    CTokenAccountVariant,
                                                >>::get_full_path(),
                                                generics: <[_]>::into_vec(
                                                    ::alloc::boxed::box_new([
                                                        anchor_lang::idl::types::IdlGenericArg::Type {
                                                            ty: anchor_lang::idl::types::IdlType::Defined {
                                                                name: <CTokenAccountVariant>::get_full_path(),
                                                                generics: ::alloc::vec::Vec::new(),
                                                            },
                                                        },
                                                    ]),
                                                ),
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    ]),
                ),
            },
        })
    }
    fn insert_types(
        types: &mut std::collections::BTreeMap<
            String,
            anchor_lang::idl::types::IdlTypeDef,
        >,
    ) {
        if let Some(ty) = <UserRecord>::create_type() {
            types.insert(<UserRecord>::get_full_path(), ty);
            <UserRecord>::insert_types(types);
        }
        if let Some(ty) = <PackedUserRecord>::create_type() {
            types.insert(<PackedUserRecord>::get_full_path(), ty);
            <PackedUserRecord>::insert_types(types);
        }
        if let Some(ty) = <GameSession>::create_type() {
            types.insert(<GameSession>::get_full_path(), ty);
            <GameSession>::insert_types(types);
        }
        if let Some(ty) = <PackedGameSession>::create_type() {
            types.insert(<PackedGameSession>::get_full_path(), ty);
            <PackedGameSession>::insert_types(types);
        }
        if let Some(ty) = <PlaceholderRecord>::create_type() {
            types.insert(<PlaceholderRecord>::get_full_path(), ty);
            <PlaceholderRecord>::insert_types(types);
        }
        if let Some(ty) = <PackedPlaceholderRecord>::create_type() {
            types.insert(<PackedPlaceholderRecord>::get_full_path(), ty);
            <PackedPlaceholderRecord>::insert_types(types);
        }
        if let Some(ty) = <light_compressed_token_sdk::compat::PackedCTokenData<
            CTokenAccountVariant,
        >>::create_type() {
            types
                .insert(
                    <light_compressed_token_sdk::compat::PackedCTokenData<
                        CTokenAccountVariant,
                    >>::get_full_path(),
                    ty,
                );
            <light_compressed_token_sdk::compat::PackedCTokenData<
                CTokenAccountVariant,
            >>::insert_types(types);
        }
        if let Some(ty) = <CTokenAccountVariant>::create_type() {
            types.insert(<CTokenAccountVariant>::get_full_path(), ty);
            <CTokenAccountVariant>::insert_types(types);
        }
        if let Some(ty) = <light_compressed_token_sdk::compat::CTokenData<
            CTokenAccountVariant,
        >>::create_type() {
            types
                .insert(
                    <light_compressed_token_sdk::compat::CTokenData<
                        CTokenAccountVariant,
                    >>::get_full_path(),
                    ty,
                );
            <light_compressed_token_sdk::compat::CTokenData<
                CTokenAccountVariant,
            >>::insert_types(types);
        }
        if let Some(ty) = <CTokenAccountVariant>::create_type() {
            types.insert(<CTokenAccountVariant>::get_full_path(), ty);
            <CTokenAccountVariant>::insert_types(types);
        }
    }
    fn get_full_path() -> String {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(
                format_args!(
                    "{0}::{1}",
                    "csdk_anchor_full_derived_test",
                    "CompressedAccountVariant",
                ),
            )
        })
    }
}
impl borsh::de::BorshDeserialize for CompressedAccountVariant
where
    UserRecord: borsh::BorshDeserialize,
    PackedUserRecord: borsh::BorshDeserialize,
    GameSession: borsh::BorshDeserialize,
    PackedGameSession: borsh::BorshDeserialize,
    PlaceholderRecord: borsh::BorshDeserialize,
    PackedPlaceholderRecord: borsh::BorshDeserialize,
    light_compressed_token_sdk::compat::PackedCTokenData<
        CTokenAccountVariant,
    >: borsh::BorshDeserialize,
    light_compressed_token_sdk::compat::CTokenData<
        CTokenAccountVariant,
    >: borsh::BorshDeserialize,
{
    fn deserialize_reader<R: borsh::maybestd::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        let tag = <u8 as borsh::de::BorshDeserialize>::deserialize_reader(reader)?;
        <Self as borsh::de::EnumExt>::deserialize_variant(reader, tag)
    }
}
impl borsh::de::EnumExt for CompressedAccountVariant
where
    UserRecord: borsh::BorshDeserialize,
    PackedUserRecord: borsh::BorshDeserialize,
    GameSession: borsh::BorshDeserialize,
    PackedGameSession: borsh::BorshDeserialize,
    PlaceholderRecord: borsh::BorshDeserialize,
    PackedPlaceholderRecord: borsh::BorshDeserialize,
    light_compressed_token_sdk::compat::PackedCTokenData<
        CTokenAccountVariant,
    >: borsh::BorshDeserialize,
    light_compressed_token_sdk::compat::CTokenData<
        CTokenAccountVariant,
    >: borsh::BorshDeserialize,
{
    fn deserialize_variant<R: borsh::maybestd::io::Read>(
        reader: &mut R,
        variant_idx: u8,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        let mut return_value = match variant_idx {
            0u8 => {
                CompressedAccountVariant::UserRecord(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            1u8 => {
                CompressedAccountVariant::PackedUserRecord(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            2u8 => {
                CompressedAccountVariant::GameSession(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            3u8 => {
                CompressedAccountVariant::PackedGameSession(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            4u8 => {
                CompressedAccountVariant::PlaceholderRecord(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            5u8 => {
                CompressedAccountVariant::PackedPlaceholderRecord(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            6u8 => {
                CompressedAccountVariant::PackedCTokenData(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            7u8 => {
                CompressedAccountVariant::CTokenData(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                )
            }
            _ => {
                return Err(
                    borsh::maybestd::io::Error::new(
                        borsh::maybestd::io::ErrorKind::InvalidInput,
                        ::alloc::__export::must_use({
                            ::alloc::fmt::format(
                                format_args!("Unexpected variant index: {0:?}", variant_idx),
                            )
                        }),
                    ),
                );
            }
        };
        Ok(return_value)
    }
}
impl Default for CompressedAccountVariant {
    fn default() -> Self {
        Self::UserRecord(UserRecord::default())
    }
}
impl light_hasher::DataHasher for CompressedAccountVariant {
    fn hash<H: light_hasher::Hasher>(
        &self,
    ) -> std::result::Result<[u8; 32], light_hasher::HasherError> {
        match self {
            CompressedAccountVariant::UserRecord(data) => {
                <UserRecord as light_hasher::DataHasher>::hash::<H>(data)
            }
            CompressedAccountVariant::PackedUserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::GameSession(data) => {
                <GameSession as light_hasher::DataHasher>::hash::<H>(data)
            }
            CompressedAccountVariant::PackedGameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PlaceholderRecord(data) => {
                <PlaceholderRecord as light_hasher::DataHasher>::hash::<H>(data)
            }
            CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::PackedCTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::CTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
    }
}
impl light_sdk::LightDiscriminator for CompressedAccountVariant {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}
impl light_sdk::compressible::HasCompressionInfo for CompressedAccountVariant {
    fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
        match self {
            CompressedAccountVariant::UserRecord(data) => {
                <UserRecord as light_sdk::compressible::HasCompressionInfo>::compression_info(
                    data,
                )
            }
            CompressedAccountVariant::PackedUserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::GameSession(data) => {
                <GameSession as light_sdk::compressible::HasCompressionInfo>::compression_info(
                    data,
                )
            }
            CompressedAccountVariant::PackedGameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PlaceholderRecord(data) => {
                <PlaceholderRecord as light_sdk::compressible::HasCompressionInfo>::compression_info(
                    data,
                )
            }
            CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::PackedCTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::CTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
    }
    fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
        match self {
            CompressedAccountVariant::UserRecord(data) => {
                <UserRecord as light_sdk::compressible::HasCompressionInfo>::compression_info_mut(
                    data,
                )
            }
            CompressedAccountVariant::PackedUserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::GameSession(data) => {
                <GameSession as light_sdk::compressible::HasCompressionInfo>::compression_info_mut(
                    data,
                )
            }
            CompressedAccountVariant::PackedGameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PlaceholderRecord(data) => {
                <PlaceholderRecord as light_sdk::compressible::HasCompressionInfo>::compression_info_mut(
                    data,
                )
            }
            CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::PackedCTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::CTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
    }
    fn compression_info_mut_opt(
        &mut self,
    ) -> &mut Option<light_sdk::compressible::CompressionInfo> {
        match self {
            CompressedAccountVariant::UserRecord(data) => {
                <UserRecord as light_sdk::compressible::HasCompressionInfo>::compression_info_mut_opt(
                    data,
                )
            }
            CompressedAccountVariant::PackedUserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::GameSession(data) => {
                <GameSession as light_sdk::compressible::HasCompressionInfo>::compression_info_mut_opt(
                    data,
                )
            }
            CompressedAccountVariant::PackedGameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PlaceholderRecord(data) => {
                <PlaceholderRecord as light_sdk::compressible::HasCompressionInfo>::compression_info_mut_opt(
                    data,
                )
            }
            CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::PackedCTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::CTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
    }
    fn set_compression_info_none(&mut self) {
        match self {
            CompressedAccountVariant::UserRecord(data) => {
                <UserRecord as light_sdk::compressible::HasCompressionInfo>::set_compression_info_none(
                    data,
                )
            }
            CompressedAccountVariant::PackedUserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::GameSession(data) => {
                <GameSession as light_sdk::compressible::HasCompressionInfo>::set_compression_info_none(
                    data,
                )
            }
            CompressedAccountVariant::PackedGameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PlaceholderRecord(data) => {
                <PlaceholderRecord as light_sdk::compressible::HasCompressionInfo>::set_compression_info_none(
                    data,
                )
            }
            CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::PackedCTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::CTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
    }
}
impl light_sdk::account::Size for CompressedAccountVariant {
    fn size(&self) -> usize {
        match self {
            CompressedAccountVariant::UserRecord(data) => {
                <UserRecord as light_sdk::account::Size>::size(data)
            }
            CompressedAccountVariant::PackedUserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::GameSession(data) => {
                <GameSession as light_sdk::account::Size>::size(data)
            }
            CompressedAccountVariant::PackedGameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PlaceholderRecord(data) => {
                <PlaceholderRecord as light_sdk::account::Size>::size(data)
            }
            CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::PackedCTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::CTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
    }
}
impl light_sdk::compressible::Pack for CompressedAccountVariant {
    type Packed = Self;
    fn pack(
        &self,
        remaining_accounts: &mut light_sdk::instruction::PackedAccounts,
    ) -> Self::Packed {
        match self {
            CompressedAccountVariant::PackedUserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::UserRecord(data) => {
                CompressedAccountVariant::PackedUserRecord(
                    <UserRecord as light_sdk::compressible::Pack>::pack(
                        data,
                        remaining_accounts,
                    ),
                )
            }
            CompressedAccountVariant::PackedGameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::GameSession(data) => {
                CompressedAccountVariant::PackedGameSession(
                    <GameSession as light_sdk::compressible::Pack>::pack(
                        data,
                        remaining_accounts,
                    ),
                )
            }
            CompressedAccountVariant::PackedPlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PlaceholderRecord(data) => {
                CompressedAccountVariant::PackedPlaceholderRecord(
                    <PlaceholderRecord as light_sdk::compressible::Pack>::pack(
                        data,
                        remaining_accounts,
                    ),
                )
            }
            Self::PackedCTokenData(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::CTokenData(data) => {
                Self::PackedCTokenData(
                    light_compressed_token_sdk::Pack::pack(data, remaining_accounts),
                )
            }
        }
    }
}
impl light_sdk::compressible::Unpack for CompressedAccountVariant {
    type Unpacked = Self;
    fn unpack(
        &self,
        remaining_accounts: &[anchor_lang::prelude::AccountInfo],
    ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
        match self {
            CompressedAccountVariant::PackedUserRecord(data) => {
                Ok(
                    CompressedAccountVariant::UserRecord(
                        <PackedUserRecord as light_sdk::compressible::Unpack>::unpack(
                            data,
                            remaining_accounts,
                        )?,
                    ),
                )
            }
            CompressedAccountVariant::UserRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PackedGameSession(data) => {
                Ok(
                    CompressedAccountVariant::GameSession(
                        <PackedGameSession as light_sdk::compressible::Unpack>::unpack(
                            data,
                            remaining_accounts,
                        )?,
                    ),
                )
            }
            CompressedAccountVariant::GameSession(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            CompressedAccountVariant::PackedPlaceholderRecord(data) => {
                Ok(
                    CompressedAccountVariant::PlaceholderRecord(
                        <PackedPlaceholderRecord as light_sdk::compressible::Unpack>::unpack(
                            data,
                            remaining_accounts,
                        )?,
                    ),
                )
            }
            CompressedAccountVariant::PlaceholderRecord(_) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
            Self::PackedCTokenData(_data) => Ok(self.clone()),
            Self::CTokenData(_data) => {
                ::core::panicking::panic("internal error: entered unreachable code")
            }
        }
    }
}
pub struct CompressedAccountData {
    pub meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
    pub data: CompressedAccountVariant,
}
#[automatically_derived]
impl ::core::clone::Clone for CompressedAccountData {
    #[inline]
    fn clone(&self) -> CompressedAccountData {
        CompressedAccountData {
            meta: ::core::clone::Clone::clone(&self.meta),
            data: ::core::clone::Clone::clone(&self.data),
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for CompressedAccountData {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "CompressedAccountData",
            "meta",
            &self.meta,
            "data",
            &&self.data,
        )
    }
}
impl borsh::de::BorshDeserialize for CompressedAccountData
where
    light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress: borsh::BorshDeserialize,
    CompressedAccountVariant: borsh::BorshDeserialize,
{
    fn deserialize_reader<R: borsh::maybestd::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            meta: borsh::BorshDeserialize::deserialize_reader(reader)?,
            data: borsh::BorshDeserialize::deserialize_reader(reader)?,
        })
    }
}
impl borsh::ser::BorshSerialize for CompressedAccountData
where
    light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress: borsh::ser::BorshSerialize,
    CompressedAccountVariant: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        borsh::BorshSerialize::serialize(&self.meta, writer)?;
        borsh::BorshSerialize::serialize(&self.data, writer)?;
        Ok(())
    }
}
impl anchor_lang::idl::build::IdlBuild for CompressedAccountData {
    fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
        Some(anchor_lang::idl::types::IdlTypeDef {
            name: Self::get_full_path(),
            docs: ::alloc::vec::Vec::new(),
            serialization: anchor_lang::idl::types::IdlSerialization::default(),
            repr: None,
            generics: ::alloc::vec::Vec::new(),
            ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                fields: Some(
                    anchor_lang::idl::types::IdlDefinedFields::Named(
                        <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                anchor_lang::idl::types::IdlField {
                                    name: "meta".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Defined {
                                        name: <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::get_full_path(),
                                        generics: ::alloc::vec::Vec::new(),
                                    },
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "data".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Defined {
                                        name: <CompressedAccountVariant>::get_full_path(),
                                        generics: ::alloc::vec::Vec::new(),
                                    },
                                },
                            ]),
                        ),
                    ),
                ),
            },
        })
    }
    fn insert_types(
        types: &mut std::collections::BTreeMap<
            String,
            anchor_lang::idl::types::IdlTypeDef,
        >,
    ) {
        if let Some(ty) = <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::create_type() {
            types
                .insert(
                    <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::get_full_path(),
                    ty,
                );
            <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::insert_types(
                types,
            );
        }
        if let Some(ty) = <CompressedAccountVariant>::create_type() {
            types.insert(<CompressedAccountVariant>::get_full_path(), ty);
            <CompressedAccountVariant>::insert_types(types);
        }
    }
    fn get_full_path() -> String {
        ::alloc::__export::must_use({
            ::alloc::fmt::format(
                format_args!(
                    "{0}::{1}",
                    "csdk_anchor_full_derived_test",
                    "CompressedAccountData",
                ),
            )
        })
    }
}
impl light_sdk::compressible::PdaSeedProvider for UserRecord {
    fn derive_pda_seeds(
        &self,
        program_id: &solana_pubkey::Pubkey,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let seeds: &[&[u8]] = &["user_record".as_bytes(), self.owner.as_ref()];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
        seeds_vec.push(seeds[0usize].to_vec());
        seeds_vec.push(seeds[1usize].to_vec());
        seeds_vec.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seeds_vec, pda)
    }
}
impl light_sdk::compressible::PdaSeedProvider for GameSession {
    fn derive_pda_seeds(
        &self,
        program_id: &solana_pubkey::Pubkey,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let seed_1 = self.session_id.to_le_bytes();
        let seeds: &[&[u8]] = &["game_session".as_bytes(), seed_1.as_ref()];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
        seeds_vec.push(seeds[0usize].to_vec());
        seeds_vec.push(seeds[1usize].to_vec());
        seeds_vec.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seeds_vec, pda)
    }
}
impl light_sdk::compressible::PdaSeedProvider for PlaceholderRecord {
    fn derive_pda_seeds(
        &self,
        program_id: &solana_pubkey::Pubkey,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let seed_1 = self.placeholder_id.to_le_bytes();
        let seeds: &[&[u8]] = &["placeholder_record".as_bytes(), seed_1.as_ref()];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
        seeds_vec.push(seeds[0usize].to_vec());
        seeds_vec.push(seeds[1usize].to_vec());
        seeds_vec.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seeds_vec, pda)
    }
}
use self::csdk_anchor_full_derived_test::*;
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let (program_id, accounts, instruction_data) = unsafe {
        ::solana_program_entrypoint::deserialize(input)
    };
    match entry(program_id, &accounts, instruction_data) {
        Ok(()) => ::solana_program_entrypoint::SUCCESS,
        Err(error) => error.into(),
    }
}
/// The Anchor codegen exposes a programming model where a user defines
/// a set of methods inside of a `#[program]` module in a way similar
/// to writing RPC request handlers. The macro then generates a bunch of
/// code wrapping these user defined methods into something that can be
/// executed on Solana.
///
/// These methods fall into one category for now.
///
/// Global methods - regular methods inside of the `#[program]`.
///
/// Care must be taken by the codegen to prevent collisions between
/// methods in these different namespaces. For this reason, Anchor uses
/// a variant of sighash to perform method dispatch, rather than
/// something like a simple enum variant discriminator.
///
/// The execution flow of the generated code can be roughly outlined:
///
/// * Start program via the entrypoint.
/// * Check whether the declared program id matches the input program
///   id. If it's not, return an error.
/// * Find and invoke the method based on whether the instruction data
///   starts with the method's discriminator.
/// * Run the method handler wrapper. This wraps the code the user
///   actually wrote, deserializing the accounts, constructing the
///   context, invoking the user's code, and finally running the exit
///   routine, which typically persists account changes.
///
/// The `entry` function here, defines the standard entry to a Solana
/// program, where execution begins.
pub fn entry<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    data: &[u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult {
    try_entry(program_id, accounts, data)
        .map_err(|e| {
            e.log();
            e.into()
        })
}
fn try_entry<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    data: &[u8],
) -> anchor_lang::Result<()> {
    if *program_id != ID {
        return Err(anchor_lang::error::ErrorCode::DeclaredProgramIdMismatch.into());
    }
    dispatch(program_id, accounts, data)
}
/// Module representing the program.
pub mod program {
    use super::*;
    /// Type representing the program.
    pub struct CsdkAnchorFullDerivedTest;
    #[automatically_derived]
    impl ::core::clone::Clone for CsdkAnchorFullDerivedTest {
        #[inline]
        fn clone(&self) -> CsdkAnchorFullDerivedTest {
            CsdkAnchorFullDerivedTest
        }
    }
    impl anchor_lang::Id for CsdkAnchorFullDerivedTest {
        fn id() -> Pubkey {
            ID
        }
    }
}
/// Performs method dispatch.
///
/// Each instruction's discriminator is checked until the given instruction data starts with
/// the current discriminator.
///
/// If a match is found, the instruction handler is called using the given instruction data
/// excluding the prepended discriminator bytes.
///
/// If no match is found, the fallback function is executed if it exists, or an error is
/// returned if it doesn't exist.
fn dispatch<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    data: &[u8],
) -> anchor_lang::Result<()> {
    if data.starts_with(instruction::CreateUserRecordAndGameSession::DISCRIMINATOR) {
        return __private::__global::create_user_record_and_game_session(
            program_id,
            accounts,
            &data[instruction::CreateUserRecordAndGameSession::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::DecompressAccountsIdempotent::DISCRIMINATOR) {
        return __private::__global::decompress_accounts_idempotent(
            program_id,
            accounts,
            &data[instruction::DecompressAccountsIdempotent::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::CompressAccountsIdempotent::DISCRIMINATOR) {
        return __private::__global::compress_accounts_idempotent(
            program_id,
            accounts,
            &data[instruction::CompressAccountsIdempotent::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::InitializeCompressionConfig::DISCRIMINATOR) {
        return __private::__global::initialize_compression_config(
            program_id,
            accounts,
            &data[instruction::InitializeCompressionConfig::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::UpdateCompressionConfig::DISCRIMINATOR) {
        return __private::__global::update_compression_config(
            program_id,
            accounts,
            &data[instruction::UpdateCompressionConfig::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(anchor_lang::idl::IDL_IX_TAG_LE) {
        return __private::__idl::__idl_dispatch(
            program_id,
            accounts,
            &data[anchor_lang::idl::IDL_IX_TAG_LE.len()..],
        );
    }
    if data.starts_with(anchor_lang::event::EVENT_IX_TAG_LE) {
        return Err(anchor_lang::error::ErrorCode::EventInstructionStub.into());
    }
    Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into())
}
/// Create a private module to not clutter the program's namespace.
/// Defines an entrypoint for each individual instruction handler
/// wrapper.
mod __private {
    use super::*;
    /// __idl mod defines handlers for injected Anchor IDL instructions.
    pub mod __idl {
        use super::*;
        #[inline(never)]
        pub fn __idl_dispatch<'info>(
            program_id: &Pubkey,
            accounts: &'info [AccountInfo<'info>],
            idl_ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            let mut accounts = accounts;
            let mut data: &[u8] = idl_ix_data;
            let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            match ix {
                anchor_lang::idl::IdlInstruction::Create { data_len } => {
                    let mut bumps = <IdlCreateAccounts as anchor_lang::Bumps>::Bumps::default();
                    let mut reallocs = std::collections::BTreeSet::new();
                    let mut accounts = IdlCreateAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                        &mut bumps,
                        &mut reallocs,
                    )?;
                    __idl_create_account(program_id, &mut accounts, data_len)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::Resize { data_len } => {
                    let mut bumps = <IdlResizeAccount as anchor_lang::Bumps>::Bumps::default();
                    let mut reallocs = std::collections::BTreeSet::new();
                    let mut accounts = IdlResizeAccount::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                        &mut bumps,
                        &mut reallocs,
                    )?;
                    __idl_resize_account(program_id, &mut accounts, data_len)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::Close => {
                    let mut bumps = <IdlCloseAccount as anchor_lang::Bumps>::Bumps::default();
                    let mut reallocs = std::collections::BTreeSet::new();
                    let mut accounts = IdlCloseAccount::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                        &mut bumps,
                        &mut reallocs,
                    )?;
                    __idl_close_account(program_id, &mut accounts)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::CreateBuffer => {
                    let mut bumps = <IdlCreateBuffer as anchor_lang::Bumps>::Bumps::default();
                    let mut reallocs = std::collections::BTreeSet::new();
                    let mut accounts = IdlCreateBuffer::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                        &mut bumps,
                        &mut reallocs,
                    )?;
                    __idl_create_buffer(program_id, &mut accounts)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::Write { data } => {
                    let mut bumps = <IdlAccounts as anchor_lang::Bumps>::Bumps::default();
                    let mut reallocs = std::collections::BTreeSet::new();
                    let mut accounts = IdlAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                        &mut bumps,
                        &mut reallocs,
                    )?;
                    __idl_write(program_id, &mut accounts, data)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::SetAuthority { new_authority } => {
                    let mut bumps = <IdlAccounts as anchor_lang::Bumps>::Bumps::default();
                    let mut reallocs = std::collections::BTreeSet::new();
                    let mut accounts = IdlAccounts::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                        &mut bumps,
                        &mut reallocs,
                    )?;
                    __idl_set_authority(program_id, &mut accounts, new_authority)?;
                    accounts.exit(program_id)?;
                }
                anchor_lang::idl::IdlInstruction::SetBuffer => {
                    let mut bumps = <IdlSetBuffer as anchor_lang::Bumps>::Bumps::default();
                    let mut reallocs = std::collections::BTreeSet::new();
                    let mut accounts = IdlSetBuffer::try_accounts(
                        program_id,
                        &mut accounts,
                        &[],
                        &mut bumps,
                        &mut reallocs,
                    )?;
                    __idl_set_buffer(program_id, &mut accounts)?;
                    accounts.exit(program_id)?;
                }
            }
            Ok(())
        }
        use anchor_lang::idl::ERASED_AUTHORITY;
        pub struct IdlAccount {
            pub authority: Pubkey,
            pub data_len: u32,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for IdlAccount {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "IdlAccount",
                    "authority",
                    &self.authority,
                    "data_len",
                    &&self.data_len,
                )
            }
        }
        impl borsh::ser::BorshSerialize for IdlAccount
        where
            Pubkey: borsh::ser::BorshSerialize,
            u32: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.authority, writer)?;
                borsh::BorshSerialize::serialize(&self.data_len, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for IdlAccount {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: ::alloc::vec::Vec::new(),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(
                            anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(
                                    ::alloc::boxed::box_new([
                                        anchor_lang::idl::types::IdlField {
                                            name: "authority".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "data_len".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::U32,
                                        },
                                    ]),
                                ),
                            ),
                        ),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) {}
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "csdk_anchor_full_derived_test::__private::__idl",
                            "IdlAccount",
                        ),
                    )
                })
            }
        }
        impl borsh::de::BorshDeserialize for IdlAccount
        where
            Pubkey: borsh::BorshDeserialize,
            u32: borsh::BorshDeserialize,
        {
            fn deserialize_reader<R: borsh::maybestd::io::Read>(
                reader: &mut R,
            ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
                Ok(Self {
                    authority: borsh::BorshDeserialize::deserialize_reader(reader)?,
                    data_len: borsh::BorshDeserialize::deserialize_reader(reader)?,
                })
            }
        }
        #[automatically_derived]
        impl ::core::clone::Clone for IdlAccount {
            #[inline]
            fn clone(&self) -> IdlAccount {
                IdlAccount {
                    authority: ::core::clone::Clone::clone(&self.authority),
                    data_len: ::core::clone::Clone::clone(&self.data_len),
                }
            }
        }
        #[automatically_derived]
        impl anchor_lang::AccountSerialize for IdlAccount {
            fn try_serialize<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> anchor_lang::Result<()> {
                if writer.write_all(IdlAccount::DISCRIMINATOR).is_err() {
                    return Err(
                        anchor_lang::error::ErrorCode::AccountDidNotSerialize.into(),
                    );
                }
                if AnchorSerialize::serialize(self, writer).is_err() {
                    return Err(
                        anchor_lang::error::ErrorCode::AccountDidNotSerialize.into(),
                    );
                }
                Ok(())
            }
        }
        #[automatically_derived]
        impl anchor_lang::AccountDeserialize for IdlAccount {
            fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                if buf.len() < IdlAccount::DISCRIMINATOR.len() {
                    return Err(
                        anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound
                            .into(),
                    );
                }
                let given_disc = &buf[..IdlAccount::DISCRIMINATOR.len()];
                if IdlAccount::DISCRIMINATOR != given_disc {
                    return Err(
                        anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                                error_name: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                    .name(),
                                error_code_number: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                    .into(),
                                error_msg: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                    .to_string(),
                                error_origin: Some(
                                    anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                        filename: "sdk-tests/csdk-anchor-full-derived-test/src/lib.rs",
                                        line: 37u32,
                                    }),
                                ),
                                compared_values: None,
                            })
                            .with_account_name("IdlAccount"),
                    );
                }
                Self::try_deserialize_unchecked(buf)
            }
            fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                let mut data: &[u8] = &buf[IdlAccount::DISCRIMINATOR.len()..];
                AnchorDeserialize::deserialize(&mut data)
                    .map_err(|_| {
                        anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into()
                    })
            }
        }
        #[automatically_derived]
        impl anchor_lang::Discriminator for IdlAccount {
            const DISCRIMINATOR: &'static [u8] = &[24, 70, 98, 191, 58, 144, 123, 158];
        }
        impl IdlAccount {
            pub fn address(program_id: &Pubkey) -> Pubkey {
                let program_signer = Pubkey::find_program_address(&[], program_id).0;
                Pubkey::create_with_seed(&program_signer, IdlAccount::seed(), program_id)
                    .expect("Seed is always valid")
            }
            pub fn seed() -> &'static str {
                "anchor:idl"
            }
        }
        impl anchor_lang::Owner for IdlAccount {
            fn owner() -> Pubkey {
                crate::ID
            }
        }
        pub struct IdlCreateAccounts<'info> {
            #[account(signer)]
            pub from: AccountInfo<'info>,
            #[account(mut)]
            pub to: AccountInfo<'info>,
            #[account(seeds = [], bump)]
            pub base: AccountInfo<'info>,
            pub system_program: Program<'info, System>,
            #[account(executable)]
            pub program: AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info, IdlCreateAccountsBumps>
        for IdlCreateAccounts<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >],
                __ix_data: &[u8],
                __bumps: &mut IdlCreateAccountsBumps,
                __reallocs: &mut std::collections::BTreeSet<
                    anchor_lang::solana_program::pubkey::Pubkey,
                >,
            ) -> anchor_lang::Result<Self> {
                let from: AccountInfo = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("from"))?;
                let to: AccountInfo = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("to"))?;
                let base: AccountInfo = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("base"))?;
                let system_program: anchor_lang::accounts::program::Program<System> = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("system_program"))?;
                let program: AccountInfo = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("program"))?;
                if !&from.is_signer {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintSigner,
                            )
                            .with_account_name("from"),
                    );
                }
                if !&to.is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("to"),
                    );
                }
                let (__pda_address, __bump) = Pubkey::find_program_address(
                    &[],
                    &__program_id,
                );
                __bumps.base = __bump;
                if base.key() != __pda_address {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintSeeds,
                            )
                            .with_account_name("base")
                            .with_pubkeys((base.key(), __pda_address)),
                    );
                }
                if !&program.executable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintExecutable,
                            )
                            .with_account_name("program"),
                    );
                }
                Ok(IdlCreateAccounts {
                    from,
                    to,
                    base,
                    system_program,
                    program,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for IdlCreateAccounts<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.from.to_account_infos());
                account_infos.extend(self.to.to_account_infos());
                account_infos.extend(self.base.to_account_infos());
                account_infos.extend(self.system_program.to_account_infos());
                account_infos.extend(self.program.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for IdlCreateAccounts<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.from.to_account_metas(Some(true)));
                account_metas.extend(self.to.to_account_metas(None));
                account_metas.extend(self.base.to_account_metas(None));
                account_metas.extend(self.system_program.to_account_metas(None));
                account_metas.extend(self.program.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for IdlCreateAccounts<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::Result<()> {
                anchor_lang::AccountsExit::exit(&self.to, program_id)
                    .map_err(|e| e.with_account_name("to"))?;
                Ok(())
            }
        }
        pub struct IdlCreateAccountsBumps {
            pub base: u8,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for IdlCreateAccountsBumps {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "IdlCreateAccountsBumps",
                    "base",
                    &&self.base,
                )
            }
        }
        impl Default for IdlCreateAccountsBumps {
            fn default() -> Self {
                IdlCreateAccountsBumps {
                    base: u8::MAX,
                }
            }
        }
        impl<'info> anchor_lang::Bumps for IdlCreateAccounts<'info>
        where
            'info: 'info,
        {
            type Bumps = IdlCreateAccountsBumps;
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_idl_create_accounts {
            use super::*;
            use anchor_lang::prelude::borsh;
            /// Generated client accounts for [`IdlCreateAccounts`].
            pub struct IdlCreateAccounts {
                pub from: Pubkey,
                pub to: Pubkey,
                pub base: Pubkey,
                pub system_program: Pubkey,
                pub program: Pubkey,
            }
            impl borsh::ser::BorshSerialize for IdlCreateAccounts
            where
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.from, writer)?;
                    borsh::BorshSerialize::serialize(&self.to, writer)?;
                    borsh::BorshSerialize::serialize(&self.base, writer)?;
                    borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                    borsh::BorshSerialize::serialize(&self.program, writer)?;
                    Ok(())
                }
            }
            impl anchor_lang::idl::build::IdlBuild for IdlCreateAccounts {
                fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                    Some(anchor_lang::idl::types::IdlTypeDef {
                        name: Self::get_full_path(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Generated client accounts for [`IdlCreateAccounts`]."
                                    .into(),
                            ]),
                        ),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Named(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlField {
                                                name: "from".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "to".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "base".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "system_program".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "program".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {}
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "csdk_anchor_full_derived_test::__private::__idl::__client_accounts_idl_create_accounts",
                                "IdlCreateAccounts",
                            ),
                        )
                    })
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for IdlCreateAccounts {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.from,
                                true,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.to,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.base,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.system_program,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.program,
                                false,
                            ),
                        );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_idl_create_accounts {
            use super::*;
            /// Generated CPI struct of the accounts for [`IdlCreateAccounts`].
            pub struct IdlCreateAccounts<'info> {
                pub from: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub to: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub base: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
                pub program: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlCreateAccounts<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.from),
                                true,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.to),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.base),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.system_program),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.program),
                                false,
                            ),
                        );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for IdlCreateAccounts<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.from),
                        );
                    account_infos
                        .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.to));
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.base),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.system_program,
                            ),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.program),
                        );
                    account_infos
                }
            }
        }
        impl<'info> IdlCreateAccounts<'info> {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlAccount,
                >,
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "from".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "to".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "base".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "system_program".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "program".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                    ]),
                )
            }
        }
        pub struct IdlAccounts<'info> {
            #[account(mut, has_one = authority)]
            pub idl: Account<'info, IdlAccount>,
            #[account(constraint = authority.key!= &ERASED_AUTHORITY)]
            pub authority: Signer<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info, IdlAccountsBumps> for IdlAccounts<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >],
                __ix_data: &[u8],
                __bumps: &mut IdlAccountsBumps,
                __reallocs: &mut std::collections::BTreeSet<
                    anchor_lang::solana_program::pubkey::Pubkey,
                >,
            ) -> anchor_lang::Result<Self> {
                let idl: anchor_lang::accounts::account::Account<IdlAccount> = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("idl"))?;
                let authority: Signer = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("authority"))?;
                if !AsRef::<AccountInfo>::as_ref(&idl).is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("idl"),
                    );
                }
                {
                    let my_key = idl.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(
                            anchor_lang::error::Error::from(
                                    anchor_lang::error::ErrorCode::ConstraintHasOne,
                                )
                                .with_account_name("idl")
                                .with_pubkeys((my_key, target_key)),
                        );
                    }
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintRaw,
                            )
                            .with_account_name("authority"),
                    );
                }
                Ok(IdlAccounts { idl, authority })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for IdlAccounts<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.idl.to_account_infos());
                account_infos.extend(self.authority.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for IdlAccounts<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.idl.to_account_metas(None));
                account_metas.extend(self.authority.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for IdlAccounts<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::Result<()> {
                anchor_lang::AccountsExit::exit(&self.idl, program_id)
                    .map_err(|e| e.with_account_name("idl"))?;
                Ok(())
            }
        }
        pub struct IdlAccountsBumps {}
        #[automatically_derived]
        impl ::core::fmt::Debug for IdlAccountsBumps {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "IdlAccountsBumps")
            }
        }
        impl Default for IdlAccountsBumps {
            fn default() -> Self {
                IdlAccountsBumps {}
            }
        }
        impl<'info> anchor_lang::Bumps for IdlAccounts<'info>
        where
            'info: 'info,
        {
            type Bumps = IdlAccountsBumps;
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_idl_accounts {
            use super::*;
            use anchor_lang::prelude::borsh;
            /// Generated client accounts for [`IdlAccounts`].
            pub struct IdlAccounts {
                pub idl: Pubkey,
                pub authority: Pubkey,
            }
            impl borsh::ser::BorshSerialize for IdlAccounts
            where
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.idl, writer)?;
                    borsh::BorshSerialize::serialize(&self.authority, writer)?;
                    Ok(())
                }
            }
            impl anchor_lang::idl::build::IdlBuild for IdlAccounts {
                fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                    Some(anchor_lang::idl::types::IdlTypeDef {
                        name: Self::get_full_path(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Generated client accounts for [`IdlAccounts`].".into(),
                            ]),
                        ),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Named(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlField {
                                                name: "idl".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "authority".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {}
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "csdk_anchor_full_derived_test::__private::__idl::__client_accounts_idl_accounts",
                                "IdlAccounts",
                            ),
                        )
                    })
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for IdlAccounts {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.idl,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.authority,
                                true,
                            ),
                        );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_idl_accounts {
            use super::*;
            /// Generated CPI struct of the accounts for [`IdlAccounts`].
            pub struct IdlAccounts<'info> {
                pub idl: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlAccounts<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.idl),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.authority),
                                true,
                            ),
                        );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for IdlAccounts<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.idl),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.authority,
                            ),
                        );
                    account_infos
                }
            }
        }
        impl<'info> IdlAccounts<'info> {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlAccount,
                >,
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                if let Some(ty) = <IdlAccount>::create_type() {
                    let account = anchor_lang::idl::types::IdlAccount {
                        name: ty.name.clone(),
                        discriminator: IdlAccount::DISCRIMINATOR.into(),
                    };
                    accounts.insert(account.name.clone(), account);
                    types.insert(ty.name.clone(), ty);
                    <IdlAccount>::insert_types(types);
                }
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "idl".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                    ]),
                )
            }
        }
        pub struct IdlResizeAccount<'info> {
            #[account(mut, has_one = authority)]
            pub idl: Account<'info, IdlAccount>,
            #[account(mut, constraint = authority.key!= &ERASED_AUTHORITY)]
            pub authority: Signer<'info>,
            pub system_program: Program<'info, System>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info, IdlResizeAccountBumps>
        for IdlResizeAccount<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >],
                __ix_data: &[u8],
                __bumps: &mut IdlResizeAccountBumps,
                __reallocs: &mut std::collections::BTreeSet<
                    anchor_lang::solana_program::pubkey::Pubkey,
                >,
            ) -> anchor_lang::Result<Self> {
                let idl: anchor_lang::accounts::account::Account<IdlAccount> = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("idl"))?;
                let authority: Signer = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("authority"))?;
                let system_program: anchor_lang::accounts::program::Program<System> = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("system_program"))?;
                if !AsRef::<AccountInfo>::as_ref(&idl).is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("idl"),
                    );
                }
                {
                    let my_key = idl.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(
                            anchor_lang::error::Error::from(
                                    anchor_lang::error::ErrorCode::ConstraintHasOne,
                                )
                                .with_account_name("idl")
                                .with_pubkeys((my_key, target_key)),
                        );
                    }
                }
                if !AsRef::<AccountInfo>::as_ref(&authority).is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("authority"),
                    );
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintRaw,
                            )
                            .with_account_name("authority"),
                    );
                }
                Ok(IdlResizeAccount {
                    idl,
                    authority,
                    system_program,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for IdlResizeAccount<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.idl.to_account_infos());
                account_infos.extend(self.authority.to_account_infos());
                account_infos.extend(self.system_program.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for IdlResizeAccount<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.idl.to_account_metas(None));
                account_metas.extend(self.authority.to_account_metas(None));
                account_metas.extend(self.system_program.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for IdlResizeAccount<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::Result<()> {
                anchor_lang::AccountsExit::exit(&self.idl, program_id)
                    .map_err(|e| e.with_account_name("idl"))?;
                anchor_lang::AccountsExit::exit(&self.authority, program_id)
                    .map_err(|e| e.with_account_name("authority"))?;
                Ok(())
            }
        }
        pub struct IdlResizeAccountBumps {}
        #[automatically_derived]
        impl ::core::fmt::Debug for IdlResizeAccountBumps {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "IdlResizeAccountBumps")
            }
        }
        impl Default for IdlResizeAccountBumps {
            fn default() -> Self {
                IdlResizeAccountBumps {}
            }
        }
        impl<'info> anchor_lang::Bumps for IdlResizeAccount<'info>
        where
            'info: 'info,
        {
            type Bumps = IdlResizeAccountBumps;
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_idl_resize_account {
            use super::*;
            use anchor_lang::prelude::borsh;
            /// Generated client accounts for [`IdlResizeAccount`].
            pub struct IdlResizeAccount {
                pub idl: Pubkey,
                pub authority: Pubkey,
                pub system_program: Pubkey,
            }
            impl borsh::ser::BorshSerialize for IdlResizeAccount
            where
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.idl, writer)?;
                    borsh::BorshSerialize::serialize(&self.authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                    Ok(())
                }
            }
            impl anchor_lang::idl::build::IdlBuild for IdlResizeAccount {
                fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                    Some(anchor_lang::idl::types::IdlTypeDef {
                        name: Self::get_full_path(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Generated client accounts for [`IdlResizeAccount`].".into(),
                            ]),
                        ),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Named(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlField {
                                                name: "idl".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "authority".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "system_program".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {}
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "csdk_anchor_full_derived_test::__private::__idl::__client_accounts_idl_resize_account",
                                "IdlResizeAccount",
                            ),
                        )
                    })
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for IdlResizeAccount {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.idl,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.authority,
                                true,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.system_program,
                                false,
                            ),
                        );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_idl_resize_account {
            use super::*;
            /// Generated CPI struct of the accounts for [`IdlResizeAccount`].
            pub struct IdlResizeAccount<'info> {
                pub idl: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlResizeAccount<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.idl),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.authority),
                                true,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.system_program),
                                false,
                            ),
                        );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for IdlResizeAccount<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.idl),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.authority,
                            ),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.system_program,
                            ),
                        );
                    account_infos
                }
            }
        }
        impl<'info> IdlResizeAccount<'info> {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlAccount,
                >,
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                if let Some(ty) = <IdlAccount>::create_type() {
                    let account = anchor_lang::idl::types::IdlAccount {
                        name: ty.name.clone(),
                        discriminator: IdlAccount::DISCRIMINATOR.into(),
                    };
                    accounts.insert(account.name.clone(), account);
                    types.insert(ty.name.clone(), ty);
                    <IdlAccount>::insert_types(types);
                }
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "idl".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "system_program".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                    ]),
                )
            }
        }
        pub struct IdlCreateBuffer<'info> {
            #[account(zero)]
            pub buffer: Account<'info, IdlAccount>,
            #[account(constraint = authority.key!= &ERASED_AUTHORITY)]
            pub authority: Signer<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info, IdlCreateBufferBumps>
        for IdlCreateBuffer<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >],
                __ix_data: &[u8],
                __bumps: &mut IdlCreateBufferBumps,
                __reallocs: &mut std::collections::BTreeSet<
                    anchor_lang::solana_program::pubkey::Pubkey,
                >,
            ) -> anchor_lang::Result<Self> {
                if __accounts.is_empty() {
                    return Err(
                        anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into(),
                    );
                }
                let buffer = &__accounts[0];
                *__accounts = &__accounts[1..];
                let authority: Signer = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("authority"))?;
                let __anchor_rent = Rent::get()?;
                let buffer: anchor_lang::accounts::account::Account<IdlAccount> = {
                    let mut __data: &[u8] = &buffer.try_borrow_data()?;
                    let __disc = &__data[..IdlAccount::DISCRIMINATOR.len()];
                    let __has_disc = __disc.iter().any(|b| *b != 0);
                    if __has_disc {
                        return Err(
                            anchor_lang::error::Error::from(
                                    anchor_lang::error::ErrorCode::ConstraintZero,
                                )
                                .with_account_name("buffer"),
                        );
                    }
                    match anchor_lang::accounts::account::Account::try_from_unchecked(
                        &buffer,
                    ) {
                        Ok(val) => val,
                        Err(e) => return Err(e.with_account_name("buffer")),
                    }
                };
                if !AsRef::<AccountInfo>::as_ref(&buffer).is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("buffer"),
                    );
                }
                if !__anchor_rent
                    .is_exempt(
                        buffer.to_account_info().lamports(),
                        buffer.to_account_info().try_data_len()?,
                    )
                {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintRentExempt,
                            )
                            .with_account_name("buffer"),
                    );
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintRaw,
                            )
                            .with_account_name("authority"),
                    );
                }
                Ok(IdlCreateBuffer {
                    buffer,
                    authority,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for IdlCreateBuffer<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.buffer.to_account_infos());
                account_infos.extend(self.authority.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for IdlCreateBuffer<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.buffer.to_account_metas(None));
                account_metas.extend(self.authority.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for IdlCreateBuffer<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::Result<()> {
                anchor_lang::AccountsExit::exit(&self.buffer, program_id)
                    .map_err(|e| e.with_account_name("buffer"))?;
                Ok(())
            }
        }
        pub struct IdlCreateBufferBumps {}
        #[automatically_derived]
        impl ::core::fmt::Debug for IdlCreateBufferBumps {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "IdlCreateBufferBumps")
            }
        }
        impl Default for IdlCreateBufferBumps {
            fn default() -> Self {
                IdlCreateBufferBumps {}
            }
        }
        impl<'info> anchor_lang::Bumps for IdlCreateBuffer<'info>
        where
            'info: 'info,
        {
            type Bumps = IdlCreateBufferBumps;
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_idl_create_buffer {
            use super::*;
            use anchor_lang::prelude::borsh;
            /// Generated client accounts for [`IdlCreateBuffer`].
            pub struct IdlCreateBuffer {
                pub buffer: Pubkey,
                pub authority: Pubkey,
            }
            impl borsh::ser::BorshSerialize for IdlCreateBuffer
            where
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.buffer, writer)?;
                    borsh::BorshSerialize::serialize(&self.authority, writer)?;
                    Ok(())
                }
            }
            impl anchor_lang::idl::build::IdlBuild for IdlCreateBuffer {
                fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                    Some(anchor_lang::idl::types::IdlTypeDef {
                        name: Self::get_full_path(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Generated client accounts for [`IdlCreateBuffer`].".into(),
                            ]),
                        ),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Named(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlField {
                                                name: "buffer".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "authority".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {}
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "csdk_anchor_full_derived_test::__private::__idl::__client_accounts_idl_create_buffer",
                                "IdlCreateBuffer",
                            ),
                        )
                    })
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for IdlCreateBuffer {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.buffer,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.authority,
                                true,
                            ),
                        );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_idl_create_buffer {
            use super::*;
            /// Generated CPI struct of the accounts for [`IdlCreateBuffer`].
            pub struct IdlCreateBuffer<'info> {
                pub buffer: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlCreateBuffer<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.buffer),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.authority),
                                true,
                            ),
                        );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for IdlCreateBuffer<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.buffer),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.authority,
                            ),
                        );
                    account_infos
                }
            }
        }
        impl<'info> IdlCreateBuffer<'info> {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlAccount,
                >,
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                if let Some(ty) = <IdlAccount>::create_type() {
                    let account = anchor_lang::idl::types::IdlAccount {
                        name: ty.name.clone(),
                        discriminator: IdlAccount::DISCRIMINATOR.into(),
                    };
                    accounts.insert(account.name.clone(), account);
                    types.insert(ty.name.clone(), ty);
                    <IdlAccount>::insert_types(types);
                }
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "buffer".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                    ]),
                )
            }
        }
        pub struct IdlSetBuffer<'info> {
            #[account(mut, constraint = buffer.authority = = idl.authority)]
            pub buffer: Account<'info, IdlAccount>,
            #[account(mut, has_one = authority)]
            pub idl: Account<'info, IdlAccount>,
            #[account(constraint = authority.key!= &ERASED_AUTHORITY)]
            pub authority: Signer<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info, IdlSetBufferBumps>
        for IdlSetBuffer<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >],
                __ix_data: &[u8],
                __bumps: &mut IdlSetBufferBumps,
                __reallocs: &mut std::collections::BTreeSet<
                    anchor_lang::solana_program::pubkey::Pubkey,
                >,
            ) -> anchor_lang::Result<Self> {
                let buffer: anchor_lang::accounts::account::Account<IdlAccount> = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("buffer"))?;
                let idl: anchor_lang::accounts::account::Account<IdlAccount> = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("idl"))?;
                let authority: Signer = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("authority"))?;
                if !AsRef::<AccountInfo>::as_ref(&buffer).is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("buffer"),
                    );
                }
                if !(buffer.authority == idl.authority) {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintRaw,
                            )
                            .with_account_name("buffer"),
                    );
                }
                if !AsRef::<AccountInfo>::as_ref(&idl).is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("idl"),
                    );
                }
                {
                    let my_key = idl.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(
                            anchor_lang::error::Error::from(
                                    anchor_lang::error::ErrorCode::ConstraintHasOne,
                                )
                                .with_account_name("idl")
                                .with_pubkeys((my_key, target_key)),
                        );
                    }
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintRaw,
                            )
                            .with_account_name("authority"),
                    );
                }
                Ok(IdlSetBuffer {
                    buffer,
                    idl,
                    authority,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for IdlSetBuffer<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.buffer.to_account_infos());
                account_infos.extend(self.idl.to_account_infos());
                account_infos.extend(self.authority.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for IdlSetBuffer<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.buffer.to_account_metas(None));
                account_metas.extend(self.idl.to_account_metas(None));
                account_metas.extend(self.authority.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for IdlSetBuffer<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::Result<()> {
                anchor_lang::AccountsExit::exit(&self.buffer, program_id)
                    .map_err(|e| e.with_account_name("buffer"))?;
                anchor_lang::AccountsExit::exit(&self.idl, program_id)
                    .map_err(|e| e.with_account_name("idl"))?;
                Ok(())
            }
        }
        pub struct IdlSetBufferBumps {}
        #[automatically_derived]
        impl ::core::fmt::Debug for IdlSetBufferBumps {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "IdlSetBufferBumps")
            }
        }
        impl Default for IdlSetBufferBumps {
            fn default() -> Self {
                IdlSetBufferBumps {}
            }
        }
        impl<'info> anchor_lang::Bumps for IdlSetBuffer<'info>
        where
            'info: 'info,
        {
            type Bumps = IdlSetBufferBumps;
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_idl_set_buffer {
            use super::*;
            use anchor_lang::prelude::borsh;
            /// Generated client accounts for [`IdlSetBuffer`].
            pub struct IdlSetBuffer {
                pub buffer: Pubkey,
                pub idl: Pubkey,
                pub authority: Pubkey,
            }
            impl borsh::ser::BorshSerialize for IdlSetBuffer
            where
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.buffer, writer)?;
                    borsh::BorshSerialize::serialize(&self.idl, writer)?;
                    borsh::BorshSerialize::serialize(&self.authority, writer)?;
                    Ok(())
                }
            }
            impl anchor_lang::idl::build::IdlBuild for IdlSetBuffer {
                fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                    Some(anchor_lang::idl::types::IdlTypeDef {
                        name: Self::get_full_path(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Generated client accounts for [`IdlSetBuffer`].".into(),
                            ]),
                        ),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Named(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlField {
                                                name: "buffer".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "idl".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "authority".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {}
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "csdk_anchor_full_derived_test::__private::__idl::__client_accounts_idl_set_buffer",
                                "IdlSetBuffer",
                            ),
                        )
                    })
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for IdlSetBuffer {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.buffer,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.idl,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.authority,
                                true,
                            ),
                        );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_idl_set_buffer {
            use super::*;
            /// Generated CPI struct of the accounts for [`IdlSetBuffer`].
            pub struct IdlSetBuffer<'info> {
                pub buffer: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
                pub idl: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlSetBuffer<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.buffer),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.idl),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.authority),
                                true,
                            ),
                        );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for IdlSetBuffer<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.buffer),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.idl),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.authority,
                            ),
                        );
                    account_infos
                }
            }
        }
        impl<'info> IdlSetBuffer<'info> {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlAccount,
                >,
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                if let Some(ty) = <IdlAccount>::create_type() {
                    let account = anchor_lang::idl::types::IdlAccount {
                        name: ty.name.clone(),
                        discriminator: IdlAccount::DISCRIMINATOR.into(),
                    };
                    accounts.insert(account.name.clone(), account);
                    types.insert(ty.name.clone(), ty);
                    <IdlAccount>::insert_types(types);
                }
                if let Some(ty) = <IdlAccount>::create_type() {
                    let account = anchor_lang::idl::types::IdlAccount {
                        name: ty.name.clone(),
                        discriminator: IdlAccount::DISCRIMINATOR.into(),
                    };
                    accounts.insert(account.name.clone(), account);
                    types.insert(ty.name.clone(), ty);
                    <IdlAccount>::insert_types(types);
                }
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "buffer".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "idl".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                    ]),
                )
            }
        }
        pub struct IdlCloseAccount<'info> {
            #[account(mut, has_one = authority, close = sol_destination)]
            pub account: Account<'info, IdlAccount>,
            #[account(constraint = authority.key!= &ERASED_AUTHORITY)]
            pub authority: Signer<'info>,
            #[account(mut)]
            pub sol_destination: AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info, IdlCloseAccountBumps>
        for IdlCloseAccount<'info>
        where
            'info: 'info,
        {
            #[inline(never)]
            fn try_accounts(
                __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >],
                __ix_data: &[u8],
                __bumps: &mut IdlCloseAccountBumps,
                __reallocs: &mut std::collections::BTreeSet<
                    anchor_lang::solana_program::pubkey::Pubkey,
                >,
            ) -> anchor_lang::Result<Self> {
                let account: anchor_lang::accounts::account::Account<IdlAccount> = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("account"))?;
                let authority: Signer = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("authority"))?;
                let sol_destination: AccountInfo = anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("sol_destination"))?;
                if !AsRef::<AccountInfo>::as_ref(&account).is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("account"),
                    );
                }
                {
                    let my_key = account.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(
                            anchor_lang::error::Error::from(
                                    anchor_lang::error::ErrorCode::ConstraintHasOne,
                                )
                                .with_account_name("account")
                                .with_pubkeys((my_key, target_key)),
                        );
                    }
                }
                {
                    if account.key() == sol_destination.key() {
                        return Err(
                            anchor_lang::error::Error::from(
                                    anchor_lang::error::ErrorCode::ConstraintClose,
                                )
                                .with_account_name("account"),
                        );
                    }
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintRaw,
                            )
                            .with_account_name("authority"),
                    );
                }
                if !&sol_destination.is_writable {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintMut,
                            )
                            .with_account_name("sol_destination"),
                    );
                }
                Ok(IdlCloseAccount {
                    account,
                    authority,
                    sol_destination,
                })
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for IdlCloseAccount<'info>
        where
            'info: 'info,
        {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(self.account.to_account_infos());
                account_infos.extend(self.authority.to_account_infos());
                account_infos.extend(self.sol_destination.to_account_infos());
                account_infos
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for IdlCloseAccount<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.extend(self.account.to_account_metas(None));
                account_metas.extend(self.authority.to_account_metas(None));
                account_metas.extend(self.sol_destination.to_account_metas(None));
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::AccountsExit<'info> for IdlCloseAccount<'info>
        where
            'info: 'info,
        {
            fn exit(
                &self,
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            ) -> anchor_lang::Result<()> {
                {
                    let sol_destination = &self.sol_destination;
                    anchor_lang::AccountsClose::close(
                            &self.account,
                            sol_destination.to_account_info(),
                        )
                        .map_err(|e| e.with_account_name("account"))?;
                }
                anchor_lang::AccountsExit::exit(&self.sol_destination, program_id)
                    .map_err(|e| e.with_account_name("sol_destination"))?;
                Ok(())
            }
        }
        pub struct IdlCloseAccountBumps {}
        #[automatically_derived]
        impl ::core::fmt::Debug for IdlCloseAccountBumps {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(f, "IdlCloseAccountBumps")
            }
        }
        impl Default for IdlCloseAccountBumps {
            fn default() -> Self {
                IdlCloseAccountBumps {}
            }
        }
        impl<'info> anchor_lang::Bumps for IdlCloseAccount<'info>
        where
            'info: 'info,
        {
            type Bumps = IdlCloseAccountBumps;
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
        /// instead of an `AccountInfo`. This is useful for clients that want
        /// to generate a list of accounts, without explicitly knowing the
        /// order all the fields should be in.
        ///
        /// To access the struct in this module, one should use the sibling
        /// `accounts` module (also generated), which re-exports this.
        pub(crate) mod __client_accounts_idl_close_account {
            use super::*;
            use anchor_lang::prelude::borsh;
            /// Generated client accounts for [`IdlCloseAccount`].
            pub struct IdlCloseAccount {
                pub account: Pubkey,
                pub authority: Pubkey,
                pub sol_destination: Pubkey,
            }
            impl borsh::ser::BorshSerialize for IdlCloseAccount
            where
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
                Pubkey: borsh::ser::BorshSerialize,
            {
                fn serialize<W: borsh::maybestd::io::Write>(
                    &self,
                    writer: &mut W,
                ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                    borsh::BorshSerialize::serialize(&self.account, writer)?;
                    borsh::BorshSerialize::serialize(&self.authority, writer)?;
                    borsh::BorshSerialize::serialize(&self.sol_destination, writer)?;
                    Ok(())
                }
            }
            impl anchor_lang::idl::build::IdlBuild for IdlCloseAccount {
                fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                    Some(anchor_lang::idl::types::IdlTypeDef {
                        name: Self::get_full_path(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Generated client accounts for [`IdlCloseAccount`].".into(),
                            ]),
                        ),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(
                                anchor_lang::idl::types::IdlDefinedFields::Named(
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            anchor_lang::idl::types::IdlField {
                                                name: "account".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "authority".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                            anchor_lang::idl::types::IdlField {
                                                name: "sol_destination".into(),
                                                docs: ::alloc::vec::Vec::new(),
                                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                                            },
                                        ]),
                                    ),
                                ),
                            ),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {}
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "csdk_anchor_full_derived_test::__private::__idl::__client_accounts_idl_close_account",
                                "IdlCloseAccount",
                            ),
                        )
                    })
                }
            }
            #[automatically_derived]
            impl anchor_lang::ToAccountMetas for IdlCloseAccount {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.account,
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                self.authority,
                                true,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                self.sol_destination,
                                false,
                            ),
                        );
                    account_metas
                }
            }
        }
        /// An internal, Anchor generated module. This is used (as an
        /// implementation detail), to generate a CPI struct for a given
        /// `#[derive(Accounts)]` implementation, where each field is an
        /// AccountInfo.
        ///
        /// To access the struct in this module, one should use the sibling
        /// [`cpi::accounts`] module (also generated), which re-exports this.
        pub(crate) mod __cpi_client_accounts_idl_close_account {
            use super::*;
            /// Generated CPI struct of the accounts for [`IdlCloseAccount`].
            pub struct IdlCloseAccount<'info> {
                pub account: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
                pub sol_destination: anchor_lang::solana_program::account_info::AccountInfo<
                    'info,
                >,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlCloseAccount<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.account),
                                false,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(&self.authority),
                                true,
                            ),
                        );
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new(
                                anchor_lang::Key::key(&self.sol_destination),
                                false,
                            ),
                        );
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for IdlCloseAccount<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(&self.account),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.authority,
                            ),
                        );
                    account_infos
                        .extend(
                            anchor_lang::ToAccountInfos::to_account_infos(
                                &self.sol_destination,
                            ),
                        );
                    account_infos
                }
            }
        }
        impl<'info> IdlCloseAccount<'info> {
            pub fn __anchor_private_gen_idl_accounts(
                accounts: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlAccount,
                >,
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                if let Some(ty) = <IdlAccount>::create_type() {
                    let account = anchor_lang::idl::types::IdlAccount {
                        name: ty.name.clone(),
                        discriminator: IdlAccount::DISCRIMINATOR.into(),
                    };
                    accounts.insert(account.name.clone(), account);
                    types.insert(ty.name.clone(), ty);
                    <IdlAccount>::insert_types(types);
                }
                <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "account".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                            name: "sol_destination".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                    ]),
                )
            }
        }
        use std::cell::{Ref, RefMut};
        pub trait IdlTrailingData<'info> {
            fn trailing_data(self) -> Ref<'info, [u8]>;
            fn trailing_data_mut(self) -> RefMut<'info, [u8]>;
        }
        impl<'a, 'info: 'a> IdlTrailingData<'a> for &'a Account<'info, IdlAccount> {
            fn trailing_data(self) -> Ref<'a, [u8]> {
                let info: &AccountInfo<'info> = self.as_ref();
                Ref::map(info.try_borrow_data().unwrap(), |d| &d[44..])
            }
            fn trailing_data_mut(self) -> RefMut<'a, [u8]> {
                let info: &AccountInfo<'info> = self.as_ref();
                RefMut::map(info.try_borrow_mut_data().unwrap(), |d| &mut d[44..])
            }
        }
        #[inline(never)]
        pub fn __idl_create_account(
            program_id: &Pubkey,
            accounts: &mut IdlCreateAccounts,
            data_len: u64,
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: IdlCreateAccount");
            if program_id != accounts.program.key {
                return Err(
                    anchor_lang::error::ErrorCode::IdlInstructionInvalidProgram.into(),
                );
            }
            let from = accounts.from.key;
            let (base, nonce) = Pubkey::find_program_address(&[], program_id);
            let seed = IdlAccount::seed();
            let owner = accounts.program.key;
            let to = Pubkey::create_with_seed(&base, seed, owner).unwrap();
            let space = std::cmp::min(
                IdlAccount::DISCRIMINATOR.len() + 32 + 4 + data_len as usize,
                10_000,
            );
            let rent = Rent::get()?;
            let lamports = rent.minimum_balance(space);
            let seeds = &[&[nonce][..]];
            let ix = anchor_lang::solana_program::system_instruction::create_account_with_seed(
                from,
                &to,
                &base,
                seed,
                lamports,
                space as u64,
                owner,
            );
            anchor_lang::solana_program::program::invoke_signed(
                &ix,
                &[
                    accounts.from.clone(),
                    accounts.to.clone(),
                    accounts.base.clone(),
                    accounts.system_program.to_account_info(),
                ],
                &[seeds],
            )?;
            let mut idl_account = {
                let mut account_data = accounts.to.try_borrow_data()?;
                let mut account_data_slice: &[u8] = &account_data;
                IdlAccount::try_deserialize_unchecked(&mut account_data_slice)?
            };
            idl_account.authority = *accounts.from.key;
            let mut data = accounts.to.try_borrow_mut_data()?;
            let dst: &mut [u8] = &mut data;
            let mut cursor = std::io::Cursor::new(dst);
            idl_account.try_serialize(&mut cursor)?;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_resize_account(
            program_id: &Pubkey,
            accounts: &mut IdlResizeAccount,
            data_len: u64,
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: IdlResizeAccount");
            let data_len: usize = data_len as usize;
            if accounts.idl.data_len != 0 {
                return Err(anchor_lang::error::ErrorCode::IdlAccountNotEmpty.into());
            }
            let idl_ref = AsRef::<AccountInfo>::as_ref(&accounts.idl);
            let new_account_space = idl_ref
                .data_len()
                .checked_add(
                    std::cmp::min(
                        data_len
                            .checked_sub(idl_ref.data_len())
                            .expect(
                                "data_len should always be >= the current account space",
                            ),
                        10_000,
                    ),
                )
                .unwrap();
            if new_account_space > idl_ref.data_len() {
                let sysvar_rent = Rent::get()?;
                let new_rent_minimum = sysvar_rent.minimum_balance(new_account_space);
                anchor_lang::system_program::transfer(
                    anchor_lang::context::CpiContext::new(
                        accounts.system_program.to_account_info(),
                        anchor_lang::system_program::Transfer {
                            from: accounts.authority.to_account_info(),
                            to: accounts.idl.to_account_info(),
                        },
                    ),
                    new_rent_minimum.checked_sub(idl_ref.lamports()).unwrap(),
                )?;
                idl_ref.realloc(new_account_space, false)?;
            }
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_close_account(
            program_id: &Pubkey,
            accounts: &mut IdlCloseAccount,
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: IdlCloseAccount");
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_create_buffer(
            program_id: &Pubkey,
            accounts: &mut IdlCreateBuffer,
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: IdlCreateBuffer");
            let mut buffer = &mut accounts.buffer;
            buffer.authority = *accounts.authority.key;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_write(
            program_id: &Pubkey,
            accounts: &mut IdlAccounts,
            idl_data: Vec<u8>,
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: IdlWrite");
            let prev_len: usize = ::std::convert::TryInto::<
                usize,
            >::try_into(accounts.idl.data_len)
                .unwrap();
            let new_len: usize = prev_len.checked_add(idl_data.len()).unwrap() as usize;
            accounts.idl.data_len = accounts
                .idl
                .data_len
                .checked_add(
                    ::std::convert::TryInto::<u32>::try_into(idl_data.len()).unwrap(),
                )
                .unwrap();
            use IdlTrailingData;
            let mut idl_bytes = accounts.idl.trailing_data_mut();
            let idl_expansion = &mut idl_bytes[prev_len..new_len];
            if idl_expansion.len() != idl_data.len() {
                return Err(
                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                            error_name: anchor_lang::error::ErrorCode::RequireEqViolated
                                .name(),
                            error_code_number: anchor_lang::error::ErrorCode::RequireEqViolated
                                .into(),
                            error_msg: anchor_lang::error::ErrorCode::RequireEqViolated
                                .to_string(),
                            error_origin: Some(
                                anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                    filename: "sdk-tests/csdk-anchor-full-derived-test/src/lib.rs",
                                    line: 37u32,
                                }),
                            ),
                            compared_values: None,
                        })
                        .with_values((idl_expansion.len(), idl_data.len())),
                );
            }
            idl_expansion.copy_from_slice(&idl_data[..]);
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_set_authority(
            program_id: &Pubkey,
            accounts: &mut IdlAccounts,
            new_authority: Pubkey,
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: IdlSetAuthority");
            accounts.idl.authority = new_authority;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_set_buffer(
            program_id: &Pubkey,
            accounts: &mut IdlSetBuffer,
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: IdlSetBuffer");
            accounts.idl.data_len = accounts.buffer.data_len;
            use IdlTrailingData;
            let buffer_len = ::std::convert::TryInto::<
                usize,
            >::try_into(accounts.buffer.data_len)
                .unwrap();
            let mut target = accounts.idl.trailing_data_mut();
            let source = &accounts.buffer.trailing_data()[..buffer_len];
            if target.len() < buffer_len {
                return Err(
                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                            error_name: anchor_lang::error::ErrorCode::RequireGteViolated
                                .name(),
                            error_code_number: anchor_lang::error::ErrorCode::RequireGteViolated
                                .into(),
                            error_msg: anchor_lang::error::ErrorCode::RequireGteViolated
                                .to_string(),
                            error_origin: Some(
                                anchor_lang::error::ErrorOrigin::Source(anchor_lang::error::Source {
                                    filename: "sdk-tests/csdk-anchor-full-derived-test/src/lib.rs",
                                    line: 37u32,
                                }),
                            ),
                            compared_values: None,
                        })
                        .with_values((target.len(), buffer_len)),
                );
            }
            target[..buffer_len].copy_from_slice(source);
            Ok(())
        }
    }
    /// __global mod defines wrapped handlers for global instructions.
    pub mod __global {
        use super::*;
        #[inline(never)]
        pub fn create_user_record_and_game_session<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: CreateUserRecordAndGameSession");
            let ix = instruction::CreateUserRecordAndGameSession::deserialize(
                    &mut &__ix_data[..],
                )
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::CreateUserRecordAndGameSession {
                account_data,
                compression_params,
            } = ix;
            let mut __bumps = <CreateUserRecordAndGameSession as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = CreateUserRecordAndGameSession::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = csdk_anchor_full_derived_test::create_user_record_and_game_session(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                account_data,
                compression_params,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn decompress_accounts_idempotent<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: DecompressAccountsIdempotent");
            let ix = instruction::DecompressAccountsIdempotent::deserialize(
                    &mut &__ix_data[..],
                )
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::DecompressAccountsIdempotent {
                proof,
                compressed_accounts,
                system_accounts_offset,
            } = ix;
            let mut __bumps = <DecompressAccountsIdempotent as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = DecompressAccountsIdempotent::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = csdk_anchor_full_derived_test::decompress_accounts_idempotent(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                proof,
                compressed_accounts,
                system_accounts_offset,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn compress_accounts_idempotent<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: CompressAccountsIdempotent");
            let ix = instruction::CompressAccountsIdempotent::deserialize(
                    &mut &__ix_data[..],
                )
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::CompressAccountsIdempotent {
                proof,
                compressed_accounts,
                signer_seeds,
                system_accounts_offset,
            } = ix;
            let mut __bumps = <CompressAccountsIdempotent as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = CompressAccountsIdempotent::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = csdk_anchor_full_derived_test::compress_accounts_idempotent(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                proof,
                compressed_accounts,
                signer_seeds,
                system_accounts_offset,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn initialize_compression_config<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: InitializeCompressionConfig");
            let ix = instruction::InitializeCompressionConfig::deserialize(
                    &mut &__ix_data[..],
                )
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::InitializeCompressionConfig {
                compression_delay,
                rent_sponsor,
                address_space,
            } = ix;
            let mut __bumps = <InitializeCompressionConfig as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = InitializeCompressionConfig::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = csdk_anchor_full_derived_test::initialize_compression_config(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                compression_delay,
                rent_sponsor,
                address_space,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn update_compression_config<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: UpdateCompressionConfig");
            let ix = instruction::UpdateCompressionConfig::deserialize(
                    &mut &__ix_data[..],
                )
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::UpdateCompressionConfig {
                new_compression_delay,
                new_rent_sponsor,
                new_address_space,
                new_update_authority,
            } = ix;
            let mut __bumps = <UpdateCompressionConfig as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = UpdateCompressionConfig::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = csdk_anchor_full_derived_test::update_compression_config(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                new_compression_delay,
                new_rent_sponsor,
                new_address_space,
                new_update_authority,
            )?;
            __accounts.exit(__program_id)
        }
    }
}
#[allow(non_snake_case)]
pub mod csdk_anchor_full_derived_test {
    use anchor_lang::solana_program::{program::invoke, sysvar::clock::Clock};
    use light_compressed_token_sdk::instructions::{
        create_mint_action_cpi, find_spl_mint_address, MintActionInputs,
    };
    use light_sdk::{
        compressible::{
            compress_account_on_init::prepare_compressed_account_on_init,
            CompressibleConfig,
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
        errors::ErrorCode, state::{GameSession, UserRecord},
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
            return Err(ProgramError::from(ErrorCode::RentRecipientMismatch).into());
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
            .into_new_address_params_assigned_packed(
                user_record.key().to_bytes().into(),
                Some(0),
            );
        let game_new_address_params = compression_params
            .game_address_tree_info
            .into_new_address_params_assigned_packed(
                game_session.key().to_bytes().into(),
                Some(1),
            );
        let mut all_compressed_infos = Vec::new();
        let user_record_info = user_record.to_account_info();
        let user_record_data_mut = &mut **user_record;
        let user_compressed_info = prepare_compressed_account_on_init::<
            UserRecord,
        >(
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
        let game_compressed_info = prepare_compressed_account_on_init::<
            GameSession,
        >(
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
        let (_, token_account_address) = get_ctokensigner_seeds(
            &ctx.accounts.user.key(),
            &mint,
        );
        let actions = <[_]>::into_vec(
            ::alloc::boxed::box_new([
                light_compressed_token_sdk::instructions::mint_action::MintActionType::MintTo {
                    recipients: <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            light_compressed_token_sdk::instructions::mint_action::MintToRecipient {
                                recipient: token_account_address,
                                amount: 1000,
                            },
                        ]),
                    ),
                    token_account_version: 3,
                },
            ]),
        );
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
        account_infos
            .push(ctx.accounts.compress_token_program_cpi_authority.to_account_info());
        account_infos.push(ctx.accounts.ctoken_program.to_account_info());
        account_infos.push(ctx.accounts.mint_authority.to_account_info());
        account_infos.push(ctx.accounts.mint_signer.to_account_info());
        account_infos.push(ctx.accounts.user.to_account_info());
        invoke(&mint_action_instruction, &account_infos)?;
        user_record.close(ctx.accounts.rent_sponsor.to_account_info())?;
        game_session.close(ctx.accounts.rent_sponsor.to_account_info())?;
        Ok(())
    }
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
        pub ctoken_rent_sponsor: AccountInfo<'info>,
        /// Compressed token program (auto-resolved constant)
        /// CHECK: Enforced to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
        #[account(
            address = solana_pubkey::pubkey!(
                "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"
            )
        )]
        pub ctoken_program: UncheckedAccount<'info>,
        /// CPI authority PDA of the compressed token program (auto-resolved constant)
        /// CHECK: Enforced to be GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy
        #[account(
            address = solana_pubkey::pubkey!(
                "GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"
            )
        )]
        pub ctoken_cpi_authority: UncheckedAccount<'info>,
        /// CHECK: CToken CompressibleConfig account (default but can be overridden)
        pub ctoken_config: UncheckedAccount<'info>,
        /// CHECK: Optional seed account - required only if decompressing dependent accounts.
        /// Validated by runtime checks when needed.
        pub mint: Option<UncheckedAccount<'info>>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, DecompressAccountsIdempotentBumps>
    for DecompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        #[inline(never)]
        fn try_accounts(
            __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >],
            __ix_data: &[u8],
            __bumps: &mut DecompressAccountsIdempotentBumps,
            __reallocs: &mut std::collections::BTreeSet<
                anchor_lang::solana_program::pubkey::Pubkey,
            >,
        ) -> anchor_lang::Result<Self> {
            let fee_payer: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("fee_payer"))?;
            let config: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("config"))?;
            let rent_payer: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("rent_payer"))?;
            let ctoken_rent_sponsor: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_rent_sponsor"))?;
            let ctoken_program: UncheckedAccount = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_program"))?;
            let ctoken_cpi_authority: UncheckedAccount = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_cpi_authority"))?;
            let ctoken_config: UncheckedAccount = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_config"))?;
            let mint: Option<UncheckedAccount> = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("mint"))?;
            if !AsRef::<AccountInfo>::as_ref(&fee_payer).is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("fee_payer"),
                );
            }
            if !AsRef::<AccountInfo>::as_ref(&rent_payer).is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("rent_payer"),
                );
            }
            if !&ctoken_rent_sponsor.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("ctoken_rent_sponsor"),
                );
            }
            {
                let actual = ctoken_program.key();
                let expected = ::solana_pubkey::Pubkey::from_str_const(
                    "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
                );
                if actual != expected {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintAddress,
                            )
                            .with_account_name("ctoken_program")
                            .with_pubkeys((actual, expected)),
                    );
                }
            }
            {
                let actual = ctoken_cpi_authority.key();
                let expected = ::solana_pubkey::Pubkey::from_str_const(
                    "GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy",
                );
                if actual != expected {
                    return Err(
                        anchor_lang::error::Error::from(
                                anchor_lang::error::ErrorCode::ConstraintAddress,
                            )
                            .with_account_name("ctoken_cpi_authority")
                            .with_pubkeys((actual, expected)),
                    );
                }
            }
            Ok(DecompressAccountsIdempotent {
                fee_payer,
                config,
                rent_payer,
                ctoken_rent_sponsor,
                ctoken_program,
                ctoken_cpi_authority,
                ctoken_config,
                mint,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info>
    for DecompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.fee_payer.to_account_infos());
            account_infos.extend(self.config.to_account_infos());
            account_infos.extend(self.rent_payer.to_account_infos());
            account_infos.extend(self.ctoken_rent_sponsor.to_account_infos());
            account_infos.extend(self.ctoken_program.to_account_infos());
            account_infos.extend(self.ctoken_cpi_authority.to_account_infos());
            account_infos.extend(self.ctoken_config.to_account_infos());
            account_infos.extend(self.mint.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for DecompressAccountsIdempotent<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.fee_payer.to_account_metas(None));
            account_metas.extend(self.config.to_account_metas(None));
            account_metas.extend(self.rent_payer.to_account_metas(None));
            account_metas.extend(self.ctoken_rent_sponsor.to_account_metas(None));
            account_metas.extend(self.ctoken_program.to_account_metas(None));
            account_metas.extend(self.ctoken_cpi_authority.to_account_metas(None));
            account_metas.extend(self.ctoken_config.to_account_metas(None));
            if let Some(mint) = &self.mint {
                account_metas.extend(mint.to_account_metas(None));
            } else {
                account_metas.push(AccountMeta::new_readonly(crate::ID, false));
            }
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info> for DecompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        fn exit(
            &self,
            program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        ) -> anchor_lang::Result<()> {
            anchor_lang::AccountsExit::exit(&self.fee_payer, program_id)
                .map_err(|e| e.with_account_name("fee_payer"))?;
            anchor_lang::AccountsExit::exit(&self.rent_payer, program_id)
                .map_err(|e| e.with_account_name("rent_payer"))?;
            anchor_lang::AccountsExit::exit(&self.ctoken_rent_sponsor, program_id)
                .map_err(|e| e.with_account_name("ctoken_rent_sponsor"))?;
            Ok(())
        }
    }
    pub struct DecompressAccountsIdempotentBumps {}
    #[automatically_derived]
    impl ::core::fmt::Debug for DecompressAccountsIdempotentBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "DecompressAccountsIdempotentBumps")
        }
    }
    impl Default for DecompressAccountsIdempotentBumps {
        fn default() -> Self {
            DecompressAccountsIdempotentBumps {
            }
        }
    }
    impl<'info> anchor_lang::Bumps for DecompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        type Bumps = DecompressAccountsIdempotentBumps;
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
    /// instead of an `AccountInfo`. This is useful for clients that want
    /// to generate a list of accounts, without explicitly knowing the
    /// order all the fields should be in.
    ///
    /// To access the struct in this module, one should use the sibling
    /// `accounts` module (also generated), which re-exports this.
    pub(crate) mod __client_accounts_decompress_accounts_idempotent {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`DecompressAccountsIdempotent`].
        pub struct DecompressAccountsIdempotent {
            pub fee_payer: Pubkey,
            ///The global config account
            pub config: Pubkey,
            ///UNCHECKED: Anyone can pay to init PDAs.
            pub rent_payer: Pubkey,
            ///UNCHECKED: Anyone can pay to init compressed tokens.
            pub ctoken_rent_sponsor: Pubkey,
            ///Compressed token program (auto-resolved constant)
            pub ctoken_program: Pubkey,
            ///CPI authority PDA of the compressed token program (auto-resolved constant)
            pub ctoken_cpi_authority: Pubkey,
            pub ctoken_config: Pubkey,
            ///Validated by runtime checks when needed.
            pub mint: Option<Pubkey>,
        }
        impl borsh::ser::BorshSerialize for DecompressAccountsIdempotent
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Option<Pubkey>: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.fee_payer, writer)?;
                borsh::BorshSerialize::serialize(&self.config, writer)?;
                borsh::BorshSerialize::serialize(&self.rent_payer, writer)?;
                borsh::BorshSerialize::serialize(&self.ctoken_rent_sponsor, writer)?;
                borsh::BorshSerialize::serialize(&self.ctoken_program, writer)?;
                borsh::BorshSerialize::serialize(&self.ctoken_cpi_authority, writer)?;
                borsh::BorshSerialize::serialize(&self.ctoken_config, writer)?;
                borsh::BorshSerialize::serialize(&self.mint, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for DecompressAccountsIdempotent {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            "Generated client accounts for [`DecompressAccountsIdempotent`]."
                                .into(),
                        ]),
                    ),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(
                            anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(
                                    ::alloc::boxed::box_new([
                                        anchor_lang::idl::types::IdlField {
                                            name: "fee_payer".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "config".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "The global config account".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "rent_payer".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "UNCHECKED: Anyone can pay to init PDAs.".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_rent_sponsor".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "UNCHECKED: Anyone can pay to init compressed tokens."
                                                        .into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_program".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Compressed token program (auto-resolved constant)".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_cpi_authority".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "CPI authority PDA of the compressed token program (auto-resolved constant)"
                                                        .into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_config".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "mint".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Validated by runtime checks when needed.".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Option(
                                                Box::new(anchor_lang::idl::types::IdlType::Pubkey),
                                            ),
                                        },
                                    ]),
                                ),
                            ),
                        ),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) {}
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::__client_accounts_decompress_accounts_idempotent",
                            "DecompressAccountsIdempotent",
                        ),
                    )
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for DecompressAccountsIdempotent {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.fee_payer,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.config,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.rent_payer,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.ctoken_rent_sponsor,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.ctoken_program,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.ctoken_cpi_authority,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.ctoken_config,
                            false,
                        ),
                    );
                if let Some(mint) = &self.mint {
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                *mint,
                                false,
                            ),
                        );
                } else {
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                crate::ID,
                                false,
                            ),
                        );
                }
                account_metas
            }
        }
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a CPI struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is an
    /// AccountInfo.
    ///
    /// To access the struct in this module, one should use the sibling
    /// [`cpi::accounts`] module (also generated), which re-exports this.
    pub(crate) mod __cpi_client_accounts_decompress_accounts_idempotent {
        use super::*;
        /// Generated CPI struct of the accounts for [`DecompressAccountsIdempotent`].
        pub struct DecompressAccountsIdempotent<'info> {
            pub fee_payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The global config account
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///UNCHECKED: Anyone can pay to init PDAs.
            pub rent_payer: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///UNCHECKED: Anyone can pay to init compressed tokens.
            pub ctoken_rent_sponsor: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///Compressed token program (auto-resolved constant)
            pub ctoken_program: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///CPI authority PDA of the compressed token program (auto-resolved constant)
            pub ctoken_cpi_authority: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            pub ctoken_config: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///Validated by runtime checks when needed.
            pub mint: Option<
                anchor_lang::solana_program::account_info::AccountInfo<'info>,
            >,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for DecompressAccountsIdempotent<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.fee_payer),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.config),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.rent_payer),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.ctoken_rent_sponsor),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.ctoken_program),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.ctoken_cpi_authority),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.ctoken_config),
                            false,
                        ),
                    );
                if let Some(mint) = &self.mint {
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                anchor_lang::Key::key(mint),
                                false,
                            ),
                        );
                } else {
                    account_metas
                        .push(
                            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                                crate::ID,
                                false,
                            ),
                        );
                }
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info>
        for DecompressAccountsIdempotent<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.fee_payer),
                    );
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.rent_payer),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_rent_sponsor,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_program,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_cpi_authority,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_config,
                        ),
                    );
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.mint));
                account_infos
            }
        }
    }
    impl<'info> DecompressAccountsIdempotent<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlAccount,
            >,
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "fee_payer".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new(["The global config account".into()]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "rent_payer".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "UNCHECKED: Anyone can pay to init PDAs.".into(),
                            ]),
                        ),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_rent_sponsor".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "UNCHECKED: Anyone can pay to init compressed tokens."
                                    .into(),
                            ]),
                        ),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_program".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Compressed token program (auto-resolved constant)".into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_cpi_authority".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "CPI authority PDA of the compressed token program (auto-resolved constant)"
                                    .into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_config".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "mint".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Validated by runtime checks when needed.".into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: true,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                ]),
            )
        }
    }
    mod __macro_helpers {
        use super::*;
        #[inline(never)]
        fn handle_packed_UserRecord<'a, 'b, 'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
            address_space: solana_pubkey::Pubkey,
            solana_accounts: &[solana_account_info::AccountInfo<'info>],
            i: usize,
            packed: &PackedUserRecord,
            meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            post_system_accounts: &[solana_account_info::AccountInfo<'info>],
            compressed_pda_infos: &mut Vec<
                light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo,
            >,
        ) -> Result<()> {
            light_sdk::compressible::handle_packed_pda_variant::<
                UserRecord,
                PackedUserRecord,
            >(
                    &accounts.rent_payer,
                    cpi_accounts,
                    address_space,
                    &solana_accounts[i],
                    i,
                    packed,
                    meta,
                    post_system_accounts,
                    compressed_pda_infos,
                    &crate::ID,
                )
                .map_err(|e| e.into())
        }
        #[inline(never)]
        fn handle_packed_GameSession<'a, 'b, 'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
            address_space: solana_pubkey::Pubkey,
            solana_accounts: &[solana_account_info::AccountInfo<'info>],
            i: usize,
            packed: &PackedGameSession,
            meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            post_system_accounts: &[solana_account_info::AccountInfo<'info>],
            compressed_pda_infos: &mut Vec<
                light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo,
            >,
        ) -> Result<()> {
            light_sdk::compressible::handle_packed_pda_variant::<
                GameSession,
                PackedGameSession,
            >(
                    &accounts.rent_payer,
                    cpi_accounts,
                    address_space,
                    &solana_accounts[i],
                    i,
                    packed,
                    meta,
                    post_system_accounts,
                    compressed_pda_infos,
                    &crate::ID,
                )
                .map_err(|e| e.into())
        }
        #[inline(never)]
        fn handle_packed_PlaceholderRecord<'a, 'b, 'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
            address_space: solana_pubkey::Pubkey,
            solana_accounts: &[solana_account_info::AccountInfo<'info>],
            i: usize,
            packed: &PackedPlaceholderRecord,
            meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            post_system_accounts: &[solana_account_info::AccountInfo<'info>],
            compressed_pda_infos: &mut Vec<
                light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo,
            >,
        ) -> Result<()> {
            light_sdk::compressible::handle_packed_pda_variant::<
                PlaceholderRecord,
                PackedPlaceholderRecord,
            >(
                    &accounts.rent_payer,
                    cpi_accounts,
                    address_space,
                    &solana_accounts[i],
                    i,
                    packed,
                    meta,
                    post_system_accounts,
                    compressed_pda_infos,
                    &crate::ID,
                )
                .map_err(|e| e.into())
        }
        #[inline(never)]
        pub fn collect_pda_and_token<'a, 'b, 'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
            address_space: solana_pubkey::Pubkey,
            compressed_accounts: Vec<CompressedAccountData>,
            solana_accounts: &[solana_account_info::AccountInfo<'info>],
        ) -> Result<
            (
                Vec<
                    light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo,
                >,
                Vec<
                    (
                        light_compressed_token_sdk::compat::PackedCTokenData<
                            CTokenAccountVariant,
                        >,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    ),
                >,
            ),
        > {
            let post_system_offset = cpi_accounts.system_accounts_end_offset();
            let all_infos = cpi_accounts.account_infos();
            let post_system_accounts = &all_infos[post_system_offset..];
            let estimated_capacity = compressed_accounts.len();
            let mut compressed_pda_infos = Vec::with_capacity(estimated_capacity);
            let mut compressed_token_accounts: Vec<
                (
                    light_compressed_token_sdk::compat::PackedCTokenData<
                        CTokenAccountVariant,
                    >,
                    light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                ),
            > = Vec::with_capacity(estimated_capacity);
            for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                let meta = compressed_data.meta;
                match compressed_data.data {
                    CompressedAccountVariant::UserRecord(_) => {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "internal error: entered unreachable code: {0}",
                                    format_args!(
                                        "Unpacked variants should not be present during decompression - accounts are always packed in-flight",
                                    ),
                                ),
                            );
                        };
                    }
                    CompressedAccountVariant::GameSession(_) => {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "internal error: entered unreachable code: {0}",
                                    format_args!(
                                        "Unpacked variants should not be present during decompression - accounts are always packed in-flight",
                                    ),
                                ),
                            );
                        };
                    }
                    CompressedAccountVariant::PlaceholderRecord(_) => {
                        {
                            ::core::panicking::panic_fmt(
                                format_args!(
                                    "internal error: entered unreachable code: {0}",
                                    format_args!(
                                        "Unpacked variants should not be present during decompression - accounts are always packed in-flight",
                                    ),
                                ),
                            );
                        };
                    }
                    CompressedAccountVariant::PackedUserRecord(packed) => {
                        handle_packed_UserRecord(
                            accounts,
                            &cpi_accounts,
                            address_space,
                            solana_accounts,
                            i,
                            &packed,
                            &meta,
                            post_system_accounts,
                            &mut compressed_pda_infos,
                        )?;
                    }
                    CompressedAccountVariant::PackedGameSession(packed) => {
                        handle_packed_GameSession(
                            accounts,
                            &cpi_accounts,
                            address_space,
                            solana_accounts,
                            i,
                            &packed,
                            &meta,
                            post_system_accounts,
                            &mut compressed_pda_infos,
                        )?;
                    }
                    CompressedAccountVariant::PackedPlaceholderRecord(packed) => {
                        handle_packed_PlaceholderRecord(
                            accounts,
                            &cpi_accounts,
                            address_space,
                            solana_accounts,
                            i,
                            &packed,
                            &meta,
                            post_system_accounts,
                            &mut compressed_pda_infos,
                        )?;
                    }
                    CompressedAccountVariant::PackedCTokenData(mut data) => {
                        data.token_data.version = 3;
                        compressed_token_accounts.push((data, meta));
                    }
                    CompressedAccountVariant::CTokenData(_) => {
                        ::core::panicking::panic(
                            "internal error: entered unreachable code",
                        );
                    }
                }
            }
            Ok((compressed_pda_infos, compressed_token_accounts))
        }
    }
    /// Local trait-based system for CToken variant seed handling
    pub mod ctoken_seed_system {
        use super::*;
        pub struct CTokenSeedContext<'a, 'info> {
            pub accounts: &'a DecompressAccountsIdempotent<'info>,
            pub remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
        }
        pub trait CTokenSeedProvider {
            /// Get seeds for the token account PDA (used for decompression)
            fn get_seeds<'a, 'info>(
                &self,
                ctx: &CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;
            /// Get authority seeds for signing during compression
            fn get_authority_seeds<'a, 'info>(
                &self,
                ctx: &CTokenSeedContext<'a, 'info>,
            ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;
        }
    }
    /// Trait implementations for standardized runtime helpers
    mod __trait_impls {
        use super::*;
        /// Implement HasTokenVariant for CompressedAccountVariant
        impl light_sdk::compressible::HasTokenVariant for CompressedAccountData {
            fn is_packed_ctoken(&self) -> bool {
                #[allow(non_exhaustive_omitted_patterns)]
                match self.data {
                    CompressedAccountVariant::PackedCTokenData(_) => true,
                    _ => false,
                }
            }
        }
        /// Implement CTokenSeedProvider for CTokenAccountVariant via local seed system
        impl light_sdk::compressible::CTokenSeedProvider for CTokenAccountVariant {
            type Accounts<'info> = DecompressAccountsIdempotent<'info>;
            fn get_seeds<'a, 'info>(
                &self,
                accounts: &'a Self::Accounts<'info>,
                remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
            ) -> std::result::Result<
                (Vec<Vec<u8>>, solana_pubkey::Pubkey),
                anchor_lang::prelude::ProgramError,
            > {
                use super::ctoken_seed_system::{
                    CTokenSeedContext, CTokenSeedProvider as LocalProvider,
                };
                let ctx = CTokenSeedContext {
                    accounts,
                    remaining_accounts,
                };
                LocalProvider::get_seeds(self, &ctx).map_err(|e| e.into())
            }
            fn get_authority_seeds<'a, 'info>(
                &self,
                accounts: &'a Self::Accounts<'info>,
                remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
            ) -> std::result::Result<
                (Vec<Vec<u8>>, solana_pubkey::Pubkey),
                anchor_lang::prelude::ProgramError,
            > {
                use super::ctoken_seed_system::{
                    CTokenSeedContext, CTokenSeedProvider as LocalProvider,
                };
                let ctx = CTokenSeedContext {
                    accounts,
                    remaining_accounts,
                };
                LocalProvider::get_authority_seeds(self, &ctx).map_err(|e| e.into())
            }
        }
        /// Also implement light_compressed_token_sdk::CTokenSeedProvider for token decompression runtime
        impl light_compressed_token_sdk::CTokenSeedProvider for CTokenAccountVariant {
            type Accounts<'info> = DecompressAccountsIdempotent<'info>;
            fn get_seeds<'a, 'info>(
                &self,
                accounts: &'a Self::Accounts<'info>,
                remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
            ) -> std::result::Result<
                (Vec<Vec<u8>>, solana_pubkey::Pubkey),
                solana_program_error::ProgramError,
            > {
                use super::ctoken_seed_system::{
                    CTokenSeedContext, CTokenSeedProvider as LocalProvider,
                };
                let ctx = CTokenSeedContext {
                    accounts,
                    remaining_accounts,
                };
                LocalProvider::get_seeds(self, &ctx)
                    .map_err(|e: anchor_lang::error::Error| {
                        let program_error: anchor_lang::prelude::ProgramError = e.into();
                        let code = match program_error {
                            anchor_lang::prelude::ProgramError::Custom(code) => code,
                            _ => 0,
                        };
                        solana_program_error::ProgramError::Custom(code)
                    })
            }
            fn get_authority_seeds<'a, 'info>(
                &self,
                accounts: &'a Self::Accounts<'info>,
                remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
            ) -> std::result::Result<
                (Vec<Vec<u8>>, solana_pubkey::Pubkey),
                solana_program_error::ProgramError,
            > {
                use super::ctoken_seed_system::{
                    CTokenSeedContext, CTokenSeedProvider as LocalProvider,
                };
                let ctx = CTokenSeedContext {
                    accounts,
                    remaining_accounts,
                };
                LocalProvider::get_authority_seeds(self, &ctx)
                    .map_err(|e: anchor_lang::error::Error| {
                        let program_error: anchor_lang::prelude::ProgramError = e.into();
                        let code = match program_error {
                            anchor_lang::prelude::ProgramError::Custom(code) => code,
                            _ => 0,
                        };
                        solana_program_error::ProgramError::Custom(code)
                    })
            }
        }
    }
    mod __decompress_context_impl {
        use super::*;
        impl<'info> light_sdk::compressible::DecompressContext<'info>
        for DecompressAccountsIdempotent<'info> {
            type CompressedData = CompressedAccountData;
            type PackedTokenData = light_compressed_token_sdk::compat::PackedCTokenData<
                CTokenAccountVariant,
            >;
            type CompressedMeta = light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
            fn fee_payer(&self) -> &solana_account_info::AccountInfo<'info> {
                self.fee_payer.as_ref()
            }
            fn config(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.config
            }
            fn rent_payer(&self) -> &solana_account_info::AccountInfo<'info> {
                self.rent_payer.as_ref()
            }
            fn ctoken_rent_sponsor(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_rent_sponsor
            }
            fn ctoken_program(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_program.to_account_info()
            }
            fn ctoken_cpi_authority(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_cpi_authority.to_account_info()
            }
            fn ctoken_config(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_config.to_account_info()
            }
            fn collect_pda_and_token<'b>(
                &self,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
                address_space: solana_pubkey::Pubkey,
                compressed_accounts: Vec<Self::CompressedData>,
                solana_accounts: &[solana_account_info::AccountInfo<'info>],
            ) -> std::result::Result<
                (
                    Vec<
                        light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo,
                    >,
                    Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
                ),
                anchor_lang::prelude::ProgramError,
            > {
                let post_system_offset = cpi_accounts.system_accounts_end_offset();
                let all_infos = cpi_accounts.account_infos();
                let post_system_accounts = &all_infos[post_system_offset..];
                let program_id = &crate::ID;
                let mut compressed_pda_infos = Vec::with_capacity(
                    compressed_accounts.len(),
                );
                let mut compressed_token_accounts = Vec::with_capacity(
                    compressed_accounts.len(),
                );
                for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                    let meta = compressed_data.meta;
                    match compressed_data.data {
                        CompressedAccountVariant::PackedUserRecord(packed) => {
                            light_sdk::compressible::handle_packed_pda_variant::<
                                UserRecord,
                                PackedUserRecord,
                            >(
                                    &self.rent_payer,
                                    cpi_accounts,
                                    address_space,
                                    &solana_accounts[i],
                                    i,
                                    &packed,
                                    &meta,
                                    post_system_accounts,
                                    &mut compressed_pda_infos,
                                    &program_id,
                                )
                                .map_err(|e| e.into())?;
                        }
                        CompressedAccountVariant::UserRecord(_) => {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "internal error: entered unreachable code: {0}",
                                        format_args!(
                                            "Unpacked variants should not be present during decompression",
                                        ),
                                    ),
                                );
                            };
                        }
                        CompressedAccountVariant::PackedGameSession(packed) => {
                            light_sdk::compressible::handle_packed_pda_variant::<
                                GameSession,
                                PackedGameSession,
                            >(
                                    &self.rent_payer,
                                    cpi_accounts,
                                    address_space,
                                    &solana_accounts[i],
                                    i,
                                    &packed,
                                    &meta,
                                    post_system_accounts,
                                    &mut compressed_pda_infos,
                                    &program_id,
                                )
                                .map_err(|e| e.into())?;
                        }
                        CompressedAccountVariant::GameSession(_) => {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "internal error: entered unreachable code: {0}",
                                        format_args!(
                                            "Unpacked variants should not be present during decompression",
                                        ),
                                    ),
                                );
                            };
                        }
                        CompressedAccountVariant::PackedPlaceholderRecord(packed) => {
                            light_sdk::compressible::handle_packed_pda_variant::<
                                PlaceholderRecord,
                                PackedPlaceholderRecord,
                            >(
                                    &self.rent_payer,
                                    cpi_accounts,
                                    address_space,
                                    &solana_accounts[i],
                                    i,
                                    &packed,
                                    &meta,
                                    post_system_accounts,
                                    &mut compressed_pda_infos,
                                    &program_id,
                                )
                                .map_err(|e| e.into())?;
                        }
                        CompressedAccountVariant::PlaceholderRecord(_) => {
                            {
                                ::core::panicking::panic_fmt(
                                    format_args!(
                                        "internal error: entered unreachable code: {0}",
                                        format_args!(
                                            "Unpacked variants should not be present during decompression",
                                        ),
                                    ),
                                );
                            };
                        }
                        CompressedAccountVariant::PackedCTokenData(mut data) => {
                            data.token_data.version = 3;
                            compressed_token_accounts.push((data, meta));
                        }
                        CompressedAccountVariant::CTokenData(_) => {
                            ::core::panicking::panic(
                                "internal error: entered unreachable code",
                            );
                        }
                    }
                }
                Ok((compressed_pda_infos, compressed_token_accounts))
            }
            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn process_tokens<'b>(
                &self,
                remaining_accounts: &[solana_account_info::AccountInfo<'info>],
                fee_payer: &solana_account_info::AccountInfo<'info>,
                ctoken_program: &solana_account_info::AccountInfo<'info>,
                ctoken_rent_sponsor: &solana_account_info::AccountInfo<'info>,
                ctoken_cpi_authority: &solana_account_info::AccountInfo<'info>,
                ctoken_config: &solana_account_info::AccountInfo<'info>,
                config: &solana_account_info::AccountInfo<'info>,
                ctoken_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
                proof: light_sdk::instruction::ValidityProof,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
                post_system_accounts: &[solana_account_info::AccountInfo<'info>],
                has_pdas: bool,
            ) -> std::result::Result<(), anchor_lang::prelude::ProgramError> {
                light_compressed_token_sdk::decompress_runtime::process_decompress_tokens_runtime(
                    self,
                    remaining_accounts,
                    fee_payer,
                    ctoken_program,
                    ctoken_rent_sponsor,
                    ctoken_cpi_authority,
                    ctoken_config,
                    config,
                    ctoken_accounts,
                    proof,
                    cpi_accounts,
                    post_system_accounts,
                    has_pdas,
                    &crate::ID,
                )
            }
        }
    }
    mod __processor_functions {
        use super::*;
        #[inline(never)]
        pub fn process_decompress_accounts_idempotent<'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<CompressedAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_sdk::compressible::process_decompress_accounts_idempotent(
                    accounts,
                    remaining_accounts,
                    compressed_accounts,
                    proof,
                    system_accounts_offset,
                    LIGHT_CPI_SIGNER,
                    &crate::ID,
                )
                .map_err(|e| e.into())
        }
        /// Core processor for compress_accounts_idempotent.
        ///
        /// Thin wrapper that delegates to compressed-token-sdk runtime.
        #[inline(never)]
        pub fn process_compress_accounts_idempotent<'info>(
            accounts: &CompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            compressed_accounts: Vec<
                light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
            >,
            signer_seeds: Vec<Vec<Vec<u8>>>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_compressed_token_sdk::compress_runtime::process_compress_accounts_idempotent(
                    accounts,
                    remaining_accounts,
                    compressed_accounts,
                    signer_seeds,
                    system_accounts_offset,
                    LIGHT_CPI_SIGNER,
                    &crate::ID,
                )
                .map_err(|e| e.into())
        }
    }
    /// Auto-generated decompress_accounts_idempotent instruction.
    #[inline(never)]
    pub fn decompress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
        proof: light_sdk::instruction::ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        __processor_functions::process_decompress_accounts_idempotent(
            &ctx.accounts,
            &ctx.remaining_accounts,
            proof,
            compressed_accounts,
            system_accounts_offset,
        )
    }
    pub struct CompressAccountsIdempotent<'info> {
        #[account(mut)]
        pub fee_payer: Signer<'info>,
        /// The global config account
        /// CHECK: Config is validated by the SDK's load_checked method
        pub config: AccountInfo<'info>,
        /// Rent sponsor - must match config
        /// CHECK: Rent sponsor is validated against the config
        #[account(mut)]
        pub rent_sponsor: AccountInfo<'info>,
        /// CHECK: compression_authority must be the rent_authority defined when creating the PDA account.
        #[account(mut)]
        pub compression_authority: AccountInfo<'info>,
        /// CHECK: token_compression_authority must be the rent_authority defined when creating the token account.
        #[account(mut)]
        pub ctoken_compression_authority: AccountInfo<'info>,
        /// Token rent sponsor - must match config
        /// CHECK: Token rent sponsor is validated against the config
        #[account(mut)]
        pub ctoken_rent_sponsor: AccountInfo<'info>,
        /// Compressed token program (always required in mixed variant)
        /// CHECK: Program ID validated to be cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m
        pub ctoken_program: UncheckedAccount<'info>,
        /// CPI authority PDA of the compressed token program (always required in mixed variant)
        /// CHECK: PDA derivation validated with seeds ["cpi_authority"] and bump 254
        pub ctoken_cpi_authority: UncheckedAccount<'info>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, CompressAccountsIdempotentBumps>
    for CompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        #[inline(never)]
        fn try_accounts(
            __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >],
            __ix_data: &[u8],
            __bumps: &mut CompressAccountsIdempotentBumps,
            __reallocs: &mut std::collections::BTreeSet<
                anchor_lang::solana_program::pubkey::Pubkey,
            >,
        ) -> anchor_lang::Result<Self> {
            let fee_payer: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("fee_payer"))?;
            let config: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("config"))?;
            let rent_sponsor: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("rent_sponsor"))?;
            let compression_authority: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("compression_authority"))?;
            let ctoken_compression_authority: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_compression_authority"))?;
            let ctoken_rent_sponsor: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_rent_sponsor"))?;
            let ctoken_program: UncheckedAccount = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_program"))?;
            let ctoken_cpi_authority: UncheckedAccount = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("ctoken_cpi_authority"))?;
            if !AsRef::<AccountInfo>::as_ref(&fee_payer).is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("fee_payer"),
                );
            }
            if !&rent_sponsor.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("rent_sponsor"),
                );
            }
            if !&compression_authority.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("compression_authority"),
                );
            }
            if !&ctoken_compression_authority.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("ctoken_compression_authority"),
                );
            }
            if !&ctoken_rent_sponsor.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("ctoken_rent_sponsor"),
                );
            }
            Ok(CompressAccountsIdempotent {
                fee_payer,
                config,
                rent_sponsor,
                compression_authority,
                ctoken_compression_authority,
                ctoken_rent_sponsor,
                ctoken_program,
                ctoken_cpi_authority,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for CompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.fee_payer.to_account_infos());
            account_infos.extend(self.config.to_account_infos());
            account_infos.extend(self.rent_sponsor.to_account_infos());
            account_infos.extend(self.compression_authority.to_account_infos());
            account_infos.extend(self.ctoken_compression_authority.to_account_infos());
            account_infos.extend(self.ctoken_rent_sponsor.to_account_infos());
            account_infos.extend(self.ctoken_program.to_account_infos());
            account_infos.extend(self.ctoken_cpi_authority.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for CompressAccountsIdempotent<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.fee_payer.to_account_metas(None));
            account_metas.extend(self.config.to_account_metas(None));
            account_metas.extend(self.rent_sponsor.to_account_metas(None));
            account_metas.extend(self.compression_authority.to_account_metas(None));
            account_metas
                .extend(self.ctoken_compression_authority.to_account_metas(None));
            account_metas.extend(self.ctoken_rent_sponsor.to_account_metas(None));
            account_metas.extend(self.ctoken_program.to_account_metas(None));
            account_metas.extend(self.ctoken_cpi_authority.to_account_metas(None));
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info> for CompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        fn exit(
            &self,
            program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        ) -> anchor_lang::Result<()> {
            anchor_lang::AccountsExit::exit(&self.fee_payer, program_id)
                .map_err(|e| e.with_account_name("fee_payer"))?;
            anchor_lang::AccountsExit::exit(&self.rent_sponsor, program_id)
                .map_err(|e| e.with_account_name("rent_sponsor"))?;
            anchor_lang::AccountsExit::exit(&self.compression_authority, program_id)
                .map_err(|e| e.with_account_name("compression_authority"))?;
            anchor_lang::AccountsExit::exit(
                    &self.ctoken_compression_authority,
                    program_id,
                )
                .map_err(|e| e.with_account_name("ctoken_compression_authority"))?;
            anchor_lang::AccountsExit::exit(&self.ctoken_rent_sponsor, program_id)
                .map_err(|e| e.with_account_name("ctoken_rent_sponsor"))?;
            Ok(())
        }
    }
    pub struct CompressAccountsIdempotentBumps {}
    #[automatically_derived]
    impl ::core::fmt::Debug for CompressAccountsIdempotentBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "CompressAccountsIdempotentBumps")
        }
    }
    impl Default for CompressAccountsIdempotentBumps {
        fn default() -> Self {
            CompressAccountsIdempotentBumps {}
        }
    }
    impl<'info> anchor_lang::Bumps for CompressAccountsIdempotent<'info>
    where
        'info: 'info,
    {
        type Bumps = CompressAccountsIdempotentBumps;
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
    /// instead of an `AccountInfo`. This is useful for clients that want
    /// to generate a list of accounts, without explicitly knowing the
    /// order all the fields should be in.
    ///
    /// To access the struct in this module, one should use the sibling
    /// `accounts` module (also generated), which re-exports this.
    pub(crate) mod __client_accounts_compress_accounts_idempotent {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`CompressAccountsIdempotent`].
        pub struct CompressAccountsIdempotent {
            pub fee_payer: Pubkey,
            ///The global config account
            pub config: Pubkey,
            ///Rent sponsor - must match config
            pub rent_sponsor: Pubkey,
            pub compression_authority: Pubkey,
            pub ctoken_compression_authority: Pubkey,
            ///Token rent sponsor - must match config
            pub ctoken_rent_sponsor: Pubkey,
            ///Compressed token program (always required in mixed variant)
            pub ctoken_program: Pubkey,
            ///CPI authority PDA of the compressed token program (always required in mixed variant)
            pub ctoken_cpi_authority: Pubkey,
        }
        impl borsh::ser::BorshSerialize for CompressAccountsIdempotent
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.fee_payer, writer)?;
                borsh::BorshSerialize::serialize(&self.config, writer)?;
                borsh::BorshSerialize::serialize(&self.rent_sponsor, writer)?;
                borsh::BorshSerialize::serialize(&self.compression_authority, writer)?;
                borsh::BorshSerialize::serialize(
                    &self.ctoken_compression_authority,
                    writer,
                )?;
                borsh::BorshSerialize::serialize(&self.ctoken_rent_sponsor, writer)?;
                borsh::BorshSerialize::serialize(&self.ctoken_program, writer)?;
                borsh::BorshSerialize::serialize(&self.ctoken_cpi_authority, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for CompressAccountsIdempotent {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            "Generated client accounts for [`CompressAccountsIdempotent`]."
                                .into(),
                        ]),
                    ),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(
                            anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(
                                    ::alloc::boxed::box_new([
                                        anchor_lang::idl::types::IdlField {
                                            name: "fee_payer".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "config".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "The global config account".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "rent_sponsor".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Rent sponsor - must match config".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "compression_authority".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_compression_authority".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_rent_sponsor".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Token rent sponsor - must match config".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_program".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "Compressed token program (always required in mixed variant)"
                                                        .into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "ctoken_cpi_authority".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "CPI authority PDA of the compressed token program (always required in mixed variant)"
                                                        .into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                    ]),
                                ),
                            ),
                        ),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) {}
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::__client_accounts_compress_accounts_idempotent",
                            "CompressAccountsIdempotent",
                        ),
                    )
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CompressAccountsIdempotent {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.fee_payer,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.config,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.rent_sponsor,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.compression_authority,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.ctoken_compression_authority,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.ctoken_rent_sponsor,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.ctoken_program,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.ctoken_cpi_authority,
                            false,
                        ),
                    );
                account_metas
            }
        }
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a CPI struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is an
    /// AccountInfo.
    ///
    /// To access the struct in this module, one should use the sibling
    /// [`cpi::accounts`] module (also generated), which re-exports this.
    pub(crate) mod __cpi_client_accounts_compress_accounts_idempotent {
        use super::*;
        /// Generated CPI struct of the accounts for [`CompressAccountsIdempotent`].
        pub struct CompressAccountsIdempotent<'info> {
            pub fee_payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The global config account
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///Rent sponsor - must match config
            pub rent_sponsor: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            pub compression_authority: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            pub ctoken_compression_authority: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///Token rent sponsor - must match config
            pub ctoken_rent_sponsor: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///Compressed token program (always required in mixed variant)
            pub ctoken_program: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///CPI authority PDA of the compressed token program (always required in mixed variant)
            pub ctoken_cpi_authority: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for CompressAccountsIdempotent<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.fee_payer),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.config),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.rent_sponsor),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.compression_authority),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.ctoken_compression_authority),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.ctoken_rent_sponsor),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.ctoken_program),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.ctoken_cpi_authority),
                            false,
                        ),
                    );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info>
        for CompressAccountsIdempotent<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.fee_payer),
                    );
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.rent_sponsor),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.compression_authority,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_compression_authority,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_rent_sponsor,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_program,
                        ),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.ctoken_cpi_authority,
                        ),
                    );
                account_infos
            }
        }
    }
    impl<'info> CompressAccountsIdempotent<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlAccount,
            >,
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "fee_payer".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new(["The global config account".into()]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "rent_sponsor".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Rent sponsor - must match config".into(),
                            ]),
                        ),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "compression_authority".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_compression_authority".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_rent_sponsor".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Token rent sponsor - must match config".into(),
                            ]),
                        ),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_program".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "Compressed token program (always required in mixed variant)"
                                    .into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "ctoken_cpi_authority".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "CPI authority PDA of the compressed token program (always required in mixed variant)"
                                    .into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                ]),
            )
        }
    }
    mod __compress_context_impl {
        use super::*;
        impl<'info> light_sdk::compressible::CompressContext<'info>
        for CompressAccountsIdempotent<'info> {
            fn fee_payer(&self) -> &solana_account_info::AccountInfo<'info> {
                self.fee_payer.as_ref()
            }
            fn config(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.config
            }
            fn rent_sponsor(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.rent_sponsor
            }
            fn ctoken_rent_sponsor(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_rent_sponsor
            }
            fn compression_authority(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.compression_authority
            }
            fn ctoken_compression_authority(
                &self,
            ) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_compression_authority
            }
            fn ctoken_program(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_program.to_account_info()
            }
            fn ctoken_cpi_authority(&self) -> &solana_account_info::AccountInfo<'info> {
                &self.ctoken_cpi_authority.to_account_info()
            }
            fn compress_pda_account(
                &self,
                account_info: &solana_account_info::AccountInfo<'info>,
                meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'_, 'info>,
                compression_config: &light_sdk::compressible::CompressibleConfig,
                program_id: &solana_pubkey::Pubkey,
            ) -> std::result::Result<
                Option<
                    light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo,
                >,
                solana_program_error::ProgramError,
            > {
                let data = account_info
                    .try_borrow_data()
                    .map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err
                            .into();
                        let code = match program_error {
                            anchor_lang::prelude::ProgramError::Custom(code) => code,
                            _ => 0,
                        };
                        solana_program_error::ProgramError::Custom(code)
                    })?;
                let discriminator = &data[0..8];
                match discriminator {
                    d if d == UserRecord::LIGHT_DISCRIMINATOR => {
                        drop(data);
                        let data_borrow = account_info
                            .try_borrow_data()
                            .map_err(|e| {
                                let err: anchor_lang::error::Error = e.into();
                                let program_error: anchor_lang::prelude::ProgramError = err
                                    .into();
                                let code = match program_error {
                                    anchor_lang::prelude::ProgramError::Custom(code) => code,
                                    _ => 0,
                                };
                                solana_program_error::ProgramError::Custom(code)
                            })?;
                        let mut account_data = UserRecord::try_deserialize(
                                &mut &data_borrow[..],
                            )
                            .map_err(|e| {
                                let err: anchor_lang::error::Error = e.into();
                                let program_error: anchor_lang::prelude::ProgramError = err
                                    .into();
                                let code = match program_error {
                                    anchor_lang::prelude::ProgramError::Custom(code) => code,
                                    _ => 0,
                                };
                                solana_program_error::ProgramError::Custom(code)
                            })?;
                        drop(data_borrow);
                        let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<
                            UserRecord,
                        >(
                            program_id,
                            account_info,
                            &mut account_data,
                            meta,
                            cpi_accounts,
                            &compression_config.compression_delay,
                            &compression_config.address_space,
                        )?;
                        Ok(Some(compressed_info))
                    }
                    d if d == GameSession::LIGHT_DISCRIMINATOR => {
                        drop(data);
                        let data_borrow = account_info
                            .try_borrow_data()
                            .map_err(|e| {
                                let err: anchor_lang::error::Error = e.into();
                                let program_error: anchor_lang::prelude::ProgramError = err
                                    .into();
                                let code = match program_error {
                                    anchor_lang::prelude::ProgramError::Custom(code) => code,
                                    _ => 0,
                                };
                                solana_program_error::ProgramError::Custom(code)
                            })?;
                        let mut account_data = GameSession::try_deserialize(
                                &mut &data_borrow[..],
                            )
                            .map_err(|e| {
                                let err: anchor_lang::error::Error = e.into();
                                let program_error: anchor_lang::prelude::ProgramError = err
                                    .into();
                                let code = match program_error {
                                    anchor_lang::prelude::ProgramError::Custom(code) => code,
                                    _ => 0,
                                };
                                solana_program_error::ProgramError::Custom(code)
                            })?;
                        drop(data_borrow);
                        let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<
                            GameSession,
                        >(
                            program_id,
                            account_info,
                            &mut account_data,
                            meta,
                            cpi_accounts,
                            &compression_config.compression_delay,
                            &compression_config.address_space,
                        )?;
                        Ok(Some(compressed_info))
                    }
                    d if d == PlaceholderRecord::LIGHT_DISCRIMINATOR => {
                        drop(data);
                        let data_borrow = account_info
                            .try_borrow_data()
                            .map_err(|e| {
                                let err: anchor_lang::error::Error = e.into();
                                let program_error: anchor_lang::prelude::ProgramError = err
                                    .into();
                                let code = match program_error {
                                    anchor_lang::prelude::ProgramError::Custom(code) => code,
                                    _ => 0,
                                };
                                solana_program_error::ProgramError::Custom(code)
                            })?;
                        let mut account_data = PlaceholderRecord::try_deserialize(
                                &mut &data_borrow[..],
                            )
                            .map_err(|e| {
                                let err: anchor_lang::error::Error = e.into();
                                let program_error: anchor_lang::prelude::ProgramError = err
                                    .into();
                                let code = match program_error {
                                    anchor_lang::prelude::ProgramError::Custom(code) => code,
                                    _ => 0,
                                };
                                solana_program_error::ProgramError::Custom(code)
                            })?;
                        drop(data_borrow);
                        let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<
                            PlaceholderRecord,
                        >(
                            program_id,
                            account_info,
                            &mut account_data,
                            meta,
                            cpi_accounts,
                            &compression_config.compression_delay,
                            &compression_config.address_space,
                        )?;
                        Ok(Some(compressed_info))
                    }
                    _ => {
                        let err: anchor_lang::error::Error = anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                            .into();
                        let program_error: anchor_lang::prelude::ProgramError = err
                            .into();
                        let code = match program_error {
                            anchor_lang::prelude::ProgramError::Custom(code) => code,
                            _ => 0,
                        };
                        Err(solana_program_error::ProgramError::Custom(code))
                    }
                }
            }
        }
    }
    /// Auto-generated compress_accounts_idempotent instruction.
    #[inline(never)]
    pub fn compress_accounts_idempotent<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressAccountsIdempotent<'info>>,
        proof: light_sdk::instruction::ValidityProof,
        compressed_accounts: Vec<
            light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
        >,
        signer_seeds: Vec<Vec<Vec<u8>>>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        __processor_functions::process_compress_accounts_idempotent(
            &ctx.accounts,
            &ctx.remaining_accounts,
            compressed_accounts,
            signer_seeds,
            system_accounts_offset,
        )
    }
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
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, InitializeCompressionConfigBumps>
    for InitializeCompressionConfig<'info>
    where
        'info: 'info,
    {
        #[inline(never)]
        fn try_accounts(
            __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >],
            __ix_data: &[u8],
            __bumps: &mut InitializeCompressionConfigBumps,
            __reallocs: &mut std::collections::BTreeSet<
                anchor_lang::solana_program::pubkey::Pubkey,
            >,
        ) -> anchor_lang::Result<Self> {
            let payer: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("payer"))?;
            let config: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("config"))?;
            let program_data: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("program_data"))?;
            let authority: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("authority"))?;
            let system_program: anchor_lang::accounts::program::Program<System> = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("system_program"))?;
            if !AsRef::<AccountInfo>::as_ref(&payer).is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("payer"),
                );
            }
            if !&config.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("config"),
                );
            }
            Ok(InitializeCompressionConfig {
                payer,
                config,
                program_data,
                authority,
                system_program,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for InitializeCompressionConfig<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.payer.to_account_infos());
            account_infos.extend(self.config.to_account_infos());
            account_infos.extend(self.program_data.to_account_infos());
            account_infos.extend(self.authority.to_account_infos());
            account_infos.extend(self.system_program.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for InitializeCompressionConfig<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.payer.to_account_metas(None));
            account_metas.extend(self.config.to_account_metas(None));
            account_metas.extend(self.program_data.to_account_metas(None));
            account_metas.extend(self.authority.to_account_metas(None));
            account_metas.extend(self.system_program.to_account_metas(None));
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info> for InitializeCompressionConfig<'info>
    where
        'info: 'info,
    {
        fn exit(
            &self,
            program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        ) -> anchor_lang::Result<()> {
            anchor_lang::AccountsExit::exit(&self.payer, program_id)
                .map_err(|e| e.with_account_name("payer"))?;
            anchor_lang::AccountsExit::exit(&self.config, program_id)
                .map_err(|e| e.with_account_name("config"))?;
            Ok(())
        }
    }
    pub struct InitializeCompressionConfigBumps {}
    #[automatically_derived]
    impl ::core::fmt::Debug for InitializeCompressionConfigBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "InitializeCompressionConfigBumps")
        }
    }
    impl Default for InitializeCompressionConfigBumps {
        fn default() -> Self {
            InitializeCompressionConfigBumps {
            }
        }
    }
    impl<'info> anchor_lang::Bumps for InitializeCompressionConfig<'info>
    where
        'info: 'info,
    {
        type Bumps = InitializeCompressionConfigBumps;
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
    /// instead of an `AccountInfo`. This is useful for clients that want
    /// to generate a list of accounts, without explicitly knowing the
    /// order all the fields should be in.
    ///
    /// To access the struct in this module, one should use the sibling
    /// `accounts` module (also generated), which re-exports this.
    pub(crate) mod __client_accounts_initialize_compression_config {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`InitializeCompressionConfig`].
        pub struct InitializeCompressionConfig {
            pub payer: Pubkey,
            pub config: Pubkey,
            ///The program's data account
            pub program_data: Pubkey,
            ///The program's upgrade authority (must sign)
            pub authority: Pubkey,
            pub system_program: Pubkey,
        }
        impl borsh::ser::BorshSerialize for InitializeCompressionConfig
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.payer, writer)?;
                borsh::BorshSerialize::serialize(&self.config, writer)?;
                borsh::BorshSerialize::serialize(&self.program_data, writer)?;
                borsh::BorshSerialize::serialize(&self.authority, writer)?;
                borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for InitializeCompressionConfig {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            "Generated client accounts for [`InitializeCompressionConfig`]."
                                .into(),
                        ]),
                    ),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(
                            anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(
                                    ::alloc::boxed::box_new([
                                        anchor_lang::idl::types::IdlField {
                                            name: "payer".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "config".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "program_data".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "The program's data account".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "authority".into(),
                                            docs: <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    "The program's upgrade authority (must sign)".into(),
                                                ]),
                                            ),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "system_program".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                    ]),
                                ),
                            ),
                        ),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) {}
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::__client_accounts_initialize_compression_config",
                            "InitializeCompressionConfig",
                        ),
                    )
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for InitializeCompressionConfig {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.payer,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.config,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.program_data,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.authority,
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.system_program,
                            false,
                        ),
                    );
                account_metas
            }
        }
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a CPI struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is an
    /// AccountInfo.
    ///
    /// To access the struct in this module, one should use the sibling
    /// [`cpi::accounts`] module (also generated), which re-exports this.
    pub(crate) mod __cpi_client_accounts_initialize_compression_config {
        use super::*;
        /// Generated CPI struct of the accounts for [`InitializeCompressionConfig`].
        pub struct InitializeCompressionConfig<'info> {
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The program's data account
            pub program_data: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
            ///The program's upgrade authority (must sign)
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for InitializeCompressionConfig<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.payer),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.config),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.program_data),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.authority),
                            true,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.system_program),
                            false,
                        ),
                    );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info>
        for InitializeCompressionConfig<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.program_data),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.authority),
                    );
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(
                            &self.system_program,
                        ),
                    );
                account_infos
            }
        }
    }
    impl<'info> InitializeCompressionConfig<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlAccount,
            >,
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "payer".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "program_data".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "The program's data account".into(),
                            ]),
                        ),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "authority".into(),
                        docs: <[_]>::into_vec(
                            ::alloc::boxed::box_new([
                                "The program's upgrade authority (must sign)".into(),
                            ]),
                        ),
                        writable: false,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "system_program".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                ]),
            )
        }
    }
    pub struct UpdateCompressionConfig<'info> {
        /// CHECK: config account is validated by the SDK
        #[account(mut)]
        pub config: AccountInfo<'info>,
        /// CHECK: authority must be the current update authority
        pub authority: Signer<'info>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, UpdateCompressionConfigBumps>
    for UpdateCompressionConfig<'info>
    where
        'info: 'info,
    {
        #[inline(never)]
        fn try_accounts(
            __program_id: &anchor_lang::solana_program::pubkey::Pubkey,
            __accounts: &mut &'info [anchor_lang::solana_program::account_info::AccountInfo<
                'info,
            >],
            __ix_data: &[u8],
            __bumps: &mut UpdateCompressionConfigBumps,
            __reallocs: &mut std::collections::BTreeSet<
                anchor_lang::solana_program::pubkey::Pubkey,
            >,
        ) -> anchor_lang::Result<Self> {
            let config: AccountInfo = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("config"))?;
            let authority: Signer = anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("authority"))?;
            if !&config.is_writable {
                return Err(
                    anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintMut,
                        )
                        .with_account_name("config"),
                );
            }
            Ok(UpdateCompressionConfig {
                config,
                authority,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateCompressionConfig<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.config.to_account_infos());
            account_infos.extend(self.authority.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for UpdateCompressionConfig<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.config.to_account_metas(None));
            account_metas.extend(self.authority.to_account_metas(None));
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info> for UpdateCompressionConfig<'info>
    where
        'info: 'info,
    {
        fn exit(
            &self,
            program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        ) -> anchor_lang::Result<()> {
            anchor_lang::AccountsExit::exit(&self.config, program_id)
                .map_err(|e| e.with_account_name("config"))?;
            Ok(())
        }
    }
    pub struct UpdateCompressionConfigBumps {}
    #[automatically_derived]
    impl ::core::fmt::Debug for UpdateCompressionConfigBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "UpdateCompressionConfigBumps")
        }
    }
    impl Default for UpdateCompressionConfigBumps {
        fn default() -> Self {
            UpdateCompressionConfigBumps {}
        }
    }
    impl<'info> anchor_lang::Bumps for UpdateCompressionConfig<'info>
    where
        'info: 'info,
    {
        type Bumps = UpdateCompressionConfigBumps;
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is a Pubkey,
    /// instead of an `AccountInfo`. This is useful for clients that want
    /// to generate a list of accounts, without explicitly knowing the
    /// order all the fields should be in.
    ///
    /// To access the struct in this module, one should use the sibling
    /// `accounts` module (also generated), which re-exports this.
    pub(crate) mod __client_accounts_update_compression_config {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`UpdateCompressionConfig`].
        pub struct UpdateCompressionConfig {
            pub config: Pubkey,
            pub authority: Pubkey,
        }
        impl borsh::ser::BorshSerialize for UpdateCompressionConfig
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.config, writer)?;
                borsh::BorshSerialize::serialize(&self.authority, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for UpdateCompressionConfig {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            "Generated client accounts for [`UpdateCompressionConfig`]."
                                .into(),
                        ]),
                    ),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(
                            anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(
                                    ::alloc::boxed::box_new([
                                        anchor_lang::idl::types::IdlField {
                                            name: "config".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                        anchor_lang::idl::types::IdlField {
                                            name: "authority".into(),
                                            docs: ::alloc::vec::Vec::new(),
                                            ty: anchor_lang::idl::types::IdlType::Pubkey,
                                        },
                                    ]),
                                ),
                            ),
                        ),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<
                    String,
                    anchor_lang::idl::types::IdlTypeDef,
                >,
            ) {}
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::__client_accounts_update_compression_config",
                            "UpdateCompressionConfig",
                        ),
                    )
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for UpdateCompressionConfig {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            self.config,
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.authority,
                            true,
                        ),
                    );
                account_metas
            }
        }
    }
    /// An internal, Anchor generated module. This is used (as an
    /// implementation detail), to generate a CPI struct for a given
    /// `#[derive(Accounts)]` implementation, where each field is an
    /// AccountInfo.
    ///
    /// To access the struct in this module, one should use the sibling
    /// [`cpi::accounts`] module (also generated), which re-exports this.
    pub(crate) mod __cpi_client_accounts_update_compression_config {
        use super::*;
        /// Generated CPI struct of the accounts for [`UpdateCompressionConfig`].
        pub struct UpdateCompressionConfig<'info> {
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for UpdateCompressionConfig<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new(
                            anchor_lang::Key::key(&self.config),
                            false,
                        ),
                    );
                account_metas
                    .push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.authority),
                            true,
                        ),
                    );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info>
        for UpdateCompressionConfig<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos
                    .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos
                    .extend(
                        anchor_lang::ToAccountInfos::to_account_infos(&self.authority),
                    );
                account_infos
            }
        }
    }
    impl<'info> UpdateCompressionConfig<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlAccount,
            >,
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(anchor_lang::idl::types::IdlInstructionAccount {
                        name: "authority".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    }),
                ]),
            )
        }
    }
    /// Initialize compression config for the program
    #[inline(never)]
    pub fn initialize_compression_config<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeCompressionConfig<'info>>,
        compression_delay: u32,
        rent_sponsor: Pubkey,
        address_space: Vec<Pubkey>,
    ) -> Result<()> {
        light_sdk::compressible::process_initialize_compression_config_checked(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.program_data.to_account_info(),
            &rent_sponsor,
            address_space,
            compression_delay,
            0,
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )?;
        Ok(())
    }
    /// Update compression config for the program
    #[inline(never)]
    pub fn update_compression_config<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateCompressionConfig<'info>>,
        new_compression_delay: Option<u32>,
        new_rent_sponsor: Option<Pubkey>,
        new_address_space: Option<Vec<Pubkey>>,
        new_update_authority: Option<Pubkey>,
    ) -> Result<()> {
        light_sdk::compressible::process_update_compression_config(
            ctx.accounts.config.as_ref(),
            ctx.accounts.authority.as_ref(),
            new_update_authority.as_ref(),
            new_rent_sponsor.as_ref(),
            new_address_space,
            new_compression_delay,
            &crate::ID,
        )?;
        Ok(())
    }
    /// Auto-generated CTokenSeedProvider implementation
    impl ctoken_seed_system::CTokenSeedProvider for CTokenAccountVariant {
        fn get_seeds<'a, 'info>(
            &self,
            ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
        ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)> {
            match self {
                CTokenAccountVariant::CTokenSigner => {
                    let seed_1 = ctx
                        .accounts
                        .fee_payer
                        .as_ref()
                        .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                        .key();
                    let seed_2 = ctx
                        .accounts
                        .mint
                        .as_ref()
                        .ok_or(CompressibleInstructionError::MissingSeedAccount)?
                        .key();
                    let seeds: &[&[u8]] = &[
                        "ctoken_signer".as_bytes(),
                        seed_1.as_ref(),
                        seed_2.as_ref(),
                    ];
                    let (token_account_pda, bump) = solana_pubkey::Pubkey::find_program_address(
                        seeds,
                        &crate::ID,
                    );
                    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                    seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                    seeds_vec.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
                    Ok((seeds_vec, token_account_pda))
                }
                _ => {
                    Err(
                        anchor_lang::prelude::ProgramError::Custom(
                            CompressibleInstructionError::MissingSeedAccount.into(),
                        ),
                    )
                }
            }
        }
        fn get_authority_seeds<'a, 'info>(
            &self,
            ctx: &ctoken_seed_system::CTokenSeedContext<'a, 'info>,
        ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)> {
            match self {
                CTokenAccountVariant::CTokenSigner => {
                    let seeds: &[&[u8]] = &[LIGHT_CPI_SIGNER.cpi_signer.as_ref()];
                    let (authority_pda, bump) = solana_pubkey::Pubkey::find_program_address(
                        seeds,
                        &crate::ID,
                    );
                    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                    seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                    seeds_vec.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
                    Ok((seeds_vec, authority_pda))
                }
                _ => {
                    Err(
                        anchor_lang::prelude::ProgramError::Custom(
                            CompressibleInstructionError::MissingSeedAccount.into(),
                        ),
                    )
                }
            }
        }
    }
}
/// An Anchor generated module containing the program's set of
/// instructions, where each method handler in the `#[program]` mod is
/// associated with a struct defining the input arguments to the
/// method. These should be used directly, when one wants to serialize
/// Anchor instruction data, for example, when speciying
/// instructions on a client.
pub mod instruction {
    use super::*;
    /// Instruction.
    pub struct CreateUserRecordAndGameSession {
        pub account_data: AccountCreationData,
        pub compression_params: CompressionParams,
    }
    impl borsh::ser::BorshSerialize for CreateUserRecordAndGameSession
    where
        AccountCreationData: borsh::ser::BorshSerialize,
        CompressionParams: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.account_data, writer)?;
            borsh::BorshSerialize::serialize(&self.compression_params, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for CreateUserRecordAndGameSession {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(::alloc::boxed::box_new(["Instruction.".into()])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "account_data".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <AccountCreationData>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_params".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <CompressionParams>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <AccountCreationData>::create_type() {
                types.insert(<AccountCreationData>::get_full_path(), ty);
                <AccountCreationData>::insert_types(types);
            }
            if let Some(ty) = <CompressionParams>::create_type() {
                types.insert(<CompressionParams>::get_full_path(), ty);
                <CompressionParams>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::instruction",
                        "CreateUserRecordAndGameSession",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for CreateUserRecordAndGameSession
    where
        AccountCreationData: borsh::BorshDeserialize,
        CompressionParams: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                account_data: borsh::BorshDeserialize::deserialize_reader(reader)?,
                compression_params: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for CreateUserRecordAndGameSession {
        const DISCRIMINATOR: &'static [u8] = &[130, 196, 129, 145, 131, 124, 218, 98];
    }
    impl anchor_lang::InstructionData for CreateUserRecordAndGameSession {}
    impl anchor_lang::Owner for CreateUserRecordAndGameSession {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct DecompressAccountsIdempotent {
        pub proof: light_sdk::instruction::ValidityProof,
        pub compressed_accounts: Vec<CompressedAccountData>,
        pub system_accounts_offset: u8,
    }
    impl borsh::ser::BorshSerialize for DecompressAccountsIdempotent
    where
        light_sdk::instruction::ValidityProof: borsh::ser::BorshSerialize,
        Vec<CompressedAccountData>: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.compressed_accounts, writer)?;
            borsh::BorshSerialize::serialize(&self.system_accounts_offset, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for DecompressAccountsIdempotent {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(::alloc::boxed::box_new(["Instruction.".into()])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "proof".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <light_sdk::instruction::ValidityProof>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "compressed_accounts".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Vec(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <CompressedAccountData>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "system_accounts_offset".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <light_sdk::instruction::ValidityProof>::create_type() {
                types
                    .insert(
                        <light_sdk::instruction::ValidityProof>::get_full_path(),
                        ty,
                    );
                <light_sdk::instruction::ValidityProof>::insert_types(types);
            }
            if let Some(ty) = <CompressedAccountData>::create_type() {
                types.insert(<CompressedAccountData>::get_full_path(), ty);
                <CompressedAccountData>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::instruction",
                        "DecompressAccountsIdempotent",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for DecompressAccountsIdempotent
    where
        light_sdk::instruction::ValidityProof: borsh::BorshDeserialize,
        Vec<CompressedAccountData>: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                compressed_accounts: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                system_accounts_offset: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
            })
        }
    }
    impl anchor_lang::Discriminator for DecompressAccountsIdempotent {
        const DISCRIMINATOR: &'static [u8] = &[114, 67, 61, 123, 234, 31, 1, 112];
    }
    impl anchor_lang::InstructionData for DecompressAccountsIdempotent {}
    impl anchor_lang::Owner for DecompressAccountsIdempotent {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct CompressAccountsIdempotent {
        pub proof: light_sdk::instruction::ValidityProof,
        pub compressed_accounts: Vec<
            light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
        >,
        pub signer_seeds: Vec<Vec<Vec<u8>>>,
        pub system_accounts_offset: u8,
    }
    impl borsh::ser::BorshSerialize for CompressAccountsIdempotent
    where
        light_sdk::instruction::ValidityProof: borsh::ser::BorshSerialize,
        Vec<
            light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
        >: borsh::ser::BorshSerialize,
        Vec<Vec<Vec<u8>>>: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.compressed_accounts, writer)?;
            borsh::BorshSerialize::serialize(&self.signer_seeds, writer)?;
            borsh::BorshSerialize::serialize(&self.system_accounts_offset, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for CompressAccountsIdempotent {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(::alloc::boxed::box_new(["Instruction.".into()])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "proof".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <light_sdk::instruction::ValidityProof>::get_full_path(),
                                            generics: ::alloc::vec::Vec::new(),
                                        },
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "compressed_accounts".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Vec(
                                            Box::new(anchor_lang::idl::types::IdlType::Defined {
                                                name: <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::get_full_path(),
                                                generics: ::alloc::vec::Vec::new(),
                                            }),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "signer_seeds".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Vec(
                                            Box::new(
                                                anchor_lang::idl::types::IdlType::Vec(
                                                    Box::new(anchor_lang::idl::types::IdlType::Bytes),
                                                ),
                                            ),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "system_accounts_offset".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U8,
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {
            if let Some(ty) = <light_sdk::instruction::ValidityProof>::create_type() {
                types
                    .insert(
                        <light_sdk::instruction::ValidityProof>::get_full_path(),
                        ty,
                    );
                <light_sdk::instruction::ValidityProof>::insert_types(types);
            }
            if let Some(ty) = <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::create_type() {
                types
                    .insert(
                        <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::get_full_path(),
                        ty,
                    );
                <light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>::insert_types(
                    types,
                );
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::instruction",
                        "CompressAccountsIdempotent",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for CompressAccountsIdempotent
    where
        light_sdk::instruction::ValidityProof: borsh::BorshDeserialize,
        Vec<
            light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
        >: borsh::BorshDeserialize,
        Vec<Vec<Vec<u8>>>: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                compressed_accounts: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                signer_seeds: borsh::BorshDeserialize::deserialize_reader(reader)?,
                system_accounts_offset: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
            })
        }
    }
    impl anchor_lang::Discriminator for CompressAccountsIdempotent {
        const DISCRIMINATOR: &'static [u8] = &[70, 236, 171, 120, 164, 93, 113, 181];
    }
    impl anchor_lang::InstructionData for CompressAccountsIdempotent {}
    impl anchor_lang::Owner for CompressAccountsIdempotent {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct InitializeCompressionConfig {
        pub compression_delay: u32,
        pub rent_sponsor: Pubkey,
        pub address_space: Vec<Pubkey>,
    }
    impl borsh::ser::BorshSerialize for InitializeCompressionConfig
    where
        u32: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Vec<Pubkey>: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_delay, writer)?;
            borsh::BorshSerialize::serialize(&self.rent_sponsor, writer)?;
            borsh::BorshSerialize::serialize(&self.address_space, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for InitializeCompressionConfig {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(::alloc::boxed::box_new(["Instruction.".into()])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "compression_delay".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::U32,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "rent_sponsor".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Pubkey,
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "address_space".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Vec(
                                            Box::new(anchor_lang::idl::types::IdlType::Pubkey),
                                        ),
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {}
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::instruction",
                        "InitializeCompressionConfig",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for InitializeCompressionConfig
    where
        u32: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Vec<Pubkey>: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_delay: borsh::BorshDeserialize::deserialize_reader(reader)?,
                rent_sponsor: borsh::BorshDeserialize::deserialize_reader(reader)?,
                address_space: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for InitializeCompressionConfig {
        const DISCRIMINATOR: &'static [u8] = &[133, 228, 12, 169, 56, 76, 222, 61];
    }
    impl anchor_lang::InstructionData for InitializeCompressionConfig {}
    impl anchor_lang::Owner for InitializeCompressionConfig {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct UpdateCompressionConfig {
        pub new_compression_delay: Option<u32>,
        pub new_rent_sponsor: Option<Pubkey>,
        pub new_address_space: Option<Vec<Pubkey>>,
        pub new_update_authority: Option<Pubkey>,
    }
    impl borsh::ser::BorshSerialize for UpdateCompressionConfig
    where
        Option<u32>: borsh::ser::BorshSerialize,
        Option<Pubkey>: borsh::ser::BorshSerialize,
        Option<Vec<Pubkey>>: borsh::ser::BorshSerialize,
        Option<Pubkey>: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.new_compression_delay, writer)?;
            borsh::BorshSerialize::serialize(&self.new_rent_sponsor, writer)?;
            borsh::BorshSerialize::serialize(&self.new_address_space, writer)?;
            borsh::BorshSerialize::serialize(&self.new_update_authority, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for UpdateCompressionConfig {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(::alloc::boxed::box_new(["Instruction.".into()])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(
                        anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlField {
                                        name: "new_compression_delay".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::U32),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "new_rent_sponsor".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Pubkey),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "new_address_space".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(
                                                anchor_lang::idl::types::IdlType::Vec(
                                                    Box::new(anchor_lang::idl::types::IdlType::Pubkey),
                                                ),
                                            ),
                                        ),
                                    },
                                    anchor_lang::idl::types::IdlField {
                                        name: "new_update_authority".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Option(
                                            Box::new(anchor_lang::idl::types::IdlType::Pubkey),
                                        ),
                                    },
                                ]),
                            ),
                        ),
                    ),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<
                String,
                anchor_lang::idl::types::IdlTypeDef,
            >,
        ) {}
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "csdk_anchor_full_derived_test::instruction",
                        "UpdateCompressionConfig",
                    ),
                )
            })
        }
    }
    impl borsh::de::BorshDeserialize for UpdateCompressionConfig
    where
        Option<u32>: borsh::BorshDeserialize,
        Option<Pubkey>: borsh::BorshDeserialize,
        Option<Vec<Pubkey>>: borsh::BorshDeserialize,
        Option<Pubkey>: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                new_compression_delay: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                new_rent_sponsor: borsh::BorshDeserialize::deserialize_reader(reader)?,
                new_address_space: borsh::BorshDeserialize::deserialize_reader(reader)?,
                new_update_authority: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
            })
        }
    }
    impl anchor_lang::Discriminator for UpdateCompressionConfig {
        const DISCRIMINATOR: &'static [u8] = &[135, 215, 243, 81, 163, 146, 33, 70];
    }
    impl anchor_lang::InstructionData for UpdateCompressionConfig {}
    impl anchor_lang::Owner for UpdateCompressionConfig {
        fn owner() -> Pubkey {
            ID
        }
    }
}
/// An Anchor generated module, providing a set of structs
/// mirroring the structs deriving `Accounts`, where each field is
/// a `Pubkey`. This is useful for specifying accounts for a client.
pub mod accounts {
    pub use crate::__client_accounts_update_compression_config::*;
    pub use crate::__client_accounts_decompress_accounts_idempotent::*;
    pub use crate::__client_accounts_create_user_record_and_game_session::*;
    pub use crate::__client_accounts_initialize_compression_config::*;
    pub use crate::__client_accounts_compress_accounts_idempotent::*;
}
/// Client-side seed derivation functions (not program instructions)
/// These are helper functions for clients, not Anchor program instructions
mod __client_seed_functions {
    use super::*;
    /// Auto-generated client-side seed function
    pub fn get_user_record_seeds(
        owner: &Pubkey,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let mut seed_values = Vec::with_capacity(2usize + 1);
        seed_values.push(("user_record".as_bytes()).to_vec());
        seed_values.push((owner.as_ref()).to_vec());
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(
            &seed_slices,
            &crate::ID,
        );
        seed_values.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seed_values, pda)
    }
    /// Auto-generated client-side seed function
    pub fn get_game_session_seeds(
        session_id: u64,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let mut seed_values = Vec::with_capacity(2usize + 1);
        seed_values.push(("game_session".as_bytes()).to_vec());
        seed_values.push((session_id.to_le_bytes().as_ref()).to_vec());
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(
            &seed_slices,
            &crate::ID,
        );
        seed_values.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seed_values, pda)
    }
    /// Auto-generated client-side seed function
    pub fn get_placeholder_record_seeds(
        placeholder_id: u64,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let mut seed_values = Vec::with_capacity(2usize + 1);
        seed_values.push(("placeholder_record".as_bytes()).to_vec());
        seed_values.push((placeholder_id.to_le_bytes().as_ref()).to_vec());
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(
            &seed_slices,
            &crate::ID,
        );
        seed_values.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seed_values, pda)
    }
    /// Auto-generated client-side CToken seed function
    pub fn get_ctokensigner_seeds(
        fee_payer: &solana_pubkey::Pubkey,
        mint: &solana_pubkey::Pubkey,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let mut seed_values = Vec::with_capacity(3usize + 1);
        seed_values.push(("ctoken_signer".as_bytes()).to_vec());
        seed_values.push((fee_payer.as_ref()).to_vec());
        seed_values.push((mint.as_ref()).to_vec());
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(
            &seed_slices,
            &crate::ID,
        );
        seed_values.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seed_values, pda)
    }
    /// Auto-generated authority seed function for compression signing
    pub fn get_ctokensigner_authority_seeds(
        _program_id: &solana_pubkey::Pubkey,
    ) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
        let mut seed_values = Vec::with_capacity(1usize + 1);
        seed_values.push((LIGHT_CPI_SIGNER.cpi_signer.as_ref()).to_vec());
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(
            &seed_slices,
            _program_id,
        );
        seed_values.push(<[_]>::into_vec(::alloc::boxed::box_new([bump])));
        (seed_values, pda)
    }
}
pub use __client_seed_functions::*;
