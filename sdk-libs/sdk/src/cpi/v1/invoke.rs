use light_compressed_account::instruction_data::{
    compressed_proof::ValidityProof, invoke_cpi::InstructionDataInvokeCpi,
};
use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};

#[cfg(feature = "poseidon")]
use crate::{account::poseidon::LightAccount as LightAccountPoseidon, DataHasher};
use crate::{
    account::LightAccount,
    cpi::CpiSigner,
    error::LightSdkError,
    instruction::account_info::CompressedAccountInfoTrait,
    AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError,
};

use super::{
    lowlevel::{get_account_metas_from_config, CpiInstructionConfig},
    CpiAccounts,
};

/// Light system program CPI instruction data builder.
///
/// Use this builder to construct instructions for compressed account operations:
/// creating, updating, closing accounts, and compressing/decompressing SOL.
///
/// # Builder Methods
///
/// ## Common Methods
///
/// - [`with_light_account()`](Self::with_light_account) - Add a compressed account
/// - [`with_new_addresses()`](Self::with_new_addresses) - Create new compressed account addresses
/// - [`compress_lamports()`](Self::compress_lamports) - Compress SOL into compressed accounts
/// - [`decompress_lamports()`](Self::decompress_lamports) - Decompress SOL from compressed accounts
///
/// **Note**: An instruction can either compress **or** decompress lamports, not both.
///
/// ## Advanced Methods
///
/// For fine-grained control, use these low-level methods instead of [`with_light_account()`](Self::with_light_account):
///
/// - [`with_input_compressed_accounts_with_merkle_context()`](Self::with_input_compressed_accounts_with_merkle_context) - Manually specify input accounts
/// - [`with_output_compressed_accounts()`](Self::with_output_compressed_accounts) - Manually specify output accounts
///
/// # Examples
///
/// ## Create a compressed account with an address
/// ```rust,no_run
/// # use light_sdk::cpi::{v1::LightSystemProgramCpi, CpiSigner};
/// # use light_sdk::instruction::ValidityProof;
/// # use light_compressed_account::instruction_data::data::NewAddressParamsPacked;
/// # use light_sdk::{LightAccount, LightDiscriminator};
/// # use borsh::{BorshSerialize, BorshDeserialize};
/// # use solana_pubkey::Pubkey;
/// # use solana_program_error::ProgramError;
/// #
/// # const LIGHT_CPI_SIGNER: CpiSigner = CpiSigner {
/// #     program_id: [0; 32],
/// #     cpi_signer: [0; 32],
/// #     bump: 255,
/// # };
/// #
/// # #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
/// # pub struct MyAccount {
/// #     pub value: u64,
/// # }
/// #
/// # fn example() -> Result<(), ProgramError> {
/// # let proof = ValidityProof::default();
/// # let new_address_params = NewAddressParamsPacked::default();
/// # let program_id = Pubkey::new_unique();
/// # let account = LightAccount::<MyAccount>::new_init(&program_id, None, 0);
/// # let key = Pubkey::new_unique();
/// # let owner = Pubkey::default();
/// # let mut lamports = 0u64;
/// # let mut data = [];
/// # let fee_payer = &solana_account_info::AccountInfo::new(
/// #     &key,
/// #     true,
/// #     true,
/// #     &mut lamports,
/// #     &mut data,
/// #     &owner,
/// #     false,
/// #     0,
/// # );
/// # let cpi_accounts = light_sdk::cpi::v1::CpiAccounts::new(fee_payer, &[], LIGHT_CPI_SIGNER);
/// LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof)
///     .with_new_addresses(&[new_address_params])
///     .with_light_account(account)?
///     .invoke(cpi_accounts)?;
/// # Ok(())
/// # }
/// ```
/// ## Update a compressed account
/// ```rust,no_run
/// # use light_sdk::cpi::{v1::LightSystemProgramCpi, CpiSigner};
/// # use light_sdk::instruction::ValidityProof;
/// # use light_sdk::{LightAccount, LightDiscriminator};
/// # use light_sdk::instruction::account_meta::CompressedAccountMeta;
/// # use borsh::{BorshSerialize, BorshDeserialize};
/// # use solana_pubkey::Pubkey;
/// # use solana_program_error::ProgramError;
/// #
/// # const LIGHT_CPI_SIGNER: CpiSigner = CpiSigner {
/// #     program_id: [0; 32],
/// #     cpi_signer: [0; 32],
/// #     bump: 255,
/// # };
/// #
/// # #[derive(Clone, Debug, Default, LightDiscriminator, BorshSerialize, BorshDeserialize)]
/// # pub struct MyAccount {
/// #     pub value: u64,
/// # }
/// #
/// # fn example() -> Result<(), ProgramError> {
/// # let proof = ValidityProof::default();
/// # let program_id = Pubkey::new_unique();
/// # let account_meta = CompressedAccountMeta::default();
/// # let account_data = MyAccount::default();
/// # let account = LightAccount::<MyAccount>::new_mut(&program_id, &account_meta, account_data)?;
/// # let key = Pubkey::new_unique();
/// # let owner = Pubkey::default();
/// # let mut lamports = 0u64;
/// # let mut data = [];
/// # let fee_payer = &solana_account_info::AccountInfo::new(
/// #     &key,
/// #     true,
/// #     true,
/// #     &mut lamports,
/// #     &mut data,
/// #     &owner,
/// #     false,
/// #     0,
/// # );
/// # let cpi_accounts = light_sdk::cpi::v1::CpiAccounts::new(fee_payer, &[], LIGHT_CPI_SIGNER);
/// LightSystemProgramCpi::new_cpi(LIGHT_CPI_SIGNER, proof)
///     .with_light_account(account)?
///     .invoke(cpi_accounts)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct LightSystemProgramCpi {
    cpi_signer: CpiSigner,
    instruction_data: InstructionDataInvokeCpi,
}

