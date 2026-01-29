use light_compressed_account::compressed_account::PackedMerkleContext;
use light_sdk_types::instruction::PackedStateTreeInfo;
use light_token_interface::instructions::{
    create_token_account::CreateTokenAccountInstructionData,
    extensions::{CompressToPubkey, CompressibleExtensionInstructionData},
};
pub use light_token_interface::{
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{
        extensions::{CompressedOnlyExtension, ExtensionStruct},
        AccountState, Token, TokenDataVersion,
    },
};
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

// Pack trait and PackedAccounts only available off-chain (client-side packing)
#[cfg(not(target_os = "solana"))]
use crate::{instruction::PackedAccounts, Pack};
use crate::{
    interface::{
        AccountType, DecompressCtx, LightAccountVariantTrait, PackedLightAccountVariantTrait,
        PackedTokenSeeds, UnpackedTokenSeeds,
    },
    AnchorDeserialize, AnchorSerialize, Unpack,
};

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct TokenDataWithSeeds<S> {
    pub seeds: S,
    pub token_data: Token,
}
#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct PackedTokenData {
    pub owner: u8,
    pub amount: u64,
    pub has_delegate: bool, // Optional delegate is set
    pub delegate: u8,
    pub mint: u8,
    pub version: u8,
}

#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TokenDataWithPackedSeeds<
    S: Unpack + AnchorSerialize + AnchorDeserialize + Clone + std::fmt::Debug,
> {
    pub seeds: S,
    pub token_data: PackedTokenData,
    pub extension: Option<CompressedOnlyExtensionInstructionData>,
}

#[cfg(not(target_os = "solana"))]
impl<S> Pack for TokenDataWithSeeds<S>
where
    S: Pack,
    S::Packed: Unpack + AnchorDeserialize + AnchorSerialize + Clone + std::fmt::Debug,
{
    type Packed = TokenDataWithPackedSeeds<S::Packed>;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError> {
        let seeds = self.seeds.pack(remaining_accounts)?;

        let owner_index = remaining_accounts
            .insert_or_get(Pubkey::new_from_array(self.token_data.owner.to_bytes()));

        let token_data = PackedTokenData {
            owner: owner_index,
            amount: self.token_data.amount,
            has_delegate: self.token_data.delegate.is_some(),
            delegate: self
                .token_data
                .delegate
                .map(|d| remaining_accounts.insert_or_get(Pubkey::new_from_array(d.to_bytes())))
                .unwrap_or(0),
            mint: remaining_accounts
                .insert_or_get(Pubkey::new_from_array(self.token_data.mint.to_bytes())),
            version: TokenDataVersion::ShaFlat as u8,
        };

        // Extract CompressedOnly extension from Token state if present.
        let extension = self.token_data.extensions.as_ref().and_then(|exts| {
            exts.iter().find_map(|ext| {
                if let ExtensionStruct::CompressedOnly(co) = ext {
                    Some(CompressedOnlyExtensionInstructionData {
                        delegated_amount: co.delegated_amount,
                        withheld_transfer_fee: co.withheld_transfer_fee,
                        is_frozen: self.token_data.state == AccountState::Frozen,
                        compression_index: 0,
                        is_ata: co.is_ata != 0,
                        bump: 0,
                        owner_index,
                    })
                } else {
                    None
                }
            })
        });

        Ok(TokenDataWithPackedSeeds {
            seeds,
            token_data,
            extension,
        })
    }
}

