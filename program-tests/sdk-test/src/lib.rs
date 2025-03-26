use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    compressed_account::{PackedReadOnlyCompressedAccount, ReadOnlyCompressedAccount},
    instruction_data::{
        account_info::{SystemInfoInstructionData, ZCAccountInfoMut, ZCInAccountInfoMut},
        compressed_proof::{CompressedProof, ZCompressedProof},
        data::{NewAddressParamsPacked, PackedReadOnlyAddress},
        meta::{
            InputAccountMetaWithAddressNoLamports, ZInputAccountMetaTrait,
            ZInputAccountMetaWithAddressNoLamports,
        },
    },
    pubkey::Pubkey,
    CompressedAccountError,
};
use light_hasher::{DataHasher, Discriminator, Poseidon};
use light_macros::pubkey;
use light_sdk::{
    account_info::{LightAccountInfo, LightInputAccountInfo},
    address::derive_address,
    error::LightSdkError,
    instruction_data::LightInstructionData,
    program_merkle_context::unpack_address_merkle_context,
    system_accounts::{LightCpiAccounts, SystemAccountInfoConfig},
    verify::{verify_light_account_infos, verify_system_info},
    LightDiscriminator, LightHasher,
};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, ZeroCopy, ZeroCopyEq};
use solana_program::{
    account_info::AccountInfo, entrypoint, log::sol_log_compute_units, program_error::ProgramError,
};
pub const ID: solana_program::pubkey::Pubkey =
    pubkey!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

#[repr(u8)]
pub enum InstructionType {
    CreatePdaBorsh = 0,
    UpdatePdaBorsh = 1,
    // TODO: add CreatePdaZeroCopy
}

impl TryFrom<u8> for InstructionType {
    type Error = LightSdkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InstructionType::CreatePdaBorsh),
            _ => panic!("Invalid instruction discriminator."),
        }
    }
}

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &solana_program::pubkey::Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    let discriminator = InstructionType::try_from(instruction_data[0]).unwrap();
    match discriminator {
        InstructionType::CreatePdaBorsh => create_pda(accounts, &instruction_data[1..]),
        InstructionType::UpdatePdaBorsh => {
            update_pda_with_light_account_loader(accounts, &instruction_data[1..])
        }
    }?;
    Ok(())
}

pub fn create_pda(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), LightSdkError> {
    let (inputs, instruction_data) = LightInstructionData::zero_copy_at(instruction_data).unwrap();

    let address_merkle_context = unpack_address_merkle_context(
        inputs.new_addresses.as_ref().unwrap()[0].clone().into(),
        &accounts[9..],
    );
    solana_program::msg!(
        "create_pda address_merkle_context {:?}",
        address_merkle_context
    );
    let account_data = &instruction_data[..31];
    solana_program::msg!("create_pda account_data {:?}", account_data);
    let (address, address_seed) = derive_address(
        &[b"compressed", account_data],
        &address_merkle_context,
        &crate::ID,
    );
    solana_program::msg!("create_pda address {:?}", address);
    solana_program::msg!("create_pda address_seed {:?}", address_seed);

    let my_compressed_account = MyCompressedAccount {
        signer: (*accounts[0].key).into(),
        data: account_data.try_into().unwrap(),
    };
    let account_info = LightAccountInfo::init_with_address(
        &crate::ID,
        MyCompressedAccount::discriminator(),
        my_compressed_account.try_to_vec().unwrap(),
        // TODO: make poseidon default, and change to hash_with::<GenericHasher>
        my_compressed_account.hash::<Poseidon>().unwrap(),
        address,
        0,
    );

    let config = SystemAccountInfoConfig {
        self_program: crate::ID,
        cpi_context: false,
        sol_pool_pda: false,
        sol_compression_recipient: false,
    };
    let light_cpi_accounts =
        LightCpiAccounts::new_with_config(&accounts[0], &accounts[1..], config)?;
    solana_program::msg!("my_compressed_account {:?}", my_compressed_account);
    let address_context = &inputs.new_addresses.unwrap()[0];
    verify_light_account_infos(
        &light_cpi_accounts,
        inputs.proof.map(|x| x.into()),
        &[account_info],
        Some(vec![NewAddressParamsPacked {
            seed: address_seed,
            address_queue_account_index: address_context.address_queue_pubkey_index,
            address_merkle_tree_account_index: address_context.address_merkle_tree_pubkey_index,
            address_merkle_tree_root_index: address_context.root_index.into(),
        }]),
        None,
        false,
        None,
    )
}

