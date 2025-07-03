use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{DataHasher, Hasher};
use light_sdk::{
    account::LightAccount,
    cpi::{CpiAccounts, CpiAccountsConfig, CpiInputs},
    error::LightSdkError,
    instruction::{
        account_meta::{CompressedAccountMeta, CompressedAccountMetaTrait},
        ValidityProof,
    },
    LightDiscriminator, LightHasher,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program::invoke_signed, pubkey::Pubkey,
    rent::Rent, system_instruction, sysvar::Sysvar,
};

pub const SLOTS_UNTIL_COMPRESSION: u64 = 100;

/// Account structure for the decompressed PDA
#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressedPdaAccount {
    /// The compressed account address this PDA was derived from
    pub compressed_address: [u8; 32],
    /// Slot when this account was last written
    pub last_written_slot: u64,
    /// Number of slots until this account can be compressed again
    pub slots_until_compression: u64,
    /// The actual account data
    pub data: [u8; 31],
    /// Flag to indicate if this is a decompressed account
    pub is_decompressed: bool,
}

/// Compressed account structure with decompression flag
#[derive(
    Clone, Debug, Default, LightHasher, LightDiscriminator, BorshDeserialize, BorshSerialize,
)]
pub struct DecompressedMarkerAccount {
    /// Flag to indicate this account has been decompressed
    pub is_decompressed: bool,
}

/// Decompresses a compressed account into a PDA
/// The PDA is derived from the compressed account's address and other seeds
pub fn decompress_to_pda(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    msg!("Decompressing compressed account to PDA");

    let mut instruction_data = instruction_data;
    let instruction_data = DecompressToPdaInstructionData::deserialize(&mut instruction_data)
        .map_err(|_| LightSdkError::Borsh)?;

    // Get accounts
    let fee_payer = &accounts[0];
    let pda_account = &accounts[1];
    let rent_payer = &accounts[2]; // Account that pays for PDA rent
    let system_program = &accounts[3];

    // Derive PDA from compressed address
    let compressed_address = instruction_data.compressed_account.meta.address;
    let (pda_pubkey, pda_bump) = Pubkey::find_program_address(
        &[
            b"decompressed_pda",
            &compressed_address,
            &instruction_data.additional_seed,
        ],
        &crate::ID,
    );

    // Verify PDA matches
    if pda_pubkey != *pda_account.key {
        msg!("Invalid PDA pubkey");
        return Err(LightSdkError::ConstraintViolation);
    }

    // Get current slot
    let clock = Clock::get().map_err(|_| LightSdkError::Borsh)?;
    let current_slot = clock.slot;

    // Calculate space needed for PDA
    let space = std::mem::size_of::<DecompressedPdaAccount>() + 8; // +8 for discriminator

    // Get minimum rent
    let rent = Rent::get().map_err(|_| LightSdkError::Borsh)?;
    let minimum_balance = rent.minimum_balance(space);

    // Create PDA account (rent payer pays for the PDA creation)
    let create_account_ix = system_instruction::create_account(
        rent_payer.key,
        pda_account.key,
        minimum_balance,
        space as u64,
        &crate::ID,
    );

    let signer_seeds = &[
        b"decompressed_pda".as_ref(),
        compressed_address.as_ref(),
        instruction_data.additional_seed.as_ref(),
        &[pda_bump],
    ];

    invoke_signed(
        &create_account_ix,
        &[
            rent_payer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        &[signer_seeds],
    )?;

    // Initialize PDA with decompressed data
    let decompressed_pda = DecompressedPdaAccount {
        compressed_address,
        last_written_slot: current_slot,
        slots_until_compression: SLOTS_UNTIL_COMPRESSION,
        data: instruction_data.compressed_account.data,
        is_decompressed: true,
    };

    // Write data to PDA
    decompressed_pda
        .serialize(&mut &mut pda_account.try_borrow_mut_data()?[8..])
        .map_err(|_| LightSdkError::Borsh)?;

    // Write discriminator
    pda_account.try_borrow_mut_data()?[..8].copy_from_slice(b"decomppd");

    // Now handle the compressed account side
    // Create a marker account that indicates this compressed account has been decompressed
    let marker_account = LightAccount::<'_, DecompressedMarkerAccount>::new_mut(
        &crate::ID,
        &instruction_data.compressed_account.meta,
        DecompressedMarkerAccount {
            is_decompressed: true,
        },
    )?;

    // Set up CPI accounts for light system program
    let mut config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
    config.sol_pool_pda = false;
    config.sol_compression_recipient = true; // We need to decompress SOL to the PDA

    let cpi_accounts = CpiAccounts::new_with_config(
        fee_payer,
        &accounts[instruction_data.system_accounts_offset as usize..],
        config,
    );

    // Create CPI inputs with decompression
    let mut cpi_inputs = CpiInputs::new(
        instruction_data.proof,
        vec![marker_account.to_account_info()?],
    );

    // Set decompression parameters
    // Transfer all lamports from compressed account to the PDA
    let lamports_to_decompress = instruction_data
        .compressed_account
        .meta
        .get_lamports()
        .unwrap_or(0);

    cpi_inputs.compress_or_decompress_lamports = Some(lamports_to_decompress);
    cpi_inputs.is_compress = false; // This is decompression

    // Invoke light system program
    cpi_inputs.invoke_light_system_program(cpi_accounts)?;

    msg!("Successfully decompressed account to PDA");
    Ok(())
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressToPdaInstructionData {
    pub proof: ValidityProof,
    pub compressed_account: DecompressMyCompressedAccount,
    pub additional_seed: [u8; 32], // Additional seed for PDA derivation
    pub system_accounts_offset: u8,
}

#[derive(Clone, Debug, Default, BorshDeserialize, BorshSerialize)]
pub struct DecompressMyCompressedAccount {
    pub meta: CompressedAccountMeta,
    pub data: [u8; 31],
}

// Implement required traits for DecompressedPdaAccount
impl DataHasher for DecompressedPdaAccount {
    fn hash<H: Hasher>(&self) -> Result<[u8; 32], light_hasher::HasherError> {
        let mut bytes = vec![];
        self.serialize(&mut bytes).unwrap();
        H::hashv(&[&bytes])
    }
}

impl LightDiscriminator for DecompressedPdaAccount {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0xDE, 0xC0, 0x11, 0x9D, 0xA0, 0x00, 0x00, 0x00];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] =
        &[0xDE, 0xC0, 0x11, 0x9D, 0xA0, 0x00, 0x00, 0x00];
}

impl crate::sdk::compress_pda::PdaTimingData for DecompressedPdaAccount {
    fn last_touched_slot(&self) -> u64 {
        self.last_written_slot
    }

    fn slots_buffer(&self) -> u64 {
        self.slots_until_compression
    }
}
