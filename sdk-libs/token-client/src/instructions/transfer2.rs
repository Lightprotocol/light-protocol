use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::Rpc,
};
use light_compressed_token_sdk::{
    compressed_token::{
        transfer2::{
            account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction,
            Transfer2Config, Transfer2Inputs,
        },
        CTokenAccount2,
    },
    spl_interface::find_spl_interface_pda_with_index,
};
use light_sdk::instruction::{PackedAccounts, PackedStateTreeInfo};
use light_token::error::TokenSdkError;
use light_token_interface::{
    instructions::{
        extensions::ExtensionInstructionData,
        transfer2::{MultiInputTokenDataWithContext, MultiTokenTransferOutputData},
    },
    state::TokenDataVersion,
    LIGHT_TOKEN_PROGRAM_ID,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

#[allow(clippy::too_many_arguments)]
pub fn pack_input_token_account(
    account: &CompressedTokenAccount,
    tree_info: &PackedStateTreeInfo,
    packed_accounts: &mut PackedAccounts,
    in_lamports: &mut Vec<u64>,
    is_delegate_transfer: bool, // Explicitly specify if delegate is signing
    token_data_version: TokenDataVersion,
    override_owner: Option<Pubkey>, // For is_ata: use destination Light Token owner instead
    is_ata: bool,                   // For ATA decompress: owner (ATA pubkey) is not a signer
) -> MultiInputTokenDataWithContext {
    // Check if account has a delegate
    let has_delegate = account.token.delegate.is_some();

    // Determine who should be the signer
    // For delegate transfers, the account MUST have a delegate set
    // If is_delegate_transfer is true but no delegate exists, owner must sign
    // For ATA decompress, the owner (ATA pubkey) cannot sign - wallet owner signs as fee payer
    let owner_is_signer = !is_ata && (!is_delegate_transfer || !has_delegate);

    let delegate_index = if let Some(delegate) = account.token.delegate {
        // Delegate is signer only if this is explicitly a delegate transfer
        packed_accounts.insert_or_get_config(delegate, is_delegate_transfer, false)
    } else {
        0
    };

    if account.account.lamports != 0 {
        in_lamports.push(account.account.lamports);
    }

    // For is_ata, use override_owner (wallet owner from destination Light Token)
    // For regular accounts, use the compressed account's owner
    let effective_owner = override_owner.unwrap_or(account.token.owner);

    MultiInputTokenDataWithContext {
        amount: account.token.amount,
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: tree_info.root_index,
        mint: packed_accounts.insert_or_get_read_only(account.token.mint),
        owner: packed_accounts.insert_or_get_config(effective_owner, owner_is_signer, false),
        has_delegate, // Indicates if account has a delegate set
        delegate: delegate_index,
        version: token_data_version as u8, // V2 for batched Merkle trees
    }
}

pub async fn create_decompress_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_token_account: &[CompressedTokenAccount],
    decompress_amount: u64,
    solana_token_account: Pubkey,
    payer: Pubkey,
    decimals: u8,
) -> Result<Instruction, TokenSdkError> {
    create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: compressed_token_account.to_vec(),
            decompress_amount,
            solana_token_account,
            amount: decompress_amount,
            pool_index: None,
            decimals,
            in_tlv: None,
        })],
        payer,
        false,
    )
    .await
}
#[derive(Debug, Clone, PartialEq)]
pub struct TransferInput {
    pub compressed_token_account: Vec<CompressedTokenAccount>,
    pub to: Pubkey,
    pub amount: u64,
    pub is_delegate_transfer: bool, // Indicates if delegate is the signer
    pub mint: Option<Pubkey>,       // Required when compressed_token_account is empty
    pub change_amount: Option<u64>, // Optional: explicitly set change amount to keep
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecompressInput {
    pub compressed_token_account: Vec<CompressedTokenAccount>,
    pub decompress_amount: u64,
    pub solana_token_account: Pubkey,
    pub amount: u64,
    pub pool_index: Option<u8>, // For SPL only. None = default (0), Some(n) = specific pool
    pub decimals: u8,           // Mint decimals for SPL transfer_checked
    /// TLV extensions for each input compressed account (required for version 3 accounts with extensions).
    pub in_tlv: Option<Vec<Vec<ExtensionInstructionData>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressInput {
    pub compressed_token_account: Option<Vec<CompressedTokenAccount>>,
    pub solana_token_account: Pubkey,
    pub to: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    pub output_queue: Pubkey,
    pub pool_index: Option<u8>, // For SPL only. None = default (0), Some(n) = specific pool
    pub decimals: u8,           // Mint decimals for SPL transfer_checked
    pub version: Option<TokenDataVersion>, // Optional: specify output version. None = ShaFlat (3)
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressAndCloseInput {
    pub solana_ctoken_account: Pubkey,
    pub authority: Pubkey,
    pub output_queue: Pubkey,
    pub destination: Option<Pubkey>,
    pub is_compressible: bool, // If true, account has extensions; if false, regular Light Token ATA
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApproveInput {
    pub compressed_token_account: Vec<CompressedTokenAccount>,
    pub delegate: Pubkey,
    pub delegate_amount: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Transfer2InstructionType {
    Compress(CompressInput),
    Decompress(DecompressInput),
    Transfer(TransferInput),
    Approve(ApproveInput),
    CompressAndClose(CompressAndCloseInput),
}

// Note doesn't support multiple signers.
pub async fn create_generic_transfer2_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    actions: Vec<Transfer2InstructionType>,
    payer: Pubkey,
    should_filter_zero_outputs: bool,
) -> Result<Instruction, TokenSdkError> {
    // // Get a single shared output queue for ALL compress/compress-and-close operations
    // // This prevents reordering issues caused by the sort_by_key at the end
    // let shared_output_queue = rpc
    //     .get_random_state_tree_info()
    //     .unwrap()
    //     .get_output_pubkey()
    //     .unwrap();

    let mut hashes = Vec::new();
    actions.iter().for_each(|account| match account {
        Transfer2InstructionType::Compress(input) => {
            // Also collect hashes from compressed inputs if present
            if let Some(ref compressed_accounts) = input.compressed_token_account {
                compressed_accounts
                    .iter()
                    .for_each(|account| hashes.push(account.account.hash));
            }
        }
        Transfer2InstructionType::CompressAndClose(_) => {
            // CompressAndClose doesn't have compressed inputs, only Solana Light Token account
        }
        Transfer2InstructionType::Decompress(input) => input
            .compressed_token_account
            .iter()
            .for_each(|account| hashes.push(account.account.hash)),
        Transfer2InstructionType::Transfer(input) => input
            .compressed_token_account
            .iter()
            .for_each(|account| hashes.push(account.account.hash)),
        Transfer2InstructionType::Approve(input) => input
            .compressed_token_account
            .iter()
            .for_each(|account| hashes.push(account.account.hash)),
    });
    let rpc_proof_result = rpc
        .get_validity_proof(hashes, vec![], None)
        .await
        .unwrap()
        .value;

    let mut packed_tree_accounts = PackedAccounts::default();
    // tree infos must be packed before packing the token input accounts
    let packed_tree_infos = rpc_proof_result.pack_tree_infos(&mut packed_tree_accounts);

    // We use a single shared output queue for all compress/compress-and-close operations to avoid ordering failures.
    let shared_output_queue = if packed_tree_infos.address_trees.is_empty() {
        let shared_output_queue = rpc
            .get_random_state_tree_info()
            .unwrap()
            .get_output_pubkey()
            .unwrap();
        packed_tree_accounts.insert_or_get(shared_output_queue)
    } else {
        packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .output_tree_index
    };

    let mut inputs_offset = 0;
    let mut in_lamports = Vec::new();
    let mut out_lamports = Vec::new();
    let mut token_accounts = Vec::new();
    let mut collected_in_tlv: Vec<Vec<ExtensionInstructionData>> = Vec::new();
    let mut has_any_tlv = false;
    for action in actions {
        match action {
            Transfer2InstructionType::Compress(input) => {
                let mut token_account =
                    if let Some(ref input_token_account) = input.compressed_token_account {
                        let token_data = input_token_account
                            .iter()
                            .zip(
                                packed_tree_infos
                                    .state_trees
                                    .as_ref()
                                    .unwrap()
                                    .packed_tree_infos[inputs_offset..]
                                    .iter(),
                            )
                            .map(|(account, rpc_account)| {
                                if input.to != account.token.owner {
                                    return Err(TokenSdkError::InvalidCompressInputOwner);
                                }
                                Ok(pack_input_token_account(
                                    account,
                                    rpc_account,
                                    &mut packed_tree_accounts,
                                    &mut in_lamports,
                                    false, // Compress is always owner-signed
                                    TokenDataVersion::from_discriminator(
                                        account.account.data.as_ref().unwrap().discriminator,
                                    )
                                    .unwrap(),
                                    None,  // No override for compress
                                    false, // Not an ATA decompress
                                ))
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        inputs_offset += token_data.len();
                        CTokenAccount2::new(token_data)?
                    } else {
                        let owner_index = packed_tree_accounts.insert_or_get(input.to);
                        let mint_index = packed_tree_accounts.insert_or_get(input.mint);
                        let version = input.version.unwrap_or(TokenDataVersion::ShaFlat) as u8;
                        CTokenAccount2 {
                            inputs: vec![],
                            output: MultiTokenTransferOutputData {
                                owner: owner_index,
                                amount: 0,
                                delegate: 0,
                                mint: mint_index,
                                version,
                                has_delegate: false,
                            },
                            compression: None,
                            delegate_is_set: false,
                            method_used: false,
                        }
                    };

                let source_index = packed_tree_accounts.insert_or_get(input.solana_token_account);
                let authority_index =
                    packed_tree_accounts.insert_or_get_config(input.authority, true, false);

                // Check if source account is an SPL token account
                let source_account_owner = rpc
                    .get_account(input.solana_token_account)
                    .await
                    .unwrap()
                    .unwrap()
                    .owner;

                if source_account_owner.to_bytes() != LIGHT_TOKEN_PROGRAM_ID {
                    // For SPL compression, get mint first
                    let mint = input.mint;

                    // Add the SPL Token program that owns the account
                    let _token_program_index =
                        packed_tree_accounts.insert_or_get_read_only(source_account_owner);

                    // Use pool_index from input, default to 0
                    let pool_index = input.pool_index.unwrap_or(0);
                    let (spl_interface_pda, bump) =
                        find_spl_interface_pda_with_index(&mint, pool_index, false);
                    let pool_account_index = packed_tree_accounts.insert_or_get(spl_interface_pda);

                    // Use the new SPL-specific compress method
                    token_account.compress_spl(
                        input.amount,
                        source_index,
                        authority_index,
                        pool_account_index,
                        pool_index,
                        bump,
                        input.decimals,
                    )?;
                } else {
                    // Regular compression for compressed token accounts
                    token_account.compress(input.amount, source_index, authority_index)?;
                }
                token_accounts.push(token_account);
            }
            Transfer2InstructionType::Decompress(input) => {
                // Collect in_tlv data if provided
                if let Some(ref tlv_data) = input.in_tlv {
                    has_any_tlv = true;
                    collected_in_tlv.extend(tlv_data.iter().cloned());
                } else {
                    // Add empty TLV entries for each input (needed for proper indexing)
                    for _ in 0..input.compressed_token_account.len() {
                        collected_in_tlv.push(Vec::new());
                    }
                }

                // Check if any input has is_ata=true in the TLV
                // If so, we need to use the destination Light Token's owner as the signer
                let is_ata = input.in_tlv.as_ref().is_some_and(|tlv| {
                    tlv.iter().flatten().any(|ext| {
                        matches!(ext, ExtensionInstructionData::CompressedOnly(data) if data.is_ata)
                    })
                });

                // Add recipient account and get account info
                let recipient_index =
                    packed_tree_accounts.insert_or_get(input.solana_token_account);
                let recipient_account = rpc
                    .get_account(input.solana_token_account)
                    .await
                    .unwrap()
                    .unwrap();
                let recipient_account_owner = recipient_account.owner;

                // For is_ata, the compressed account owner is the ATA pubkey (stored during compress_and_close)
                // We keep that for hash calculation. The wallet owner signs instead of ATA pubkey.
                // Get the wallet owner from the destination Light Token account and add as signer.
                if is_ata && recipient_account_owner.to_bytes() == LIGHT_TOKEN_PROGRAM_ID {
                    // Deserialize Token to get wallet owner
                    use borsh::BorshDeserialize;
                    use light_token_interface::state::Token;
                    if let Ok(ctoken) = Token::deserialize(&mut &recipient_account.data[..]) {
                        let wallet_owner = Pubkey::from(ctoken.owner.to_bytes());
                        // Add wallet owner as signer and get its index
                        let wallet_owner_index =
                            packed_tree_accounts.insert_or_get_config(wallet_owner, true, false);
                        // Update the owner_index in collected_in_tlv for CompressedOnly extensions
                        for tlv in collected_in_tlv.iter_mut() {
                            for ext in tlv.iter_mut() {
                                if let ExtensionInstructionData::CompressedOnly(data) = ext {
                                    if data.is_ata {
                                        data.owner_index = wallet_owner_index;
                                    }
                                }
                            }
                        }
                    }
                }

                let token_data = input
                    .compressed_token_account
                    .iter()
                    .zip(
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .packed_tree_infos[inputs_offset..]
                            .iter(),
                    )
                    .map(|(account, rpc_account)| {
                        pack_input_token_account(
                            account,
                            rpc_account,
                            &mut packed_tree_accounts,
                            &mut in_lamports,
                            false, // Decompress is always owner-signed
                            TokenDataVersion::from_discriminator(
                                account.account.data.as_ref().unwrap().discriminator,
                            )
                            .unwrap(),
                            None,   // No override - use stored owner (ATA pubkey for is_ata)
                            is_ata, // For ATA: owner (ATA pubkey) is not signer
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                let mut token_account = CTokenAccount2::new(token_data)?;

                if recipient_account_owner.to_bytes() != LIGHT_TOKEN_PROGRAM_ID {
                    // For SPL decompression, get mint first
                    let mint = input.compressed_token_account[0].token.mint;

                    // Add the SPL Token program that owns the account
                    let _token_program_index =
                        packed_tree_accounts.insert_or_get_read_only(recipient_account_owner);

                    // Use pool_index from input, default to 0
                    let pool_index = input.pool_index.unwrap_or(0);
                    let (spl_interface_pda, bump) =
                        find_spl_interface_pda_with_index(&mint, pool_index, false);
                    let pool_account_index = packed_tree_accounts.insert_or_get(spl_interface_pda);

                    // Use the new SPL-specific decompress method
                    token_account.decompress_spl(
                        input.decompress_amount,
                        recipient_index,
                        pool_account_index,
                        pool_index,
                        bump,
                        input.decimals,
                    )?;
                } else {
                    // Use the new SPL-specific decompress method
                    token_account.decompress(input.decompress_amount, recipient_index)?;
                }

                out_lamports.push(
                    input
                        .compressed_token_account
                        .iter()
                        .map(|account| account.account.lamports)
                        .sum::<u64>(),
                );

                token_accounts.push(token_account);
            }
            Transfer2InstructionType::Transfer(input) => {
                let token_data = input
                    .compressed_token_account
                    .iter()
                    .zip(
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .packed_tree_infos[inputs_offset..]
                            .iter(),
                    )
                    .map(|(account, rpc_account)| {
                        pack_input_token_account(
                            account,
                            rpc_account,
                            &mut packed_tree_accounts,
                            &mut in_lamports,
                            input.is_delegate_transfer, // Use the flag from TransferInput
                            TokenDataVersion::from_discriminator(
                                account.account.data.as_ref().unwrap().discriminator,
                            )
                            .unwrap(),
                            None,  // No override for transfer
                            false, // Not an ATA decompress
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                if token_data.is_empty() {
                    // When no input accounts, create recipient account directly
                    // This requires mint to be specified in the input
                    let mint = input.mint.ok_or(TokenSdkError::InvalidAccountData)?;

                    let recipient_index = packed_tree_accounts.insert_or_get(input.to);
                    let mint_index = packed_tree_accounts.insert_or_get_read_only(mint);

                    let recipient_token_account = CTokenAccount2 {
                        inputs: vec![],
                        output: MultiTokenTransferOutputData {
                            owner: recipient_index,
                            amount: input.amount,
                            has_delegate: false,
                            delegate: 0,
                            mint: mint_index,
                            version: TokenDataVersion::V2 as u8, // Default to V2
                        },
                        compression: None,
                        delegate_is_set: false,
                        method_used: true, // Mark that this account was used/created
                    };

                    out_lamports.push(0);
                    token_accounts.push(recipient_token_account);
                } else {
                    // Only use new_delegated if the input accounts actually have delegates
                    let has_delegates = token_data.iter().any(|data| data.has_delegate);
                    let mut token_account = if input.is_delegate_transfer && has_delegates {
                        CTokenAccount2::new_delegated(token_data)
                    } else {
                        CTokenAccount2::new(token_data)
                    }?;
                    let recipient_index = packed_tree_accounts.insert_or_get(input.to);
                    let recipient_token_account =
                        token_account.transfer(recipient_index, input.amount)?;
                    if let Some(amount) = input.change_amount {
                        token_account.output.amount = amount;
                    }
                    // all lamports go to the sender.
                    out_lamports.push(
                        input
                            .compressed_token_account
                            .iter()
                            .map(|account| account.account.lamports)
                            .sum::<u64>(),
                    );
                    // For consistency add 0 lamports for the recipient.
                    out_lamports.push(0);
                    token_accounts.push(token_account);
                    token_accounts.push(recipient_token_account);
                }
            }
            Transfer2InstructionType::Approve(input) => {
                let token_data = input
                    .compressed_token_account
                    .iter()
                    .zip(
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .packed_tree_infos[inputs_offset..]
                            .iter(),
                    )
                    .map(|(account, rpc_account)| {
                        pack_input_token_account(
                            account,
                            rpc_account,
                            &mut packed_tree_accounts,
                            &mut in_lamports,
                            false, // Approve is always owner-signed
                            TokenDataVersion::from_discriminator(
                                account.account.data.as_ref().unwrap().discriminator,
                            )
                            .unwrap(),
                            None,  // No override for approve
                            false, // Not an ATA decompress
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                let mut token_account = CTokenAccount2::new(token_data)?;
                let delegate_index = packed_tree_accounts.insert_or_get(input.delegate);
                let delegated_token_account =
                    token_account.approve(delegate_index, input.delegate_amount)?;
                // all lamports stay with the owner
                out_lamports.push(
                    input
                        .compressed_token_account
                        .iter()
                        .map(|account| account.account.lamports)
                        .sum::<u64>(),
                );
                out_lamports.push(0);
                // For consistency add 0 lamports for the delegated account
                token_accounts.push(token_account);
                token_accounts.push(delegated_token_account);
            }
            Transfer2InstructionType::CompressAndClose(input) => {
                // Get token account info to extract mint, balance, owner, and rent_sponsor
                let token_account_info = rpc
                    .get_account(input.solana_ctoken_account)
                    .await
                    .map_err(|_| TokenSdkError::InvalidAccountData)?
                    .ok_or(TokenSdkError::InvalidAccountData)?;

                // Parse the compressed token account using zero-copy deserialization
                use light_token_interface::state::Token;
                use light_zero_copy::traits::ZeroCopyAt;
                let (compressed_token, _) = Token::zero_copy_at(&token_account_info.data)
                    .map_err(|_| TokenSdkError::InvalidAccountData)?;
                let mint = compressed_token.mint;
                let balance: u64 = compressed_token.amount.into();
                let owner = compressed_token.owner;

                // Extract rent_sponsor, compression_authority, and compress_to_pubkey from Compressible extension
                let compressible_ext = compressed_token
                    .get_compressible_extension()
                    .ok_or(TokenSdkError::MissingCompressibleExtension)?;
                let rent_sponsor = compressible_ext.info.rent_sponsor;
                let _compression_authority = compressible_ext.info.compression_authority;
                let compress_to_pubkey = compressible_ext.info.compress_to_pubkey == 1;

                // Add source account first (it's being closed, so needs to be writable)
                let source_index = packed_tree_accounts.insert_or_get(input.solana_ctoken_account);

                // Determine the owner index for the compressed output
                // If compress_to_pubkey is true, reuse source_index; otherwise add original owner
                let owner_index = if compress_to_pubkey {
                    source_index // Reuse the source account index as owner
                } else {
                    packed_tree_accounts.insert_or_get(Pubkey::from(owner.to_bytes()))
                };

                let mint_index =
                    packed_tree_accounts.insert_or_get_read_only(Pubkey::from(mint.to_bytes()));
                let rent_sponsor_index =
                    packed_tree_accounts.insert_or_get(Pubkey::from(rent_sponsor));

                // Create token account with the full balance
                let mut token_account = CTokenAccount2::new_empty(owner_index, mint_index);
                // Authority needs to be writable if it's also the destination (receives lamports from close)
                let authority_needs_writable = input.destination.is_none();
                let authority_index = packed_tree_accounts.insert_or_get_config(
                    input.authority,
                    true,
                    authority_needs_writable,
                );

                // Use compress_and_close method with the actual balance
                // The compressed_account_index should match the position in token_accounts
                // Destination always receives the compression incentive (11k lamports)
                let destination_index = input
                    .destination
                    .map(|d| packed_tree_accounts.insert_or_get(d))
                    .unwrap_or(authority_index); // Default to authority if no destination specified

                token_account.compress_and_close(
                    balance,
                    source_index,
                    authority_index,
                    rent_sponsor_index,         // Use the extracted rent_sponsor
                    token_accounts.len() as u8, // Index in the output array
                    destination_index,
                )?;

                token_accounts.push(token_account);
            }
        }
    }

    // // Sort token accounts by merkle_tree index to ensure OutputMerkleTreeIndicesNotInOrder error doesn't occur
    // // The system program requires output merkle tree indices to be in ascending order
    // token_accounts.sort_by_key(|account| account.output.merkle_tree);
    let transfer_config = if should_filter_zero_outputs {
        Transfer2Config::default().filter_zero_amount_outputs()
    } else {
        Transfer2Config::default()
    };
    let packed_accounts = packed_tree_accounts.to_account_metas().0;
    let inputs = Transfer2Inputs {
        validity_proof: rpc_proof_result.proof,
        transfer_config,
        meta_config: Transfer2AccountsMetaConfig {
            fee_payer: Some(payer),
            packed_accounts: Some(packed_accounts),
            ..Default::default()
        },
        in_lamports: if in_lamports.is_empty() {
            None
        } else {
            Some(in_lamports)
        },
        out_lamports: if out_lamports.iter().all(|lamports| *lamports == 0) {
            None
        } else {
            Some(out_lamports)
        },
        token_accounts,
        output_queue: shared_output_queue,
        in_tlv: if has_any_tlv {
            Some(collected_in_tlv)
        } else {
            None
        },
    };
    create_transfer2_instruction(inputs)
}
