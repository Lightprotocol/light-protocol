use light_compressed_account::{
    compressed_account::PackedReadOnlyCompressedAccount,
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsAssignedPacked, NewAddressParamsPacked, PackedReadOnlyAddress},
        invoke_cpi::InstructionDataInvokeCpi,
        with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
    },
};
use light_sdk_types::{
    constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID},
    cpi_context_write::CpiContextWriteAccounts,
};
#[allow(unused_imports)] // TODO: Remove.
use solana_msg::msg;

#[cfg(feature = "v2")]
use crate::cpi::{to_account_metas_small, CpiAccountsSmall};
use crate::{
    cpi::{
        accounts_cpi_context::get_account_metas_from_config_cpi_context,
        get_account_metas_from_config, CpiAccounts, CpiInstructionConfig,
    },
    error::{LightSdkError, Result},
    instruction::{account_info::CompressedAccountInfoTrait, ValidityProof},
    invoke_signed, AccountInfo, AnchorSerialize, Instruction,
};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct CpiInputs {
    pub proof: ValidityProof,
    pub account_infos: Option<Vec<CompressedAccountInfo>>,
    pub read_only_accounts: Option<Vec<PackedReadOnlyCompressedAccount>>,
    pub new_addresses: Option<Vec<NewAddressParamsPacked>>,
    pub new_assigned_addresses: Option<Vec<NewAddressParamsAssignedPacked>>,
    pub read_only_address: Option<Vec<PackedReadOnlyAddress>>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CompressedCpiContext>,
}

/// Builder pattern implementation for CpiInputs.
///
/// This provides a fluent API for constructing CPI inputs with various configurations.
/// The most common pattern is to use one of the constructor methods and then chain
/// builder methods to add additional configuration.
///
/// # Examples
///
/// Most common CPI context usage (no proof, assigned addresses):
/// ```rust
/// let cpi_inputs = CpiInputs::new_for_cpi_context(
///     all_compressed_infos,
///     vec![pool_new_address_params, observation_new_address_params],
/// );
/// ```
///
/// Basic usage with CPI context and custom proof:
/// ```rust
/// let cpi_inputs = CpiInputs::new_with_assigned_address(
///     light_proof,
///     all_compressed_infos,
///     vec![pool_new_address_params, observation_new_address_params],
/// )
/// .with_first_set_cpi_context();
/// ```
///
/// Advanced usage with multiple configurations:
/// ```rust
/// let cpi_inputs = CpiInputs::new(proof, account_infos)
///     .with_first_set_cpi_context()
///     .with_compress_lamports(1000000);
/// ```
impl CpiInputs {
    pub fn new(proof: ValidityProof, account_infos: Vec<CompressedAccountInfo>) -> Self {
        Self {
            proof,
            account_infos: Some(account_infos),
            ..Default::default()
        }
    }

    pub fn new_with_address(
        proof: ValidityProof,
        account_infos: Vec<CompressedAccountInfo>,
        new_addresses: Vec<NewAddressParamsPacked>,
    ) -> Self {
        Self {
            proof,
            account_infos: Some(account_infos),
            new_addresses: Some(new_addresses),
            ..Default::default()
        }
    }

    pub fn new_with_assigned_address(
        proof: ValidityProof,
        account_infos: Vec<CompressedAccountInfo>,
        new_addresses: Vec<NewAddressParamsAssignedPacked>,
    ) -> Self {
        Self {
            proof,
            account_infos: Some(account_infos),
            new_assigned_addresses: Some(new_addresses),
            ..Default::default()
        }
    }

    // TODO: check if always unused!
    /// Creates CpiInputs for the common CPI context pattern: no proof (None),
    /// assigned addresses, and first set CPI context.
    ///
    /// This is the most common pattern when using CPI context for cross-program
    /// compressed account operations.
    ///
    /// # Example
    /// ```rust
    /// let cpi_inputs = CpiInputs::new_for_cpi_context(
    ///     all_compressed_infos,
    ///     vec![user_new_address_params, game_new_address_params],
    /// );
    /// ```
    pub fn new_first_cpi(
        account_infos: Vec<CompressedAccountInfo>,
        new_addresses: Vec<NewAddressParamsAssignedPacked>,
    ) -> Self {
        Self {
            proof: ValidityProof(None),
            account_infos: Some(account_infos),
            new_assigned_addresses: Some(new_addresses),
            cpi_context: Some(CompressedCpiContext {
                set_context: false,
                first_set_context: true,
                cpi_context_account_index: 0, // unused
            }),
            ..Default::default()
        }
    }

    /// Sets a custom CPI context.
    ///
    /// # Example
    /// ```
    /// let cpi_inputs = CpiInputs::new_with_assigned_address(proof, infos, addresses)
    ///     .with_cpi_context(CompressedCpiContext {
    ///         set_context: true,
    ///         first_set_context: false,
    ///         cpi_context_account_index: 1,
    ///     });
    /// ```
    pub fn with_cpi_context(mut self, cpi_context: CompressedCpiContext) -> Self {
        self.cpi_context = Some(cpi_context);
        self
    }