impl<S> Unpack for TokenDataWithPackedSeeds<S>
where
    S: Unpack + AnchorSerialize + AnchorDeserialize + Clone + std::fmt::Debug,
{
    type Unpacked = TokenDataWithSeeds<S::Unpacked>;

    fn unpack(&self, remaining_accounts: &[AccountInfo]) -> Result<Self::Unpacked, ProgramError> {
        let seeds = self.seeds.unpack(remaining_accounts)?;

        let owner_key = remaining_accounts
            .get(self.token_data.owner as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;
        let mint_key = remaining_accounts
            .get(self.token_data.mint as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;
        let delegate = if self.token_data.has_delegate {
            let delegate_key = remaining_accounts
                .get(self.token_data.delegate as usize)
                .ok_or(ProgramError::InvalidAccountData)?
                .key;
            Some(light_compressed_account::Pubkey::from(
                delegate_key.to_bytes(),
            ))
        } else {
            None
        };

        // Reconstruct extensions from instruction extension data.
        let extensions = self.extension.map(|ext| {
            vec![ExtensionStruct::CompressedOnly(CompressedOnlyExtension {
                delegated_amount: ext.delegated_amount,
                withheld_transfer_fee: ext.withheld_transfer_fee,
                is_ata: ext.is_ata as u8,
            })]
        });

        let state = self.extension.map_or(AccountState::Initialized, |ext| {
            if ext.is_frozen {
                AccountState::Frozen
            } else {
                AccountState::Initialized
            }
        });

        let delegated_amount = self.extension.map_or(0, |ext| ext.delegated_amount);

        let token_data = Token {
            mint: light_compressed_account::Pubkey::from(mint_key.to_bytes()),
            owner: light_compressed_account::Pubkey::from(owner_key.to_bytes()),
            amount: self.token_data.amount,
            delegate,
            state,
            is_native: None,
            delegated_amount,
            close_authority: None,
            account_type: TokenDataVersion::ShaFlat as u8,
            extensions,
        };

        Ok(TokenDataWithSeeds { seeds, token_data })
    }
}

// =============================================================================
// Blanket impls: LightAccountVariantTrait / PackedLightAccountVariantTrait
// for TokenDataWithSeeds<S> / TokenDataWithPackedSeeds<S>
// where S implements the seed-specific helper traits.
// =============================================================================

impl<const N: usize, S> LightAccountVariantTrait<N> for TokenDataWithSeeds<S>
where
    S: UnpackedTokenSeeds<N>,
    S::Packed: PackedTokenSeeds<N> + Unpack<Unpacked = S>,
{
    const PROGRAM_ID: Pubkey = S::PROGRAM_ID;
    type Seeds = S;
    type Data = Token;
    type Packed = TokenDataWithPackedSeeds<S::Packed>;

    fn data(&self) -> &Self::Data {
        &self.token_data
    }

    fn seed_vec(&self) -> Vec<Vec<u8>> {
        self.seeds.seed_vec()
    }

    fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; N] {
        self.seeds.seed_refs_with_bump(bump_storage)
    }
}

impl<const N: usize, S> PackedLightAccountVariantTrait<N> for TokenDataWithPackedSeeds<S>
where
    S: PackedTokenSeeds<N>,
    S::Unpacked: UnpackedTokenSeeds<N, Packed = S>,
{
    type Unpacked = TokenDataWithSeeds<S::Unpacked>;

    const ACCOUNT_TYPE: AccountType = AccountType::Token;

    fn bump(&self) -> u8 {
        self.seeds.bump()
    }

    fn unpack(&self, accounts: &[AccountInfo]) -> anchor_lang::Result<Self::Unpacked> {
        <Self as Unpack>::unpack(self, accounts).map_err(anchor_lang::error::Error::from)
    }

    fn seed_refs_with_bump<'a>(
        &'a self,
        accounts: &'a [AccountInfo],
        bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; N], ProgramError> {
        self.seeds.seed_refs_with_bump(accounts, bump_storage)
    }

    fn into_in_token_data(
        &self,
        tree_info: &PackedStateTreeInfo,
        output_queue_index: u8,
    ) -> anchor_lang::Result<MultiInputTokenDataWithContext> {
        Ok(MultiInputTokenDataWithContext {
            amount: self.token_data.amount,
            mint: self.token_data.mint,
            owner: self.token_data.owner,
            version: self.token_data.version,
            has_delegate: self.token_data.has_delegate,
            delegate: self.token_data.delegate,
            root_index: tree_info.root_index,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: output_queue_index,
                leaf_index: tree_info.leaf_index,
                prove_by_index: tree_info.prove_by_index,
            },
        })
    }

    fn into_in_tlv(&self) -> anchor_lang::Result<Option<Vec<ExtensionInstructionData>>> {
        Ok(self
            .extension
            .as_ref()
            .map(|ext| vec![ExtensionInstructionData::CompressedOnly(*ext)]))
    }
}