impl LightSystemProgramCpi {
    pub fn new_cpi(cpi_signer: CpiSigner, proof: ValidityProof) -> Self {
        Self {
            cpi_signer,
            instruction_data: InstructionDataInvokeCpi::new(proof.into()),
        }
    }

    #[must_use = "with_light_account returns a new value"]
    pub fn with_light_account<A>(mut self, account: LightAccount<A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default,
    {
        use light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext;

        let account_info = account.to_account_info()?;

        if let Some(input_account) = account_info
            .input_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            let packed_input = PackedCompressedAccountWithMerkleContext {
                compressed_account: input_account.compressed_account,
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                read_only: false,
            };
            self.instruction_data
                .input_compressed_accounts_with_merkle_context
                .push(packed_input);
        }

        if let Some(output_account) = account_info
            .output_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.instruction_data
                .output_compressed_accounts
                .push(output_account);
        }

        Ok(self)
    }

    #[cfg(feature = "poseidon")]
    #[must_use = "with_light_account_poseidon returns a new value"]
    pub fn with_light_account_poseidon<A>(
        mut self,
        account: LightAccountPoseidon<A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default,
    {
        use light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext;

        let account_info = account.to_account_info()?;

        if let Some(input_account) = account_info
            .input_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            let packed_input = PackedCompressedAccountWithMerkleContext {
                compressed_account: input_account.compressed_account,
                merkle_context: input_account.merkle_context,
                root_index: input_account.root_index,
                read_only: false,
            };
            self.instruction_data
                .input_compressed_accounts_with_merkle_context
                .push(packed_input);
        }

        if let Some(output_account) = account_info
            .output_compressed_account(self.cpi_signer.program_id.into())
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?
        {
            self.instruction_data
                .output_compressed_accounts
                .push(output_account);
        }

        Ok(self)
    }

    #[must_use = "with_new_addresses returns a new value"]
    pub fn with_new_addresses(
        mut self,
        new_address_params: &[light_compressed_account::instruction_data::data::NewAddressParamsPacked],
    ) -> Self {
        self.instruction_data = self.instruction_data.with_new_addresses(new_address_params);
        self
    }

    #[must_use = "with_input_compressed_accounts_with_merkle_context returns a new value"]
    pub fn with_input_compressed_accounts_with_merkle_context(
        mut self,
        input_compressed_accounts_with_merkle_context: &[light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext],
    ) -> Self {
        self.instruction_data = self
            .instruction_data
            .with_input_compressed_accounts_with_merkle_context(
                input_compressed_accounts_with_merkle_context,
            );
        self
    }

    #[must_use = "with_output_compressed_accounts returns a new value"]
    pub fn with_output_compressed_accounts(
        mut self,
        output_compressed_accounts: &[light_compressed_account::instruction_data::data::OutputCompressedAccountWithPackedContext],
    ) -> Self {
        self.instruction_data = self
            .instruction_data
            .with_output_compressed_accounts(output_compressed_accounts);
        self
    }

    #[must_use = "compress_lamports returns a new value"]
    pub fn compress_lamports(mut self, lamports: u64) -> Self {
        self.instruction_data = self.instruction_data.compress_lamports(lamports);
        self
    }

    #[must_use = "decompress_lamports returns a new value"]
    pub fn decompress_lamports(mut self, lamports: u64) -> Self {
        self.instruction_data = self.instruction_data.decompress_lamports(lamports);
        self
    }

    #[cfg(feature = "cpi-context")]
    #[must_use = "write_to_cpi_context_set returns a new value"]
    pub fn write_to_cpi_context_set(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_set();
        self
    }

    #[cfg(feature = "cpi-context")]
    #[must_use = "write_to_cpi_context_first returns a new value"]
    pub fn write_to_cpi_context_first(mut self) -> Self {
        self.instruction_data = self.instruction_data.write_to_cpi_context_first();
        self
    }

    #[cfg(feature = "cpi-context")]
    #[must_use = "with_cpi_context returns a new value"]
    pub fn with_cpi_context(
        mut self,
        cpi_context: light_compressed_account::instruction_data::cpi_context::CompressedCpiContext,
    ) -> Self {
        self.instruction_data = self.instruction_data.with_cpi_context(cpi_context);
        self
    }

    /// Invoke the Light system program via CPI.
    pub fn invoke(self, cpi_accounts: CpiAccounts<'_, '_>) -> Result<(), ProgramError> {
        use light_compressed_account::instruction_data::traits::LightInstructionData;

        let data = self
            .instruction_data
            .data()
            .map_err(LightSdkError::from)
            .map_err(ProgramError::from)?;

        let account_infos = cpi_accounts.to_account_infos();
        let config =
            CpiInstructionConfig::try_from(&cpi_accounts).map_err(ProgramError::from)?;
        let account_metas = get_account_metas_from_config(config);

        let instruction = solana_instruction::Instruction {
            program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
            accounts: account_metas,
            data,
        };
        let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[self.cpi_signer.bump]];
        solana_cpi::invoke_signed(&instruction, &account_infos, &[signer_seeds.as_slice()])
    }
}

// Manual BorshSerialize implementation that only serializes instruction_data
impl AnchorSerialize for LightSystemProgramCpi {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.instruction_data.serialize(writer)
    }
}