    // TODO: check if always unused!
    /// Sets CPI context to first set context (clears any existing context).
    /// This is the most common pattern for initializing CPI context.
    ///
    /// # Example
    /// ```
    /// let cpi_inputs = CpiInputs::new_with_assigned_address(proof, infos, addresses)
    ///     .with_first_set_cpi_context();
    /// ```
    pub fn with_first_set_cpi_context(mut self) -> Self {
        self.cpi_context = Some(CompressedCpiContext {
            set_context: false,
            first_set_context: true,
            cpi_context_account_index: 0, // unused.
        });
        self
    }

    /// Sets CPI context to set context (updates existing context).
    /// Use this when you want to update an existing CPI context.
    ///
    /// # Example
    /// ```
    /// let cpi_inputs = CpiInputs::new_with_assigned_address(proof, infos, addresses)
    ///     .with_set_cpi_context(0);
    /// ```
    pub fn with_last_cpi_context(mut self, cpi_context_account_index: u8) -> Self {
        self.cpi_context = Some(CompressedCpiContext {
            set_context: true,
            first_set_context: false,
            cpi_context_account_index,
        });
        self
    }

    pub fn invoke_light_system_program(self, cpi_accounts: CpiAccounts<'_, '_>) -> Result<()> {
        let bump = cpi_accounts.bump();
        let account_infos = cpi_accounts.to_account_infos();
        let instruction = create_light_system_progam_instruction_invoke_cpi(self, cpi_accounts)?;
        invoke_light_system_program(account_infos.as_slice(), instruction, bump)
    }

    #[cfg(feature = "v2")]
    pub fn invoke_light_system_program_small(
        self,
        cpi_accounts: CpiAccountsSmall<'_, '_>,
    ) -> Result<()> {
        let bump = cpi_accounts.bump();
        let account_infos = cpi_accounts.to_account_infos();
        let instruction =
            create_light_system_progam_instruction_invoke_cpi_small(self, cpi_accounts)?;
        invoke_light_system_program(account_infos.as_slice(), instruction, bump)
    }
    #[inline(never)]
    #[cold]
    pub fn invoke_light_system_program_cpi_context(
        self,
        cpi_accounts: CpiContextWriteAccounts<AccountInfo>,
    ) -> Result<()> {
        let bump = cpi_accounts.bump();
        let account_infos = cpi_accounts.to_account_infos();
        let instruction =
            create_light_system_progam_instruction_invoke_cpi_context_write(self, cpi_accounts)?;
        invoke_light_system_program(account_infos.as_slice(), instruction, bump)
    }
}

#[cfg(feature = "v2")]
pub fn create_light_system_progam_instruction_invoke_cpi_small(
    cpi_inputs: CpiInputs,
    cpi_accounts: CpiAccountsSmall<'_, '_>,
) -> Result<Instruction> {
    if cpi_inputs.new_addresses.is_some() {
        unimplemented!("new_addresses must be new assigned addresses.");
    }

    let inputs = InstructionDataInvokeCpiWithAccountInfo {
        proof: cpi_inputs.proof.into(),
        mode: 1,
        bump: cpi_accounts.bump(),
        invoking_program_id: cpi_accounts.invoking_program().into(),
        new_address_params: cpi_inputs.new_assigned_addresses.unwrap_or_default(),
        read_only_accounts: cpi_inputs.read_only_accounts.unwrap_or_default(),
        read_only_addresses: cpi_inputs.read_only_address.unwrap_or_default(),
        account_infos: cpi_inputs.account_infos.unwrap_or_default(),
        with_transaction_hash: false,
        compress_or_decompress_lamports: cpi_inputs
            .compress_or_decompress_lamports
            .unwrap_or_default(),
        is_compress: cpi_inputs.is_compress,
        with_cpi_context: cpi_inputs.cpi_context.is_some(),
        cpi_context: cpi_inputs.cpi_context.unwrap_or_default(),
    };
    // TODO: bench vs zero copy and set.
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let mut data = Vec::with_capacity(8 + inputs.len());
    data.extend_from_slice(
        &light_compressed_account::discriminators::INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION,
    );
    data.extend(inputs);

    let account_metas = to_account_metas_small(cpi_accounts)?;

    Ok(Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    })
}