// pub fn update_pda(accounts: &[AccountInfo], instruction_data: &[u8]) -> Result<(), LightSdkError> {
//     let (instruction_data, inputs) = LightInstructionData::deserialize(instruction_data)?;
//     let (instruction_data, _) = UpdateInstructionData::zero_copy_at(instruction_data).unwrap();

//     let mut input_my_compressed_account_bytes =
//         vec![0u8; std::mem::size_of::<MyCompressedAccount>()];
//     let (mut input_my_compressed_account, _) =
//         MyCompressedAccount::zero_copy_at_mut(input_my_compressed_account_bytes.as_mut_slice())
//             .unwrap();
//     input_my_compressed_account.signer = (*accounts[0].key).into();
//     input_my_compressed_account.data = *instruction_data.new_data;

//     // Should do sth about type conversions, could use const generics to signal a mode.
//     // Could implement into LightInputAccountInfo. Could we derive it? #[into_light_input_account_info] (will detect whether lamports, etc exist)
//     let input_metadata = LightInputAccountInfo::from_input_account_meta_with_address_no_lamports(
//         &instruction_data.input_compressed_account.meta,
//         input_my_compressed_account.hash::<Poseidon>().unwrap(),
//     )
//     .unwrap();

//     let mut account_info = LightAccountInfo::from_meta_mut(
//         input_metadata,
//         &crate::ID,
//         // We need to clone the bytes if we use borsh.
//         // my_compressed_account.try_to_vec().unwrap(),
//         input_my_compressed_account_bytes,
//         MyCompressedAccount::discriminator(),
//         0,
//     )
//     .unwrap();
//     {
//         // Ugly af, can be avoided by separating input and output accounts.
//         let data_slice: &mut [u8] = &mut account_info.data.as_mut().unwrap().borrow_mut();
//         let (mut my_account, _) = MyCompressedAccount::zero_copy_at_mut(data_slice).unwrap();
//         my_account.data = *instruction_data.new_data;
//     }
//     let light_cpi_accounts = LightCpiAccounts::new(&accounts[0], &accounts[1..], crate::ID)?;
//     // solana_program::msg!("my_compressed_account {:?}", my_compressed_account);
//     verify_light_account_infos(
//         &light_cpi_accounts,
//         inputs.proof,
//         &[account_info],
//         None,
//         None,
//         false,
//         None,
//     )
// }

// pub fn update_pda_borsh(
//     accounts: &[AccountInfo],
//     instruction_data: &[u8],
// ) -> Result<(), LightSdkError> {
//     let (instruction_data, inputs) = LightInstructionData::deserialize(instruction_data)?;
//     let (instruction_data, _) = UpdateInstructionData::zero_copy_at(instruction_data).unwrap();
//     let input_my_compressed_account = MyCompressedAccount {
//         signer: (*accounts[0].key).into(),
//         data: instruction_data.input_compressed_account.data,
//     };
//     // Should do sth about type conversions, could use const generics to signal a mode.
//     // Could implement into LightInputAccountInfo. Could we derive it? #[into_light_input_account_info] (will detect whether lamports, etc exist)
//     let input_metadata = LightInputAccountInfo::from_input_account_meta_with_address_no_lamports(
//         &instruction_data.input_compressed_account.meta,
//         input_my_compressed_account.hash::<Poseidon>().unwrap(),
//     )
//     .unwrap();