/// Build a CreateAssociatedTokenAccountIdempotent instruction for ATA decompression.
///
/// Creates a compressible ATA with compression_only mode (required for ATA decompression).
///
/// # Account order (per on-chain handler):
/// 0. owner (non-mut, non-signer) - The wallet owner
/// 1. mint (non-mut, non-signer) - The token mint
/// 2. fee_payer (signer, writable) - Pays for account creation
/// 3. associated_token_account (writable, NOT signer) - The ATA to create
/// 4. system_program (readonly) - System program
/// 5. compressible_config (readonly) - Compressible config PDA
/// 6. rent_payer (writable) - Rent sponsor account
///
/// # Arguments
/// * `wallet_owner` - The wallet owner (ATA derivation seed)
/// * `mint` - The token mint
/// * `fee_payer` - Pays for account creation
/// * `ata` - The ATA pubkey (derived from wallet_owner, program_id, mint)
/// * `bump` - The ATA derivation bump
/// * `compressible_config` - Compressible config PDA
/// * `rent_sponsor` - Rent sponsor account
/// * `write_top_up` - Lamports per write for top-up
#[allow(clippy::too_many_arguments)]
pub fn build_create_ata_instruction(
    wallet_owner: &Pubkey,
    mint: &Pubkey,
    fee_payer: &Pubkey,
    ata: &Pubkey,
    bump: u8,
    compressible_config: &Pubkey,
    rent_sponsor: &Pubkey,
    write_top_up: u32,
) -> Result<Instruction, ProgramError> {
    use light_token_interface::instructions::{
        create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
        extensions::CompressibleExtensionInstructionData,
    };

    let instruction_data = CreateAssociatedTokenAccountInstructionData {
        bump,
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: 3, // ShaFlat version (required)
            rent_payment: 16,         // 24h, TODO: make configurable
            compression_only: 1,      // Required for ATA
            write_top_up,
            compress_to_account_pubkey: None, // Required to be None for ATA
        }),
    };

    let mut data = Vec::new();
    data.push(102u8); // CreateAssociatedTokenAccountIdempotent discriminator
    instruction_data
        .serialize(&mut data)
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let accounts = vec![
        AccountMeta::new_readonly(*wallet_owner, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new(*ata, false), // NOT a signer - ATA is derived
        AccountMeta::new_readonly(Pubkey::default(), false), // system_program
        AccountMeta::new_readonly(*compressible_config, false),
        AccountMeta::new(*rent_sponsor, false),
    ];

    Ok(Instruction {
        program_id: light_token_interface::LIGHT_TOKEN_PROGRAM_ID.into(),
        accounts,
        data,
    })
}

/// Build a CreateTokenAccount instruction for decompression.
///
/// Creates a compressible token account with ShaFlat version (required by light token program).
///
/// # Account order:
/// 0. token_account (signer, writable) - The token account PDA to create
/// 1. mint (readonly) - The token mint
/// 2. fee_payer (signer, writable) - Pays for account creation
/// 3. compressible_config (readonly) - Compressible config PDA
/// 4. system_program (readonly) - System program
/// 5. rent_sponsor (writable) - Rent sponsor account
///
/// # Arguments
/// * `signer_seeds` - Seeds including bump for the token account PDA
/// * `program_id` - Program ID that owns the token account PDA
#[allow(clippy::too_many_arguments)]
pub fn build_create_token_account_instruction(
    token_account: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    fee_payer: &Pubkey,
    compressible_config: &Pubkey,
    rent_sponsor: &Pubkey,
    write_top_up: u32,
    signer_seeds: &[&[u8]],
    program_id: &Pubkey,
) -> Result<Instruction, ProgramError> {
    // Build CompressToPubkey from signer_seeds (last seed is bump)
    let bump = signer_seeds
        .last()
        .and_then(|s| s.first().copied())
        .ok_or(ProgramError::InvalidSeeds)?;
    let seeds_without_bump: Vec<Vec<u8>> = signer_seeds
        .iter()
        .take(signer_seeds.len().saturating_sub(1))
        .map(|s| s.to_vec())
        .collect();

    let compress_to_account_pubkey = CompressToPubkey {
        bump,
        program_id: program_id.to_bytes(),
        seeds: seeds_without_bump,
    };

    let instruction_data = CreateTokenAccountInstructionData {
        owner: light_compressed_account::Pubkey::from(owner.to_bytes()),
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: 3, // ShaFlat version (required)
            rent_payment: 16,         // 24h, TODO: make configurable
            compression_only: 0,      // Regular tokens can be transferred, not compression-only
            write_top_up,
            compress_to_account_pubkey: Some(compress_to_account_pubkey),
        }),
    };

    let mut data = Vec::new();
    data.push(18u8); // InitializeAccount3 opcode
    instruction_data
        .serialize(&mut data)
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    let accounts = vec![
        AccountMeta::new(*token_account, true),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*fee_payer, true),
        AccountMeta::new_readonly(*compressible_config, false),
        AccountMeta::new_readonly(Pubkey::default(), false), // system_program
        AccountMeta::new(*rent_sponsor, false),
    ];

    Ok(Instruction {
        program_id: light_token_interface::LIGHT_TOKEN_PROGRAM_ID.into(),
        accounts,
        data,
    })
}

pub fn prepare_token_account_for_decompression<'info, const SEED_COUNT: usize, P>(
    packed: &P,
    tree_info: &PackedStateTreeInfo,
    output_queue_index: u8,
    token_account_info: &AccountInfo<'info>,
    ctx: &mut DecompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>
