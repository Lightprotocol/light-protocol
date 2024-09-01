#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use std::net::{Ipv4Addr, Ipv6Addr};
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    compressed_account::LightAccount, light_account, light_accounts, light_program,
    merkle_context::{PackedAddressMerkleContext, PackedMerkleContext},
};
use light_system_program::invoke::processor::CompressedProof;
/// The static program ID
pub static ID: anchor_lang::solana_program::pubkey::Pubkey = anchor_lang::solana_program::pubkey::Pubkey::new_from_array([
    103u8,
    186u8,
    35u8,
    56u8,
    74u8,
    39u8,
    56u8,
    29u8,
    249u8,
    31u8,
    113u8,
    30u8,
    131u8,
    88u8,
    230u8,
    29u8,
    11u8,
    82u8,
    34u8,
    3u8,
    26u8,
    209u8,
    19u8,
    177u8,
    99u8,
    37u8,
    129u8,
    210u8,
    155u8,
    223u8,
    251u8,
    241u8,
]);
/// Confirms that a given pubkey is equivalent to the program ID
pub fn check_id(id: &anchor_lang::solana_program::pubkey::Pubkey) -> bool {
    id == &ID
}
/// Returns the program ID
pub fn id() -> anchor_lang::solana_program::pubkey::Pubkey {
    ID
}
use self::name_service::*;
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let (program_id, accounts, instruction_data) = unsafe {
        ::solana_program::entrypoint::deserialize(input)
    };
    match entry(&program_id, &accounts, &instruction_data) {
        Ok(()) => ::solana_program::entrypoint::SUCCESS,
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
/// * Strip method identifier off the first 8 bytes of the instruction
///   data and invoke the identified method. The method identifier
///   is a variant of sighash. See docs.rs for `anchor_lang` for details.
/// * If the method identifier is an IDL identifier, execute the IDL
///   instructions, which are a special set of hardcoded instructions
///   baked into every Anchor program. Then exit.
/// * Otherwise, the method identifier is for a user defined
///   instruction, i.e., one of the methods in the user defined
///   `#[program]` module. Perform method dispatch, i.e., execute the
///   big match statement mapping method identifier to method handler
///   wrapper.
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
    if data.len() < 8 {
        return Err(anchor_lang::error::ErrorCode::InstructionMissing.into());
    }
    dispatch(program_id, accounts, data)
}
/// Module representing the program.
pub mod program {
    use super::*;
    /// Type representing the program.
    pub struct NameService;
    #[automatically_derived]
    impl ::core::clone::Clone for NameService {
        #[inline]
        fn clone(&self) -> NameService {
            NameService
        }
    }
    impl anchor_lang::Id for NameService {
        fn id() -> Pubkey {
            ID
        }
    }
}
/// Performs method dispatch.
///
/// Each method in an anchor program is uniquely defined by a namespace
/// and a rust identifier (i.e., the name given to the method). These
/// two pieces can be combined to creater a method identifier,
/// specifically, Anchor uses
///
/// Sha256("<namespace>:<rust-identifier>")[..8],
///
/// where the namespace can be one type. "global" for a
/// regular instruction.
///
/// With this 8 byte identifier, Anchor performs method dispatch,
/// matching the given 8 byte identifier to the associated method
/// handler, which leads to user defined code being eventually invoked.
fn dispatch<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    data: &[u8],
) -> anchor_lang::Result<()> {
    let mut ix_data: &[u8] = data;
    let sighash: [u8; 8] = {
        let mut sighash: [u8; 8] = [0; 8];
        sighash.copy_from_slice(&ix_data[..8]);
        ix_data = &ix_data[8..];
        sighash
    };
    use anchor_lang::Discriminator;
    match sighash {
        instruction::CreateRecord::DISCRIMINATOR => {
            __private::__global::create_record(program_id, accounts, ix_data)
        }
        instruction::UpdateRecord::DISCRIMINATOR => {
            __private::__global::update_record(program_id, accounts, ix_data)
        }
        instruction::DeleteRecord::DISCRIMINATOR => {
            __private::__global::delete_record(program_id, accounts, ix_data)
        }
        anchor_lang::idl::IDL_IX_TAG_LE => {
            __private::__idl::__idl_dispatch(program_id, accounts, &ix_data)
        }
        anchor_lang::event::EVENT_IX_TAG_LE => {
            Err(anchor_lang::error::ErrorCode::EventInstructionStub.into())
        }
        _ => Err(anchor_lang::error::ErrorCode::InstructionFallbackNotFound.into()),
    }
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
        impl anchor_lang::anchor_syn::idl::build::IdlBuild for IdlAccount {
            fn __anchor_private_full_path() -> String {
                {
                    let res = ::alloc::fmt::format(
                        format_args!(
                            "{0}::{1}",
                            "name_service::__private::__idl",
                            "IdlAccount",
                        ),
                    );
                    res
                }
            }
            fn __anchor_private_gen_idl_type() -> Option<
                anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
            > {
                Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                    name: Self::__anchor_private_full_path(),
                    generics: None,
                    docs: None,
                    ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                        fields: <[_]>::into_vec(
                            #[rustc_box]
                            ::alloc::boxed::Box::new([
                                anchor_lang::anchor_syn::idl::types::IdlField {
                                    name: "authority".into(),
                                    docs: None,
                                    ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                },
                                anchor_lang::anchor_syn::idl::types::IdlField {
                                    name: "dataLen".into(),
                                    docs: None,
                                    ty: anchor_lang::anchor_syn::idl::types::IdlType::U32,
                                },
                            ]),
                        ),
                    },
                })
            }
            fn __anchor_private_insert_idl_defined(
                defined_types: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
            ) {}
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
                if writer.write_all(&[24, 70, 98, 191, 58, 144, 123, 158]).is_err() {
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
                if buf.len() < [24, 70, 98, 191, 58, 144, 123, 158].len() {
                    return Err(
                        anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound
                            .into(),
                    );
                }
                let given_disc = &buf[..8];
                if &[24, 70, 98, 191, 58, 144, 123, 158] != given_disc {
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
                                        filename: "examples/name-service/programs/name-service/src/lib.rs",
                                        line: 15u32,
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
                let mut data: &[u8] = &buf[8..];
                AnchorDeserialize::deserialize(&mut data)
                    .map_err(|_| {
                        anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into()
                    })
            }
        }
        #[automatically_derived]
        impl anchor_lang::Discriminator for IdlAccount {
            const DISCRIMINATOR: [u8; 8] = [24, 70, 98, 191, 58, 144, 123, 158];
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
            impl anchor_lang::anchor_syn::idl::build::IdlBuild for IdlCreateAccounts {
                fn __anchor_private_full_path() -> String {
                    {
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "name_service::__private::__idl::__client_accounts_idl_create_accounts",
                                "IdlCreateAccounts",
                            ),
                        );
                        res
                    }
                }
                fn __anchor_private_gen_idl_type() -> Option<
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                > {
                    Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                        name: Self::__anchor_private_full_path(),
                        generics: None,
                        docs: Some(
                            <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    "Generated client accounts for [`IdlCreateAccounts`]."
                                        .into(),
                                ]),
                            ),
                        ),
                        ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                            fields: <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "from".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "to".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "base".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "systemProgram".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "program".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                ]),
                            ),
                        },
                    })
                }
                fn __anchor_private_insert_idl_defined(
                    defined_types: &mut std::collections::HashMap<
                        String,
                        anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                    >,
                ) {}
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
                accounts: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
                defined_types: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
            ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "from".into(),
                            is_mut: false,
                            is_signer: true,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "to".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "base".into(),
                            is_mut: false,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "systemProgram".into(),
                            is_mut: false,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "program".into(),
                            is_mut: false,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
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
            impl anchor_lang::anchor_syn::idl::build::IdlBuild for IdlAccounts {
                fn __anchor_private_full_path() -> String {
                    {
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "name_service::__private::__idl::__client_accounts_idl_accounts",
                                "IdlAccounts",
                            ),
                        );
                        res
                    }
                }
                fn __anchor_private_gen_idl_type() -> Option<
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                > {
                    Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                        name: Self::__anchor_private_full_path(),
                        generics: None,
                        docs: Some(
                            <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    "Generated client accounts for [`IdlAccounts`].".into(),
                                ]),
                            ),
                        ),
                        ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                            fields: <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "idl".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "authority".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                ]),
                            ),
                        },
                    })
                }
                fn __anchor_private_insert_idl_defined(
                    defined_types: &mut std::collections::HashMap<
                        String,
                        anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                    >,
                ) {}
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
                accounts: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
                defined_types: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
            ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
                {
                    <IdlAccount>::__anchor_private_insert_idl_defined(defined_types);
                    let path = <IdlAccount>::__anchor_private_full_path();
                    <IdlAccount>::__anchor_private_gen_idl_type()
                        .and_then(|ty| accounts.insert(path, ty));
                }
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "idl".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "authority".into(),
                            is_mut: false,
                            is_signer: true,
                            is_optional: None,
                            docs: None,
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
            impl anchor_lang::anchor_syn::idl::build::IdlBuild for IdlResizeAccount {
                fn __anchor_private_full_path() -> String {
                    {
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "name_service::__private::__idl::__client_accounts_idl_resize_account",
                                "IdlResizeAccount",
                            ),
                        );
                        res
                    }
                }
                fn __anchor_private_gen_idl_type() -> Option<
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                > {
                    Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                        name: Self::__anchor_private_full_path(),
                        generics: None,
                        docs: Some(
                            <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    "Generated client accounts for [`IdlResizeAccount`].".into(),
                                ]),
                            ),
                        ),
                        ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                            fields: <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "idl".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "authority".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "systemProgram".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                ]),
                            ),
                        },
                    })
                }
                fn __anchor_private_insert_idl_defined(
                    defined_types: &mut std::collections::HashMap<
                        String,
                        anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                    >,
                ) {}
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
                accounts: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
                defined_types: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
            ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
                {
                    <IdlAccount>::__anchor_private_insert_idl_defined(defined_types);
                    let path = <IdlAccount>::__anchor_private_full_path();
                    <IdlAccount>::__anchor_private_gen_idl_type()
                        .and_then(|ty| accounts.insert(path, ty));
                }
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "idl".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "authority".into(),
                            is_mut: true,
                            is_signer: true,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "systemProgram".into(),
                            is_mut: false,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
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
                    let mut __disc_bytes = [0u8; 8];
                    __disc_bytes.copy_from_slice(&__data[..8]);
                    let __discriminator = u64::from_le_bytes(__disc_bytes);
                    if __discriminator != 0 {
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
            impl anchor_lang::anchor_syn::idl::build::IdlBuild for IdlCreateBuffer {
                fn __anchor_private_full_path() -> String {
                    {
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "name_service::__private::__idl::__client_accounts_idl_create_buffer",
                                "IdlCreateBuffer",
                            ),
                        );
                        res
                    }
                }
                fn __anchor_private_gen_idl_type() -> Option<
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                > {
                    Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                        name: Self::__anchor_private_full_path(),
                        generics: None,
                        docs: Some(
                            <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    "Generated client accounts for [`IdlCreateBuffer`].".into(),
                                ]),
                            ),
                        ),
                        ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                            fields: <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "buffer".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "authority".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                ]),
                            ),
                        },
                    })
                }
                fn __anchor_private_insert_idl_defined(
                    defined_types: &mut std::collections::HashMap<
                        String,
                        anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                    >,
                ) {}
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
                accounts: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
                defined_types: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
            ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
                {
                    <IdlAccount>::__anchor_private_insert_idl_defined(defined_types);
                    let path = <IdlAccount>::__anchor_private_full_path();
                    <IdlAccount>::__anchor_private_gen_idl_type()
                        .and_then(|ty| accounts.insert(path, ty));
                }
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "buffer".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "authority".into(),
                            is_mut: false,
                            is_signer: true,
                            is_optional: None,
                            docs: None,
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
            impl anchor_lang::anchor_syn::idl::build::IdlBuild for IdlSetBuffer {
                fn __anchor_private_full_path() -> String {
                    {
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "name_service::__private::__idl::__client_accounts_idl_set_buffer",
                                "IdlSetBuffer",
                            ),
                        );
                        res
                    }
                }
                fn __anchor_private_gen_idl_type() -> Option<
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                > {
                    Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                        name: Self::__anchor_private_full_path(),
                        generics: None,
                        docs: Some(
                            <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    "Generated client accounts for [`IdlSetBuffer`].".into(),
                                ]),
                            ),
                        ),
                        ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                            fields: <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "buffer".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "idl".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "authority".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                ]),
                            ),
                        },
                    })
                }
                fn __anchor_private_insert_idl_defined(
                    defined_types: &mut std::collections::HashMap<
                        String,
                        anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                    >,
                ) {}
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
                accounts: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
                defined_types: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
            ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
                {
                    <IdlAccount>::__anchor_private_insert_idl_defined(defined_types);
                    let path = <IdlAccount>::__anchor_private_full_path();
                    <IdlAccount>::__anchor_private_gen_idl_type()
                        .and_then(|ty| accounts.insert(path, ty));
                };
                {
                    <IdlAccount>::__anchor_private_insert_idl_defined(defined_types);
                    let path = <IdlAccount>::__anchor_private_full_path();
                    <IdlAccount>::__anchor_private_gen_idl_type()
                        .and_then(|ty| accounts.insert(path, ty));
                }
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "buffer".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "idl".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "authority".into(),
                            is_mut: false,
                            is_signer: true,
                            is_optional: None,
                            docs: None,
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
            impl anchor_lang::anchor_syn::idl::build::IdlBuild for IdlCloseAccount {
                fn __anchor_private_full_path() -> String {
                    {
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "{0}::{1}",
                                "name_service::__private::__idl::__client_accounts_idl_close_account",
                                "IdlCloseAccount",
                            ),
                        );
                        res
                    }
                }
                fn __anchor_private_gen_idl_type() -> Option<
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                > {
                    Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                        name: Self::__anchor_private_full_path(),
                        generics: None,
                        docs: Some(
                            <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    "Generated client accounts for [`IdlCloseAccount`].".into(),
                                ]),
                            ),
                        ),
                        ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                            fields: <[_]>::into_vec(
                                #[rustc_box]
                                ::alloc::boxed::Box::new([
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "account".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "authority".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                    anchor_lang::anchor_syn::idl::types::IdlField {
                                        name: "solDestination".into(),
                                        docs: None,
                                        ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                                    },
                                ]),
                            ),
                        },
                    })
                }
                fn __anchor_private_insert_idl_defined(
                    defined_types: &mut std::collections::HashMap<
                        String,
                        anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                    >,
                ) {}
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
                accounts: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
                defined_types: &mut std::collections::HashMap<
                    String,
                    anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
                >,
            ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
                {
                    <IdlAccount>::__anchor_private_insert_idl_defined(defined_types);
                    let path = <IdlAccount>::__anchor_private_full_path();
                    <IdlAccount>::__anchor_private_gen_idl_type()
                        .and_then(|ty| accounts.insert(path, ty));
                }
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "account".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "authority".into(),
                            is_mut: false,
                            is_signer: true,
                            is_optional: None,
                            docs: None,
                            pda: None,
                            relations: ::alloc::vec::Vec::new(),
                        }),
                        anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                            name: "solDestination".into(),
                            is_mut: true,
                            is_signer: false,
                            is_optional: None,
                            docs: None,
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
            ::solana_program::log::sol_log("Instruction: IdlCreateAccount");
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
            let space = std::cmp::min(8 + 32 + 4 + data_len as usize, 10_000);
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
            ::solana_program::log::sol_log("Instruction: IdlResizeAccount");
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
                            to: accounts.idl.to_account_info().clone(),
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
            ::solana_program::log::sol_log("Instruction: IdlCloseAccount");
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_create_buffer(
            program_id: &Pubkey,
            accounts: &mut IdlCreateBuffer,
        ) -> anchor_lang::Result<()> {
            ::solana_program::log::sol_log("Instruction: IdlCreateBuffer");
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
            ::solana_program::log::sol_log("Instruction: IdlWrite");
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
                                    filename: "examples/name-service/programs/name-service/src/lib.rs",
                                    line: 15u32,
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
            ::solana_program::log::sol_log("Instruction: IdlSetAuthority");
            accounts.idl.authority = new_authority;
            Ok(())
        }
        #[inline(never)]
        pub fn __idl_set_buffer(
            program_id: &Pubkey,
            accounts: &mut IdlSetBuffer,
        ) -> anchor_lang::Result<()> {
            ::solana_program::log::sol_log("Instruction: IdlSetBuffer");
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
                                    filename: "examples/name-service/programs/name-service/src/lib.rs",
                                    line: 15u32,
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
        pub fn create_record<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_program::log::sol_log("Instruction: CreateRecord");
            let ix = instruction::CreateRecord::deserialize(&mut &__ix_data[..])
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::CreateRecord {
                inputs,
                proof,
                merkle_context,
                merkle_tree_root_index,
                address_merkle_context,
                address_merkle_tree_root_index,
                name,
                rdata,
            } = ix;
            let mut __bumps = <CreateRecord as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = CreateRecord::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = name_service::create_record(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                inputs,
                proof,
                merkle_context,
                merkle_tree_root_index,
                address_merkle_context,
                address_merkle_tree_root_index,
                name,
                rdata,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn update_record<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_program::log::sol_log("Instruction: UpdateRecord");
            let ix = instruction::UpdateRecord::deserialize(&mut &__ix_data[..])
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::UpdateRecord {
                inputs,
                proof,
                merkle_context,
                merkle_tree_root_index,
                address_merkle_context,
                address_merkle_tree_root_index,
                new_rdata,
            } = ix;
            let mut __bumps = <UpdateRecord as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = UpdateRecord::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = name_service::update_record(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                inputs,
                proof,
                merkle_context,
                merkle_tree_root_index,
                address_merkle_context,
                address_merkle_tree_root_index,
                new_rdata,
            )?;
            __accounts.exit(__program_id)
        }
        #[inline(never)]
        pub fn delete_record<'info>(
            __program_id: &Pubkey,
            __accounts: &'info [AccountInfo<'info>],
            __ix_data: &[u8],
        ) -> anchor_lang::Result<()> {
            ::solana_program::log::sol_log("Instruction: DeleteRecord");
            let ix = instruction::DeleteRecord::deserialize(&mut &__ix_data[..])
                .map_err(|_| {
                    anchor_lang::error::ErrorCode::InstructionDidNotDeserialize
                })?;
            let instruction::DeleteRecord {
                inputs,
                proof,
                merkle_context,
                merkle_tree_root_index,
                address_merkle_context,
                address_merkle_tree_root_index,
            } = ix;
            let mut __bumps = <DeleteRecord as anchor_lang::Bumps>::Bumps::default();
            let mut __reallocs = std::collections::BTreeSet::new();
            let mut __remaining_accounts: &[AccountInfo] = __accounts;
            let mut __accounts = DeleteRecord::try_accounts(
                __program_id,
                &mut __remaining_accounts,
                __ix_data,
                &mut __bumps,
                &mut __reallocs,
            )?;
            let result = name_service::delete_record(
                anchor_lang::context::Context::new(
                    __program_id,
                    &mut __accounts,
                    __remaining_accounts,
                    __bumps,
                ),
                inputs,
                proof,
                merkle_context,
                merkle_tree_root_index,
                address_merkle_context,
                address_merkle_tree_root_index,
            )?;
            __accounts.exit(__program_id)
        }
    }
}
pub mod name_service {
    use super::*;
    #[allow(clippy::too_many_arguments)]
    pub fn create_record<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateRecord<'info>>,
        inputs: Vec<Vec<u8>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        name: String,
        rdata: RData,
    ) -> Result<()> {
        let mut ctx: ::light_sdk::context::LightContext<
            CreateRecord,
            LightCreateRecord,
        > = ::light_sdk::context::LightContext::new(
            ctx,
            inputs,
            merkle_context,
            merkle_tree_root_index,
            address_merkle_context,
            address_merkle_tree_root_index,
        )?;
        ctx.light_accounts.record.owner = ctx.accounts.signer.key();
        ctx.light_accounts.record.name = name;
        ctx.light_accounts.record.rdata = rdata;
        ctx.verify(proof)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn update_record<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateRecord<'info>>,
        inputs: Vec<Vec<u8>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        new_rdata: RData,
    ) -> Result<()> {
        let mut ctx: ::light_sdk::context::LightContext<
            UpdateRecord,
            LightUpdateRecord,
        > = ::light_sdk::context::LightContext::new(
            ctx,
            inputs,
            merkle_context,
            merkle_tree_root_index,
            address_merkle_context,
            address_merkle_tree_root_index,
        )?;
        ctx.verify(proof)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn delete_record<'info>(
        ctx: Context<'_, '_, '_, 'info, DeleteRecord<'info>>,
        inputs: Vec<Vec<u8>>,
        proof: CompressedProof,
        merkle_context: PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
    ) -> Result<()> {
        let mut ctx: ::light_sdk::context::LightContext<
            DeleteRecord,
            LightDeleteRecord,
        > = ::light_sdk::context::LightContext::new(
            ctx,
            inputs,
            merkle_context,
            merkle_tree_root_index,
            address_merkle_context,
            address_merkle_tree_root_index,
        )?;
        ctx.verify(proof)?;
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
    pub struct CreateRecord {
        pub inputs: Vec<Vec<u8>>,
        pub proof: CompressedProof,
        pub merkle_context: PackedMerkleContext,
        pub merkle_tree_root_index: u16,
        pub address_merkle_context: PackedAddressMerkleContext,
        pub address_merkle_tree_root_index: u16,
        pub name: String,
        pub rdata: RData,
    }
    impl borsh::ser::BorshSerialize for CreateRecord
    where
        Vec<Vec<u8>>: borsh::ser::BorshSerialize,
        CompressedProof: borsh::ser::BorshSerialize,
        PackedMerkleContext: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
        PackedAddressMerkleContext: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
        String: borsh::ser::BorshSerialize,
        RData: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.inputs, writer)?;
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.merkle_context, writer)?;
            borsh::BorshSerialize::serialize(&self.merkle_tree_root_index, writer)?;
            borsh::BorshSerialize::serialize(&self.address_merkle_context, writer)?;
            borsh::BorshSerialize::serialize(
                &self.address_merkle_tree_root_index,
                writer,
            )?;
            borsh::BorshSerialize::serialize(&self.name, writer)?;
            borsh::BorshSerialize::serialize(&self.rdata, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::anchor_syn::idl::build::IdlBuild for CreateRecord {
        fn __anchor_private_full_path() -> String {
            {
                let res = ::alloc::fmt::format(
                    format_args!("{0}::{1}", "name_service::instruction", "CreateRecord"),
                );
                res
            }
        }
        fn __anchor_private_gen_idl_type() -> Option<
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        > {
            Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: None,
                docs: Some(
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new(["Instruction.".into()]),
                    ),
                ),
                ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                    fields: <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "inputs".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Vec(
                                    Box::new(
                                        anchor_lang::anchor_syn::idl::types::IdlType::Bytes,
                                    ),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "proof".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <CompressedProof>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "merkleContext".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <PackedMerkleContext>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "merkleTreeRootIndex".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::U16,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "addressMerkleContext".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <PackedAddressMerkleContext>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "addressMerkleTreeRootIndex".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::U16,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "name".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::String,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "rdata".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <RData>::__anchor_private_full_path(),
                                ),
                            },
                        ]),
                    ),
                },
            })
        }
        fn __anchor_private_insert_idl_defined(
            defined_types: &mut std::collections::HashMap<
                String,
                anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
            >,
        ) {
            {
                <CompressedProof>::__anchor_private_insert_idl_defined(defined_types);
                let path = <CompressedProof>::__anchor_private_full_path();
                <CompressedProof>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <PackedMerkleContext>::__anchor_private_insert_idl_defined(
                    defined_types,
                );
                let path = <PackedMerkleContext>::__anchor_private_full_path();
                <PackedMerkleContext>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <PackedAddressMerkleContext>::__anchor_private_insert_idl_defined(
                    defined_types,
                );
                let path = <PackedAddressMerkleContext>::__anchor_private_full_path();
                <PackedAddressMerkleContext>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <RData>::__anchor_private_insert_idl_defined(defined_types);
                let path = <RData>::__anchor_private_full_path();
                <RData>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            }
        }
    }
    impl borsh::de::BorshDeserialize for CreateRecord
    where
        Vec<Vec<u8>>: borsh::BorshDeserialize,
        CompressedProof: borsh::BorshDeserialize,
        PackedMerkleContext: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
        PackedAddressMerkleContext: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
        String: borsh::BorshDeserialize,
        RData: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                inputs: borsh::BorshDeserialize::deserialize_reader(reader)?,
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                merkle_context: borsh::BorshDeserialize::deserialize_reader(reader)?,
                merkle_tree_root_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                address_merkle_context: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                address_merkle_tree_root_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                name: borsh::BorshDeserialize::deserialize_reader(reader)?,
                rdata: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for CreateRecord {
        const DISCRIMINATOR: [u8; 8] = [116, 124, 63, 58, 126, 204, 178, 10];
    }
    impl anchor_lang::InstructionData for CreateRecord {}
    impl anchor_lang::Owner for CreateRecord {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct UpdateRecord {
        pub inputs: Vec<Vec<u8>>,
        pub proof: CompressedProof,
        pub merkle_context: PackedMerkleContext,
        pub merkle_tree_root_index: u16,
        pub address_merkle_context: PackedAddressMerkleContext,
        pub address_merkle_tree_root_index: u16,
        pub new_rdata: RData,
    }
    impl borsh::ser::BorshSerialize for UpdateRecord
    where
        Vec<Vec<u8>>: borsh::ser::BorshSerialize,
        CompressedProof: borsh::ser::BorshSerialize,
        PackedMerkleContext: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
        PackedAddressMerkleContext: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
        RData: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.inputs, writer)?;
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.merkle_context, writer)?;
            borsh::BorshSerialize::serialize(&self.merkle_tree_root_index, writer)?;
            borsh::BorshSerialize::serialize(&self.address_merkle_context, writer)?;
            borsh::BorshSerialize::serialize(
                &self.address_merkle_tree_root_index,
                writer,
            )?;
            borsh::BorshSerialize::serialize(&self.new_rdata, writer)?;
            Ok(())
        }
    }
    impl anchor_lang::anchor_syn::idl::build::IdlBuild for UpdateRecord {
        fn __anchor_private_full_path() -> String {
            {
                let res = ::alloc::fmt::format(
                    format_args!("{0}::{1}", "name_service::instruction", "UpdateRecord"),
                );
                res
            }
        }
        fn __anchor_private_gen_idl_type() -> Option<
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        > {
            Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: None,
                docs: Some(
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new(["Instruction.".into()]),
                    ),
                ),
                ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                    fields: <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "inputs".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Vec(
                                    Box::new(
                                        anchor_lang::anchor_syn::idl::types::IdlType::Bytes,
                                    ),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "proof".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <CompressedProof>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "merkleContext".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <PackedMerkleContext>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "merkleTreeRootIndex".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::U16,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "addressMerkleContext".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <PackedAddressMerkleContext>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "addressMerkleTreeRootIndex".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::U16,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "newRdata".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <RData>::__anchor_private_full_path(),
                                ),
                            },
                        ]),
                    ),
                },
            })
        }
        fn __anchor_private_insert_idl_defined(
            defined_types: &mut std::collections::HashMap<
                String,
                anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
            >,
        ) {
            {
                <CompressedProof>::__anchor_private_insert_idl_defined(defined_types);
                let path = <CompressedProof>::__anchor_private_full_path();
                <CompressedProof>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <PackedMerkleContext>::__anchor_private_insert_idl_defined(
                    defined_types,
                );
                let path = <PackedMerkleContext>::__anchor_private_full_path();
                <PackedMerkleContext>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <PackedAddressMerkleContext>::__anchor_private_insert_idl_defined(
                    defined_types,
                );
                let path = <PackedAddressMerkleContext>::__anchor_private_full_path();
                <PackedAddressMerkleContext>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <RData>::__anchor_private_insert_idl_defined(defined_types);
                let path = <RData>::__anchor_private_full_path();
                <RData>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            }
        }
    }
    impl borsh::de::BorshDeserialize for UpdateRecord
    where
        Vec<Vec<u8>>: borsh::BorshDeserialize,
        CompressedProof: borsh::BorshDeserialize,
        PackedMerkleContext: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
        PackedAddressMerkleContext: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
        RData: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                inputs: borsh::BorshDeserialize::deserialize_reader(reader)?,
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                merkle_context: borsh::BorshDeserialize::deserialize_reader(reader)?,
                merkle_tree_root_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                address_merkle_context: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                address_merkle_tree_root_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                new_rdata: borsh::BorshDeserialize::deserialize_reader(reader)?,
            })
        }
    }
    impl anchor_lang::Discriminator for UpdateRecord {
        const DISCRIMINATOR: [u8; 8] = [54, 194, 108, 162, 199, 12, 5, 60];
    }
    impl anchor_lang::InstructionData for UpdateRecord {}
    impl anchor_lang::Owner for UpdateRecord {
        fn owner() -> Pubkey {
            ID
        }
    }
    /// Instruction.
    pub struct DeleteRecord {
        pub inputs: Vec<Vec<u8>>,
        pub proof: CompressedProof,
        pub merkle_context: PackedMerkleContext,
        pub merkle_tree_root_index: u16,
        pub address_merkle_context: PackedAddressMerkleContext,
        pub address_merkle_tree_root_index: u16,
    }
    impl borsh::ser::BorshSerialize for DeleteRecord
    where
        Vec<Vec<u8>>: borsh::ser::BorshSerialize,
        CompressedProof: borsh::ser::BorshSerialize,
        PackedMerkleContext: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
        PackedAddressMerkleContext: borsh::ser::BorshSerialize,
        u16: borsh::ser::BorshSerialize,
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.inputs, writer)?;
            borsh::BorshSerialize::serialize(&self.proof, writer)?;
            borsh::BorshSerialize::serialize(&self.merkle_context, writer)?;
            borsh::BorshSerialize::serialize(&self.merkle_tree_root_index, writer)?;
            borsh::BorshSerialize::serialize(&self.address_merkle_context, writer)?;
            borsh::BorshSerialize::serialize(
                &self.address_merkle_tree_root_index,
                writer,
            )?;
            Ok(())
        }
    }
    impl anchor_lang::anchor_syn::idl::build::IdlBuild for DeleteRecord {
        fn __anchor_private_full_path() -> String {
            {
                let res = ::alloc::fmt::format(
                    format_args!("{0}::{1}", "name_service::instruction", "DeleteRecord"),
                );
                res
            }
        }
        fn __anchor_private_gen_idl_type() -> Option<
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        > {
            Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: None,
                docs: Some(
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new(["Instruction.".into()]),
                    ),
                ),
                ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                    fields: <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "inputs".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Vec(
                                    Box::new(
                                        anchor_lang::anchor_syn::idl::types::IdlType::Bytes,
                                    ),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "proof".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <CompressedProof>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "merkleContext".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <PackedMerkleContext>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "merkleTreeRootIndex".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::U16,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "addressMerkleContext".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                    <PackedAddressMerkleContext>::__anchor_private_full_path(),
                                ),
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "addressMerkleTreeRootIndex".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::U16,
                            },
                        ]),
                    ),
                },
            })
        }
        fn __anchor_private_insert_idl_defined(
            defined_types: &mut std::collections::HashMap<
                String,
                anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
            >,
        ) {
            {
                <CompressedProof>::__anchor_private_insert_idl_defined(defined_types);
                let path = <CompressedProof>::__anchor_private_full_path();
                <CompressedProof>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <PackedMerkleContext>::__anchor_private_insert_idl_defined(
                    defined_types,
                );
                let path = <PackedMerkleContext>::__anchor_private_full_path();
                <PackedMerkleContext>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            };
            {
                <PackedAddressMerkleContext>::__anchor_private_insert_idl_defined(
                    defined_types,
                );
                let path = <PackedAddressMerkleContext>::__anchor_private_full_path();
                <PackedAddressMerkleContext>::__anchor_private_gen_idl_type()
                    .and_then(|ty| defined_types.insert(path, ty));
            }
        }
    }
    impl borsh::de::BorshDeserialize for DeleteRecord
    where
        Vec<Vec<u8>>: borsh::BorshDeserialize,
        CompressedProof: borsh::BorshDeserialize,
        PackedMerkleContext: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
        PackedAddressMerkleContext: borsh::BorshDeserialize,
        u16: borsh::BorshDeserialize,
    {
        fn deserialize_reader<R: borsh::maybestd::io::Read>(
            reader: &mut R,
        ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
            Ok(Self {
                inputs: borsh::BorshDeserialize::deserialize_reader(reader)?,
                proof: borsh::BorshDeserialize::deserialize_reader(reader)?,
                merkle_context: borsh::BorshDeserialize::deserialize_reader(reader)?,
                merkle_tree_root_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                address_merkle_context: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
                address_merkle_tree_root_index: borsh::BorshDeserialize::deserialize_reader(
                    reader,
                )?,
            })
        }
    }
    impl anchor_lang::Discriminator for DeleteRecord {
        const DISCRIMINATOR: [u8; 8] = [177, 191, 85, 153, 140, 226, 175, 112];
    }
    impl anchor_lang::InstructionData for DeleteRecord {}
    impl anchor_lang::Owner for DeleteRecord {
        fn owner() -> Pubkey {
            ID
        }
    }
}
/// An Anchor generated module, providing a set of structs
/// mirroring the structs deriving `Accounts`, where each field is
/// a `Pubkey`. This is useful for specifying accounts for a client.
pub mod accounts {
    pub use crate::__client_accounts_delete_record::*;
    pub use crate::__client_accounts_update_record::*;
    pub use crate::__client_accounts_create_record::*;
}
pub enum RData {
    A(Ipv4Addr),
    AAAA(Ipv6Addr),
    CName(String),
}
#[automatically_derived]
impl ::core::clone::Clone for RData {
    #[inline]
    fn clone(&self) -> RData {
        match self {
            RData::A(__self_0) => RData::A(::core::clone::Clone::clone(__self_0)),
            RData::AAAA(__self_0) => RData::AAAA(::core::clone::Clone::clone(__self_0)),
            RData::CName(__self_0) => RData::CName(::core::clone::Clone::clone(__self_0)),
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for RData {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            RData::A(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "A", &__self_0)
            }
            RData::AAAA(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "AAAA", &__self_0)
            }
            RData::CName(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "CName", &__self_0)
            }
        }
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for RData {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) -> () {
        let _: ::core::cmp::AssertParamIsEq<Ipv4Addr>;
        let _: ::core::cmp::AssertParamIsEq<Ipv6Addr>;
        let _: ::core::cmp::AssertParamIsEq<String>;
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for RData {}
#[automatically_derived]
impl ::core::cmp::PartialEq for RData {
    #[inline]
    fn eq(&self, other: &RData) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
            && match (self, other) {
                (RData::A(__self_0), RData::A(__arg1_0)) => *__self_0 == *__arg1_0,
                (RData::AAAA(__self_0), RData::AAAA(__arg1_0)) => *__self_0 == *__arg1_0,
                (RData::CName(__self_0), RData::CName(__arg1_0)) => {
                    *__self_0 == *__arg1_0
                }
                _ => unsafe { ::core::intrinsics::unreachable() }
            }
    }
}
impl borsh::de::BorshDeserialize for RData
where
    Ipv4Addr: borsh::BorshDeserialize,
    Ipv6Addr: borsh::BorshDeserialize,
    String: borsh::BorshDeserialize,
{
    fn deserialize_reader<R: borsh::maybestd::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        let tag = <u8 as borsh::de::BorshDeserialize>::deserialize_reader(reader)?;
        <Self as borsh::de::EnumExt>::deserialize_variant(reader, tag)
    }
}
impl borsh::de::EnumExt for RData
where
    Ipv4Addr: borsh::BorshDeserialize,
    Ipv6Addr: borsh::BorshDeserialize,
    String: borsh::BorshDeserialize,
{
    fn deserialize_variant<R: borsh::maybestd::io::Read>(
        reader: &mut R,
        variant_idx: u8,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        let mut return_value = match variant_idx {
            0u8 => RData::A(borsh::BorshDeserialize::deserialize_reader(reader)?),
            1u8 => RData::AAAA(borsh::BorshDeserialize::deserialize_reader(reader)?),
            2u8 => RData::CName(borsh::BorshDeserialize::deserialize_reader(reader)?),
            _ => {
                return Err(
                    borsh::maybestd::io::Error::new(
                        borsh::maybestd::io::ErrorKind::InvalidInput,
                        {
                            let res = ::alloc::fmt::format(
                                format_args!("Unexpected variant index: {0:?}", variant_idx),
                            );
                            res
                        },
                    ),
                );
            }
        };
        Ok(return_value)
    }
}
impl borsh::ser::BorshSerialize for RData
where
    Ipv4Addr: borsh::ser::BorshSerialize,
    Ipv6Addr: borsh::ser::BorshSerialize,
    String: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        let variant_idx: u8 = match self {
            RData::A(..) => 0u8,
            RData::AAAA(..) => 1u8,
            RData::CName(..) => 2u8,
        };
        writer.write_all(&variant_idx.to_le_bytes())?;
        match self {
            RData::A(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            RData::AAAA(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
            RData::CName(id0) => {
                borsh::BorshSerialize::serialize(id0, writer)?;
            }
        }
        Ok(())
    }
}
impl anchor_lang::IdlBuild for RData {}
impl AsByteVec for RData {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        match self {
            Self::A(ipv4_addr) => {
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([ipv4_addr.octets().to_vec()]),
                )
            }
            Self::AAAA(ipv6_addr) => {
                <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([ipv6_addr.octets().to_vec()]),
                )
            }
            Self::CName(cname) => cname.as_byte_vec(),
        }
    }
}
impl Default for RData {
    fn default() -> Self {
        Self::A(Ipv4Addr::new(127, 0, 0, 1))
    }
}
pub struct NameRecord {
    #[truncate]
    pub owner: Pubkey,
    #[truncate]
    pub name: String,
    pub rdata: RData,
}
#[automatically_derived]
impl ::core::clone::Clone for NameRecord {
    #[inline]
    fn clone(&self) -> NameRecord {
        NameRecord {
            owner: ::core::clone::Clone::clone(&self.owner),
            name: ::core::clone::Clone::clone(&self.name),
            rdata: ::core::clone::Clone::clone(&self.rdata),
        }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for NameRecord {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field3_finish(
            f,
            "NameRecord",
            "owner",
            &self.owner,
            "name",
            &self.name,
            "rdata",
            &&self.rdata,
        )
    }
}
#[automatically_derived]
impl ::core::default::Default for NameRecord {
    #[inline]
    fn default() -> NameRecord {
        NameRecord {
            owner: ::core::default::Default::default(),
            name: ::core::default::Default::default(),
            rdata: ::core::default::Default::default(),
        }
    }
}
impl borsh::de::BorshDeserialize for NameRecord
where
    Pubkey: borsh::BorshDeserialize,
    String: borsh::BorshDeserialize,
    RData: borsh::BorshDeserialize,
{
    fn deserialize_reader<R: borsh::maybestd::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<Self, borsh::maybestd::io::Error> {
        Ok(Self {
            owner: borsh::BorshDeserialize::deserialize_reader(reader)?,
            name: borsh::BorshDeserialize::deserialize_reader(reader)?,
            rdata: borsh::BorshDeserialize::deserialize_reader(reader)?,
        })
    }
}
impl borsh::ser::BorshSerialize for NameRecord
where
    Pubkey: borsh::ser::BorshSerialize,
    String: borsh::ser::BorshSerialize,
    RData: borsh::ser::BorshSerialize,
{
    fn serialize<W: borsh::maybestd::io::Write>(
        &self,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
        borsh::BorshSerialize::serialize(&self.owner, writer)?;
        borsh::BorshSerialize::serialize(&self.name, writer)?;
        borsh::BorshSerialize::serialize(&self.rdata, writer)?;
        Ok(())
    }
}
impl anchor_lang::anchor_syn::idl::build::IdlBuild for NameRecord {
    fn __anchor_private_full_path() -> String {
        {
            let res = ::alloc::fmt::format(
                format_args!("{0}::{1}", "name_service", "NameRecord"),
            );
            res
        }
    }
    fn __anchor_private_gen_idl_type() -> Option<
        anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
    > {
        Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
            name: Self::__anchor_private_full_path(),
            generics: None,
            docs: None,
            ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                fields: <[_]>::into_vec(
                    #[rustc_box]
                    ::alloc::boxed::Box::new([
                        anchor_lang::anchor_syn::idl::types::IdlField {
                            name: "owner".into(),
                            docs: None,
                            ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                        },
                        anchor_lang::anchor_syn::idl::types::IdlField {
                            name: "name".into(),
                            docs: None,
                            ty: anchor_lang::anchor_syn::idl::types::IdlType::String,
                        },
                        anchor_lang::anchor_syn::idl::types::IdlField {
                            name: "rdata".into(),
                            docs: None,
                            ty: anchor_lang::anchor_syn::idl::types::IdlType::Defined(
                                <RData>::__anchor_private_full_path(),
                            ),
                        },
                    ]),
                ),
            },
        })
    }
    fn __anchor_private_insert_idl_defined(
        defined_types: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
    ) {
        {
            <RData>::__anchor_private_insert_idl_defined(defined_types);
            let path = <RData>::__anchor_private_full_path();
            <RData>::__anchor_private_gen_idl_type()
                .and_then(|ty| defined_types.insert(path, ty));
        }
    }
}
impl light_hasher::Discriminator for NameRecord {
    const DISCRIMINATOR: [u8; 8] = [125, 212, 222, 204, 127, 212, 170, 183];
}
impl ::light_hasher::bytes::AsByteVec for NameRecord {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        use ::light_hasher::bytes::AsByteVec;
        let mut result: Vec<Vec<u8>> = Vec::new();
        let truncated_bytes = self
            .owner
            .as_byte_vec()
            .iter()
            .map(|bytes| {
                let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(bytes)
                    .expect(
                        "Could not truncate the field #field_name to the BN254 prime field",
                    );
                bytes.to_vec()
            })
            .collect::<Vec<Vec<u8>>>();
        result.extend_from_slice(truncated_bytes.as_slice());
        let truncated_bytes = self
            .name
            .as_byte_vec()
            .iter()
            .map(|bytes| {
                let (bytes, _) = ::light_utils::hash_to_bn254_field_size_be(bytes)
                    .expect(
                        "Could not truncate the field #field_name to the BN254 prime field",
                    );
                bytes.to_vec()
            })
            .collect::<Vec<Vec<u8>>>();
        result.extend_from_slice(truncated_bytes.as_slice());
        result.extend_from_slice(self.rdata.as_byte_vec().as_slice());
        result
    }
}
impl ::light_hasher::DataHasher for NameRecord {
    fn hash<H: light_hasher::Hasher>(
        &self,
    ) -> ::std::result::Result<[u8; 32], ::light_hasher::errors::HasherError> {
        use ::light_hasher::bytes::AsByteVec;
        H::hashv(
            self
                .as_byte_vec()
                .iter()
                .map(|v| v.as_slice())
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }
}
#[repr(u32)]
pub enum CustomError {
    Unauthorized,
    NoData,
    InvalidDataHash,
}
#[automatically_derived]
impl ::core::fmt::Debug for CustomError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                CustomError::Unauthorized => "Unauthorized",
                CustomError::NoData => "NoData",
                CustomError::InvalidDataHash => "InvalidDataHash",
            },
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for CustomError {
    #[inline]
    fn clone(&self) -> CustomError {
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for CustomError {}
impl CustomError {
    /// Gets the name of this [#enum_name].
    pub fn name(&self) -> String {
        match self {
            CustomError::Unauthorized => "Unauthorized".to_string(),
            CustomError::NoData => "NoData".to_string(),
            CustomError::InvalidDataHash => "InvalidDataHash".to_string(),
        }
    }
}
impl From<CustomError> for u32 {
    fn from(e: CustomError) -> u32 {
        e as u32 + anchor_lang::error::ERROR_CODE_OFFSET
    }
}
impl From<CustomError> for anchor_lang::error::Error {
    fn from(error_code: CustomError) -> anchor_lang::error::Error {
        anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
            error_name: error_code.name(),
            error_code_number: error_code.into(),
            error_msg: error_code.to_string(),
            error_origin: None,
            compared_values: None,
        })
    }
}
impl std::fmt::Display for CustomError {
    fn fmt(
        &self,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            CustomError::Unauthorized => {
                fmt.write_fmt(format_args!("No authority to perform this action"))
            }
            CustomError::NoData => {
                fmt.write_fmt(format_args!("Record account has no data"))
            }
            CustomError::InvalidDataHash => {
                fmt.write_fmt(
                    format_args!("Provided data hash does not match the computed hash"),
                )
            }
        }
    }
}
pub struct CreateRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    pub light_system_program: Program<
        'info,
        ::light_system_program::program::LightSystemProgram,
    >,
    pub system_program: Program<'info, System>,
    pub account_compression_program: Program<
        'info,
        ::account_compression::program::AccountCompression,
    >,
    pub registered_program_pda: Account<'info, ::account_compression::RegisteredProgram>,
    pub noop_program: AccountInfo<'info>,
    pub account_compression_authority: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info, CreateRecordBumps> for CreateRecord<'info>
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
        __bumps: &mut CreateRecordBumps,
        __reallocs: &mut std::collections::BTreeSet<
            anchor_lang::solana_program::pubkey::Pubkey,
        >,
    ) -> anchor_lang::Result<Self> {
        let signer: Signer = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("signer"))?;
        let self_program: anchor_lang::accounts::program::Program<
            crate::program::NameService,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("self_program"))?;
        let cpi_signer: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("cpi_signer"))?;
        let light_system_program: anchor_lang::accounts::program::Program<
            ::light_system_program::program::LightSystemProgram,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("light_system_program"))?;
        let system_program: anchor_lang::accounts::program::Program<System> = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("system_program"))?;
        let account_compression_program: anchor_lang::accounts::program::Program<
            ::account_compression::program::AccountCompression,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("account_compression_program"))?;
        let registered_program_pda: anchor_lang::accounts::account::Account<
            ::account_compression::RegisteredProgram,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("registered_program_pda"))?;
        let noop_program: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("noop_program"))?;
        let account_compression_authority: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("account_compression_authority"))?;
        if !AsRef::<AccountInfo>::as_ref(&signer).is_writable {
            return Err(
                anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("signer"),
            );
        }
        Ok(CreateRecord {
            signer,
            self_program,
            cpi_signer,
            light_system_program,
            system_program,
            account_compression_program,
            registered_program_pda,
            noop_program,
            account_compression_authority,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for CreateRecord<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.signer.to_account_infos());
        account_infos.extend(self.self_program.to_account_infos());
        account_infos.extend(self.cpi_signer.to_account_infos());
        account_infos.extend(self.light_system_program.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.account_compression_program.to_account_infos());
        account_infos.extend(self.registered_program_pda.to_account_infos());
        account_infos.extend(self.noop_program.to_account_infos());
        account_infos.extend(self.account_compression_authority.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for CreateRecord<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.signer.to_account_metas(None));
        account_metas.extend(self.self_program.to_account_metas(None));
        account_metas.extend(self.cpi_signer.to_account_metas(None));
        account_metas.extend(self.light_system_program.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.account_compression_program.to_account_metas(None));
        account_metas.extend(self.registered_program_pda.to_account_metas(None));
        account_metas.extend(self.noop_program.to_account_metas(None));
        account_metas.extend(self.account_compression_authority.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for CreateRecord<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::Result<()> {
        anchor_lang::AccountsExit::exit(&self.signer, program_id)
            .map_err(|e| e.with_account_name("signer"))?;
        Ok(())
    }
}
pub struct CreateRecordBumps {}
#[automatically_derived]
impl ::core::fmt::Debug for CreateRecordBumps {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "CreateRecordBumps")
    }
}
impl Default for CreateRecordBumps {
    fn default() -> Self {
        CreateRecordBumps {}
    }
}
impl<'info> anchor_lang::Bumps for CreateRecord<'info>
where
    'info: 'info,
{
    type Bumps = CreateRecordBumps;
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
pub(crate) mod __client_accounts_create_record {
    use super::*;
    use anchor_lang::prelude::borsh;
    /// Generated client accounts for [`CreateRecord`].
    pub struct CreateRecord {
        pub signer: Pubkey,
        pub self_program: Pubkey,
        pub cpi_signer: Pubkey,
        pub light_system_program: Pubkey,
        pub system_program: Pubkey,
        pub account_compression_program: Pubkey,
        pub registered_program_pda: Pubkey,
        pub noop_program: Pubkey,
        pub account_compression_authority: Pubkey,
    }
    impl borsh::ser::BorshSerialize for CreateRecord
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
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.signer, writer)?;
            borsh::BorshSerialize::serialize(&self.self_program, writer)?;
            borsh::BorshSerialize::serialize(&self.cpi_signer, writer)?;
            borsh::BorshSerialize::serialize(&self.light_system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.account_compression_program, writer)?;
            borsh::BorshSerialize::serialize(&self.registered_program_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.noop_program, writer)?;
            borsh::BorshSerialize::serialize(
                &self.account_compression_authority,
                writer,
            )?;
            Ok(())
        }
    }
    impl anchor_lang::anchor_syn::idl::build::IdlBuild for CreateRecord {
        fn __anchor_private_full_path() -> String {
            {
                let res = ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "name_service::__client_accounts_create_record",
                        "CreateRecord",
                    ),
                );
                res
            }
        }
        fn __anchor_private_gen_idl_type() -> Option<
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        > {
            Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: None,
                docs: Some(
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            "Generated client accounts for [`CreateRecord`].".into(),
                        ]),
                    ),
                ),
                ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                    fields: <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "signer".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "selfProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "cpiSigner".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "lightSystemProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "systemProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "accountCompressionProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "registeredProgramPda".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "noopProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "accountCompressionAuthority".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                        ]),
                    ),
                },
            })
        }
        fn __anchor_private_insert_idl_defined(
            defined_types: &mut std::collections::HashMap<
                String,
                anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
            >,
        ) {}
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for CreateRecord {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.signer,
                        true,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.self_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.cpi_signer,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.light_system_program,
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
                        self.account_compression_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.registered_program_pda,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.noop_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.account_compression_authority,
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
pub(crate) mod __cpi_client_accounts_create_record {
    use super::*;
    /// Generated CPI struct of the accounts for [`CreateRecord`].
    pub struct CreateRecord<'info> {
        pub signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub self_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub cpi_signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub light_system_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub system_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub account_compression_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub registered_program_pda: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub noop_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub account_compression_authority: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for CreateRecord<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.signer),
                        true,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.self_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.cpi_signer),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.light_system_program),
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
                        anchor_lang::Key::key(&self.account_compression_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.registered_program_pda),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.noop_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.account_compression_authority),
                        false,
                    ),
                );
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for CreateRecord<'info> {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos
                .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.signer));
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.self_program),
                );
            account_infos
                .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.cpi_signer));
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.light_system_program,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.system_program),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.account_compression_program,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.registered_program_pda,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.noop_program),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.account_compression_authority,
                    ),
                );
            account_infos
        }
    }
}
impl<'info> CreateRecord<'info> {
    pub fn __anchor_private_gen_idl_accounts(
        accounts: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
        defined_types: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
    ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
        {
            <::account_compression::RegisteredProgram>::__anchor_private_insert_idl_defined(
                defined_types,
            );
            let path = <::account_compression::RegisteredProgram>::__anchor_private_full_path();
            <::account_compression::RegisteredProgram>::__anchor_private_gen_idl_type()
                .and_then(|ty| accounts.insert(path, ty));
        }
        <[_]>::into_vec(
            #[rustc_box]
            ::alloc::boxed::Box::new([
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "signer".into(),
                    is_mut: true,
                    is_signer: true,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "selfProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "cpiSigner".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "lightSystemProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "systemProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "accountCompressionProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "registeredProgramPda".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "noopProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "accountCompressionAuthority".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
            ]),
        )
    }
}
impl<'info> ::light_sdk::traits::InvokeCpiAccounts<'info> for CreateRecord<'info> {
    fn get_invoking_program(&self) -> &AccountInfo<'info> {
        &self.self_program
    }
}
impl<'info> ::light_sdk::traits::SignerAccounts<'info> for CreateRecord<'info> {
    fn get_fee_payer(&self) -> &::anchor_lang::prelude::Signer<'info> {
        &self.signer
    }
    fn get_authority(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.cpi_signer
    }
}
impl<'info> ::light_sdk::traits::LightSystemAccount<'info> for CreateRecord<'info> {
    fn get_light_system_program(
        &self,
    ) -> &::anchor_lang::prelude::Program<
        'info,
        ::light_system_program::program::LightSystemProgram,
    > {
        &self.light_system_program
    }
}
impl<'info> ::light_sdk::traits::InvokeAccounts<'info> for CreateRecord<'info> {
    fn get_registered_program_pda(
        &self,
    ) -> &::anchor_lang::prelude::Account<
        'info,
        ::account_compression::RegisteredProgram,
    > {
        &self.registered_program_pda
    }
    fn get_noop_program(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.noop_program
    }
    fn get_account_compression_authority(
        &self,
    ) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.account_compression_authority
    }
    fn get_account_compression_program(
        &self,
    ) -> &::anchor_lang::prelude::Program<
        'info,
        ::account_compression::program::AccountCompression,
    > {
        &self.account_compression_program
    }
    fn get_system_program(&self) -> &::anchor_lang::prelude::Program<'info, System> {
        &self.system_program
    }
    fn get_compressed_sol_pda(
        &self,
    ) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
        None
    }
    fn get_compression_recipient(
        &self,
    ) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
        None
    }
}
impl<'info> ::light_sdk::traits::InvokeCpiContextAccount<'info> for CreateRecord<'info> {
    fn get_cpi_context_account(
        &self,
    ) -> Option<
        &::anchor_lang::prelude::Account<
            'info,
            ::light_system_program::invoke_cpi::account::CpiContextAccount,
        >,
    > {
        None
    }
}
pub struct LightCreateRecord {
    #[light_account(init, seeds = [b"name-service"])]
    pub record: LightAccount<NameRecord>,
}
impl ::light_sdk::compressed_account::LightAccounts for LightCreateRecord {
    fn try_light_accounts(
        inputs: Vec<Vec<u8>>,
        merkle_context: ::light_sdk::merkle_context::PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: ::light_sdk::merkle_context::PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::new_init(
            &[b"name-service"],
            &crate::ID,
            &merkle_context,
            &address_merkle_context,
            address_merkle_tree_root_index,
            remaining_accounts,
        );
        Ok(Self { record })
    }
    fn new_address_params(
        &self,
    ) -> Vec<::light_sdk::compressed_account::NewAddressParamsPacked> {
        let mut new_address_params = Vec::new();
        if let Some(new_address_params_for_acc) = self.record.new_address_params() {
            new_address_params.push(new_address_params_for_acc);
        }
        new_address_params
    }
    fn input_accounts(
        &self,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<
        Vec<::light_sdk::compressed_account::PackedCompressedAccountWithMerkleContext>,
    > {
        let mut accounts = Vec::new();
        if let Some(compressed_account) = self
            .record
            .input_compressed_account(&crate::ID, remaining_accounts)?
        {
            accounts.push(compressed_account);
        }
        Ok(accounts)
    }
    fn output_accounts(
        &self,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<
        Vec<::light_sdk::compressed_account::OutputCompressedAccountWithPackedContext>,
    > {
        let mut accounts = Vec::new();
        if let Some(compressed_account) = self
            .record
            .output_compressed_account(&crate::ID, remaining_accounts)?
        {
            accounts.push(compressed_account);
        }
        Ok(accounts)
    }
}
pub struct UpdateRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    pub light_system_program: Program<
        'info,
        ::light_system_program::program::LightSystemProgram,
    >,
    pub system_program: Program<'info, System>,
    pub account_compression_program: Program<
        'info,
        ::account_compression::program::AccountCompression,
    >,
    pub registered_program_pda: Account<'info, ::account_compression::RegisteredProgram>,
    pub noop_program: AccountInfo<'info>,
    pub account_compression_authority: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info, UpdateRecordBumps> for UpdateRecord<'info>
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
        __bumps: &mut UpdateRecordBumps,
        __reallocs: &mut std::collections::BTreeSet<
            anchor_lang::solana_program::pubkey::Pubkey,
        >,
    ) -> anchor_lang::Result<Self> {
        let signer: Signer = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("signer"))?;
        let self_program: anchor_lang::accounts::program::Program<
            crate::program::NameService,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("self_program"))?;
        let cpi_signer: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("cpi_signer"))?;
        let light_system_program: anchor_lang::accounts::program::Program<
            ::light_system_program::program::LightSystemProgram,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("light_system_program"))?;
        let system_program: anchor_lang::accounts::program::Program<System> = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("system_program"))?;
        let account_compression_program: anchor_lang::accounts::program::Program<
            ::account_compression::program::AccountCompression,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("account_compression_program"))?;
        let registered_program_pda: anchor_lang::accounts::account::Account<
            ::account_compression::RegisteredProgram,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("registered_program_pda"))?;
        let noop_program: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("noop_program"))?;
        let account_compression_authority: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("account_compression_authority"))?;
        if !AsRef::<AccountInfo>::as_ref(&signer).is_writable {
            return Err(
                anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("signer"),
            );
        }
        Ok(UpdateRecord {
            signer,
            self_program,
            cpi_signer,
            light_system_program,
            system_program,
            account_compression_program,
            registered_program_pda,
            noop_program,
            account_compression_authority,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateRecord<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.signer.to_account_infos());
        account_infos.extend(self.self_program.to_account_infos());
        account_infos.extend(self.cpi_signer.to_account_infos());
        account_infos.extend(self.light_system_program.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.account_compression_program.to_account_infos());
        account_infos.extend(self.registered_program_pda.to_account_infos());
        account_infos.extend(self.noop_program.to_account_infos());
        account_infos.extend(self.account_compression_authority.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for UpdateRecord<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.signer.to_account_metas(None));
        account_metas.extend(self.self_program.to_account_metas(None));
        account_metas.extend(self.cpi_signer.to_account_metas(None));
        account_metas.extend(self.light_system_program.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.account_compression_program.to_account_metas(None));
        account_metas.extend(self.registered_program_pda.to_account_metas(None));
        account_metas.extend(self.noop_program.to_account_metas(None));
        account_metas.extend(self.account_compression_authority.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for UpdateRecord<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::Result<()> {
        anchor_lang::AccountsExit::exit(&self.signer, program_id)
            .map_err(|e| e.with_account_name("signer"))?;
        Ok(())
    }
}
pub struct UpdateRecordBumps {}
#[automatically_derived]
impl ::core::fmt::Debug for UpdateRecordBumps {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "UpdateRecordBumps")
    }
}
impl Default for UpdateRecordBumps {
    fn default() -> Self {
        UpdateRecordBumps {}
    }
}
impl<'info> anchor_lang::Bumps for UpdateRecord<'info>
where
    'info: 'info,
{
    type Bumps = UpdateRecordBumps;
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
pub(crate) mod __client_accounts_update_record {
    use super::*;
    use anchor_lang::prelude::borsh;
    /// Generated client accounts for [`UpdateRecord`].
    pub struct UpdateRecord {
        pub signer: Pubkey,
        pub self_program: Pubkey,
        pub cpi_signer: Pubkey,
        pub light_system_program: Pubkey,
        pub system_program: Pubkey,
        pub account_compression_program: Pubkey,
        pub registered_program_pda: Pubkey,
        pub noop_program: Pubkey,
        pub account_compression_authority: Pubkey,
    }
    impl borsh::ser::BorshSerialize for UpdateRecord
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
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.signer, writer)?;
            borsh::BorshSerialize::serialize(&self.self_program, writer)?;
            borsh::BorshSerialize::serialize(&self.cpi_signer, writer)?;
            borsh::BorshSerialize::serialize(&self.light_system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.account_compression_program, writer)?;
            borsh::BorshSerialize::serialize(&self.registered_program_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.noop_program, writer)?;
            borsh::BorshSerialize::serialize(
                &self.account_compression_authority,
                writer,
            )?;
            Ok(())
        }
    }
    impl anchor_lang::anchor_syn::idl::build::IdlBuild for UpdateRecord {
        fn __anchor_private_full_path() -> String {
            {
                let res = ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "name_service::__client_accounts_update_record",
                        "UpdateRecord",
                    ),
                );
                res
            }
        }
        fn __anchor_private_gen_idl_type() -> Option<
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        > {
            Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: None,
                docs: Some(
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            "Generated client accounts for [`UpdateRecord`].".into(),
                        ]),
                    ),
                ),
                ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                    fields: <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "signer".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "selfProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "cpiSigner".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "lightSystemProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "systemProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "accountCompressionProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "registeredProgramPda".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "noopProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "accountCompressionAuthority".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                        ]),
                    ),
                },
            })
        }
        fn __anchor_private_insert_idl_defined(
            defined_types: &mut std::collections::HashMap<
                String,
                anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
            >,
        ) {}
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for UpdateRecord {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.signer,
                        true,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.self_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.cpi_signer,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.light_system_program,
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
                        self.account_compression_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.registered_program_pda,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.noop_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.account_compression_authority,
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
pub(crate) mod __cpi_client_accounts_update_record {
    use super::*;
    /// Generated CPI struct of the accounts for [`UpdateRecord`].
    pub struct UpdateRecord<'info> {
        pub signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub self_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub cpi_signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub light_system_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub system_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub account_compression_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub registered_program_pda: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub noop_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub account_compression_authority: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for UpdateRecord<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.signer),
                        true,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.self_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.cpi_signer),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.light_system_program),
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
                        anchor_lang::Key::key(&self.account_compression_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.registered_program_pda),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.noop_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.account_compression_authority),
                        false,
                    ),
                );
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for UpdateRecord<'info> {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos
                .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.signer));
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.self_program),
                );
            account_infos
                .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.cpi_signer));
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.light_system_program,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.system_program),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.account_compression_program,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.registered_program_pda,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.noop_program),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.account_compression_authority,
                    ),
                );
            account_infos
        }
    }
}
impl<'info> UpdateRecord<'info> {
    pub fn __anchor_private_gen_idl_accounts(
        accounts: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
        defined_types: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
    ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
        {
            <::account_compression::RegisteredProgram>::__anchor_private_insert_idl_defined(
                defined_types,
            );
            let path = <::account_compression::RegisteredProgram>::__anchor_private_full_path();
            <::account_compression::RegisteredProgram>::__anchor_private_gen_idl_type()
                .and_then(|ty| accounts.insert(path, ty));
        }
        <[_]>::into_vec(
            #[rustc_box]
            ::alloc::boxed::Box::new([
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "signer".into(),
                    is_mut: true,
                    is_signer: true,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "selfProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "cpiSigner".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "lightSystemProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "systemProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "accountCompressionProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "registeredProgramPda".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "noopProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "accountCompressionAuthority".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
            ]),
        )
    }
}
impl<'info> ::light_sdk::traits::InvokeCpiAccounts<'info> for UpdateRecord<'info> {
    fn get_invoking_program(&self) -> &AccountInfo<'info> {
        &self.self_program
    }
}
impl<'info> ::light_sdk::traits::SignerAccounts<'info> for UpdateRecord<'info> {
    fn get_fee_payer(&self) -> &::anchor_lang::prelude::Signer<'info> {
        &self.signer
    }
    fn get_authority(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.cpi_signer
    }
}
impl<'info> ::light_sdk::traits::LightSystemAccount<'info> for UpdateRecord<'info> {
    fn get_light_system_program(
        &self,
    ) -> &::anchor_lang::prelude::Program<
        'info,
        ::light_system_program::program::LightSystemProgram,
    > {
        &self.light_system_program
    }
}
impl<'info> ::light_sdk::traits::InvokeAccounts<'info> for UpdateRecord<'info> {
    fn get_registered_program_pda(
        &self,
    ) -> &::anchor_lang::prelude::Account<
        'info,
        ::account_compression::RegisteredProgram,
    > {
        &self.registered_program_pda
    }
    fn get_noop_program(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.noop_program
    }
    fn get_account_compression_authority(
        &self,
    ) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.account_compression_authority
    }
    fn get_account_compression_program(
        &self,
    ) -> &::anchor_lang::prelude::Program<
        'info,
        ::account_compression::program::AccountCompression,
    > {
        &self.account_compression_program
    }
    fn get_system_program(&self) -> &::anchor_lang::prelude::Program<'info, System> {
        &self.system_program
    }
    fn get_compressed_sol_pda(
        &self,
    ) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
        None
    }
    fn get_compression_recipient(
        &self,
    ) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
        None
    }
}
impl<'info> ::light_sdk::traits::InvokeCpiContextAccount<'info> for UpdateRecord<'info> {
    fn get_cpi_context_account(
        &self,
    ) -> Option<
        &::anchor_lang::prelude::Account<
            'info,
            ::light_system_program::invoke_cpi::account::CpiContextAccount,
        >,
    > {
        None
    }
}
pub struct LightUpdateRecord {
    #[light_account(mut, seeds = [b"name-service"])]
    pub record: LightAccount<NameRecord>,
}
impl ::light_sdk::compressed_account::LightAccounts for LightUpdateRecord {
    fn try_light_accounts(
        inputs: Vec<Vec<u8>>,
        merkle_context: ::light_sdk::merkle_context::PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: ::light_sdk::merkle_context::PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::try_from_slice_mut(
            inputs[0usize].as_slice(),
            &[b"name-service"],
            &crate::ID,
            &merkle_context,
            merkle_tree_root_index,
            &address_merkle_context,
            remaining_accounts,
        )?;
        Ok(Self { record })
    }
    fn new_address_params(
        &self,
    ) -> Vec<::light_sdk::compressed_account::NewAddressParamsPacked> {
        let mut new_address_params = Vec::new();
        if let Some(new_address_params_for_acc) = self.record.new_address_params() {
            new_address_params.push(new_address_params_for_acc);
        }
        new_address_params
    }
    fn input_accounts(
        &self,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<
        Vec<::light_sdk::compressed_account::PackedCompressedAccountWithMerkleContext>,
    > {
        let mut accounts = Vec::new();
        if let Some(compressed_account) = self
            .record
            .input_compressed_account(&crate::ID, remaining_accounts)?
        {
            accounts.push(compressed_account);
        }
        Ok(accounts)
    }
    fn output_accounts(
        &self,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<
        Vec<::light_sdk::compressed_account::OutputCompressedAccountWithPackedContext>,
    > {
        let mut accounts = Vec::new();
        if let Some(compressed_account) = self
            .record
            .output_compressed_account(&crate::ID, remaining_accounts)?
        {
            accounts.push(compressed_account);
        }
        Ok(accounts)
    }
}
pub struct DeleteRecord<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::NameService>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    pub light_system_program: Program<
        'info,
        ::light_system_program::program::LightSystemProgram,
    >,
    pub system_program: Program<'info, System>,
    pub account_compression_program: Program<
        'info,
        ::account_compression::program::AccountCompression,
    >,
    pub registered_program_pda: Account<'info, ::account_compression::RegisteredProgram>,
    pub noop_program: AccountInfo<'info>,
    pub account_compression_authority: AccountInfo<'info>,
}
#[automatically_derived]
impl<'info> anchor_lang::Accounts<'info, DeleteRecordBumps> for DeleteRecord<'info>
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
        __bumps: &mut DeleteRecordBumps,
        __reallocs: &mut std::collections::BTreeSet<
            anchor_lang::solana_program::pubkey::Pubkey,
        >,
    ) -> anchor_lang::Result<Self> {
        let signer: Signer = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("signer"))?;
        let self_program: anchor_lang::accounts::program::Program<
            crate::program::NameService,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("self_program"))?;
        let cpi_signer: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("cpi_signer"))?;
        let light_system_program: anchor_lang::accounts::program::Program<
            ::light_system_program::program::LightSystemProgram,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("light_system_program"))?;
        let system_program: anchor_lang::accounts::program::Program<System> = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("system_program"))?;
        let account_compression_program: anchor_lang::accounts::program::Program<
            ::account_compression::program::AccountCompression,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("account_compression_program"))?;
        let registered_program_pda: anchor_lang::accounts::account::Account<
            ::account_compression::RegisteredProgram,
        > = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("registered_program_pda"))?;
        let noop_program: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("noop_program"))?;
        let account_compression_authority: AccountInfo = anchor_lang::Accounts::try_accounts(
                __program_id,
                __accounts,
                __ix_data,
                __bumps,
                __reallocs,
            )
            .map_err(|e| e.with_account_name("account_compression_authority"))?;
        if !AsRef::<AccountInfo>::as_ref(&signer).is_writable {
            return Err(
                anchor_lang::error::Error::from(
                        anchor_lang::error::ErrorCode::ConstraintMut,
                    )
                    .with_account_name("signer"),
            );
        }
        Ok(DeleteRecord {
            signer,
            self_program,
            cpi_signer,
            light_system_program,
            system_program,
            account_compression_program,
            registered_program_pda,
            noop_program,
            account_compression_authority,
        })
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountInfos<'info> for DeleteRecord<'info>
where
    'info: 'info,
{
    fn to_account_infos(
        &self,
    ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
        let mut account_infos = ::alloc::vec::Vec::new();
        account_infos.extend(self.signer.to_account_infos());
        account_infos.extend(self.self_program.to_account_infos());
        account_infos.extend(self.cpi_signer.to_account_infos());
        account_infos.extend(self.light_system_program.to_account_infos());
        account_infos.extend(self.system_program.to_account_infos());
        account_infos.extend(self.account_compression_program.to_account_infos());
        account_infos.extend(self.registered_program_pda.to_account_infos());
        account_infos.extend(self.noop_program.to_account_infos());
        account_infos.extend(self.account_compression_authority.to_account_infos());
        account_infos
    }
}
#[automatically_derived]
impl<'info> anchor_lang::ToAccountMetas for DeleteRecord<'info> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
        let mut account_metas = ::alloc::vec::Vec::new();
        account_metas.extend(self.signer.to_account_metas(None));
        account_metas.extend(self.self_program.to_account_metas(None));
        account_metas.extend(self.cpi_signer.to_account_metas(None));
        account_metas.extend(self.light_system_program.to_account_metas(None));
        account_metas.extend(self.system_program.to_account_metas(None));
        account_metas.extend(self.account_compression_program.to_account_metas(None));
        account_metas.extend(self.registered_program_pda.to_account_metas(None));
        account_metas.extend(self.noop_program.to_account_metas(None));
        account_metas.extend(self.account_compression_authority.to_account_metas(None));
        account_metas
    }
}
#[automatically_derived]
impl<'info> anchor_lang::AccountsExit<'info> for DeleteRecord<'info>
where
    'info: 'info,
{
    fn exit(
        &self,
        program_id: &anchor_lang::solana_program::pubkey::Pubkey,
    ) -> anchor_lang::Result<()> {
        anchor_lang::AccountsExit::exit(&self.signer, program_id)
            .map_err(|e| e.with_account_name("signer"))?;
        Ok(())
    }
}
pub struct DeleteRecordBumps {}
#[automatically_derived]
impl ::core::fmt::Debug for DeleteRecordBumps {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "DeleteRecordBumps")
    }
}
impl Default for DeleteRecordBumps {
    fn default() -> Self {
        DeleteRecordBumps {}
    }
}
impl<'info> anchor_lang::Bumps for DeleteRecord<'info>
where
    'info: 'info,
{
    type Bumps = DeleteRecordBumps;
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
pub(crate) mod __client_accounts_delete_record {
    use super::*;
    use anchor_lang::prelude::borsh;
    /// Generated client accounts for [`DeleteRecord`].
    pub struct DeleteRecord {
        pub signer: Pubkey,
        pub self_program: Pubkey,
        pub cpi_signer: Pubkey,
        pub light_system_program: Pubkey,
        pub system_program: Pubkey,
        pub account_compression_program: Pubkey,
        pub registered_program_pda: Pubkey,
        pub noop_program: Pubkey,
        pub account_compression_authority: Pubkey,
    }
    impl borsh::ser::BorshSerialize for DeleteRecord
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
    {
        fn serialize<W: borsh::maybestd::io::Write>(
            &self,
            writer: &mut W,
        ) -> ::core::result::Result<(), borsh::maybestd::io::Error> {
            borsh::BorshSerialize::serialize(&self.signer, writer)?;
            borsh::BorshSerialize::serialize(&self.self_program, writer)?;
            borsh::BorshSerialize::serialize(&self.cpi_signer, writer)?;
            borsh::BorshSerialize::serialize(&self.light_system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.system_program, writer)?;
            borsh::BorshSerialize::serialize(&self.account_compression_program, writer)?;
            borsh::BorshSerialize::serialize(&self.registered_program_pda, writer)?;
            borsh::BorshSerialize::serialize(&self.noop_program, writer)?;
            borsh::BorshSerialize::serialize(
                &self.account_compression_authority,
                writer,
            )?;
            Ok(())
        }
    }
    impl anchor_lang::anchor_syn::idl::build::IdlBuild for DeleteRecord {
        fn __anchor_private_full_path() -> String {
            {
                let res = ::alloc::fmt::format(
                    format_args!(
                        "{0}::{1}",
                        "name_service::__client_accounts_delete_record",
                        "DeleteRecord",
                    ),
                );
                res
            }
        }
        fn __anchor_private_gen_idl_type() -> Option<
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        > {
            Some(anchor_lang::anchor_syn::idl::types::IdlTypeDefinition {
                name: Self::__anchor_private_full_path(),
                generics: None,
                docs: Some(
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            "Generated client accounts for [`DeleteRecord`].".into(),
                        ]),
                    ),
                ),
                ty: anchor_lang::anchor_syn::idl::types::IdlTypeDefinitionTy::Struct {
                    fields: <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "signer".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "selfProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "cpiSigner".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "lightSystemProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "systemProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "accountCompressionProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "registeredProgramPda".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "noopProgram".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                            anchor_lang::anchor_syn::idl::types::IdlField {
                                name: "accountCompressionAuthority".into(),
                                docs: None,
                                ty: anchor_lang::anchor_syn::idl::types::IdlType::PublicKey,
                            },
                        ]),
                    ),
                },
            })
        }
        fn __anchor_private_insert_idl_defined(
            defined_types: &mut std::collections::HashMap<
                String,
                anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
            >,
        ) {}
    }
    #[automatically_derived]
    impl anchor_lang::ToAccountMetas for DeleteRecord {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        self.signer,
                        true,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.self_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.cpi_signer,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.light_system_program,
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
                        self.account_compression_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.registered_program_pda,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.noop_program,
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        self.account_compression_authority,
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
pub(crate) mod __cpi_client_accounts_delete_record {
    use super::*;
    /// Generated CPI struct of the accounts for [`DeleteRecord`].
    pub struct DeleteRecord<'info> {
        pub signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub self_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub cpi_signer: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub light_system_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub system_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub account_compression_program: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub registered_program_pda: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
        pub noop_program: anchor_lang::solana_program::account_info::AccountInfo<'info>,
        pub account_compression_authority: anchor_lang::solana_program::account_info::AccountInfo<
            'info,
        >,
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountMetas for DeleteRecord<'info> {
        fn to_account_metas(
            &self,
            is_signer: Option<bool>,
        ) -> Vec<anchor_lang::solana_program::instruction::AccountMeta> {
            let mut account_metas = ::alloc::vec::Vec::new();
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        anchor_lang::Key::key(&self.signer),
                        true,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.self_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.cpi_signer),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.light_system_program),
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
                        anchor_lang::Key::key(&self.account_compression_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.registered_program_pda),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.noop_program),
                        false,
                    ),
                );
            account_metas
                .push(
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        anchor_lang::Key::key(&self.account_compression_authority),
                        false,
                    ),
                );
            account_metas
        }
    }
    #[automatically_derived]
    impl<'info> anchor_lang::ToAccountInfos<'info> for DeleteRecord<'info> {
        fn to_account_infos(
            &self,
        ) -> Vec<anchor_lang::solana_program::account_info::AccountInfo<'info>> {
            let mut account_infos = ::alloc::vec::Vec::new();
            account_infos
                .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.signer));
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.self_program),
                );
            account_infos
                .extend(anchor_lang::ToAccountInfos::to_account_infos(&self.cpi_signer));
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.light_system_program,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.system_program),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.account_compression_program,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.registered_program_pda,
                    ),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(&self.noop_program),
                );
            account_infos
                .extend(
                    anchor_lang::ToAccountInfos::to_account_infos(
                        &self.account_compression_authority,
                    ),
                );
            account_infos
        }
    }
}
impl<'info> DeleteRecord<'info> {
    pub fn __anchor_private_gen_idl_accounts(
        accounts: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
        defined_types: &mut std::collections::HashMap<
            String,
            anchor_lang::anchor_syn::idl::types::IdlTypeDefinition,
        >,
    ) -> Vec<anchor_lang::anchor_syn::idl::types::IdlAccountItem> {
        {
            <::account_compression::RegisteredProgram>::__anchor_private_insert_idl_defined(
                defined_types,
            );
            let path = <::account_compression::RegisteredProgram>::__anchor_private_full_path();
            <::account_compression::RegisteredProgram>::__anchor_private_gen_idl_type()
                .and_then(|ty| accounts.insert(path, ty));
        }
        <[_]>::into_vec(
            #[rustc_box]
            ::alloc::boxed::Box::new([
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "signer".into(),
                    is_mut: true,
                    is_signer: true,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "selfProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "cpiSigner".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "lightSystemProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "systemProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "accountCompressionProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "registeredProgramPda".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "noopProgram".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
                anchor_lang::anchor_syn::idl::types::IdlAccountItem::IdlAccount(anchor_lang::anchor_syn::idl::types::IdlAccount {
                    name: "accountCompressionAuthority".into(),
                    is_mut: false,
                    is_signer: false,
                    is_optional: None,
                    docs: None,
                    pda: None,
                    relations: ::alloc::vec::Vec::new(),
                }),
            ]),
        )
    }
}
impl<'info> ::light_sdk::traits::InvokeCpiAccounts<'info> for DeleteRecord<'info> {
    fn get_invoking_program(&self) -> &AccountInfo<'info> {
        &self.self_program
    }
}
impl<'info> ::light_sdk::traits::SignerAccounts<'info> for DeleteRecord<'info> {
    fn get_fee_payer(&self) -> &::anchor_lang::prelude::Signer<'info> {
        &self.signer
    }
    fn get_authority(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.cpi_signer
    }
}
impl<'info> ::light_sdk::traits::LightSystemAccount<'info> for DeleteRecord<'info> {
    fn get_light_system_program(
        &self,
    ) -> &::anchor_lang::prelude::Program<
        'info,
        ::light_system_program::program::LightSystemProgram,
    > {
        &self.light_system_program
    }
}
impl<'info> ::light_sdk::traits::InvokeAccounts<'info> for DeleteRecord<'info> {
    fn get_registered_program_pda(
        &self,
    ) -> &::anchor_lang::prelude::Account<
        'info,
        ::account_compression::RegisteredProgram,
    > {
        &self.registered_program_pda
    }
    fn get_noop_program(&self) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.noop_program
    }
    fn get_account_compression_authority(
        &self,
    ) -> &::anchor_lang::prelude::AccountInfo<'info> {
        &self.account_compression_authority
    }
    fn get_account_compression_program(
        &self,
    ) -> &::anchor_lang::prelude::Program<
        'info,
        ::account_compression::program::AccountCompression,
    > {
        &self.account_compression_program
    }
    fn get_system_program(&self) -> &::anchor_lang::prelude::Program<'info, System> {
        &self.system_program
    }
    fn get_compressed_sol_pda(
        &self,
    ) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
        None
    }
    fn get_compression_recipient(
        &self,
    ) -> Option<&::anchor_lang::prelude::AccountInfo<'info>> {
        None
    }
}
impl<'info> ::light_sdk::traits::InvokeCpiContextAccount<'info> for DeleteRecord<'info> {
    fn get_cpi_context_account(
        &self,
    ) -> Option<
        &::anchor_lang::prelude::Account<
            'info,
            ::light_system_program::invoke_cpi::account::CpiContextAccount,
        >,
    > {
        None
    }
}
pub struct LightDeleteRecord {
    #[light_account(close, seeds = [b"name-service"])]
    pub record: LightAccount<NameRecord>,
}
impl ::light_sdk::compressed_account::LightAccounts for LightDeleteRecord {
    fn try_light_accounts(
        inputs: Vec<Vec<u8>>,
        merkle_context: ::light_sdk::merkle_context::PackedMerkleContext,
        merkle_tree_root_index: u16,
        address_merkle_context: ::light_sdk::merkle_context::PackedAddressMerkleContext,
        address_merkle_tree_root_index: u16,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<Self> {
        let record: LightAccount<NameRecord> = LightAccount::try_from_slice_close(
            inputs[0usize].as_slice(),
            &[b"name-service"],
            &crate::ID,
            &merkle_context,
            merkle_tree_root_index,
            &address_merkle_context,
            remaining_accounts,
        )?;
        Ok(Self { record })
    }
    fn new_address_params(
        &self,
    ) -> Vec<::light_sdk::compressed_account::NewAddressParamsPacked> {
        let mut new_address_params = Vec::new();
        if let Some(new_address_params_for_acc) = self.record.new_address_params() {
            new_address_params.push(new_address_params_for_acc);
        }
        new_address_params
    }
    fn input_accounts(
        &self,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<
        Vec<::light_sdk::compressed_account::PackedCompressedAccountWithMerkleContext>,
    > {
        let mut accounts = Vec::new();
        if let Some(compressed_account) = self
            .record
            .input_compressed_account(&crate::ID, remaining_accounts)?
        {
            accounts.push(compressed_account);
        }
        Ok(accounts)
    }
    fn output_accounts(
        &self,
        remaining_accounts: &[::anchor_lang::prelude::AccountInfo],
    ) -> Result<
        Vec<::light_sdk::compressed_account::OutputCompressedAccountWithPackedContext>,
    > {
        let mut accounts = Vec::new();
        if let Some(compressed_account) = self
            .record
            .output_compressed_account(&crate::ID, remaining_accounts)?
        {
            accounts.push(compressed_account);
        }
        Ok(accounts)
    }
}