//     let mut account_info = LightAccountInfo::from_meta_mut(
//         input_metadata,
//         &crate::ID,
//         // We need to clone the bytes if we use borsh.
//         input_my_compressed_account.try_to_vec().unwrap(),
//         MyCompressedAccount::discriminator(),
//         0,
//     )
//     .unwrap();
//     {
//         // Ugly af, can be avoided by separating input and output accounts.
//         let data_slice = account_info.data.as_mut().unwrap().borrow_mut();
//         let mut my_account = MyCompressedAccount::deserialize(&mut &data_slice[..]).unwrap();
//         my_account.data = *instruction_data.new_data;
//     }
//     let light_cpi_accounts = LightCpiAccounts::new(&accounts[0], &accounts[1..], crate::ID)?;
//     // solana_program::msg!("my_compressed_account {:?}", my_compressed_account);
//     verify_light_account_infos(
//         &light_cpi_accounts,
//         inputs.proof,
//         &[account_info],
//         None,
//         None,
//         false,
//         None,
//     )
// }

// pub struct AccountContext<'a> {
//     des_instruction_data: <UpdateInstructionData as Deserialize>::Output,
//     cpi_data: Vec<u8>,
//     system_account_infos: &'a [AccountInfo<'a>],
//     // account_context: CustomAccountContextStruct,
// }

// impl<'a, 'info> AccountContext<'a> {
//     fn get_context(
//         accounts: &'a [AccountInfo<'info>],
//         instruction_data: &'a [u8],
//     ) -> (Self, &'a [AccountInfo<'info>], &'a [u8]) {
//         let (des_instruction_data, remaining_data) =
//             UpdateInstructionData::zero_copy_at(remaining_data).unwrap();
//         (
//             Self {
//                 system_instruction_data,
//                 des_instruction_data,
//             },
//             accounts,
//             remaining_data,
//         )
//     }

//     // pub fn get_cpi_data()
// }

// pub fn update_pda_with_cpi_data(
//     accounts: &[AccountInfo],
//     instruction_data: &[u8],
// ) -> Result<(), LightSdkError> {
//     let (instruction_data, _) = UpdateInstructionData::zero_copy_at(instruction_data).unwrap();

//     let mut input_my_compressed_account_bytes =
//         vec![0u8; std::mem::size_of::<MyCompressedAccount>()];

//     // TODO: replace with SystemInfoInstructionData::bytes_required_for_capacity
//     let instruction_data_capacity = 10480;
//     let mut vec = vec![0u8; instruction_data_capacity];
//     // This would be new not mut
//     // We need a generic config.
//     // An array for Compressed accounts which define the type of the account.
//     // With the generics we can initialize the cpi data correctly, and provide correct init and accessor methods for the accounts.
//     // Generics {
//     //    CompressedAccountType: MyCompressedAccount, Init (no input, address: bool), Close (no output, address: bool), Mut(in and output, address: bool)
//     // }
//     let (mut cpi_data, _) =
//         SystemInfoInstructionData::zero_copy_at_mut(vec.as_mut_slice()).unwrap();
//     // Steps:
//     // 1. build input account from onchain and instruction data
//     // 2. hash input account
//     // 3. copy input account into cpi data
//     // 4. init output account in cpi data
//     // 5. modify output account
//     // 6. hash output account
//     // 7. cpi system program
//     {
//         let mut my_compressed_account_bytes = vec![0u8; std::mem::size_of::<MyCompressedAccount>()];
//         let (mut my_account, _) =
//             MyCompressedAccount::zero_copy_at_mut(my_compressed_account_bytes.as_mut_slice())
//                 .unwrap();

//         my_account.signer = (*accounts[0].key).into();
//         my_account.data = *instruction_data.new_data;
//         let input_hash = my_account.hash::<Poseidon>().unwrap();