where
    P: PackedLightAccountVariantTrait<SEED_COUNT>,
{
    let packed_accounts = ctx
        .cpi_accounts
        .packed_accounts()
        .map_err(|_| ProgramError::NotEnoughAccountKeys)?;
    let mut token_data = packed.into_in_token_data(tree_info, output_queue_index)?;

    // Get TLV extension early to detect ATA
    let in_tlv: Option<Vec<ExtensionInstructionData>> = packed.into_in_tlv()?;

    // Extract ATA info from TLV if present
    let ata_info = in_tlv.as_ref().and_then(|exts| {
        exts.iter().find_map(|ext| {
            if let ExtensionInstructionData::CompressedOnly(co) = ext {
                if co.is_ata {
                    Some((co.bump, co.owner_index))
                } else {
                    None
                }
            } else {
                None
            }
        })
    });

    // Resolve mint pubkey from packed index
    let mint_pubkey = packed_accounts
        .get(token_data.mint as usize)
        .ok_or(ProgramError::InvalidAccountData)?
        .key;

    let fee_payer = ctx.cpi_accounts.fee_payer();

    // Helper to check if token account is already initialized
    // State byte at offset 108: 0=Uninitialized, 1=Initialized, 2=Frozen
    const STATE_OFFSET: usize = 108;
    let is_already_initialized = !token_account_info.data_is_empty()
        && token_account_info.data_len() > STATE_OFFSET
        && token_account_info.try_borrow_data()?[STATE_OFFSET] != 0;

    if let Some((ata_bump, wallet_owner_index)) = ata_info {
        // ATA path: use invoke() without signer seeds
        // Resolve wallet owner pubkey from packed index
        let wallet_owner_pubkey = packed_accounts
            .get(wallet_owner_index as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;

        // Idempotency check: only create ATA if it doesn't exist
        // For ATAs, we still continue with decompression even if account exists
        if token_account_info.data_is_empty() {
            let instruction = build_create_ata_instruction(
                wallet_owner_pubkey,
                mint_pubkey,
                fee_payer.key,
                token_account_info.key,
                ata_bump,
                ctx.ctoken_compressible_config.key,
                ctx.ctoken_rent_sponsor.key,
                ctx.light_config.write_top_up,
            )?;

            // Invoke WITHOUT signer seeds - ATA is derived from light token program, not our program
            anchor_lang::solana_program::program::invoke(&instruction, ctx.remaining_accounts)?;
        }

        // For ATAs, the wallet owner must sign the Transfer2 instruction (not the ATA pubkey).
        // Override token_data.owner to point to the wallet owner index.
        token_data.owner = wallet_owner_index;

        // Don't extend token_seeds for ATAs (invoke, not invoke_signed)
    } else {
        // Regular token vault path: use invoke_signed with PDA seeds
        // For regular vaults, if already initialized, skip BOTH creation AND decompression (full idempotency)
        if is_already_initialized {
            solana_msg::msg!("Token vault is already decompressed, skipping");
            return Ok(());
        }

        let bump = &[packed.bump()];
        let seeds = packed
            .seed_refs_with_bump(packed_accounts, bump)
            .map_err(|_| ProgramError::InvalidSeeds)?;

        // Resolve owner pubkey from packed index
        let owner_pubkey = packed_accounts
            .get(token_data.owner as usize)
            .ok_or(ProgramError::InvalidAccountData)?
            .key;

        let signer_seeds: Vec<&[u8]> = seeds.iter().copied().collect();

        let instruction = build_create_token_account_instruction(
            token_account_info.key,
            mint_pubkey,
            owner_pubkey,
            fee_payer.key,
            ctx.ctoken_compressible_config.key,
            ctx.ctoken_rent_sponsor.key,
            ctx.light_config.write_top_up,
            &signer_seeds,
            ctx.program_id,
        )?;

        // Invoke with PDA seeds
        anchor_lang::solana_program::program::invoke_signed(
            &instruction,
            ctx.remaining_accounts,
            &[signer_seeds.as_slice()],
        )?;

        // Push seeds for the Transfer2 CPI (needed for invoke_signed)
        ctx.token_seeds.extend(seeds.iter().map(|s| s.to_vec()));
    }

    // Push token data for the Transfer2 CPI (common for both ATA and regular paths)
    ctx.in_token_data.push(token_data);

    // Push TLV data
    if let Some(ctx_in_tlv) = ctx.in_tlv.as_mut() {
        ctx_in_tlv.push(in_tlv.unwrap_or_default());
    } else if let Some(in_tlv) = in_tlv {
        let mut ctx_in_tlv = vec![];
        for _ in 0..ctx.in_token_data.len() - 1 {
            ctx_in_tlv.push(vec![]);
        }
        ctx_in_tlv.push(in_tlv);
        ctx.in_tlv = Some(ctx_in_tlv);
    }

    Ok(())
}
