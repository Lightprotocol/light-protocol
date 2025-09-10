use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::Rpc,
};
use light_compressed_token_sdk::{
    account2::CTokenAccount2,
    error::TokenSdkError,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
    token_pool::find_token_pool_pda_with_index,
};
use light_ctoken_types::{
    instructions::transfer2::MultiInputTokenDataWithContext, state::TokenDataVersion,
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::instruction::{PackedAccounts, PackedStateTreeInfo};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

pub fn pack_input_token_account(
    account: &CompressedTokenAccount,
    tree_info: &PackedStateTreeInfo,
    packed_accounts: &mut PackedAccounts,
    in_lamports: &mut Vec<u64>,
    is_delegate_transfer: bool, // Explicitly specify if delegate is signing
    token_data_version: TokenDataVersion,
) -> MultiInputTokenDataWithContext {
    // Check if account has a delegate
    let has_delegate = account.token.delegate.is_some();

    // Determine who should be the signer
    let owner_is_signer = !is_delegate_transfer;

    let delegate_index = if let Some(delegate) = account.token.delegate {
        // Delegate is signer only if this is explicitly a delegate transfer
        packed_accounts.insert_or_get_config(delegate, is_delegate_transfer, false)
    } else {
        0
    };

    if account.account.lamports != 0 {
        in_lamports.push(account.account.lamports);
    }

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
        owner: packed_accounts.insert_or_get_config(account.token.owner, owner_is_signer, false),
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
) -> Result<Instruction, TokenSdkError> {
    create_generic_transfer2_instruction(
        rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account,
            decompress_amount,
            solana_token_account,
            amount: decompress_amount,
        })],
        payer,
    )
    .await
}
#[derive(Debug, Clone, PartialEq)]
pub struct TransferInput<'a> {
    pub compressed_token_account: &'a [CompressedTokenAccount],
    pub to: Pubkey,
    pub amount: u64,
    pub is_delegate_transfer: bool, // Indicates if delegate is the signer
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecompressInput<'a> {
    pub compressed_token_account: &'a [CompressedTokenAccount],
    pub decompress_amount: u64,
    pub solana_token_account: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressInput<'a> {
    pub compressed_token_account: Option<&'a [CompressedTokenAccount]>,
    pub solana_token_account: Pubkey,
    pub to: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    pub output_queue: Pubkey,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressAndCloseInput {
    pub solana_ctoken_account: Pubkey,
    pub authority: Pubkey,
    pub output_queue: Pubkey,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApproveInput<'a> {
    pub compressed_token_account: &'a [CompressedTokenAccount],
    pub delegate: Pubkey,
    pub delegate_amount: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Transfer2InstructionType<'a> {
    Compress(CompressInput<'a>),
    Decompress(DecompressInput<'a>),
    Transfer(TransferInput<'a>),
    Approve(ApproveInput<'a>),
    CompressAndClose(CompressAndCloseInput),
}

// Note doesn't support multiple signers.
pub async fn create_generic_transfer2_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    actions: Vec<Transfer2InstructionType<'_>>,
    payer: Pubkey,
) -> Result<Instruction, TokenSdkError> {
    let mut hashes = Vec::new();
    actions.iter().for_each(|account| match account {
        Transfer2InstructionType::Compress(_) => {}
        Transfer2InstructionType::CompressAndClose(_) => {}
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
    let mut inputs_offset = 0;
    let mut in_lamports = Vec::new();
    let mut out_lamports = Vec::new();
    let mut token_accounts = Vec::new();
    for action in actions {
        match action {
            Transfer2InstructionType::Compress(input) => {
                let mut token_account =
                    if let Some(input_token_account) = input.compressed_token_account {
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
                                ))
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        inputs_offset += token_data.len();
                        CTokenAccount2::new(
                            token_data,
                            packed_tree_accounts.insert_or_get(input.output_queue),
                        )?
                    } else {
                        let output_queue = packed_tree_accounts.insert_or_get(input.output_queue);
                        CTokenAccount2::new_empty(
                            packed_tree_accounts.insert_or_get(input.to),
                            packed_tree_accounts.insert_or_get(input.mint),
                            output_queue,
                        )
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

                if source_account_owner.to_bytes() != COMPRESSED_TOKEN_PROGRAM_ID {
                    // For SPL compression, get mint first
                    let mint = input.mint;

                    // Add SPL Token 2022 program for SPL operations
                    let _token_program_index =
                        packed_tree_accounts.insert_or_get_read_only(spl_token_2022::ID);

                    // Add token pool account (index 0 for now, could be extended for multiple pools)
                    let pool_index = 0u8;
                    let (token_pool_pda, bump) = find_token_pool_pda_with_index(&mint, pool_index);
                    let pool_account_index = packed_tree_accounts.insert_or_get(token_pool_pda);

                    // Use the new SPL-specific compress method
                    token_account.compress_spl(
                        input.amount,
                        source_index,
                        authority_index,
                        pool_account_index,
                        pool_index,
                        bump,
                    )?;
                } else {
                    // Regular compression for compressed token accounts
                    token_account.compress(input.amount, source_index, authority_index)?;
                }
                token_accounts.push(token_account);
            }
            Transfer2InstructionType::Decompress(input) => {
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
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                let mut token_account = CTokenAccount2::new(
                    token_data,
                    packed_tree_infos
                        .state_trees
                        .as_ref()
                        .unwrap()
                        .output_tree_index,
                )?;
                // Add recipient SPL token account
                let recipient_index =
                    packed_tree_accounts.insert_or_get(input.solana_token_account);
                let recipient_account_owner = rpc
                    .get_account(input.solana_token_account)
                    .await
                    .unwrap()
                    .unwrap()
                    .owner;

                if recipient_account_owner.to_bytes() != COMPRESSED_TOKEN_PROGRAM_ID {
                    // For SPL decompression, get mint first
                    let mint = input.compressed_token_account[0].token.mint;

                    // Add SPL Token 2022 program for SPL operations
                    let _token_program_index =
                        packed_tree_accounts.insert_or_get_read_only(spl_token_2022::ID);

                    // Add token pool account (index 0 for now, could be extended for multiple pools)
                    let pool_index = 0u8;
                    let (token_pool_pda, bump) = find_token_pool_pda_with_index(&mint, pool_index);
                    let pool_account_index = packed_tree_accounts.insert_or_get(token_pool_pda);

                    // Use the new SPL-specific decompress method
                    token_account.decompress_spl(
                        input.decompress_amount,
                        recipient_index,
                        pool_account_index,
                        pool_index,
                        bump,
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
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                let mut token_account = if input.is_delegate_transfer {
                    CTokenAccount2::new_delegated(
                        token_data,
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .output_tree_index,
                    )
                } else {
                    CTokenAccount2::new(
                        token_data,
                        packed_tree_infos
                            .state_trees
                            .as_ref()
                            .unwrap()
                            .output_tree_index,
                    )
                }?;
                let recipient_index = packed_tree_accounts.insert_or_get(input.to);
                let recipient_token_account =
                    token_account.transfer(recipient_index, input.amount, None)?;

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
                        )
                    })
                    .collect::<Vec<_>>();
                inputs_offset += token_data.len();
                let mut token_account = CTokenAccount2::new(
                    token_data,
                    packed_tree_infos
                        .state_trees
                        .as_ref()
                        .unwrap()
                        .output_tree_index,
                )?;
                let delegate_index = packed_tree_accounts.insert_or_get(input.delegate);
                let delegated_token_account =
                    token_account.approve(delegate_index, input.delegate_amount, None)?;
                // all lamports stay with the owner
                out_lamports.push(
                    input
                        .compressed_token_account
                        .iter()
                        .map(|account| account.account.lamports)
                        .sum::<u64>(),
                );
                // For consistency add 0 lamports for the delegated account
                out_lamports.push(0);
                token_accounts.push(token_account);
                token_accounts.push(delegated_token_account);
            }
            Transfer2InstructionType::CompressAndClose(input) => {
                // Get token account info to extract mint, balance, owner, and rent_recipient
                let token_account_info = rpc
                    .get_account(input.solana_ctoken_account)
                    .await
                    .map_err(|_| TokenSdkError::InvalidAccountData)?
                    .ok_or(TokenSdkError::InvalidAccountData)?;

                // Parse the compressed token account using zero-copy deserialization
                use light_ctoken_types::state::{CompressedToken, ZExtensionStruct};
                use light_zero_copy::traits::ZeroCopyAt;
                let (compressed_token, _) = CompressedToken::zero_copy_at(&token_account_info.data)
                    .map_err(|_| TokenSdkError::InvalidAccountData)?;

                let mint = compressed_token.mint;
                let balance = compressed_token.amount;
                let owner = compressed_token.owner;

                // Extract rent_recipient from compressible extension
                let rent_recipient = if let Some(extensions) = compressed_token.extensions.as_ref()
                {
                    let mut found_rent_recipient = None;
                    for extension in extensions {
                        if let ZExtensionStruct::Compressible(compressible_ext) = extension {
                            found_rent_recipient = Some(compressible_ext.rent_recipient);
                            break;
                        }
                    }

                    found_rent_recipient.ok_or(TokenSdkError::InvalidAccountData)?
                } else {
                    return Err(TokenSdkError::InvalidAccountData);
                };
                let output_queue = packed_tree_accounts.insert_or_get(input.output_queue);

                let owner_index = packed_tree_accounts.insert_or_get((*owner).into());
                let mint_index = packed_tree_accounts.insert_or_get((*mint).into());
                let rent_recipient_index =
                    packed_tree_accounts.insert_or_get((*rent_recipient.unwrap()).into());

                // Create token account with the full balance
                let mut token_account =
                    CTokenAccount2::new_empty(owner_index, mint_index, output_queue);

                let source_index = packed_tree_accounts.insert_or_get(input.solana_ctoken_account);
                let authority_index =
                    packed_tree_accounts.insert_or_get_config(input.authority, true, false);

                // Use compress_and_close method with the actual balance
                // The compressed_account_index should match the position in token_accounts
                token_account.compress_and_close(
                    (*balance).into(),
                    source_index,
                    authority_index,
                    rent_recipient_index, // Use the extracted rent_recipient
                    token_accounts.len() as u8, // Index in the output array
                )?;

                token_accounts.push(token_account);
            }
        }
    }
    let packed_accounts = packed_tree_accounts.to_account_metas().0;
    let inputs = Transfer2Inputs {
        validity_proof: rpc_proof_result.proof,
        transfer_config: Transfer2Config::default(),
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
    };
    create_transfer2_instruction(inputs)
}