//         my_account.data = *instruction_data.new_data;

//         // Questions:
//         // 1. should from_z_meta_mut take data hasha nd data as inputs?
//         // 2. footguns:
//         //      1. no hashing of data
//         //      2. modifying the data after hashing
//         //      3. modify prior to adding input data.
//         cpi_data.light_account_infos[0]
//             .from_z_meta_mut(
//                 &instruction_data.input_compressed_account.meta,
//                 input_hash,
//                 MyCompressedAccount::discriminator(),
//                 my_compressed_account_bytes,
//                 instruction_data.output_merkle_tree_index,
//             )
//             .unwrap();
//     }
//     let light_cpi_accounts = LightCpiAccounts::new(&accounts[0], &accounts[1..], crate::ID)?;
//     verify_system_info(&light_cpi_accounts, vec)
// }

pub fn update_pda_with_light_account_loader(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let (instruction_data, remaining) =
        UpdateInstructionData::zero_copy_at(instruction_data).unwrap();
    let program_id = crate::ID.into();

    // TODO: replace with SystemInfoInstructionData::bytes_required_for_capacity
    let instruction_data_capacity = 10480;
    let mut cpi_data_bytes = vec![0u8; instruction_data_capacity];
    // This would be new not mut
    // We need a generic config.
    // An array for Compressed accounts which define the type of the account.
    // With the generics we can initialize the cpi data correctly, and provide correct init and accessor methods for the accounts.
    // Generics {
    //    CompressedAccountType: MyCompressedAccount, Init (no input, address: bool), Close (no output, address: bool), Mut(in and output, address: bool)
    // }
    let (mut cpi_data, _) =
        SystemInfoInstructionData::zero_copy_at_mut(cpi_data_bytes.as_mut_slice()).unwrap();
    // Steps:
    // 1. build input account from onchain and instruction data
    // 2. hash input account
    // 3. copy input account into cpi data
    // 4. init output account in cpi data
    // 5. modify output account
    // 6. hash output account
    // 7. cpi system program
    {
        let mut loader = CAccountLoader::<
            ZInputAccountMetaWithAddressNoLamports,
            MyCompressedAccount,
        >::from_cpi_account_info(
            &mut cpi_data.light_account_infos[0], &program_id
        );

        let mut my_account = loader
            .load_mut(
                &instruction_data.input_compressed_account.meta,
                MyCompressedAccount {
                    signer: (*accounts[0].key).into(),
                    data: *instruction_data.new_data,
                },
                instruction_data.output_merkle_tree_index,
            )
            .unwrap();

        my_account.data = *instruction_data.new_data;

        let output_hasher = my_account.hash::<Poseidon>().unwrap();

        loader.finalize(output_hasher).unwrap();
    }

    let light_cpi_accounts = LightCpiAccounts::new(&accounts[0], &accounts[1..], crate::ID)?;
    verify_system_info(&light_cpi_accounts, cpi_data_bytes)
}

// pub Instruction {
//     #[constraint= signer = accounts[0].key]
//     #[constraint= address = instruction_data.input_compressed_account.meta.address]
//     #[constraint= address = instruction_data.input_compressed_account.meta.address]
//     pub CompressedAccount
// }

// TODO: make loader from ZCAccountInfoMut<'a> -> so that we work over cpi memory
pub struct CAccountLoader<
    'a,
    'b,
    M: ZInputAccountMetaTrait<'a>,
    A: DeserializeMut + Deserialize + Discriminator + DataHasher,
> {
    pub owner: &'a Pubkey,
    pub(crate) account: &'a mut ZCAccountInfoMut<'a>,
    /// For close accounts we store data here.
    close_data: Option<Vec<u8>>,
    finalized: bool,

    phantom_a: std::marker::PhantomData<A>,
    phantom_m: std::marker::PhantomData<M>,
    phantom_b: std::marker::PhantomData<&'b ()>,
}