#[inline(never)]
#[cold]
pub fn create_light_system_progam_instruction_invoke_cpi_context_write(
    cpi_inputs: CpiInputs,
    cpi_accounts: CpiContextWriteAccounts<AccountInfo>,
) -> Result<Instruction> {
    if cpi_inputs.new_addresses.is_some() {
        unimplemented!("new_addresses must be new assigned addresses.");
    }

    let inputs = InstructionDataInvokeCpiWithAccountInfo {
        proof: cpi_inputs.proof.into(),
        mode: 1,
        bump: cpi_accounts.bump(),
        invoking_program_id: cpi_accounts.invoking_program().into(),
        new_address_params: cpi_inputs.new_assigned_addresses.unwrap_or_default(),
        read_only_accounts: cpi_inputs.read_only_accounts.unwrap_or_default(),
        read_only_addresses: cpi_inputs.read_only_address.unwrap_or_default(),
        account_infos: cpi_inputs.account_infos.unwrap_or_default(),
        with_transaction_hash: false,
        compress_or_decompress_lamports: cpi_inputs
            .compress_or_decompress_lamports
            .unwrap_or_default(),
        is_compress: cpi_inputs.is_compress,
        with_cpi_context: cpi_inputs.cpi_context.is_some(),
        cpi_context: cpi_inputs.cpi_context.unwrap_or_default(),
    };
    // TODO: bench vs zero copy and set.
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let mut data = Vec::with_capacity(8 + inputs.len());
    data.extend_from_slice(
        &light_compressed_account::discriminators::INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION,
    );
    data.extend(inputs);

    let account_metas = get_account_metas_from_config_cpi_context(cpi_accounts);
    Ok(Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas.to_vec(),
        data,
    })
}

pub fn create_light_system_progam_instruction_invoke_cpi(
    cpi_inputs: CpiInputs,
    cpi_accounts: CpiAccounts<'_, '_>,
) -> Result<Instruction> {
    let owner = *cpi_accounts.invoking_program()?.key;
    let (input_compressed_accounts_with_merkle_context, output_compressed_accounts) =
        if let Some(account_infos) = cpi_inputs.account_infos.as_ref() {
            let mut input_compressed_accounts_with_merkle_context =
                Vec::with_capacity(account_infos.len());
            let mut output_compressed_accounts = Vec::with_capacity(account_infos.len());
            for account_info in account_infos.iter() {
                if let Some(input_account) =
                    account_info.input_compressed_account(owner.to_bytes().into())?
                {
                    input_compressed_accounts_with_merkle_context.push(input_account);
                }
                if let Some(output_account) =
                    account_info.output_compressed_account(owner.to_bytes().into())?
                {
                    output_compressed_accounts.push(output_account);
                }
            }
            (
                input_compressed_accounts_with_merkle_context,
                output_compressed_accounts,
            )
        } else {
            (vec![], vec![])
        };
    #[cfg(not(feature = "v2"))]
    if cpi_inputs.read_only_accounts.is_some() {
        unimplemented!("read_only_accounts are only supported with v2 soon on Devnet.");
    }
    #[cfg(not(feature = "v2"))]
    if cpi_inputs.read_only_address.is_some() {
        unimplemented!("read_only_addresses are only supported with v2 soon on Devnet.");
    }

    let inputs = InstructionDataInvokeCpi {
        proof: cpi_inputs.proof.into(),
        new_address_params: cpi_inputs.new_addresses.unwrap_or_default(),
        relay_fee: None,
        input_compressed_accounts_with_merkle_context,
        output_compressed_accounts,
        compress_or_decompress_lamports: cpi_inputs.compress_or_decompress_lamports,
        is_compress: cpi_inputs.is_compress,
        cpi_context: cpi_inputs.cpi_context,
    };
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let mut data = Vec::with_capacity(8 + 4 + inputs.len());
    data.extend_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
    data.extend(inputs);

    let config = CpiInstructionConfig::try_from(&cpi_accounts)?;

    let account_metas = get_account_metas_from_config(config);

    Ok(Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    })
}

/// Invokes the light system program to verify and apply a zk-compressed state
/// transition. Serializes CPI instruction data, configures necessary accounts,
/// and executes the CPI.
pub fn verify_borsh<T>(cpi_accounts: CpiAccounts, inputs: &T) -> Result<()>
where
    T: AnchorSerialize,
{
    let inputs = inputs.try_to_vec().map_err(|_| LightSdkError::Borsh)?;

    let mut data = Vec::with_capacity(8 + 4 + inputs.len());
    data.extend_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    data.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
    data.extend(inputs);
    let account_infos = cpi_accounts.to_account_infos();

    let bump = cpi_accounts.bump();
    let config = CpiInstructionConfig::try_from(&cpi_accounts)?;
    let account_metas = get_account_metas_from_config(config);

    let instruction = Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    };
    invoke_light_system_program(account_infos.as_slice(), instruction, bump)
}

#[inline(always)]
pub fn invoke_light_system_program(
    account_infos: &[AccountInfo],
    instruction: Instruction,
    bump: u8,
) -> Result<()> {
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];

    // TODO: restore but not a priority it is a convenience check
    // It's index 0 for small instruction accounts.
    // if *account_infos[1].key != authority {
    //     #[cfg(feature = "anchor")]
    //     anchor_lang::prelude::msg!(
    //         "System program signer authority is invalid. Expected {:?}, found {:?}",
    //         authority,
    //         account_infos[1].key
    //     );
    //     #[cfg(feature = "anchor")]
    //     anchor_lang::prelude::msg!(
    //         "Seeds to derive expected pubkey: [CPI_AUTHORITY_PDA_SEED] {:?}",
    //         [CPI_AUTHORITY_PDA_SEED]
    //     );
    //     return Err(LightSdkError::InvalidCpiSignerAccount);
    // }

    invoke_signed(&instruction, account_infos, &[signer_seeds.as_slice()])?;
    Ok(())
}
