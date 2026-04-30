use account_compression::{
    program::AccountCompression, state::emit_indexer_event,
    utils::constants::CPI_AUTHORITY_PDA_SEED,
};
use anchor_lang::prelude::*;
use light_compressed_account::{
    address::derive_address,
    compressed_account::{CompressedAccount, CompressedAccountData},
    hash_to_bn254_field_size_be,
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
        invoke_cpi::InstructionDataInvokeCpi,
    },
};
use light_hasher::{errors::HasherError, DataHasher, Poseidon};
use light_system_program::program::LightSystemProgram;

pub const SHIELDED_POOL_TX_EVENT_V1_DISCRIMINATOR: [u8; 8] =
    [b's', b'h', b'l', b'd', b'p', b'l', b'v', b'1'];
pub const SHIELDED_POOL_TX_EVENT_VERSION: u8 = 1;
pub const SHIELDED_UTXO_ACCOUNT_DISCRIMINATOR: [u8; 8] = *b"shldutx1";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ShieldedPoolTxKind {
    ProoflessShield = 0,
    Transact = 1,
    ZoneTransact = 2,
    ZoneAuthorityTransact = 3,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EncryptedTxEphemeralKeyRole {
    Auditor = 0,
    Sender = 1,
    Recipient = 2,
    ProtocolAuxiliary = 3,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct EncryptedTxEphemeralKey {
    pub role: EncryptedTxEphemeralKeyRole,
    pub key_id: u32,
    pub key_version: u32,
    pub hpke_ephemeral_pubkey: [u8; 32],
    pub encrypted_tx_ephemeral_key: Vec<u8>,
    pub auth_tag: [u8; 16],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct ShieldedPublicDelta {
    pub mint: Option<[u8; 32]>,
    pub spl_amount: i128,
    pub sol_amount: i128,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ShieldedUtxoOutputEvent {
    pub output_index: u8,
    pub compressed_output_index: u32,
    pub utxo_hash: [u8; 32],
    pub encrypted_utxo: Vec<u8>,
    pub encrypted_utxo_hash: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ShieldedPoolTxEvent {
    pub event_discriminator: [u8; 8],
    pub version: u8,
    pub tx_event_index: u32,
    pub instruction_tag: u8,
    pub tx_kind: ShieldedPoolTxKind,
    pub protocol_config: [u8; 32],
    pub zone_config_hash: Option<[u8; 32]>,
    pub tx_ephemeral_pubkey: [u8; 32],
    pub encrypted_tx_ephemeral_keys: Vec<EncryptedTxEphemeralKey>,
    pub operation_commitment: [u8; 32],
    pub public_input_hash: Option<[u8; 32]>,
    pub utxo_public_inputs_hash: Option<[u8; 32]>,
    pub tree_public_inputs_hash: Option<[u8; 32]>,
    pub nullifier_chain: Option<[u8; 32]>,
    pub input_nullifiers: Vec<[u8; 32]>,
    pub public_delta: ShieldedPublicDelta,
    pub relayer_fee: Option<u64>,
    pub outputs: Vec<ShieldedUtxoOutputEvent>,
}

impl ShieldedPoolTxEvent {
    pub fn matches_discriminator(data: &[u8]) -> bool {
        data.len() >= SHIELDED_POOL_TX_EVENT_V1_DISCRIMINATOR.len()
            && data[..SHIELDED_POOL_TX_EVENT_V1_DISCRIMINATOR.len()]
                == SHIELDED_POOL_TX_EVENT_V1_DISCRIMINATOR
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProoflessShieldedAppendArgs {
    pub zone_config_hash: [u8; 32],
    pub operation_commitment: [u8; 32],
    pub utxo_hash: [u8; 32],
    pub encrypted_utxo: Vec<u8>,
    pub encrypted_utxo_hash: [u8; 32],
}

pub fn process_create_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    data: [u8; 31],
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    bump: u8,
) -> Result<()> {
    let compressed_pda = create_compressed_pda_data(data, &ctx, &new_address_params)?;
    cpi_compressed_pda_transfer_as_program(
        &ctx,
        proof,
        new_address_params,
        compressed_pda,
        None,
        bump,
    )
}

pub fn process_proofless_shielded_append<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    args: ProoflessShieldedAppendArgs,
    bump: u8,
) -> Result<()> {
    let output = OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID.into(),
            lamports: 0,
            address: None,
            data: Some(CompressedAccountData {
                discriminator: SHIELDED_UTXO_ACCOUNT_DISCRIMINATOR,
                data: Vec::new(),
                data_hash: args.utxo_hash,
            }),
        },
        merkle_tree_index: 0,
    };
    cpi_compressed_account_append_as_program(ctx, output, bump)?;

    let event = ShieldedPoolTxEvent {
        event_discriminator: SHIELDED_POOL_TX_EVENT_V1_DISCRIMINATOR,
        version: SHIELDED_POOL_TX_EVENT_VERSION,
        tx_event_index: 0,
        instruction_tag: 0,
        tx_kind: ShieldedPoolTxKind::ProoflessShield,
        protocol_config: [0x42; 32],
        zone_config_hash: Some(args.zone_config_hash),
        tx_ephemeral_pubkey: [0x33; 32],
        encrypted_tx_ephemeral_keys: vec![EncryptedTxEphemeralKey {
            role: EncryptedTxEphemeralKeyRole::Auditor,
            key_id: 1,
            key_version: 1,
            hpke_ephemeral_pubkey: [0x44; 32],
            encrypted_tx_ephemeral_key: vec![0x55; 32],
            auth_tag: [0x66; 16],
        }],
        operation_commitment: args.operation_commitment,
        public_input_hash: None,
        utxo_public_inputs_hash: None,
        tree_public_inputs_hash: None,
        nullifier_chain: None,
        input_nullifiers: Vec::new(),
        public_delta: ShieldedPublicDelta::default(),
        relayer_fee: None,
        outputs: vec![ShieldedUtxoOutputEvent {
            output_index: 0,
            compressed_output_index: 0,
            utxo_hash: args.utxo_hash,
            encrypted_utxo: args.encrypted_utxo,
            encrypted_utxo_hash: args.encrypted_utxo_hash,
        }],
    };
    emit_indexer_event(event.try_to_vec()?, &ctx.accounts.noop_program)
}

fn cpi_compressed_pda_transfer_as_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    proof: Option<CompressedProof>,
    new_address_params: NewAddressParamsPacked,
    compressed_pda: OutputCompressedAccountWithPackedContext,
    cpi_context: Option<CompressedCpiContext>,
    bump: u8,
) -> Result<()> {
    let invoking_program = ctx.accounts.self_program.to_account_info();

    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_pda],
        proof,
        new_address_params: vec![new_address_params],
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context,
    };
    // defining seeds again so that the cpi doesn't fail we want to test the check in the compressed pda program
    let seeds: [&[u8]; 2] = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program,
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: None,
    };

    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)?;
    Ok(())
}