impl<
        'a,
        'b,
        M: ZInputAccountMetaTrait<'a>,
        A: DeserializeMut + Deserialize + Discriminator + DataHasher,
    > CAccountLoader<'a, 'b, M, A>
{
    pub fn from_cpi_account_info(account: &'a mut ZCAccountInfoMut<'a>, owner: &'a Pubkey) -> Self {
        Self {
            owner,
            account,
            close_data: None,
            // loaded: None,
            finalized: false,
            phantom_a: std::marker::PhantomData,
            phantom_m: std::marker::PhantomData,
            phantom_b: std::marker::PhantomData,
        }
    }

    // TODO: add
    // load_init, load_close
    pub fn load_mut(
        &mut self,
        input_account_meta: &M,
        input_account: A,
        output_merkle_tree_index: u8,
    ) -> Result<<A as DeserializeMut>::Output<'_>, CompressedAccountError> {
        let input_data_hash = input_account.hash::<Poseidon>().unwrap();
        // Does not set the output data.
        // Can only be called once.
        self.account.from_z_meta_mut(
            input_account_meta,
            input_data_hash,
            A::discriminator(),
            output_merkle_tree_index,
        )?;
        // TODO: set output data.
        // A::new(&self.data[..], A)

        Ok(
            A::zero_copy_at_mut(&mut self.account.output.as_mut().unwrap().data[..])
                .unwrap()
                .0,
        )
    }

    /// Finalize a mut or init loaded account.
    /// Closed accounts are already finalized.
    pub fn finalize(&mut self, output_hash: [u8; 32]) -> Result<(), CompressedAccountError> {
        if self.finalized {
            solana_program::msg!("Already finalized");
            return Err(CompressedAccountError::InvalidArgument);
        }
        if let Some(output) = self.account.output.as_mut() {
            output.data_hash = output_hash;
        }
        self.finalized = true;
        Ok(())
    }

    // pub fn load_mut(
    //     &'a mut self,
    //     meta: M,
    //     params: A,
    //     output_merkle_tree_index: u8,
    // ) -> <A as DeserializeMut>::Output {
    //     // call new and return A as zero copy
    //     A::zero_copy_at_mut(&mut self.account.output.as_mut().unwrap().data[..])
    //         .unwrap()
    //         .0
    // }

    // pub fn get_zero_copy(&'a self) -> <A as Deserialize>::Output {
    //     A::zero_copy_at(&self.data[..]).unwrap().0
    // }

    // pub fn get_zero_copy_mut(&'a mut self) -> <A as DeserializeMut>::Output {
    //     A::zero_copy_at_mut(&mut self.data[..]).unwrap().0
    // }
}

// Safeguard, Accountloader has to be finalized before dropping.
impl<
        'a,
        M: ZInputAccountMetaTrait<'a>,
        A: DeserializeMut + Deserialize + Discriminator + DataHasher,
    > Drop for CAccountLoader<'a, '_, M, A>
{
    fn drop(&mut self) {
        #[cfg(target_os = "solana")]
        if !self.finalized {
            panic!("Dropped Accounloader without finalizing.")
        }
    }
}

// TODO: add account traits
#[derive(
    Clone,
    Debug,
    Default,
    LightHasher,
    LightDiscriminator,
    BorshDeserialize,
    BorshSerialize,
    ZeroCopy,
    ZeroCopyEq,
)]
#[poseidon]
pub struct MyCompressedAccount {
    signer: Pubkey,
    data: [u8; 31],
}

#[derive(Debug, ZeroCopy)]
pub struct UpdateInstructionData {
    pub input_compressed_account: InputMyCompressedAccountWithContext,
    pub new_data: [u8; 31],
    pub output_merkle_tree_index: u8,
}

#[derive(Debug, ZeroCopy)]
pub struct InputMyCompressedAccountWithContext {
    pub data: [u8; 31],
    pub meta: InputAccountMetaWithAddressNoLamports,
}
