#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use anchor_lang::prelude::*;
use light_sdk::{
    compressible::{CompressionInfo, HasCompressionInfo},
    derive_light_cpi_signer, LightDiscriminator, LightHasher,
};
use light_sdk_macros::add_compressible_instructions;
use light_sdk_types::CpiSigner;
/// The static program ID
pub static ID: anchor_lang::solana_program::pubkey::Pubkey =
    anchor_lang::solana_program::pubkey::Pubkey::new_from_array([
        3u8, 6u8, 70u8, 102u8, 100u8, 207u8, 39u8, 187u8, 147u8, 127u8, 107u8, 167u8, 33u8, 157u8,
        122u8, 92u8, 62u8, 164u8, 241u8, 111u8, 239u8, 68u8, 0u8, 202u8, 98u8, 33u8, 4u8, 120u8,
        0u8, 0u8, 0u8, 0u8,
    ]);
/// Const version of `ID`
pub const ID_CONST: anchor_lang::solana_program::pubkey::Pubkey =
    anchor_lang::solana_program::pubkey::Pubkey::new_from_array([
        3u8, 6u8, 70u8, 102u8, 100u8, 207u8, 39u8, 187u8, 147u8, 127u8, 107u8, 167u8, 33u8, 157u8,
        122u8, 92u8, 62u8, 164u8, 241u8, 111u8, 239u8, 68u8, 0u8, 202u8, 98u8, 33u8, 4u8, 120u8,
        0u8, 0u8, 0u8, 0u8,
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
pub const COMPRESSION_DELAY: u32 = 100;
pub const LIGHT_CPI_SIGNER: CpiSigner = {
    ::light_sdk_types::CpiSigner {
        program_id: [
            229, 27, 189, 177, 59, 219, 216, 77, 57, 234, 132, 178, 253, 183, 68, 203, 122, 149,
            156, 116, 234, 189, 90, 28, 138, 204, 148, 223, 113, 189, 253, 126,
        ],
        cpi_signer: [
            149, 132, 159, 193, 10, 184, 134, 173, 175, 180, 232, 110, 145, 4, 235, 205, 133, 172,
            125, 46, 47, 215, 196, 60, 67, 148, 248, 69, 200, 71, 227, 250,
        ],
        bump: 255u8,
    }
};
use self::anchor_compressible_user_derived::*;
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let (program_id, accounts, instruction_data) =
        unsafe { ::solana_program_entrypoint::deserialize(input) };
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
    try_entry(program_id, accounts, data).map_err(|e| {
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
    pub struct AnchorCompressibleUserDerived;
    #[automatically_derived]
    impl ::core::clone::Clone for AnchorCompressibleUserDerived {
        #[inline]
        fn clone(&self) -> AnchorCompressibleUserDerived {
            AnchorCompressibleUserDerived
        }
    }
    impl anchor_lang::Id for AnchorCompressibleUserDerived {
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
    if data.starts_with(instruction::CreateCompressionConfig::DISCRIMINATOR) {
        return __private::__global::create_compression_config(
            program_id,
            accounts,
            &data[instruction::CreateCompressionConfig::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::UpdateCompressionConfig::DISCRIMINATOR) {
        return __private::__global::update_compression_config(
            program_id,
            accounts,
            &data[instruction::UpdateCompressionConfig::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::DecompressMultiplePdas::DISCRIMINATOR) {
        return __private::__global::decompress_multiple_pdas(
            program_id,
            accounts,
            &data[instruction::DecompressMultiplePdas::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::CompressUserRecord::DISCRIMINATOR) {
        return __private::__global::compress_user_record(
            program_id,
            accounts,
            &data[instruction::CompressUserRecord::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(instruction::CompressGameSession::DISCRIMINATOR) {
        return __private::__global::compress_game_session(
            program_id,
            accounts,
            &data[instruction::CompressGameSession::DISCRIMINATOR.len()..],
        );
    }
    if data.starts_with(anchor_lang::idl::IDL_IX_TAG_LE) {
        #[cfg(not(feature = "no-idl"))]
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
        #[cfg(not(feature = "no-idl"))]
        pub fn __idl_dispatch<'info>(
            program_id: &Pubkey,
            accounts: &'info [AccountInfo<'info>],
            idl_ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            let mut accounts = accounts;
            let mut data: &[u8] = idl_ix_data;
            let ix = anchor_lang::idl::IdlInstruction::deserialize(&mut data)
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
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
                        fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(::alloc::boxed::box_new([
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
                            ])),
                        )),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
            ) {
            }
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(format_args!(
                        "{0}::{1}",
                        "anchor_compressible_user_derived::__private::__idl", "IdlAccount",
                    ));
                    res
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
            fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
                if writer.write_all(IdlAccount::DISCRIMINATOR).is_err() {
                    return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
                }
                if AnchorSerialize::serialize(self, writer).is_err() {
                    return Err(anchor_lang::error::ErrorCode::AccountDidNotSerialize.into());
                }
                Ok(())
            }
        }
        #[automatically_derived]
        impl anchor_lang::AccountDeserialize for IdlAccount {
            fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
                if buf.len() < IdlAccount::DISCRIMINATOR.len() {
                    return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
                }
                let given_disc = &buf[..IdlAccount::DISCRIMINATOR.len()];
                if IdlAccount::DISCRIMINATOR != given_disc {
                    return Err(
                        anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                            error_name: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .name(),
                            error_code_number:
                                anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into(),
                            error_msg: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                                .to_string(),
                            error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                                anchor_lang::error::Source {
                                    filename:
                                        "program-tests/anchor-compressible-user-derived/src/lib.rs",
                                    line: 17u32,
                                },
                            )),
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
                    .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
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
        impl<'info> anchor_lang::Accounts<'info, IdlCreateAccountsBumps> for IdlCreateAccounts<'info>
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
                let system_program: anchor_lang::accounts::program::Program<System> =
                    anchor_lang::Accounts::try_accounts(
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
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintSigner,
                    )
                    .with_account_name("from"));
                }
                if !&to.is_writable {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("to"));
                }
                let (__pda_address, __bump) = Pubkey::find_program_address(&[], &__program_id);
                __bumps.base = __bump;
                if base.key() != __pda_address {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintSeeds,
                    )
                    .with_account_name("base")
                    .with_pubkeys((base.key(), __pda_address)));
                }
                if !&program.executable {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintExecutable,
                    )
                    .with_account_name("program"));
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
                IdlCreateAccountsBumps { base: u8::MAX }
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
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Generated client accounts for [`IdlCreateAccounts`].".into(),
                        ])),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(::alloc::boxed::box_new([
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
                                ])),
                            )),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {
                }
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "anchor_compressible_user_derived::__private::__idl::__client_accounts_idl_create_accounts",
                                "IdlCreateAccounts",
                            ),
                        );
                        res
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
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.from, true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.to, false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.base, false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.system_program,
                            false,
                        ),
                    );
                    account_metas.push(
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
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlCreateAccounts<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.from),
                            true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.to),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.base),
                            false,
                        ),
                    );
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.system_program),
                            false,
                        ),
                    );
                    account_metas.push(
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
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.from));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.to));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.base));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.system_program,
                    ));
                    account_infos
                        .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.program));
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
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
            ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
                <[_]>::into_vec(::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "from".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "to".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "base".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "system_program".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "program".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                ]))
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
                let idl: anchor_lang::accounts::account::Account<IdlAccount> =
                    anchor_lang::Accounts::try_accounts(
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
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("idl"));
                }
                {
                    let my_key = idl.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintHasOne,
                        )
                        .with_account_name("idl")
                        .with_pubkeys((my_key, target_key)));
                    }
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintRaw,
                    )
                    .with_account_name("authority"));
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
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Generated client accounts for [`IdlAccounts`].".into(),
                        ])),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(::alloc::boxed::box_new([
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
                                ])),
                            )),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {
                }
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "anchor_compressible_user_derived::__private::__idl::__client_accounts_idl_accounts",
                                "IdlAccounts",
                            ),
                        );
                        res
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
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.idl, false,
                    ));
                    account_metas.push(
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
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlAccounts<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.idl),
                        false,
                    ));
                    account_metas.push(
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
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.idl));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.authority,
                    ));
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
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
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
                <[_]>::into_vec(::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "idl".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                ]))
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
        impl<'info> anchor_lang::Accounts<'info, IdlResizeAccountBumps> for IdlResizeAccount<'info>
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
                let idl: anchor_lang::accounts::account::Account<IdlAccount> =
                    anchor_lang::Accounts::try_accounts(
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
                let system_program: anchor_lang::accounts::program::Program<System> =
                    anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("system_program"))?;
                if !AsRef::<AccountInfo>::as_ref(&idl).is_writable {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("idl"));
                }
                {
                    let my_key = idl.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintHasOne,
                        )
                        .with_account_name("idl")
                        .with_pubkeys((my_key, target_key)));
                    }
                }
                if !AsRef::<AccountInfo>::as_ref(&authority).is_writable {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("authority"));
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintRaw,
                    )
                    .with_account_name("authority"));
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
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Generated client accounts for [`IdlResizeAccount`].".into(),
                        ])),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(::alloc::boxed::box_new([
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
                                ])),
                            )),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {
                }
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "anchor_compressible_user_derived::__private::__idl::__client_accounts_idl_resize_account",
                                "IdlResizeAccount",
                            ),
                        );
                        res
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
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.idl, false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.authority,
                        true,
                    ));
                    account_metas.push(
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
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlResizeAccount<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.idl),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ));
                    account_metas.push(
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
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.idl));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.authority,
                    ));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.system_program,
                    ));
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
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
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
                <[_]>::into_vec(::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "idl".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "system_program".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                ]))
            }
        }
        pub struct IdlCreateBuffer<'info> {
            #[account(zero)]
            pub buffer: Account<'info, IdlAccount>,
            #[account(constraint = authority.key!= &ERASED_AUTHORITY)]
            pub authority: Signer<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::Accounts<'info, IdlCreateBufferBumps> for IdlCreateBuffer<'info>
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
                    return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
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
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintZero,
                        )
                        .with_account_name("buffer"));
                    }
                    match anchor_lang::accounts::account::Account::try_from_unchecked(&buffer) {
                        Ok(val) => val,
                        Err(e) => return Err(e.with_account_name("buffer")),
                    }
                };
                if !AsRef::<AccountInfo>::as_ref(&buffer).is_writable {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("buffer"));
                }
                if !__anchor_rent.is_exempt(
                    buffer.to_account_info().lamports(),
                    buffer.to_account_info().try_data_len()?,
                ) {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintRentExempt,
                    )
                    .with_account_name("buffer"));
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintRaw,
                    )
                    .with_account_name("authority"));
                }
                Ok(IdlCreateBuffer { buffer, authority })
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
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Generated client accounts for [`IdlCreateBuffer`].".into(),
                        ])),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(::alloc::boxed::box_new([
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
                                ])),
                            )),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {
                }
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "anchor_compressible_user_derived::__private::__idl::__client_accounts_idl_create_buffer",
                                "IdlCreateBuffer",
                            ),
                        );
                        res
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
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.buffer,
                        false,
                    ));
                    account_metas.push(
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
                pub buffer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlCreateBuffer<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.buffer),
                        false,
                    ));
                    account_metas.push(
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
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.buffer));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.authority,
                    ));
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
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
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
                <[_]>::into_vec(::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "buffer".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                ]))
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
        impl<'info> anchor_lang::Accounts<'info, IdlSetBufferBumps> for IdlSetBuffer<'info>
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
                let buffer: anchor_lang::accounts::account::Account<IdlAccount> =
                    anchor_lang::Accounts::try_accounts(
                        __program_id,
                        __accounts,
                        __ix_data,
                        __bumps,
                        __reallocs,
                    )
                    .map_err(|e| e.with_account_name("buffer"))?;
                let idl: anchor_lang::accounts::account::Account<IdlAccount> =
                    anchor_lang::Accounts::try_accounts(
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
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("buffer"));
                }
                if !(buffer.authority == idl.authority) {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintRaw,
                    )
                    .with_account_name("buffer"));
                }
                if !AsRef::<AccountInfo>::as_ref(&idl).is_writable {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("idl"));
                }
                {
                    let my_key = idl.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintHasOne,
                        )
                        .with_account_name("idl")
                        .with_pubkeys((my_key, target_key)));
                    }
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintRaw,
                    )
                    .with_account_name("authority"));
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
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Generated client accounts for [`IdlSetBuffer`].".into(),
                        ])),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(::alloc::boxed::box_new([
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
                                ])),
                            )),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {
                }
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "anchor_compressible_user_derived::__private::__idl::__client_accounts_idl_set_buffer",
                                "IdlSetBuffer",
                            ),
                        );
                        res
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
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.buffer,
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.idl, false,
                    ));
                    account_metas.push(
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
                pub buffer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub idl: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlSetBuffer<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.buffer),
                        false,
                    ));
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.idl),
                        false,
                    ));
                    account_metas.push(
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
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.buffer));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.idl));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.authority,
                    ));
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
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
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
                <[_]>::into_vec(::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "buffer".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "idl".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                ]))
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
        impl<'info> anchor_lang::Accounts<'info, IdlCloseAccountBumps> for IdlCloseAccount<'info>
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
                let account: anchor_lang::accounts::account::Account<IdlAccount> =
                    anchor_lang::Accounts::try_accounts(
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
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("account"));
                }
                {
                    let my_key = account.authority;
                    let target_key = authority.key();
                    if my_key != target_key {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintHasOne,
                        )
                        .with_account_name("account")
                        .with_pubkeys((my_key, target_key)));
                    }
                }
                {
                    if account.key() == sol_destination.key() {
                        return Err(anchor_lang::error::Error::from(
                            anchor_lang::error::ErrorCode::ConstraintClose,
                        )
                        .with_account_name("account"));
                    }
                }
                if !(authority.key != &ERASED_AUTHORITY) {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintRaw,
                    )
                    .with_account_name("authority"));
                }
                if !&sol_destination.is_writable {
                    return Err(anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("sol_destination"));
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
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Generated client accounts for [`IdlCloseAccount`].".into(),
                        ])),
                        serialization: anchor_lang::idl::types::IdlSerialization::default(),
                        repr: None,
                        generics: ::alloc::vec::Vec::new(),
                        ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                                <[_]>::into_vec(::alloc::boxed::box_new([
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
                                ])),
                            )),
                        },
                    })
                }
                fn insert_types(
                    types: &mut std::collections::BTreeMap<
                        String,
                        anchor_lang::idl::types::IdlTypeDef,
                    >,
                ) {
                }
                fn get_full_path() -> String {
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "anchor_compressible_user_derived::__private::__idl::__client_accounts_idl_close_account",
                                "IdlCloseAccount",
                            ),
                        );
                        res
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
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.account,
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            self.authority,
                            true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.sol_destination,
                        false,
                    ));
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
                pub account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
                pub sol_destination: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountMetas for IdlCloseAccount<'info> {
                fn to_account_metas(
                    &self,
                    is_signer: Option<bool>,
                ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                    let mut account_metas = ::alloc::vec::Vec::new();
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.account),
                        false,
                    ));
                    account_metas.push(
                        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                            anchor_lang::Key::key(&self.authority),
                            true,
                        ),
                    );
                    account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.sol_destination),
                        false,
                    ));
                    account_metas
                }
            }
            #[automatically_derived]
            impl<'info> anchor_lang::ToAccountInfos<'info> for IdlCloseAccount<'info> {
                fn to_account_infos(
                    &self,
                ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>>
                {
                    let mut account_infos = ::alloc::vec::Vec::new();
                    account_infos
                        .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.account));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.authority,
                    ));
                    account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                        &self.sol_destination,
                    ));
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
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
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
                <[_]>::into_vec(::alloc::boxed::box_new([
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "account".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "authority".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: false,
                            signer: true,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                    anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                        anchor_lang::idl::types::IdlInstructionAccount {
                            name: "sol_destination".into(),
                            docs: ::alloc::vec::Vec::new(),
                            writable: true,
                            signer: false,
                            optional: false,
                            address: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        },
                    ),
                ]))
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
                return Err(anchor_lang::error::ErrorCode::IdlInstructionInvalidProgram.into());
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
                .checked_add(std::cmp::min(
                    data_len
                        .checked_sub(idl_ref.data_len())
                        .expect("data_len should always be >= the current account space"),
                    10_000,
                ))
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
            let prev_len: usize =
                ::std::convert::TryInto::<usize>::try_into(accounts.idl.data_len).unwrap();
            let new_len: usize = prev_len.checked_add(idl_data.len()).unwrap() as usize;
            accounts.idl.data_len = accounts
                .idl
                .data_len
                .checked_add(::std::convert::TryInto::<u32>::try_into(idl_data.len()).unwrap())
                .unwrap();
            use IdlTrailingData;
            let mut idl_bytes = accounts.idl.trailing_data_mut();
            let idl_expansion = &mut idl_bytes[prev_len..new_len];
            if idl_expansion.len() != idl_data.len() {
                return Err(
                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                        error_name: anchor_lang::error::ErrorCode::RequireEqViolated.name(),
                        error_code_number: anchor_lang::error::ErrorCode::RequireEqViolated.into(),
                        error_msg: anchor_lang::error::ErrorCode::RequireEqViolated.to_string(),
                        error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                            anchor_lang::error::Source {
                                filename:
                                    "program-tests/anchor-compressible-user-derived/src/lib.rs",
                                line: 17u32,
                            },
                        )),
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
            let buffer_len =
                ::std::convert::TryInto::<usize>::try_into(accounts.buffer.data_len).unwrap();
            let mut target = accounts.idl.trailing_data_mut();
            let source = &accounts.buffer.trailing_data()[..buffer_len];
            if target.len() < buffer_len {
                return Err(
                    anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                        error_name: anchor_lang::error::ErrorCode::RequireGteViolated.name(),
                        error_code_number: anchor_lang::error::ErrorCode::RequireGteViolated.into(),
                        error_msg: anchor_lang::error::ErrorCode::RequireGteViolated.to_string(),
                        error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                            anchor_lang::error::Source {
                                filename:
                                    "program-tests/anchor-compressible-user-derived/src/lib.rs",
                                line: 17u32,
                            },
                        )),
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
        pub fn create_compression_config<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: CreateCompressionConfig");
            let ix = instruction::CreateCompressionConfig::deserialize(&mut &__ix_data[..])
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::CreateCompressionConfig {
                compression_delay,
                rent_recipient,
                address_space,
            } = ix;
            let mut __bumps = <CreateCompressibleConfig as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = CreateCompressibleConfig::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = anchor_compressible_user_derived::create_compression_config(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                compression_delay,
                rent_recipient,
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
            let ix = instruction::UpdateCompressionConfig::deserialize(&mut &__ix_data[..])
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::UpdateCompressionConfig {
                new_compression_delay,
                new_rent_recipient,
                new_address_space,
                new_update_authority,
            } = ix;
            let mut __bumps = <UpdateCompressibleConfig as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = UpdateCompressibleConfig::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = anchor_compressible_user_derived::update_compression_config(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                new_compression_delay,
                new_rent_recipient,
                new_address_space,
                new_update_authority,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn decompress_multiple_pdas<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: DecompressMultiplePdas");
            let ix = instruction::DecompressMultiplePdas::deserialize(&mut &__ix_data[..])
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::DecompressMultiplePdas {
                proof,
                compressed_accounts,
                bumps,
                system_accounts_offset,
            } = ix;
            let mut __bumps = <DecompressMultiplePdas as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = DecompressMultiplePdas::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = anchor_compressible_user_derived::decompress_multiple_pdas(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                proof,
                compressed_accounts,
                bumps,
                system_accounts_offset,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn compress_user_record<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: CompressUserRecord");
            let ix = instruction::CompressUserRecord::deserialize(&mut &__ix_data[..])
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::CompressUserRecord {
                proof,
                compressed_account_meta,
            } = ix;
            let mut __bumps = <CompressUserRecord as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = CompressUserRecord::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = anchor_compressible_user_derived::compress_user_record(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                proof,
                compressed_account_meta,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn compress_game_session<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_msg::sol_log("Instruction: CompressGameSession");
            let ix = instruction::CompressGameSession::deserialize(&mut &__ix_data[..])
                .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotDeserialize)?;
            let instruction::CompressGameSession {
                proof,
                compressed_account_meta,
            } = ix;
            let mut __bumps = <CompressGameSession as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = CompressGameSession::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = anchor_compressible_user_derived::compress_game_session(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                proof,
                compressed_account_meta,
            )?;
            __accounts.exit(__program_id)
        }
    }
}
pub mod anchor_compressible_user_derived {
    use super::*;
    /// Unified enum that can hold any account type
    pub enum CompressedAccountVariant {
        UserRecord(UserRecord),
        GameSession(GameSession),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for CompressedAccountVariant {
        #[inline]
        fn clone(&self) -> CompressedAccountVariant {
            match self {
                CompressedAccountVariant::UserRecord(__self_0) => {
                    CompressedAccountVariant::UserRecord(::core::clone::Clone::clone(__self_0))
                }
                CompressedAccountVariant::GameSession(__self_0) => {
                    CompressedAccountVariant::GameSession(::core::clone::Clone::clone(__self_0))
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
                    ::core::fmt::Formatter::debug_tuple_field1_finish(f, "UserRecord", &__self_0)
                }
                CompressedAccountVariant::GameSession(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(f, "GameSession", &__self_0)
                }
            }
        }
    }
    impl borsh::ser::BorshSerialize for CompressedAccountVariant
    where
        UserRecord: borsh::ser::BorshSerialize,
        GameSession: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            let variant_idx: u8 = match self {
                CompressedAccountVariant::UserRecord(..) => 0u8,
                CompressedAccountVariant::GameSession(..) => 1u8,
            };
            writer.write_all(&variant_idx.to_le_bytes())?;
            match self {
                CompressedAccountVariant::UserRecord(id0) => {
                    borsh::BorshSerialize::serialize(id0, writer)?;
                }
                CompressedAccountVariant::GameSession(id0) => {
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
                docs: <[_]>::into_vec(::alloc::boxed::box_new([
                    "Unified enum that can hold any account type".into(),
                ])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Enum {
                    variants: <[_]>::into_vec(::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "UserRecord".into(),
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                <[_]>::into_vec(::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlType::Defined {
                                        name: <UserRecord>::get_full_path(),
                                        generics: ::alloc::vec::Vec::new(),
                                    },
                                ])),
                            )),
                        },
                        anchor_lang::idl::types::IdlEnumVariant {
                            name: "GameSession".into(),
                            fields: Some(anchor_lang::idl::types::IdlDefinedFields::Tuple(
                                <[_]>::into_vec(::alloc::boxed::box_new([
                                    anchor_lang::idl::types::IdlType::Defined {
                                        name: <GameSession>::get_full_path(),
                                        generics: ::alloc::vec::Vec::new(),
                                    },
                                ])),
                            )),
                        },
                    ])),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) {
            if let Some(ty) = <UserRecord>::create_type() {
                types.insert(<UserRecord>::get_full_path(), ty);
                <UserRecord>::insert_types(types);
            }
            if let Some(ty) = <GameSession>::create_type() {
                types.insert(<GameSession>::get_full_path(), ty);
                <GameSession>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!(
                    "{0}::{1}",
                    "anchor_compressible_user_derived::anchor_compressible_user_derived",
                    "CompressedAccountVariant",
                ));
                res
            })
        }
    }
    impl borsh::de::BorshDeserialize for CompressedAccountVariant
    where
        UserRecord: borsh::BorshDeserialize,
        GameSession: borsh::BorshDeserialize,
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
        GameSession: borsh::BorshDeserialize,
    {
        fn deserialize_variant<R: borsh::maybestd::io::Read>(
            reader: &mut R,
            variant_idx: u8,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            let mut return_value = match variant_idx {
                0u8 => CompressedAccountVariant::UserRecord(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                ),
                1u8 => CompressedAccountVariant::GameSession(
                    borsh::BorshDeserialize::deserialize_reader(reader)?,
                ),
                _ => {
                    return Err(borsh::maybestd::io::Error::new(
                        borsh::maybestd::io::ErrorKind::InvalidInput,
                        ::alloc::__export::must_use({
                            let res = ::alloc::fmt::format(format_args!(
                                "Unexpected variant index: {0:?}",
                                variant_idx
                            ));
                            res
                        }),
                    ));
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
    impl light_sdk::light_hasher::DataHasher for CompressedAccountVariant {
        fn hash<H: light_sdk::light_hasher::Hasher>(
            &self,
        ) -> std::result::Result<[u8; 32], light_sdk::light_hasher::HasherError> {
            match self {
                Self::UserRecord(data) => data.hash::<H>(),
                Self::GameSession(data) => data.hash::<H>(),
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
                Self::UserRecord(data) => data.compression_info(),
                Self::GameSession(data) => data.compression_info(),
            }
        }
        fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
            match self {
                Self::UserRecord(data) => data.compression_info_mut(),
                Self::GameSession(data) => data.compression_info_mut(),
            }
        }
    }
    /// Client-side data structure for passing compressed accounts
    pub struct CompressedAccountData {
        pub meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
        pub data: CompressedAccountVariant,
        pub seeds: Vec<Vec<u8>>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for CompressedAccountData {
        #[inline]
        fn clone(&self) -> CompressedAccountData {
            CompressedAccountData {
                meta: ::core::clone::Clone::clone(&self.meta),
                data: ::core::clone::Clone::clone(&self.data),
                seeds: ::core::clone::Clone::clone(&self.seeds),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for CompressedAccountData {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "CompressedAccountData",
                "meta",
                &self.meta,
                "data",
                &self.data,
                "seeds",
                &&self.seeds,
            )
        }
    }
    impl borsh::de::BorshDeserialize for CompressedAccountData
    where
        light_sdk_types::instruction::account_meta::CompressedAccountMeta: borsh::BorshDeserialize,
        CompressedAccountVariant: borsh::BorshDeserialize,
        Vec<Vec<u8>>: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                meta: borsh::BorshDeserialize::deserialize_reader(reader)?,
                data: borsh::BorshDeserialize::deserialize_reader(reader)?,
                seeds: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl borsh::ser::BorshSerialize for CompressedAccountData
    where
        light_sdk_types::instruction::account_meta::CompressedAccountMeta:
            borsh::ser::BorshSerialize,
        CompressedAccountVariant: borsh::ser::BorshSerialize,
        Vec<Vec<u8>>: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.meta, writer)?;
            borsh::BorshSerialize::serialize(&self.data, writer)?;
            borsh::BorshSerialize::serialize(&self.seeds, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for CompressedAccountData {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(
                    ::alloc::boxed::box_new([
                        "Client-side data structure for passing compressed accounts"
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
                                        name: "meta".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::get_full_path(),
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
                                    anchor_lang::idl::types::IdlField {
                                        name: "seeds".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Vec(
                                            Box::new(anchor_lang::idl::types::IdlType::Bytes),
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
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) {
            if let Some(ty) =
                <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::create_type()
            {
                types
                    .insert(
                        <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::get_full_path(),
                        ty,
                    );
                <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::insert_types(
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
                let res = ::alloc::fmt::format(format_args!(
                    "{0}::{1}",
                    "anchor_compressible_user_derived::anchor_compressible_user_derived",
                    "CompressedAccountData",
                ));
                res
            })
        }
    }
    pub struct CreateCompressibleConfig<'info> {
        #[account(mut)]
        pub payer: Signer<'info>,
        /// The config PDA to be created
        #[account(mut, seeds = [b"compressible_config"], bump)]
        pub config: AccountInfo<'info>,
        /// The program's data account
        pub program_data: AccountInfo<'info>,
        /// The program's upgrade authority (must sign)
        pub authority: Signer<'info>,
        pub system_program: Program<'info, System>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, CreateCompressibleConfigBumps>
        for CreateCompressibleConfig<'info>
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
            __bumps: &mut CreateCompressibleConfigBumps,
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
            let system_program: anchor_lang::accounts::program::Program<System> =
                anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("system_program"))?;
            if !AsRef::<AccountInfo>::as_ref(&payer).is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("payer"));
            }
            let (__pda_address, __bump) =
                Pubkey::find_program_address(&[b"compressible_config"], &__program_id);
            __bumps.config = __bump;
            if config.key() != __pda_address {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintSeeds,
                )
                .with_account_name("config")
                .with_pubkeys((config.key(), __pda_address)));
            }
            if !&config.is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("config"));
            }
            Ok(CreateCompressibleConfig {
                payer,
                config,
                program_data,
                authority,
                system_program,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for CreateCompressibleConfig<'info>
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
    impl<'info> anchor_lang::ToAccountMetas for CreateCompressibleConfig<'info> {
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
    impl<'info> anchor_lang::AccountsExit<'info> for CreateCompressibleConfig<'info>
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
    pub struct CreateCompressibleConfigBumps {
        pub config: u8,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for CreateCompressibleConfigBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "CreateCompressibleConfigBumps",
                "config",
                &&self.config,
            )
        }
    }
    impl Default for CreateCompressibleConfigBumps {
        fn default() -> Self {
            CreateCompressibleConfigBumps { config: u8::MAX }
        }
    }
    impl<'info> anchor_lang::Bumps for CreateCompressibleConfig<'info>
    where
        'info: 'info,
    {
        type Bumps = CreateCompressibleConfigBumps;
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
    pub(crate) mod __client_accounts_create_compressible_config {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`CreateCompressibleConfig`].
        pub struct CreateCompressibleConfig {
            pub payer: Pubkey,
            ///The config PDA to be created
            pub config: Pubkey,
            ///The program's data account
            pub program_data: Pubkey,
            ///The program's upgrade authority (must sign)
            pub authority: Pubkey,
            pub system_program: Pubkey,
        }
        impl borsh::ser::BorshSerialize for CreateCompressibleConfig
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
        impl anchor_lang::idl::build::IdlBuild for CreateCompressibleConfig {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                        "Generated client accounts for [`CreateCompressibleConfig`].".into(),
                    ])),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(::alloc::boxed::box_new([
                                anchor_lang::idl::types::IdlField {
                                    name: "payer".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "config".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "The config PDA to be created".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "program_data".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "The program's data account".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "authority".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "The program's upgrade authority (must sign)".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "system_program".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                            ])),
                        )),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
            ) {
            }
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "anchor_compressible_user_derived::anchor_compressible_user_derived::__client_accounts_create_compressible_config",
                            "CreateCompressibleConfig",
                        ),
                    );
                    res
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CreateCompressibleConfig {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.payer, true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.config,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.program_data,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.authority,
                        true,
                    ),
                );
                account_metas.push(
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
    pub(crate) mod __cpi_client_accounts_create_compressible_config {
        use super::*;
        /// Generated CPI struct of the accounts for [`CreateCompressibleConfig`].
        pub struct CreateCompressibleConfig<'info> {
            pub payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The config PDA to be created
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The program's data account
            pub program_data: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The program's upgrade authority (must sign)
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for CreateCompressibleConfig<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.payer),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.config),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.program_data),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for CreateCompressibleConfig<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.payer));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.program_data,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos
            }
        }
    }
    impl<'info> CreateCompressibleConfig<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlAccount>,
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            <[_]>::into_vec(::alloc::boxed::box_new([
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "payer".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "The config PDA to be created".into(),
                        ])),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "program_data".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "The program's data account".into(),
                        ])),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "authority".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "The program's upgrade authority (must sign)".into(),
                        ])),
                        writable: false,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "system_program".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
            ]))
        }
    }
    pub struct UpdateCompressibleConfig<'info> {
        #[account(mut, seeds = [b"compressible_config"], bump)]
        pub config: AccountInfo<'info>,
        /// Must match the update authority stored in config
        pub authority: Signer<'info>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, UpdateCompressibleConfigBumps>
        for UpdateCompressibleConfig<'info>
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
            __bumps: &mut UpdateCompressibleConfigBumps,
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
            let (__pda_address, __bump) =
                Pubkey::find_program_address(&[b"compressible_config"], &__program_id);
            __bumps.config = __bump;
            if config.key() != __pda_address {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintSeeds,
                )
                .with_account_name("config")
                .with_pubkeys((config.key(), __pda_address)));
            }
            if !&config.is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("config"));
            }
            Ok(UpdateCompressibleConfig { config, authority })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateCompressibleConfig<'info>
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
    impl<'info> anchor_lang::ToAccountMetas for UpdateCompressibleConfig<'info> {
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
    impl<'info> anchor_lang::AccountsExit<'info> for UpdateCompressibleConfig<'info>
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
    pub struct UpdateCompressibleConfigBumps {
        pub config: u8,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UpdateCompressibleConfigBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "UpdateCompressibleConfigBumps",
                "config",
                &&self.config,
            )
        }
    }
    impl Default for UpdateCompressibleConfigBumps {
        fn default() -> Self {
            UpdateCompressibleConfigBumps { config: u8::MAX }
        }
    }
    impl<'info> anchor_lang::Bumps for UpdateCompressibleConfig<'info>
    where
        'info: 'info,
    {
        type Bumps = UpdateCompressibleConfigBumps;
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
    pub(crate) mod __client_accounts_update_compressible_config {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`UpdateCompressibleConfig`].
        pub struct UpdateCompressibleConfig {
            pub config: Pubkey,
            ///Must match the update authority stored in config
            pub authority: Pubkey,
        }
        impl borsh::ser::BorshSerialize for UpdateCompressibleConfig
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
        impl anchor_lang::idl::build::IdlBuild for UpdateCompressibleConfig {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                        "Generated client accounts for [`UpdateCompressibleConfig`].".into(),
                    ])),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(::alloc::boxed::box_new([
                                anchor_lang::idl::types::IdlField {
                                    name: "config".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "authority".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "Must match the update authority stored in config".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                            ])),
                        )),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
            ) {
            }
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "anchor_compressible_user_derived::anchor_compressible_user_derived::__client_accounts_update_compressible_config",
                            "UpdateCompressibleConfig",
                        ),
                    );
                    res
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for UpdateCompressibleConfig {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.config,
                    false,
                ));
                account_metas.push(
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
    pub(crate) mod __cpi_client_accounts_update_compressible_config {
        use super::*;
        /// Generated CPI struct of the accounts for [`UpdateCompressibleConfig`].
        pub struct UpdateCompressibleConfig<'info> {
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///Must match the update authority stored in config
            pub authority: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for UpdateCompressibleConfig<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.config),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.authority),
                        true,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateCompressibleConfig<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.authority,
                ));
                account_infos
            }
        }
    }
    impl<'info> UpdateCompressibleConfig<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlAccount>,
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            <[_]>::into_vec(::alloc::boxed::box_new([
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "authority".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Must match the update authority stored in config".into(),
                        ])),
                        writable: false,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
            ]))
        }
    }
    /// Create compressible config - only callable by program upgrade authority
    pub fn create_compression_config(
        ctx: Context<CreateCompressibleConfig>,
        compression_delay: u32,
        rent_recipient: Pubkey,
        address_space: Pubkey,
    ) -> Result<()> {
        light_sdk::compressible::create_compression_config_checked(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.program_data.to_account_info(),
            &rent_recipient,
            &address_space,
            compression_delay,
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        Ok(())
    }
    /// Update compressible config - only callable by config's update authority
    pub fn update_compression_config(
        ctx: Context<UpdateCompressibleConfig>,
        new_compression_delay: Option<u32>,
        new_rent_recipient: Option<Pubkey>,
        new_address_space: Option<Pubkey>,
        new_update_authority: Option<Pubkey>,
    ) -> Result<()> {
        light_sdk::compressible::update_config(
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            new_update_authority.as_ref(),
            new_rent_recipient.as_ref(),
            new_address_space.as_ref(),
            new_compression_delay,
            &crate::ID,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        Ok(())
    }
    pub struct DecompressMultiplePdas<'info> {
        #[account(mut)]
        pub fee_payer: Signer<'info>,
        #[account(mut)]
        pub rent_payer: Signer<'info>,
        pub system_program: Program<'info, System>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, DecompressMultiplePdasBumps>
        for DecompressMultiplePdas<'info>
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
            __bumps: &mut DecompressMultiplePdasBumps,
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
            let rent_payer: Signer = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("rent_payer"))?;
            let system_program: anchor_lang::accounts::program::Program<System> =
                anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("system_program"))?;
            if !AsRef::<AccountInfo>::as_ref(&fee_payer).is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("fee_payer"));
            }
            if !AsRef::<AccountInfo>::as_ref(&rent_payer).is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("rent_payer"));
            }
            Ok(DecompressMultiplePdas {
                fee_payer,
                rent_payer,
                system_program,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for DecompressMultiplePdas<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.fee_payer.to_account_infos());
            account_infos.extend(self.rent_payer.to_account_infos());
            account_infos.extend(self.system_program.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for DecompressMultiplePdas<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.fee_payer.to_account_metas(None));
            account_metas.extend(self.rent_payer.to_account_metas(None));
            account_metas.extend(self.system_program.to_account_metas(None));
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info> for DecompressMultiplePdas<'info>
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
            Ok(())
        }
    }
    pub struct DecompressMultiplePdasBumps {}
    #[automatically_derived]
    impl ::core::fmt::Debug for DecompressMultiplePdasBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "DecompressMultiplePdasBumps")
        }
    }
    impl Default for DecompressMultiplePdasBumps {
        fn default() -> Self {
            DecompressMultiplePdasBumps {}
        }
    }
    impl<'info> anchor_lang::Bumps for DecompressMultiplePdas<'info>
    where
        'info: 'info,
    {
        type Bumps = DecompressMultiplePdasBumps;
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
    pub(crate) mod __client_accounts_decompress_multiple_pdas {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`DecompressMultiplePdas`].
        pub struct DecompressMultiplePdas {
            pub fee_payer: Pubkey,
            pub rent_payer: Pubkey,
            pub system_program: Pubkey,
        }
        impl borsh::ser::BorshSerialize for DecompressMultiplePdas
        where
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
            Pubkey: borsh::ser::BorshSerialize,
        {
            fn serialize<W: borsh::maybestd::io::Write>(
                &self,
                writer: &mut W,
            ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
                borsh::BorshSerialize::serialize(&self.fee_payer, writer)?;
                borsh::BorshSerialize::serialize(&self.rent_payer, writer)?;
                borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for DecompressMultiplePdas {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                        "Generated client accounts for [`DecompressMultiplePdas`].".into(),
                    ])),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(::alloc::boxed::box_new([
                                anchor_lang::idl::types::IdlField {
                                    name: "fee_payer".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "rent_payer".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "system_program".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                            ])),
                        )),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
            ) {
            }
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "anchor_compressible_user_derived::anchor_compressible_user_derived::__client_accounts_decompress_multiple_pdas",
                            "DecompressMultiplePdas",
                        ),
                    );
                    res
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for DecompressMultiplePdas {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.fee_payer,
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.rent_payer,
                    true,
                ));
                account_metas.push(
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
    pub(crate) mod __cpi_client_accounts_decompress_multiple_pdas {
        use super::*;
        /// Generated CPI struct of the accounts for [`DecompressMultiplePdas`].
        pub struct DecompressMultiplePdas<'info> {
            pub fee_payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub rent_payer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for DecompressMultiplePdas<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.fee_payer),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.rent_payer),
                    true,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for DecompressMultiplePdas<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.fee_payer,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.rent_payer,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos
            }
        }
    }
    impl<'info> DecompressMultiplePdas<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlAccount>,
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            <[_]>::into_vec(::alloc::boxed::box_new([
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "fee_payer".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "rent_payer".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "system_program".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
            ]))
        }
    }
    /// Decompresses multiple compressed PDAs of any supported account type in a single transaction
    pub fn decompress_multiple_pdas<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressMultiplePdas<'info>>,
        proof: light_sdk::instruction::ValidityProof,
        compressed_accounts: Vec<CompressedAccountData>,
        bumps: Vec<u8>,
        system_accounts_offset: u8,
    ) -> Result<()> {
        let pda_accounts_end = system_accounts_offset as usize;
        let pda_accounts = &ctx.remaining_accounts[..pda_accounts_end];
        if pda_accounts.len() != compressed_accounts.len() || pda_accounts.len() != bumps.len() {
            return Err(anchor_lang::error::Error::from(
                anchor_lang::error::AnchorError {
                    error_name: ErrorCode::InvalidAccountCount.name(),
                    error_code_number: ErrorCode::InvalidAccountCount.into(),
                    error_msg: ErrorCode::InvalidAccountCount.to_string(),
                    error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                        anchor_lang::error::Source {
                            filename: "program-tests/anchor-compressible-user-derived/src/lib.rs",
                            line: 16u32,
                        },
                    )),
                    compared_values: None,
                },
            ));
        }
        let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
            &ctx.accounts.fee_payer,
            &ctx.remaining_accounts[system_accounts_offset as usize..],
            LIGHT_CPI_SIGNER,
        );
        let mut light_accounts = Vec::new();
        let mut pda_account_refs = Vec::new();
        let mut signer_seeds_storage = Vec::new();
        for (i, (compressed_data, bump)) in compressed_accounts
            .into_iter()
            .zip(bumps.iter())
            .enumerate()
        {
            let unified_account = match compressed_data.data {
                CompressedAccountVariant::UserRecord(data) => {
                    CompressedAccountVariant::UserRecord(data)
                }
                CompressedAccountVariant::GameSession(data) => {
                    CompressedAccountVariant::GameSession(data)
                }
            };
            let light_account =
                light_sdk::account::LightAccount::<'_, CompressedAccountVariant>::new_mut(
                    &crate::ID,
                    &compressed_data.meta,
                    unified_account.clone(),
                )
                .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
            let seeds = match &unified_account {
                CompressedAccountVariant::UserRecord(data) => {
                    let mut seeds = compressed_data.seeds.clone();
                    seeds.push(<[_]>::into_vec(::alloc::boxed::box_new([*bump])));
                    seeds
                }
                CompressedAccountVariant::GameSession(data) => {
                    let mut seeds = compressed_data.seeds.clone();
                    seeds.push(<[_]>::into_vec(::alloc::boxed::box_new([*bump])));
                    seeds
                }
            };
            signer_seeds_storage.push(seeds);
            light_accounts.push(light_account);
            pda_account_refs.push(&pda_accounts[i]);
        }
        let signer_seeds_refs: Vec<Vec<&[u8]>> = signer_seeds_storage
            .iter()
            .map(|seeds| seeds.iter().map(|s| s.as_slice()).collect())
            .collect();
        let signer_seeds_slices: Vec<&[&[u8]]> = signer_seeds_refs
            .iter()
            .map(|seeds| seeds.as_slice())
            .collect();
        light_sdk::compressible::decompress_multiple_idempotent::<CompressedAccountVariant>(
            &pda_account_refs,
            light_accounts,
            &signer_seeds_slices,
            proof,
            cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_payer,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        Ok(())
    }
    #[repr(u32)]
    pub enum ErrorCode {
        InvalidAccountCount,
        InvalidRentRecipient,
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
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ErrorCode {
        #[inline]
        fn clone(&self) -> ErrorCode {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for ErrorCode {}
    impl ErrorCode {
        /// Gets the name of this [#enum_name].
        pub fn name(&self) -> String {
            match self {
                ErrorCode::InvalidAccountCount => "InvalidAccountCount".to_string(),
                ErrorCode::InvalidRentRecipient => "InvalidRentRecipient".to_string(),
            }
        }
    }
    impl From<ErrorCode> for u32 {
        fn from(e: ErrorCode) -> u32 {
            e as u32 + anchor_lang::error::ERROR_CODE_OFFSET
        }
    }
    impl From<ErrorCode> for anchor_lang::error::Error {
        fn from(error_code: ErrorCode) -> anchor_lang::error::Error {
            anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                error_name: error_code.name(),
                error_code_number: error_code.into(),
                error_msg: error_code.to_string(),
                error_origin: None,
                compared_values: None,
            })
        }
    }
    impl std::fmt::Display for ErrorCode {
        fn fmt(
            &self,
            fmt: &mut std::fmt::Formatter<'_>,
        ) -> std::result::Result<(), std::fmt::Error> {
            match self {
                ErrorCode::InvalidAccountCount => fmt.write_fmt(format_args!(
                    "Invalid account count: PDAs and compressed accounts must match",
                )),
                ErrorCode::InvalidRentRecipient => {
                    fmt.write_fmt(format_args!("Rent recipient does not match config"))
                }
            }
        }
    }
    pub struct CompressUserRecord<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut, seeds = [b"user_record", user.key().as_ref()], bump)]
        pub pda_account: Account<'info, UserRecord>,
        pub system_program: Program<'info, System>,
        /// The global config account
        #[account(seeds = [b"compressible_config"], bump)]
        pub config: AccountInfo<'info>,
        /// Rent recipient - validated against config
        pub rent_recipient: AccountInfo<'info>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, CompressUserRecordBumps> for CompressUserRecord<'info>
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
            __bumps: &mut CompressUserRecordBumps,
            __reallocs: &mut std::collections::BTreeSet<
                anchor_lang::solana_program::pubkey::Pubkey,
            >,
        ) -> anchor_lang::Result<Self> {
            let user: Signer = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("user"))?;
            let pda_account: anchor_lang::accounts::account::Account<UserRecord> =
                anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("pda_account"))?;
            let system_program: anchor_lang::accounts::program::Program<System> =
                anchor_lang::Accounts::try_accounts(
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
            let rent_recipient: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("rent_recipient"))?;
            if !AsRef::<AccountInfo>::as_ref(&user).is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("user"));
            }
            let (__pda_address, __bump) =
                Pubkey::find_program_address(&[b"user_record", user.key().as_ref()], &__program_id);
            __bumps.pda_account = __bump;
            if pda_account.key() != __pda_address {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintSeeds,
                )
                .with_account_name("pda_account")
                .with_pubkeys((pda_account.key(), __pda_address)));
            }
            if !AsRef::<AccountInfo>::as_ref(&pda_account).is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("pda_account"));
            }
            let (__pda_address, __bump) =
                Pubkey::find_program_address(&[b"compressible_config"], &__program_id);
            __bumps.config = __bump;
            if config.key() != __pda_address {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintSeeds,
                )
                .with_account_name("config")
                .with_pubkeys((config.key(), __pda_address)));
            }
            Ok(CompressUserRecord {
                user,
                pda_account,
                system_program,
                config,
                rent_recipient,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for CompressUserRecord<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.user.to_account_infos());
            account_infos.extend(self.pda_account.to_account_infos());
            account_infos.extend(self.system_program.to_account_infos());
            account_infos.extend(self.config.to_account_infos());
            account_infos.extend(self.rent_recipient.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for CompressUserRecord<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.user.to_account_metas(None));
            account_metas.extend(self.pda_account.to_account_metas(None));
            account_metas.extend(self.system_program.to_account_metas(None));
            account_metas.extend(self.config.to_account_metas(None));
            account_metas.extend(self.rent_recipient.to_account_metas(None));
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info> for CompressUserRecord<'info>
    where
        'info: 'info,
    {
        fn exit(
            &self,
            program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        ) -> anchor_lang::Result<()> {
            anchor_lang::AccountsExit::exit(&self.user, program_id)
                .map_err(|e| e.with_account_name("user"))?;
            anchor_lang::AccountsExit::exit(&self.pda_account, program_id)
                .map_err(|e| e.with_account_name("pda_account"))?;
            Ok(())
        }
    }
    pub struct CompressUserRecordBumps {
        pub pda_account: u8,
        pub config: u8,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for CompressUserRecordBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "CompressUserRecordBumps",
                "pda_account",
                &self.pda_account,
                "config",
                &&self.config,
            )
        }
    }
    impl Default for CompressUserRecordBumps {
        fn default() -> Self {
            CompressUserRecordBumps {
                pda_account: u8::MAX,
                config: u8::MAX,
            }
        }
    }
    impl<'info> anchor_lang::Bumps for CompressUserRecord<'info>
    where
        'info: 'info,
    {
        type Bumps = CompressUserRecordBumps;
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
    pub(crate) mod __client_accounts_compress_user_record {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`CompressUserRecord`].
        pub struct CompressUserRecord {
            pub user: Pubkey,
            pub pda_account: Pubkey,
            pub system_program: Pubkey,
            ///The global config account
            pub config: Pubkey,
            ///Rent recipient - validated against config
            pub rent_recipient: Pubkey,
        }
        impl borsh::ser::BorshSerialize for CompressUserRecord
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
                borsh::BorshSerialize::serialize(&self.user, writer)?;
                borsh::BorshSerialize::serialize(&self.pda_account, writer)?;
                borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                borsh::BorshSerialize::serialize(&self.config, writer)?;
                borsh::BorshSerialize::serialize(&self.rent_recipient, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for CompressUserRecord {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                        "Generated client accounts for [`CompressUserRecord`].".into(),
                    ])),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(::alloc::boxed::box_new([
                                anchor_lang::idl::types::IdlField {
                                    name: "user".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "pda_account".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "system_program".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "config".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "The global config account".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "rent_recipient".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "Rent recipient - validated against config".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                            ])),
                        )),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
            ) {
            }
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "anchor_compressible_user_derived::anchor_compressible_user_derived::__client_accounts_compress_user_record",
                            "CompressUserRecord",
                        ),
                    );
                    res
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CompressUserRecord {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.user, true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.pda_account,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.system_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.config,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.rent_recipient,
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
    pub(crate) mod __cpi_client_accounts_compress_user_record {
        use super::*;
        /// Generated CPI struct of the accounts for [`CompressUserRecord`].
        pub struct CompressUserRecord<'info> {
            pub user: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub pda_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The global config account
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///Rent recipient - validated against config
            pub rent_recipient: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for CompressUserRecord<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.user),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.pda_account),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.config),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.rent_recipient),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for CompressUserRecord<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.user));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.pda_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.rent_recipient,
                ));
                account_infos
            }
        }
    }
    impl<'info> CompressUserRecord<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlAccount>,
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
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
            <[_]>::into_vec(::alloc::boxed::box_new([
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "user".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "pda_account".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "system_program".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "The global config account".into(),
                        ])),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "rent_recipient".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Rent recipient - validated against config".into(),
                        ])),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
            ]))
        }
    }
    /// Compresses a #struct_name PDA using config values
    pub fn compress_user_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressUserRecord<'info>>,
        proof: light_sdk::instruction::ValidityProof,
        compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
    ) -> Result<()> {
        let config = light_sdk::compressible::CompressibleConfig::load_checked(
            &ctx.accounts.config,
            &crate::ID,
        )
        .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return Err(anchor_lang::error::Error::from(
                anchor_lang::error::AnchorError {
                    error_name: ErrorCode::InvalidRentRecipient.name(),
                    error_code_number: ErrorCode::InvalidRentRecipient.into(),
                    error_msg: ErrorCode::InvalidRentRecipient.to_string(),
                    error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                        anchor_lang::error::Source {
                            filename: "program-tests/anchor-compressible-user-derived/src/lib.rs",
                            line: 16u32,
                        },
                    )),
                    compared_values: None,
                },
            ));
        }
        let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
            &ctx.accounts.user,
            &ctx.remaining_accounts[..],
            LIGHT_CPI_SIGNER,
        );
        light_sdk::compressible::compress_pda::<UserRecord>(
            &ctx.accounts.pda_account.to_account_info(),
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        Ok(())
    }
    pub struct CompressGameSession<'info> {
        #[account(mut)]
        pub user: Signer<'info>,
        #[account(mut, seeds = [b"user_record", user.key().as_ref()], bump)]
        pub pda_account: Account<'info, GameSession>,
        pub system_program: Program<'info, System>,
        /// The global config account
        #[account(seeds = [b"compressible_config"], bump)]
        pub config: AccountInfo<'info>,
        /// Rent recipient - validated against config
        pub rent_recipient: AccountInfo<'info>,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::Accounts<'info, CompressGameSessionBumps> for CompressGameSession<'info>
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
            __bumps: &mut CompressGameSessionBumps,
            __reallocs: &mut std::collections::BTreeSet<
                anchor_lang::solana_program::pubkey::Pubkey,
            >,
        ) -> anchor_lang::Result<Self> {
            let user: Signer = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("user"))?;
            let pda_account: anchor_lang::accounts::account::Account<GameSession> =
                anchor_lang::Accounts::try_accounts(
                    __program_id,
                    __accounts,
                    __ix_data,
                    __bumps,
                    __reallocs,
                )
                .map_err(|e| e.with_account_name("pda_account"))?;
            let system_program: anchor_lang::accounts::program::Program<System> =
                anchor_lang::Accounts::try_accounts(
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
            let rent_recipient: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("rent_recipient"))?;
            if !AsRef::<AccountInfo>::as_ref(&user).is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("user"));
            }
            let (__pda_address, __bump) =
                Pubkey::find_program_address(&[b"user_record", user.key().as_ref()], &__program_id);
            __bumps.pda_account = __bump;
            if pda_account.key() != __pda_address {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintSeeds,
                )
                .with_account_name("pda_account")
                .with_pubkeys((pda_account.key(), __pda_address)));
            }
            if !AsRef::<AccountInfo>::as_ref(&pda_account).is_writable {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintMut,
                )
                .with_account_name("pda_account"));
            }
            let (__pda_address, __bump) =
                Pubkey::find_program_address(&[b"compressible_config"], &__program_id);
            __bumps.config = __bump;
            if config.key() != __pda_address {
                return Err(anchor_lang::error::Error::from(
                    anchor_lang::error::ErrorCode::ConstraintSeeds,
                )
                .with_account_name("config")
                .with_pubkeys((config.key(), __pda_address)));
            }
            Ok(CompressGameSession {
                user,
                pda_account,
                system_program,
                config,
                rent_recipient,
            })
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for CompressGameSession<'info>
    where
        'info: 'info,
    {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos.extend(self.user.to_account_infos());
            account_infos.extend(self.pda_account.to_account_infos());
            account_infos.extend(self.system_program.to_account_infos());
            account_infos.extend(self.config.to_account_infos());
            account_infos.extend(self.rent_recipient.to_account_infos());
            account_infos
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for CompressGameSession<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas.extend(self.user.to_account_metas(None));
            account_metas.extend(self.pda_account.to_account_metas(None));
            account_metas.extend(self.system_program.to_account_metas(None));
            account_metas.extend(self.config.to_account_metas(None));
            account_metas.extend(self.rent_recipient.to_account_metas(None));
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::AccountsExit<'info> for CompressGameSession<'info>
    where
        'info: 'info,
    {
        fn exit(
            &self,
            program_id: &anchor_lang::solana_program::pubkey::Pubkey,
        ) -> anchor_lang::Result<()> {
            anchor_lang::AccountsExit::exit(&self.user, program_id)
                .map_err(|e| e.with_account_name("user"))?;
            anchor_lang::AccountsExit::exit(&self.pda_account, program_id)
                .map_err(|e| e.with_account_name("pda_account"))?;
            Ok(())
        }
    }
    pub struct CompressGameSessionBumps {
        pub pda_account: u8,
        pub config: u8,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for CompressGameSessionBumps {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "CompressGameSessionBumps",
                "pda_account",
                &self.pda_account,
                "config",
                &&self.config,
            )
        }
    }
    impl Default for CompressGameSessionBumps {
        fn default() -> Self {
            CompressGameSessionBumps {
                pda_account: u8::MAX,
                config: u8::MAX,
            }
        }
    }
    impl<'info> anchor_lang::Bumps for CompressGameSession<'info>
    where
        'info: 'info,
    {
        type Bumps = CompressGameSessionBumps;
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
    pub(crate) mod __client_accounts_compress_game_session {
        use super::*;
        use anchor_lang::prelude::borsh;
        /// Generated client accounts for [`CompressGameSession`].
        pub struct CompressGameSession {
            pub user: Pubkey,
            pub pda_account: Pubkey,
            pub system_program: Pubkey,
            ///The global config account
            pub config: Pubkey,
            ///Rent recipient - validated against config
            pub rent_recipient: Pubkey,
        }
        impl borsh::ser::BorshSerialize for CompressGameSession
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
                borsh::BorshSerialize::serialize(&self.user, writer)?;
                borsh::BorshSerialize::serialize(&self.pda_account, writer)?;
                borsh::BorshSerialize::serialize(&self.system_program, writer)?;
                borsh::BorshSerialize::serialize(&self.config, writer)?;
                borsh::BorshSerialize::serialize(&self.rent_recipient, writer)?;
                Ok(())
            }
        }
        impl anchor_lang::idl::build::IdlBuild for CompressGameSession {
            fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
                Some(anchor_lang::idl::types::IdlTypeDef {
                    name: Self::get_full_path(),
                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                        "Generated client accounts for [`CompressGameSession`].".into(),
                    ])),
                    serialization: anchor_lang::idl::types::IdlSerialization::default(),
                    repr: None,
                    generics: ::alloc::vec::Vec::new(),
                    ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                        fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                            <[_]>::into_vec(::alloc::boxed::box_new([
                                anchor_lang::idl::types::IdlField {
                                    name: "user".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "pda_account".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "system_program".into(),
                                    docs: ::alloc::vec::Vec::new(),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "config".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "The global config account".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                                anchor_lang::idl::types::IdlField {
                                    name: "rent_recipient".into(),
                                    docs: <[_]>::into_vec(::alloc::boxed::box_new([
                                        "Rent recipient - validated against config".into(),
                                    ])),
                                    ty: anchor_lang::idl::types::IdlType::Pubkey,
                                },
                            ])),
                        )),
                    },
                })
            }
            fn insert_types(
                types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
            ) {
            }
            fn get_full_path() -> String {
                ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "anchor_compressible_user_derived::anchor_compressible_user_derived::__client_accounts_compress_game_session",
                            "CompressGameSession",
                        ),
                    );
                    res
                })
            }
        }
        #[automatically_derived]
        impl anchor_lang::ToAccountMetas for CompressGameSession {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.user, true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    self.pda_account,
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.system_program,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.config,
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.rent_recipient,
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
    pub(crate) mod __cpi_client_accounts_compress_game_session {
        use super::*;
        /// Generated CPI struct of the accounts for [`CompressGameSession`].
        pub struct CompressGameSession<'info> {
            pub user: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub pda_account: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            pub system_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///The global config account
            pub config: anchor_lang::solana_program::account_info::AccountInfo<'info>,
            ///Rent recipient - validated against config
            pub rent_recipient: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountMetas for CompressGameSession<'info> {
            fn to_account_metas(
                &self,
                is_signer: Option<bool>,
            ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
                let mut account_metas = ::alloc::vec::Vec::new();
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.user),
                    true,
                ));
                account_metas.push(anchor_lang::solana_program::instruction::AccountMeta::new(
                    anchor_lang::Key::key(&self.pda_account),
                    false,
                ));
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.system_program),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.config),
                        false,
                    ),
                );
                account_metas.push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.rent_recipient),
                        false,
                    ),
                );
                account_metas
            }
        }
        #[automatically_derived]
        impl<'info> anchor_lang::ToAccountInfos<'info> for CompressGameSession<'info> {
            fn to_account_infos(
                &self,
            ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
                let mut account_infos = ::alloc::vec::Vec::new();
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.user));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.pda_account,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.system_program,
                ));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.config));
                account_infos.extend(anchor_lang::ToAccountInfos::to_account_infos(
                    &self.rent_recipient,
                ));
                account_infos
            }
        }
    }
    impl<'info> CompressGameSession<'info> {
        pub fn __anchor_private_gen_idl_accounts(
            accounts: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlAccount>,
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) -> Vec<anchor_lang::idl::types::IdlInstructionAccountItem> {
            if let Some(ty) = <GameSession>::create_type() {
                let account = anchor_lang::idl::types::IdlAccount {
                    name: ty.name.clone(),
                    discriminator: GameSession::DISCRIMINATOR.into(),
                };
                accounts.insert(account.name.clone(), account);
                types.insert(ty.name.clone(), ty);
                <GameSession>::insert_types(types);
            }
            <[_]>::into_vec(::alloc::boxed::box_new([
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "user".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: true,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "pda_account".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: true,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "system_program".into(),
                        docs: ::alloc::vec::Vec::new(),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "config".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "The global config account".into(),
                        ])),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
                anchor_lang::idl::types::IdlInstructionAccountItem::Single(
                    anchor_lang::idl::types::IdlInstructionAccount {
                        name: "rent_recipient".into(),
                        docs: <[_]>::into_vec(::alloc::boxed::box_new([
                            "Rent recipient - validated against config".into(),
                        ])),
                        writable: false,
                        signer: false,
                        optional: false,
                        address: None,
                        pda: None,
                        relations: ::alloc::vec::Vec::new(),
                    },
                ),
            ]))
        }
    }
    /// Compresses a #struct_name PDA using config values
    pub fn compress_game_session<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressGameSession<'info>>,
        proof: light_sdk::instruction::ValidityProof,
        compressed_account_meta: light_sdk_types::instruction::account_meta::CompressedAccountMeta,
    ) -> Result<()> {
        let config = light_sdk::compressible::CompressibleConfig::load_checked(
            &ctx.accounts.config,
            &crate::ID,
        )
        .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
        if ctx.accounts.rent_recipient.key() != config.rent_recipient {
            return Err(anchor_lang::error::Error::from(
                anchor_lang::error::AnchorError {
                    error_name: ErrorCode::InvalidRentRecipient.name(),
                    error_code_number: ErrorCode::InvalidRentRecipient.into(),
                    error_msg: ErrorCode::InvalidRentRecipient.to_string(),
                    error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                        anchor_lang::error::Source {
                            filename: "program-tests/anchor-compressible-user-derived/src/lib.rs",
                            line: 16u32,
                        },
                    )),
                    compared_values: None,
                },
            ));
        }
        let cpi_accounts = light_sdk::cpi::CpiAccounts::new(
            &ctx.accounts.user,
            &ctx.remaining_accounts[..],
            LIGHT_CPI_SIGNER,
        );
        light_sdk::compressible::compress_pda::<GameSession>(
            &ctx.accounts.pda_account.to_account_info(),
            &compressed_account_meta,
            proof,
            cpi_accounts,
            &crate::ID,
            &ctx.accounts.rent_recipient,
            &config.compression_delay,
        )
        .map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;
        Ok(())
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
    pub struct CreateCompressionConfig {
        pub compression_delay: u32,
        pub rent_recipient: Pubkey,
        pub address_space: Pubkey,
    }
    impl borsh::ser::BorshSerialize for CreateCompressionConfig
    where
        u32: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
        Pubkey: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.compression_delay, writer)?;
            borsh::BorshSerialize::serialize(&self.rent_recipient, writer)?;
            borsh::BorshSerialize::serialize(&self.address_space, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for CreateCompressionConfig {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(::alloc::boxed::box_new(["Instruction.".into()])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                        <[_]>::into_vec(::alloc::boxed::box_new([
                            anchor_lang::idl::types::IdlField {
                                name: "compression_delay".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::U32,
                            },
                            anchor_lang::idl::types::IdlField {
                                name: "rent_recipient".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                            },
                            anchor_lang::idl::types::IdlField {
                                name: "address_space".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::Pubkey,
                            },
                        ])),
                    )),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) {
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!(
                    "{0}::{1}",
                    "anchor_compressible_user_derived::instruction", "CreateCompressionConfig",
                ));
                res
            })
        }
    }
    impl borsh::de::BorshDeserialize for CreateCompressionConfig
    where
        u32: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
        Pubkey: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                compression_delay: borsh::BorshDeserialize::deserialize_reader(reader)?,
                rent_recipient: borsh::BorshDeserialize::deserialize_reader(reader)?,
                address_space: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for CreateCompressionConfig {
        const DISCRIMINATOR: &'static [u8] = &[141, 78, 142, 238, 145, 47, 224, 57];
    }
    impl anchor_lang::InstructionData for CreateCompressionConfig {}
    impl anchor_lang::Owner for CreateCompressionConfig {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct UpdateCompressionConfig {
        pub new_compression_delay: Option<u32>,
        pub new_rent_recipient: Option<Pubkey>,
        pub new_address_space: Option<Pubkey>,
        pub new_update_authority: Option<Pubkey>,
    }
    impl borsh::ser::BorshSerialize for UpdateCompressionConfig
    where
        Option<u32>: borsh::ser::BorshSerialize,
        Option<Pubkey>: borsh::ser::BorshSerialize,
        Option<Pubkey>: borsh::ser::BorshSerialize,
        Option<Pubkey>: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.new_compression_delay, writer)?;
            borsh::BorshSerialize::serialize(&self.new_rent_recipient, writer)?;
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
                    fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                        <[_]>::into_vec(::alloc::boxed::box_new([
                            anchor_lang::idl::types::IdlField {
                                name: "new_compression_delay".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::Option(Box::new(
                                    anchor_lang::idl::types::IdlType::U32,
                                )),
                            },
                            anchor_lang::idl::types::IdlField {
                                name: "new_rent_recipient".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::Option(Box::new(
                                    anchor_lang::idl::types::IdlType::Pubkey,
                                )),
                            },
                            anchor_lang::idl::types::IdlField {
                                name: "new_address_space".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::Option(Box::new(
                                    anchor_lang::idl::types::IdlType::Pubkey,
                                )),
                            },
                            anchor_lang::idl::types::IdlField {
                                name: "new_update_authority".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::Option(Box::new(
                                    anchor_lang::idl::types::IdlType::Pubkey,
                                )),
                            },
                        ])),
                    )),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) {
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!(
                    "{0}::{1}",
                    "anchor_compressible_user_derived::instruction", "UpdateCompressionConfig",
                ));
                res
            })
        }
    }
    impl borsh::de::BorshDeserialize for UpdateCompressionConfig
    where
        Option<u32>: borsh::BorshDeserialize,
        Option<Pubkey>: borsh::BorshDeserialize,
        Option<Pubkey>: borsh::BorshDeserialize,
        Option<Pubkey>: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                new_compression_delay: borsh::BorshDeserialize::deserialize_reader(reader)?,
                new_rent_recipient: borsh::BorshDeserialize::deserialize_reader(reader)?,
                new_address_space: borsh::BorshDeserialize::deserialize_reader(reader)?,
                new_update_authority: borsh::BorshDeserialize::deserialize_reader(reader)?,
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
    /// Instruction.
    pub struct DecompressMultiplePdas {
        pub proof: light_sdk::instruction::ValidityProof,
        pub compressed_accounts: Vec<CompressedAccountData>,
        pub bumps: Vec<u8>,
        pub system_accounts_offset: u8,
    }
    impl borsh::ser::BorshSerialize for DecompressMultiplePdas
    where
        light_sdk::instruction::ValidityProof: borsh::ser::BorshSerialize,
        Vec<CompressedAccountData>: borsh::ser::BorshSerialize,
        Vec<u8>: borsh::ser::BorshSerialize,
        u8: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.compressed_accounts, writer)?;
            borsh::BorshSerialize::serialize(&self.bumps, writer)?;
            borsh::BorshSerialize::serialize(&self.system_accounts_offset, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for DecompressMultiplePdas {
        fn create_type() -> Option<anchor_lang::idl::types::IdlTypeDef> {
            Some(anchor_lang::idl::types::IdlTypeDef {
                name: Self::get_full_path(),
                docs: <[_]>::into_vec(::alloc::boxed::box_new(["Instruction.".into()])),
                serialization: anchor_lang::idl::types::IdlSerialization::default(),
                repr: None,
                generics: ::alloc::vec::Vec::new(),
                ty: anchor_lang::idl::types::IdlTypeDefTy::Struct {
                    fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                        <[_]>::into_vec(::alloc::boxed::box_new([
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
                                ty: anchor_lang::idl::types::IdlType::Vec(Box::new(
                                    anchor_lang::idl::types::IdlType::Defined {
                                        name: <CompressedAccountData>::get_full_path(),
                                        generics: ::alloc::vec::Vec::new(),
                                    },
                                )),
                            },
                            anchor_lang::idl::types::IdlField {
                                name: "bumps".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::Bytes,
                            },
                            anchor_lang::idl::types::IdlField {
                                name: "system_accounts_offset".into(),
                                docs: ::alloc::vec::Vec::new(),
                                ty: anchor_lang::idl::types::IdlType::U8,
                            },
                        ])),
                    )),
                },
            })
        }
        fn insert_types(
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) {
            if let Some(ty) = <light_sdk::instruction::ValidityProof>::create_type() {
                types.insert(<light_sdk::instruction::ValidityProof>::get_full_path(), ty);
                <light_sdk::instruction::ValidityProof>::insert_types(types);
            }
            if let Some(ty) = <CompressedAccountData>::create_type() {
                types.insert(<CompressedAccountData>::get_full_path(), ty);
                <CompressedAccountData>::insert_types(types);
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!(
                    "{0}::{1}",
                    "anchor_compressible_user_derived::instruction", "DecompressMultiplePdas",
                ));
                res
            })
        }
    }
    impl borsh::de::BorshDeserialize for DecompressMultiplePdas
    where
        light_sdk::instruction::ValidityProof: borsh::BorshDeserialize,
        Vec<CompressedAccountData>: borsh::BorshDeserialize,
        Vec<u8>: borsh::BorshDeserialize,
        u8: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                compressed_accounts: borsh::BorshDeserialize::deserialize_reader(reader)?,
                bumps: borsh::BorshDeserialize::deserialize_reader(reader)?,
                system_accounts_offset: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for DecompressMultiplePdas {
        const DISCRIMINATOR: &'static [u8] = &[94, 169, 150, 235, 138, 51, 254, 223];
    }
    impl anchor_lang::InstructionData for DecompressMultiplePdas {}
    impl anchor_lang::Owner for DecompressMultiplePdas {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct CompressUserRecord {
        pub proof: light_sdk::instruction::ValidityProof,
        pub compressed_account_meta:
            light_sdk_types::instruction::account_meta::CompressedAccountMeta,
    }
    impl borsh::ser::BorshSerialize for CompressUserRecord
    where
        light_sdk::instruction::ValidityProof: borsh::ser::BorshSerialize,
        light_sdk_types::instruction::account_meta::CompressedAccountMeta:
            borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.compressed_account_meta, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for CompressUserRecord {
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
                                        name: "compressed_account_meta".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::get_full_path(),
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
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) {
            if let Some(ty) = <light_sdk::instruction::ValidityProof>::create_type() {
                types.insert(<light_sdk::instruction::ValidityProof>::get_full_path(), ty);
                <light_sdk::instruction::ValidityProof>::insert_types(types);
            }
            if let Some(ty) =
                <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::create_type()
            {
                types
                    .insert(
                        <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::get_full_path(),
                        ty,
                    );
                <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::insert_types(
                    types,
                );
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!(
                    "{0}::{1}",
                    "anchor_compressible_user_derived::instruction", "CompressUserRecord",
                ));
                res
            })
        }
    }
    impl borsh::de::BorshDeserialize for CompressUserRecord
    where
        light_sdk::instruction::ValidityProof: borsh::BorshDeserialize,
        light_sdk_types::instruction::account_meta::CompressedAccountMeta: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                compressed_account_meta: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for CompressUserRecord {
        const DISCRIMINATOR: &'static [u8] = &[121, 36, 116, 111, 233, 192, 60, 76];
    }
    impl anchor_lang::InstructionData for CompressUserRecord {}
    impl anchor_lang::Owner for CompressUserRecord {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct CompressGameSession {
        pub proof: light_sdk::instruction::ValidityProof,
        pub compressed_account_meta:
            light_sdk_types::instruction::account_meta::CompressedAccountMeta,
    }
    impl borsh::ser::BorshSerialize for CompressGameSession
    where
        light_sdk::instruction::ValidityProof: borsh::ser::BorshSerialize,
        light_sdk_types::instruction::account_meta::CompressedAccountMeta:
            borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.compressed_account_meta, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::idl::build::IdlBuild for CompressGameSession {
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
                                        name: "compressed_account_meta".into(),
                                        docs: ::alloc::vec::Vec::new(),
                                        ty: anchor_lang::idl::types::IdlType::Defined {
                                            name: <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::get_full_path(),
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
            types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
        ) {
            if let Some(ty) = <light_sdk::instruction::ValidityProof>::create_type() {
                types.insert(<light_sdk::instruction::ValidityProof>::get_full_path(), ty);
                <light_sdk::instruction::ValidityProof>::insert_types(types);
            }
            if let Some(ty) =
                <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::create_type()
            {
                types
                    .insert(
                        <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::get_full_path(),
                        ty,
                    );
                <light_sdk_types::instruction::account_meta::CompressedAccountMeta>::insert_types(
                    types,
                );
            }
        }
        fn get_full_path() -> String {
            ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!(
                    "{0}::{1}",
                    "anchor_compressible_user_derived::instruction", "CompressGameSession",
                ));
                res
            })
        }
    }
    impl borsh::de::BorshDeserialize for CompressGameSession
    where
        light_sdk::instruction::ValidityProof: borsh::BorshDeserialize,
        light_sdk_types::instruction::account_meta::CompressedAccountMeta: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                compressed_account_meta: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for CompressGameSession {
        const DISCRIMINATOR: &'static [u8] = &[200, 21, 38, 181, 112, 114, 198, 180];
    }
    impl anchor_lang::InstructionData for CompressGameSession {}
    impl anchor_lang::Owner for CompressGameSession {
        fn owner() -> Pubkey {
            ID
        }
    }
}
/// An Anchor generated module, providing a set of structs
/// mirroring the structs deriving `Accounts`, where each field is
/// a `Pubkey`. This is useful for specifying accounts for a client.
pub mod accounts {
    pub use crate::__client_accounts_compress_game_session::*;
    pub use crate::__client_accounts_compress_user_record::*;
    pub use crate::__client_accounts_create_compressible_config::*;
    pub use crate::__client_accounts_decompress_multiple_pdas::*;
    pub use crate::__client_accounts_update_compressible_config::*;
}
pub struct UserRecord {
    #[skip]
    pub compression_info: CompressionInfo,
    #[hash]
    pub owner: Pubkey,
    #[hash]
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}
impl borsh::ser::BorshSerialize for UserRecord
where
    CompressionInfo: borsh::ser::BorshSerialize,
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
                fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                    <[_]>::into_vec(::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlField {
                            name: "compression_info".into(),
                            docs: ::alloc::vec::Vec::new(),
                            ty: anchor_lang::idl::types::IdlType::Defined {
                                name: <CompressionInfo>::get_full_path(),
                                generics: ::alloc::vec::Vec::new(),
                            },
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
                    ])),
                )),
            },
        })
    }
    fn insert_types(
        types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
    ) {
        if let Some(ty) = <CompressionInfo>::create_type() {
            types.insert(<CompressionInfo>::get_full_path(), ty);
            <CompressionInfo>::insert_types(types);
        }
    }
    fn get_full_path() -> String {
        ::alloc::__export::must_use({
            let res = ::alloc::fmt::format(format_args!(
                "{0}::{1}",
                "anchor_compressible_user_derived", "UserRecord",
            ));
            res
        })
    }
}
impl borsh::de::BorshDeserialize for UserRecord
where
    CompressionInfo: borsh::BorshDeserialize,
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
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
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
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let given_disc = &buf[..UserRecord::DISCRIMINATOR.len()];
        if UserRecord::DISCRIMINATOR != given_disc {
            return Err(
                anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                    error_name: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.name(),
                    error_code_number: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                        .into(),
                    error_msg: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                        .to_string(),
                    error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                        anchor_lang::error::Source {
                            filename: "program-tests/anchor-compressible-user-derived/src/lib.rs",
                            line: 34u32,
                        },
                    )),
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
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
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
    fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
    }
}
impl ::light_hasher::DataHasher for UserRecord {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        #[cfg(debug_assertions)]
        {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                let debug_prints: Vec<[u8; 32]> = <[_]>::into_vec(::alloc::boxed::box_new([
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                        self.owner.as_ref(),
                    ),
                    self.name.hash_to_field_size()?,
                    self.score.to_byte_array()?,
                ]));
                {
                    ::std::io::_print(format_args!(
                        "DataHasher::hash inputs {0:?}\n",
                        debug_prints
                    ));
                };
            }
        }
        H::hashv(&[
            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.owner.as_ref())
                .as_slice(),
            self.name.hash_to_field_size()?.as_slice(),
            self.score.to_byte_array()?.as_slice(),
        ])
    }
}
impl LightDiscriminator for UserRecord {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [102, 153, 211, 164, 62, 220, 128, 15];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
    fn discriminator() -> [u8; 8] {
        Self::LIGHT_DISCRIMINATOR
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
impl anchor_lang::Space for UserRecord {
    const INIT_SPACE: usize =
        0 + <CompressionInfo as anchor_lang::Space>::INIT_SPACE + 32 + (4 + 32) + 8;
}
impl HasCompressionInfo for UserRecord {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }
    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}
pub struct GameSession {
    #[skip]
    pub compression_info: CompressionInfo,
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    #[hash]
    #[max_len(32)]
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}
impl borsh::ser::BorshSerialize for GameSession
where
    CompressionInfo: borsh::ser::BorshSerialize,
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
                fields: Some(anchor_lang::idl::types::IdlDefinedFields::Named(
                    <[_]>::into_vec(::alloc::boxed::box_new([
                        anchor_lang::idl::types::IdlField {
                            name: "compression_info".into(),
                            docs: ::alloc::vec::Vec::new(),
                            ty: anchor_lang::idl::types::IdlType::Defined {
                                name: <CompressionInfo>::get_full_path(),
                                generics: ::alloc::vec::Vec::new(),
                            },
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
                            ty: anchor_lang::idl::types::IdlType::Option(Box::new(
                                anchor_lang::idl::types::IdlType::U64,
                            )),
                        },
                        anchor_lang::idl::types::IdlField {
                            name: "score".into(),
                            docs: ::alloc::vec::Vec::new(),
                            ty: anchor_lang::idl::types::IdlType::U64,
                        },
                    ])),
                )),
            },
        })
    }
    fn insert_types(
        types: &mut std::collections::BTreeMap<String, anchor_lang::idl::types::IdlTypeDef>,
    ) {
        if let Some(ty) = <CompressionInfo>::create_type() {
            types.insert(<CompressionInfo>::get_full_path(), ty);
            <CompressionInfo>::insert_types(types);
        }
    }
    fn get_full_path() -> String {
        ::alloc::__export::must_use({
            let res = ::alloc::fmt::format(format_args!(
                "{0}::{1}",
                "anchor_compressible_user_derived", "GameSession",
            ));
            res
        })
    }
}
impl borsh::de::BorshDeserialize for GameSession
where
    CompressionInfo: borsh::BorshDeserialize,
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
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
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
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let given_disc = &buf[..GameSession::DISCRIMINATOR.len()];
        if GameSession::DISCRIMINATOR != given_disc {
            return Err(
                anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
                    error_name: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.name(),
                    error_code_number: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                        .into(),
                    error_msg: anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
                        .to_string(),
                    error_origin: Some(anchor_lang::error::ErrorOrigin::Source(
                        anchor_lang::error::Source {
                            filename: "program-tests/anchor-compressible-user-derived/src/lib.rs",
                            line: 57u32,
                        },
                    )),
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
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
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
        ::core::fmt::Formatter::debug_struct_fields_finish(f, "GameSession", names, values)
    }
}
impl ::light_hasher::to_byte_array::ToByteArray for GameSession {
    const NUM_FIELDS: usize = 7usize;
    fn to_byte_array(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError> {
        ::light_hasher::DataHasher::hash::<::light_hasher::Poseidon>(self)
    }
}
impl ::light_hasher::DataHasher for GameSession {
    fn hash<H>(&self) -> ::std::result::Result<[u8; 32], ::light_hasher::HasherError>
    where
        H: ::light_hasher::Hasher,
    {
        use ::light_hasher::hash_to_field_size::HashToFieldSize;
        use ::light_hasher::to_byte_array::ToByteArray;
        use ::light_hasher::DataHasher;
        use ::light_hasher::Hasher;
        #[cfg(debug_assertions)]
        {
            if std::env::var("RUST_BACKTRACE").is_ok() {
                let debug_prints: Vec<[u8; 32]> = <[_]>::into_vec(::alloc::boxed::box_new([
                    self.session_id.to_byte_array()?,
                    ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(
                        self.player.as_ref(),
                    ),
                    self.game_type.hash_to_field_size()?,
                    self.start_time.to_byte_array()?,
                    self.end_time.to_byte_array()?,
                    self.score.to_byte_array()?,
                ]));
                {
                    ::std::io::_print(format_args!(
                        "DataHasher::hash inputs {0:?}\n",
                        debug_prints
                    ));
                };
            }
        }
        H::hashv(&[
            self.session_id.to_byte_array()?.as_slice(),
            ::light_hasher::hash_to_field_size::hash_to_bn254_field_size_be(self.player.as_ref())
                .as_slice(),
            self.game_type.hash_to_field_size()?.as_slice(),
            self.start_time.to_byte_array()?.as_slice(),
            self.end_time.to_byte_array()?.as_slice(),
            self.score.to_byte_array()?.as_slice(),
        ])
    }
}
impl LightDiscriminator for GameSession {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [190, 139, 94, 145, 249, 130, 60, 133];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
    fn discriminator() -> [u8; 8] {
        Self::LIGHT_DISCRIMINATOR
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
impl anchor_lang::Space for GameSession {
    const INIT_SPACE: usize = 0
        + <CompressionInfo as anchor_lang::Space>::INIT_SPACE
        + 8
        + 32
        + (4 + 32)
        + 8
        + (1 + 8)
        + 8;
}
impl HasCompressionInfo for GameSession {
    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }
    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }
}