fn cpi_compressed_account_append_as_program<'info>(
    ctx: &Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
    compressed_account: OutputCompressedAccountWithPackedContext,
    bump: u8,
) -> Result<()> {
    let inputs_struct = InstructionDataInvokeCpi {
        relay_fee: None,
        input_compressed_accounts_with_merkle_context: Vec::new(),
        output_compressed_accounts: vec![compressed_account],
        proof: None,
        new_address_params: Vec::new(),
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: None,
    };
    let seeds: [&[u8]; 2] = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    let signer_seeds: [&[&[u8]]; 1] = [&seeds[..]];

    let cpi_accounts = light_system_program::cpi::accounts::InvokeCpiInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        invoking_program: ctx.accounts.self_program.to_account_info(),
        sol_pool_pda: None,
        decompression_recipient: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: None,
    };

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.light_system_program.to_account_info(),
        cpi_accounts,
        &signer_seeds,
    );
    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();

    let mut inputs = Vec::new();
    InstructionDataInvokeCpi::serialize(&inputs_struct, &mut inputs)?;
    light_system_program::cpi::invoke_cpi(cpi_ctx, inputs)
}

fn create_compressed_pda_data(
    data: [u8; 31],
    ctx: &Context<'_, '_, '_, '_, CreateCompressedPda<'_>>,
    new_address_params: &NewAddressParamsPacked,
) -> Result<OutputCompressedAccountWithPackedContext> {
    let timelock_compressed_pda = RegisteredUser {
        user_pubkey: *ctx.accounts.signer.key,
        data,
    };
    let compressed_account_data = CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: timelock_compressed_pda.try_to_vec().unwrap(),
        data_hash: timelock_compressed_pda.hash::<Poseidon>().unwrap(),
    };
    let mut discriminator_bytes = [0u8; 8];

    discriminator_bytes.copy_from_slice(
        &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
            .try_borrow_data()?[0..8],
    );
    let address = derive_address(
        &new_address_params.seed,
        &ctx.remaining_accounts[new_address_params.address_merkle_tree_account_index as usize]
            .key()
            .to_bytes(),
        &crate::ID.to_bytes(),
    );

    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account: CompressedAccount {
            owner: crate::ID.into(), // should be crate::ID, test can provide an invalid owner
            lamports: 0,
            address: Some(address),
            data: Some(compressed_account_data),
        },
        merkle_tree_index: 0,
    })
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct RegisteredUser {
    pub user_pubkey: Pubkey,
    pub data: [u8; 31],
}

impl light_hasher::DataHasher for RegisteredUser {
    fn hash<H: light_hasher::Hasher>(&self) -> std::result::Result<[u8; 32], HasherError> {
        let truncated_user_pubkey = hash_to_bn254_field_size_be(&self.user_pubkey.to_bytes());

        let mut data_bytes = [0u8; 32];
        data_bytes[1..].copy_from_slice(&self.data);
        H::hashv(&[truncated_user_pubkey.as_slice(), &data_bytes])
    }
}

#[derive(Accounts)]
pub struct CreateCompressedPda<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    pub noop_program: AccountInfo<'info>,
    pub self_program: Program<'info, crate::program::SystemCpiTest>,
    /// CHECK:
    pub cpi_signer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
